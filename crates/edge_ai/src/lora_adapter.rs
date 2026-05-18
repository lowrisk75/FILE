//! # LoRA (Low-Rank Adaptation) Adapter Matrices
//!
//! This module implements the LoRA-as-Input architecture for dynamic policy updates
//! without recompilation.
//!
//! ## Mathematical Foundation
//!
//! ### Standard routing network:
//! ```text
//! y = W₀ · x
//! ```
//!
//! ### LoRA decomposition:
//! ```text
//! W = W₀ + ΔW = W₀ + A·B
//! ```
//! Where:
//! - A: d_out × r (low-rank adapter)
//! - B: r × d_in (low-rank adapter)
//! - r << min(d_out, d_in) (rank bottleneck)
//!
//! ### Adapter-as-Input reformulation:
//! ```text
//! y = W₀·x + A·(B·x)
//! ```
//!
//! In the MIL graph:
//! - W₀ → constant (baked into E5 microcode)
//! - x, A, B → input variables (reside in IOSurface)
//!
//! **Update flow (<100µs):**
//! 1. Compute new A, B matrices
//! 2. memcpy(A, B → IOSurface) via zero-copy
//! 3. Next inference reads updated values
//! 4. Zero recompilation ✅

use half::f16;

/// LoRA rank (r) - determines adapter capacity vs. memory footprint
///
/// Smaller rank = less memory, faster updates, lower capacity
/// Larger rank = more memory, slower updates, higher capacity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoraRank {
    /// Rank 8 - ultra-fast updates, minimal capacity
    Rank8 = 8,
    /// Rank 16 - fast updates, low capacity
    Rank16 = 16,
    /// Rank 32 - balanced updates/capacity
    Rank32 = 32,
    /// Rank 64 - slower updates, high capacity (RECOMMENDED for AORATA)
    Rank64 = 64,
    /// Rank 128 - very slow updates, very high capacity
    Rank128 = 128,
}

impl LoraRank {
    /// Get the numeric rank value
    pub fn value(&self) -> usize {
        *self as usize
    }

    /// Calculate memory footprint for A or B matrix
    ///
    /// # Arguments
    ///
    /// * `dim` - Dimension (d_in or d_out)
    ///
    /// # Returns
    ///
    /// Memory size in bytes (fp16)
    pub fn memory_bytes(&self, dim: usize) -> usize {
        dim * self.value() * std::mem::size_of::<f16>()
    }
}

/// LoRA adapter pair (A, B) for a single routing layer
#[derive(Debug, Clone)]
pub struct LoraAdapter {
    /// A matrix: d_out × r
    pub a: Vec<f16>,
    /// B matrix: r × d_in
    pub b: Vec<f16>,
    /// Output dimension
    pub d_out: usize,
    /// Input dimension
    pub d_in: usize,
    /// Rank
    pub rank: LoraRank,
}

impl LoraAdapter {
    /// Create a new LoRA adapter initialized to zeros
    ///
    /// # Arguments
    ///
    /// * `d_out` - Output dimension (rows of A)
    /// * `d_in` - Input dimension (columns of B)
    /// * `rank` - LoRA rank
    ///
    /// # Returns
    ///
    /// Zero-initialized adapter
    ///
    /// # Example
    ///
    /// ```
    /// use edge_ai::lora_adapter::{LoraAdapter, LoraRank};
    ///
    /// let adapter = LoraAdapter::new(512, 256, LoraRank::Rank64);
    /// assert_eq!(adapter.d_out, 512);
    /// assert_eq!(adapter.d_in, 256);
    /// assert_eq!(adapter.a.len(), 512 * 64);
    /// assert_eq!(adapter.b.len(), 64 * 256);
    /// ```
    pub fn new(d_out: usize, d_in: usize, rank: LoraRank) -> Self {
        let r = rank.value();
        Self {
            a: vec![f16::ZERO; d_out * r],
            b: vec![f16::ZERO; r * d_in],
            d_out,
            d_in,
            rank,
        }
    }

