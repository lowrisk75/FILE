//! # AI Backend Abstraction Layer
//!
//! Unified interface for multi-tier AI acceleration across platforms:
//!
//! ## Tier 1: Core ML (App Store baseline)
//! - **Platform**: iOS 15+, macOS 12+
//! - **Latency**: 300-800µs LoRA update, 1-2ms inference
//! - **Distribution**: App Store, TestFlight, notarized
//! - **Hardware**: ANE automatic scheduling
//!
//! ## Tier 2: ANE Helper (macOS power users)
//! - **Platform**: macOS 12+ (direct download only)
//! - **Latency**: <100µs LoRA update, <1ms inference
//! - **Distribution**: lorislab.fr/aorata-helper.dmg (non-notarized)
//! - **Hardware**: ANE direct access via private APIs
//!
//! ## Tier 3: Android
//! - **Platform**: Android 8.1+ (NNAPI), Snapdragon (Hexagon DSP)
//! - **Latency**: 500-1500µs (NNAPI), <200µs (Hexagon)
//! - **Distribution**: APK direct, F-Droid
//! - **Hardware**: GPU/NPU automatic, DSP direct
//!
//! ## Runtime Detection
//!
//! ```rust
//! use edge_ai::backend::{AiBackend, detect_backend};
//!
//! let backend = detect_backend()?;
//! println!("Using backend: {}", backend.backend_name());
//! ```

use half::f16;
use std::time::Duration;
use thiserror::Error;

/// Errors that can occur during backend operations
#[derive(Debug, Error)]
pub enum BackendError {
    #[error("Backend initialization failed: {0}")]
    InitializationFailed(String),

    #[error("LoRA update failed: {0}")]
    LoraUpdateFailed(String),

    #[error("Inference failed: {0}")]
    InferenceFailed(String),

    #[error("Backend not available on this platform")]
    UnsupportedPlatform,

    #[error("ANE Helper not installed or not responding")]
    HelperNotAvailable,

    #[error("Model not loaded")]
    ModelNotLoaded,

    #[error("Invalid tensor dimensions: expected {expected}, got {actual}")]
    InvalidDimensions { expected: String, actual: String },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),
}

pub type Result<T> = std::result::Result<T, BackendError>;

/// Unified AI backend interface
///
/// All backends must implement this trait to provide:
/// - Dynamic LoRA weight updates
/// - Fast inference (<5ms)
/// - Latency reporting
///
/// **Note**: All methods are async. Non-async backends should use `async { ... }` blocks.
#[async_trait::async_trait]
pub trait AiBackend: Send + Sync {
    /// Update LoRA adapter weights (A and B matrices)
    ///
    /// # Arguments
    /// - `a`: Matrix A (d_out × rank) in row-major order
    /// - `b`: Matrix B (rank × d_in) in row-major order
    ///
    /// # Returns
    /// Duration of the update operation
    ///
    /// # Latency Targets
    /// - Core ML: 300-800µs
    /// - ANE Helper: <100µs
    /// - NNAPI: 500-1500µs
    /// - Hexagon DSP: <200µs
    async fn update_lora(&mut self, a: &[f16], b: &[f16]) -> Result<Duration>;

    /// Run inference on input tensor
    ///
    /// # Arguments
    /// - `input`: Input tensor (fp16)
    ///
    /// # Returns
    /// Output tensor (fp16) and inference duration
    async fn infer(&self, input: &[f16]) -> Result<(Vec<f16>, Duration)>;

    /// Get backend implementation name
    fn backend_name(&self) -> &str;

    /// Get backend tier (1, 2, or 3)
    fn backend_tier(&self) -> u8;

    /// Check if backend is currently available
    fn is_available(&self) -> bool;
}

/// Detected backend type with platform-specific availability
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    /// Core ML (iOS/macOS App Store baseline)
    CoreML,

    /// ANE Helper (macOS direct download, private APIs)
    AneHelper,

    /// Android NNAPI (baseline, all Android 8.1+ devices)
    Nnapi,

    /// Qualcomm Hexagon DSP (Snapdragon only)
    HexagonDsp,
}

impl BackendType {
    /// Get human-readable backend name
    pub fn name(&self) -> &str {
        match self {
            Self::CoreML => "Core ML",
            Self::AneHelper => "ANE Helper",
            Self::Nnapi => "Android NNAPI",
            Self::HexagonDsp => "Qualcomm Hexagon DSP",
        }
    }

    /// Get backend tier (1 = baseline, 2 = power user, 3 = Android)
    pub fn tier(&self) -> u8 {
        match self {
            Self::CoreML => 1,
            Self::AneHelper => 2,
            Self::Nnapi => 3,
            Self::HexagonDsp => 3,
        }
    }

    /// Get expected latency range (µs)
    pub fn latency_range(&self) -> (u64, u64) {
        match self {
            Self::CoreML => (300, 800),
            Self::AneHelper => (50, 100),
            Self::Nnapi => (500, 1500),
            Self::HexagonDsp => (100, 200),
        }
    }
}

/// Detect best available backend for current platform
///
/// Priority order:
/// 1. ANE Helper (macOS only, if installed)
/// 2. Core ML (iOS/macOS)
/// 3. Hexagon DSP (Snapdragon Android)
/// 4. NNAPI (Android baseline)
pub fn detect_backend() -> BackendType {
    #[cfg(all(target_os = "macos", feature = "tier2-ane-helper"))]
    {
        if crate::ane_helper_client::AneHelperClient::is_available() {
            return BackendType::AneHelper;
        }
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        return BackendType::CoreML;
    }

    #[cfg(target_os = "android")]
    {
        #[cfg(feature = "tier3-android")]
        {
            if crate::android_backend::hexagon_available() {
                return BackendType::HexagonDsp;
            }
        }
        return BackendType::Nnapi;
    }

    // Fallback (should never reach here on supported platforms)
    #[allow(unreachable_code)]
    BackendType::CoreML
}

/// Check if a specific backend is available
pub fn backend_available(backend: BackendType) -> bool {
    match backend {
        #[cfg(all(target_os = "macos", feature = "tier2-ane-helper"))]
        BackendType::AneHelper => crate::ane_helper_client::AneHelperClient::is_available(),

        #[cfg(any(target_os = "macos", target_os = "ios"))]
        BackendType::CoreML => true,

        #[cfg(target_os = "android")]
        BackendType::Nnapi => true,

        #[cfg(all(target_os = "android", feature = "tier3-android"))]
        BackendType::HexagonDsp => crate::android_backend::hexagon_available(),

        #[allow(unreachable_patterns)]
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_detection() {
        let backend = detect_backend();
        println!("Detected backend: {:?}", backend);

        // Should always detect something
        assert!(backend_available(backend));
    }

    #[test]
    fn test_backend_metadata() {
        let backend = BackendType::CoreML;
        assert_eq!(backend.tier(), 1);
        assert_eq!(backend.name(), "Core ML");
        let (min, max) = backend.latency_range();
        assert!(min < max);
        assert!(max < 5000); // All backends should be <5ms
    }

    #[test]
    fn test_ane_helper_latency() {
        let backend = BackendType::AneHelper;
        let (min, max) = backend.latency_range();
        assert!(max <= 100); // ANE Helper target <100µs
    }

    #[test]
    fn test_all_backends_valid_tier() {
        for backend in [
            BackendType::CoreML,
            BackendType::AneHelper,
            BackendType::Nnapi,
            BackendType::HexagonDsp,
        ] {
            let tier = backend.tier();
            assert!(tier >= 1 && tier <= 3);
        }
    }
}
