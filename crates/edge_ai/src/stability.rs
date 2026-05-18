//! # Numerical Stability: FP16 NaN Cascade Prevention
//!
//! The Apple Neural Engine executes all arithmetic in IEEE 754 half-precision (fp16).
//! The maximum representable value is ±65,504. Any calculation exceeding this bound
//! overflows to infinity, which then propagates through non-linear activations
//! (softmax, layer_norm) and causes catastrophic NaN divergence.
//!
//! This module implements **activation clamping** to arrest the overflow cascade.

use half::f16;

/// Maximum safe fp16 value before overflow to infinity
pub const FP16_MAX: f16 = f16::from_f32_const(65504.0);

/// Minimum safe fp16 value before overflow to negative infinity
pub const FP16_MIN: f16 = f16::from_f32_const(-65504.0);

/// Clamp a single fp16 value to the representable range
///
/// # Arguments
///
/// * `value` - Input fp16 value
///
/// # Returns
///
/// Clamped value in range `[FP16_MIN, FP16_MAX]`
///
/// # Example
///
/// ```
/// use edge_ai::stability::{clamp_fp16, FP16_MAX};
/// use half::f16;
///
/// let safe = clamp_fp16(f16::from_f32(100.0));  // Returns 100.0
/// let overflow = clamp_fp16(f16::INFINITY);      // Returns 65504.0
/// ```
#[inline]
pub fn clamp_fp16(value: f16) -> f16 {
    if value > FP16_MAX {
        FP16_MAX
    } else if value < FP16_MIN {
        FP16_MIN
    } else {
        value
    }
}

/// Clamp an entire tensor of fp16 values in-place
///
/// # Arguments
///
/// * `tensor` - Mutable slice of fp16 values
///
/// # Example
///
/// ```
/// use edge_ai::stability::clamp_tensor_fp16;
/// use half::f16;
///
/// let mut data = vec![f16::from_f32(100.0), f16::INFINITY, f16::from_f32(-70000.0)];
/// clamp_tensor_fp16(&mut data);
///
/// assert_eq!(data[0], f16::from_f32(100.0));       // Unchanged
/// assert_eq!(data[1], f16::from_f32(65504.0));     // Clamped max
/// assert_eq!(data[2], f16::from_f32(-65504.0));    // Clamped min
/// ```
pub fn clamp_tensor_fp16(tensor: &mut [f16]) {
    for value in tensor.iter_mut() {
        *value = clamp_fp16(*value);
    }
}

/// Generate MIL IR text for a clamp operation
///
/// Produces a MIL `clamp` operation that the ANE executes natively with fused
/// comparators (zero latency overhead).
///
/// # Arguments
///
/// * `input_var` - MIL variable name for input tensor
/// * `output_var` - MIL variable name for output tensor
///
/// # Returns
///
/// MIL IR text snippet
///
/// # Example
///
/// ```
/// use edge_ai::stability::generate_clamp_mil;
///
/// let mil = generate_clamp_mil("activations", "clamped_activations");
/// assert!(mil.contains("clamp"));
/// assert!(mil.contains("-65504.0"));
/// assert!(mil.contains("65504.0"));
/// ```
pub fn generate_clamp_mil(input_var: &str, output_var: &str) -> String {
    format!("{output_var} = clamp({input_var}, min=-65504.0, max=65504.0)")
}

/// Detect potential NaN risk in a tensor
///
/// Scans tensor for values near the fp16 overflow boundary. Returns `true` if
/// any value exceeds 90% of the max threshold, indicating high NaN risk.
///
/// # Arguments
///
/// * `tensor` - Slice of fp16 values to check
///
/// # Returns
///
/// `true` if overflow risk detected, `false` otherwise
///
/// # Example
///
/// ```
/// use edge_ai::stability::detect_overflow_risk;
/// use half::f16;
///
/// let safe = vec![f16::from_f32(100.0), f16::from_f32(200.0)];
/// assert!(!detect_overflow_risk(&safe));
///
/// let risky = vec![f16::from_f32(60000.0), f16::from_f32(200.0)];
/// assert!(detect_overflow_risk(&risky));
/// ```
pub fn detect_overflow_risk(tensor: &[f16]) -> bool {
    const THRESHOLD: f16 = f16::from_f32_const(58953.6); // 90% of FP16_MAX

    tensor.iter().any(|&v| {
        // Manual abs since f16::abs() might not be available in all versions
        let abs = if v < f16::ZERO { -v } else { v };
        abs > THRESHOLD
    })
}