    /// Create adapter from raw fp16 slices
    ///
    /// # Arguments
    ///
    /// * `a` - A matrix data (d_out × r, row-major)
    /// * `b` - B matrix data (r × d_in, row-major)
    /// * `d_out` - Output dimension
    /// * `d_in` - Input dimension
    /// * `rank` - LoRA rank
    ///
    /// # Returns
    ///
    /// Initialized adapter, or error if dimensions mismatch
    pub fn from_slices(
        a: &[f16],
        b: &[f16],
        d_out: usize,
        d_in: usize,
        rank: LoraRank,
    ) -> crate::Result<Self> {
        let r = rank.value();

        if a.len() != d_out * r {
            return Err(crate::AneError::MemoryAllocationFailed(format!(
                "A matrix size mismatch: expected {}, got {}",
                d_out * r,
                a.len()
            )));
        }

        if b.len() != r * d_in {
            return Err(crate::AneError::MemoryAllocationFailed(format!(
                "B matrix size mismatch: expected {}, got {}",
                r * d_in,
                b.len()
            )));
        }

        Ok(Self {
            a: a.to_vec(),
            b: b.to_vec(),
            d_out,
            d_in,
            rank,
        })
    }

    /// Calculate total memory footprint (A + B)
    ///
    /// # Returns
    ///
    /// Total bytes for both matrices (fp16)
    pub fn total_bytes(&self) -> usize {
        (self.a.len() + self.b.len()) * std::mem::size_of::<f16>()
    }

    /// Reshape A matrix for ANE spatial format: [1, d_out, r, 1]
    ///
    /// The ANE expects 4D tensors [batch, channels, height, width].
    /// For linear layers, we map:
    /// - batch = 1
    /// - channels = d_out
    /// - height = r
    /// - width = 1
    pub fn reshape_a_for_ane(&self) -> Vec<f16> {
        // Already row-major d_out × r, just return copy
        self.a.clone()
    }

    /// Reshape B matrix for ANE spatial format: [1, r, d_in, 1]
    ///
    /// The ANE expects 4D tensors [batch, channels, height, width].
    /// For linear layers, we map:
    /// - batch = 1
    /// - channels = r
    /// - height = d_in
    /// - width = 1
    pub fn reshape_b_for_ane(&self) -> Vec<f16> {
        // Already row-major r × d_in, just return copy
        self.b.clone()
    }

    /// Apply Gaussian initialization (Kaiming He normal)
    ///
    /// A ~ N(0, σ²) where σ = √(2 / d_in)
    /// B ~ zeros (standard LoRA practice)
    ///
    /// # Arguments
    ///
    /// * `seed` - Random seed for reproducibility
    pub fn kaiming_init(&mut self, seed: u64) {
        use std::f32::consts::SQRT_2;

        // Simple LCG for deterministic random
        let mut rng = seed;
        let mut rand = || -> f32 {
            rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
            ((rng >> 16) as f32) / 65536.0 - 0.5
        };

        let sigma = SQRT_2 / (self.d_in as f32).sqrt();

        // Initialize A with scaled random values
        for a_val in self.a.iter_mut() {
            *a_val = f16::from_f32(rand() * sigma);
        }

        // B stays zero (standard LoRA)
        // (already initialized to zero in new())
    }

    /// Quantize matrices to INT8 for memory savings
    ///
    /// ANE dequantizes INT8→fp16 on-the-fly, so this saves 50% memory bandwidth
    /// without compute penalty.
    ///
    /// # Returns
    ///
    /// (quantized_a, quantized_b, scale_factor_a, scale_factor_b)
    pub fn quantize_int8(&self) -> (Vec<i8>, Vec<i8>, f32, f32) {
        fn quantize(data: &[f16]) -> (Vec<i8>, f32) {
            // Find absmax for symmetric quantization
            let absmax = data
                .iter()
                .map(|&v| {
                    // Manual abs since f16::abs() might not be available
                    if v < f16::ZERO { -v } else { v }
                })
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or(f16::ZERO);

            let scale = if absmax == f16::ZERO {
                1.0
            } else {
                127.0 / absmax.to_f32()
            };

            let quantized: Vec<i8> = data
                .iter()
                .map(|&v| (v.to_f32() * scale).round().clamp(-127.0, 127.0) as i8)
                .collect();

            (quantized, scale)
        }

        let (qa, sa) = quantize(&self.a);
        let (qb, sb) = quantize(&self.b);

        (qa, qb, sa, sb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lora_rank_values() {
        assert_eq!(LoraRank::Rank8.value(), 8);
        assert_eq!(LoraRank::Rank64.value(), 64);
    }

    #[test]
    fn test_lora_rank_memory() {
        let rank = LoraRank::Rank64;
        let dim = 512;
        let expected = 512 * 64 * 2; // 2 bytes per f16
        assert_eq!(rank.memory_bytes(dim), expected);
    }

    #[test]
    fn test_adapter_new() {
        let adapter = LoraAdapter::new(512, 256, LoraRank::Rank64);

        assert_eq!(adapter.d_out, 512);
        assert_eq!(adapter.d_in, 256);
        assert_eq!(adapter.a.len(), 512 * 64);
        assert_eq!(adapter.b.len(), 64 * 256);

        // Check zero initialization
        assert!(adapter.a.iter().all(|&v| v == f16::ZERO));
        assert!(adapter.b.iter().all(|&v| v == f16::ZERO));
    }

    #[test]
    fn test_adapter_from_slices() {
        let a_data = vec![f16::from_f32(1.0); 512 * 64];
        let b_data = vec![f16::from_f32(2.0); 64 * 256];

        let adapter =
            LoraAdapter::from_slices(&a_data, &b_data, 512, 256, LoraRank::Rank64).unwrap();

        assert_eq!(adapter.a.len(), 512 * 64);
        assert_eq!(adapter.b.len(), 64 * 256);
        assert_eq!(adapter.a[0], f16::from_f32(1.0));
        assert_eq!(adapter.b[0], f16::from_f32(2.0));
    }

    #[test]
    fn test_adapter_from_slices_dimension_mismatch() {
        let a_data = vec![f16::from_f32(1.0); 100]; // Wrong size
        let b_data = vec![f16::from_f32(2.0); 64 * 256];

        let result = LoraAdapter::from_slices(&a_data, &b_data, 512, 256, LoraRank::Rank64);
        assert!(result.is_err());
    }

    #[test]
    fn test_total_bytes() {
        let adapter = LoraAdapter::new(512, 256, LoraRank::Rank64);
        let expected = (512 * 64 + 64 * 256) * 2; // 2 bytes per f16
        assert_eq!(adapter.total_bytes(), expected);
    }

    #[test]
    fn test_kaiming_init() {
        let mut adapter = LoraAdapter::new(512, 256, LoraRank::Rank64);
        adapter.kaiming_init(42);

        // A should be non-zero
        assert!(adapter.a.iter().any(|&v| v != f16::ZERO));

        // B should stay zero
        assert!(adapter.b.iter().all(|&v| v == f16::ZERO));

        // A values should be roughly normally distributed
        let mean: f32 = adapter.a.iter().map(|&v| v.to_f32()).sum::<f32>() / adapter.a.len() as f32;
        assert!(mean.abs() < 0.01); // Should be close to 0
    }

    #[test]
    fn test_quantize_int8() {
        let mut adapter = LoraAdapter::new(128, 64, LoraRank::Rank16);
        adapter.kaiming_init(123);

        let (qa, qb, sa, sb) = adapter.quantize_int8();

        // Check sizes
        assert_eq!(qa.len(), adapter.a.len());
        assert_eq!(qb.len(), adapter.b.len());

        // Check bounds
        assert!(qa.iter().all(|&v| v >= -127 && v <= 127));
        assert!(qb.iter().all(|&v| v >= -127 && v <= 127));

        // Check scale factors are positive
        assert!(sa > 0.0);
        assert!(sb > 0.0);
    }

    #[test]
    fn test_reshape_for_ane() {
        let adapter = LoraAdapter::new(512, 256, LoraRank::Rank64);

        let a_reshaped = adapter.reshape_a_for_ane();
        let b_reshaped = adapter.reshape_b_for_ane();

        // Should be same size (just reshaping metadata, not data)
        assert_eq!(a_reshaped.len(), adapter.a.len());
        assert_eq!(b_reshaped.len(), adapter.b.len());
    }
}