/// Statistics about a tensor's numerical range
#[derive(Debug, Clone, PartialEq)]
pub struct TensorStats {
    /// Minimum value
    pub min: f16,
    /// Maximum value
    pub max: f16,
    /// Count of values that hit the clamp boundary
    pub clamped_count: usize,
    /// Total element count
    pub total_count: usize,
}

impl TensorStats {
    /// Compute statistics for a tensor
    ///
    /// # Arguments
    ///
    /// * `tensor` - Slice of fp16 values
    ///
    /// # Returns
    ///
    /// Statistics struct
    pub fn compute(tensor: &[f16]) -> Self {
        let min = tensor
            .iter()
            .copied()
            .min_by(|a, b| a.total_cmp(b))
            .unwrap_or(f16::ZERO);
        let max = tensor
            .iter()
            .copied()
            .max_by(|a, b| a.total_cmp(b))
            .unwrap_or(f16::ZERO);

        let clamped_count = tensor
            .iter()
            .filter(|&&v| v == FP16_MAX || v == FP16_MIN)
            .count();

        TensorStats {
            min,
            max,
            clamped_count,
            total_count: tensor.len(),
        }
    }

    /// Calculate percentage of values that were clamped
    pub fn clamp_percentage(&self) -> f32 {
        if self.total_count == 0 {
            0.0
        } else {
            (self.clamped_count as f32 / self.total_count as f32) * 100.0
        }
    }

    /// Check if tensor is well-behaved (< 0.1% clamp rate)
    pub fn is_stable(&self) -> bool {
        self.clamp_percentage() < 0.1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp_fp16_normal() {
        let value = f16::from_f32(100.0);
        assert_eq!(clamp_fp16(value), value);
    }

    #[test]
    fn test_clamp_fp16_overflow() {
        assert_eq!(clamp_fp16(f16::INFINITY), FP16_MAX);
        assert_eq!(clamp_fp16(f16::NEG_INFINITY), FP16_MIN);
    }

    #[test]
    fn test_clamp_tensor() {
        let mut data = vec![
            f16::from_f32(100.0),
            f16::INFINITY,
            f16::from_f32(-70000.0),
            f16::from_f32(0.0),
        ];

        clamp_tensor_fp16(&mut data);

        assert_eq!(data[0], f16::from_f32(100.0));
        assert_eq!(data[1], FP16_MAX);
        assert_eq!(data[2], FP16_MIN);
        assert_eq!(data[3], f16::ZERO);
    }

    #[test]
    fn test_generate_clamp_mil() {
        let mil = generate_clamp_mil("x", "y");
        assert!(mil.contains("y = clamp(x"));
        assert!(mil.contains("min=-65504.0"));
        assert!(mil.contains("max=65504.0"));
    }

    #[test]
    fn test_detect_overflow_risk() {
        let safe = vec![f16::from_f32(100.0), f16::from_f32(1000.0)];
        assert!(!detect_overflow_risk(&safe));

        let risky = vec![f16::from_f32(60000.0)];
        assert!(detect_overflow_risk(&risky));
    }

    #[test]
    fn test_tensor_stats() {
        let data = vec![
            f16::from_f32(1.0),
            f16::from_f32(100.0),
            FP16_MAX, // Clamped
            f16::from_f32(-50.0),
            FP16_MIN, // Clamped
        ];

        let stats = TensorStats::compute(&data);

        assert_eq!(stats.min, FP16_MIN);
        assert_eq!(stats.max, FP16_MAX);
        assert_eq!(stats.clamped_count, 2);
        assert_eq!(stats.total_count, 5);
        assert_eq!(stats.clamp_percentage(), 40.0);
        assert!(!stats.is_stable()); // 40% > 0.1%
    }

    #[test]
    fn test_tensor_stats_stable() {
        let data = vec![f16::from_f32(1.0); 1000];
        let stats = TensorStats::compute(&data);
        assert!(stats.is_stable());
    }
}
