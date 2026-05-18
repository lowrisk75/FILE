//! # Android Backend (Tier 3)
//!
//! Android neural network acceleration via NNAPI or Qualcomm Hexagon DSP.
//!
//! ## NNAPI (Android 8.1+)
//!
//! - **Availability**: All Android 8.1+ devices
//! - **Hardware**: GPU, NPU, DSP (automatic selection)
//! - **Latency**: 500-1500µs
//! - **API**: Android NDK NNAPI
//!
//! ## Hexagon DSP (Snapdragon only)
//!
//! - **Availability**: Snapdragon 8 Gen 2+ recommended
//! - **Hardware**: Hexagon DSP direct access
//! - **Latency**: <200µs
//! - **API**: Qualcomm SNPE (Snapdragon Neural Processing Engine)
//!
//! ## Architecture
//!
//! ```text
//! edge_ai (Rust)
//!     │
//!     │ JNI
//!     ↓
//! AndroidBackend.kt (Kotlin wrapper)
//!     │
//!     ├─► NNAPI (baseline)
//!     │   └─► GPU/NPU/DSP automatic
//!     │
//!     └─► SNPE (Hexagon)
//!         └─► Hexagon DSP direct
//! ```
//!
//! ## Kotlin Wrapper (AndroidBackend.kt)
//!
//! ```kotlin
//! import org.tensorflow.lite.Interpreter
//! import com.qualcomm.qti.snpe.SNPE
//!
//! class AndroidBackend(modelPath: String) {
//!     private val nnapi: Interpreter
//!     private val hexagon: NeuralNetwork?
//!
//!     init {
//!         // NNAPI baseline
//!         nnapi = Interpreter(
//!             File(modelPath),
//!             Interpreter.Options().apply {
//!                 useNNAPI = true
//!                 setNumThreads(4)
//!             }
//!         )
//!
//!         // Hexagon DSP (if available)
//!         hexagon = try {
//!             SNPE.NeuralNetworkBuilder(context)
//!                 .setRuntimeOrder(NeuralNetwork.Runtime.DSP)
//!                 .setModel(modelPath)
//!                 .build()
//!         } catch (e: Exception) {
//!             null  // Fallback to NNAPI
//!         }
//!     }
//!
//!     @JvmName("updateLora")
//!     external fun updateLora(a: FloatArray, b: FloatArray): Long
//!
//!     @JvmName("infer")
//!     external fun infer(input: FloatArray): FloatArray
//! }
//! ```
//!
//! ## Current Implementation
//!
//! **TODO**: This module is a stub. Android support planned for Phase 4.2.
//!
//! ## Required Dependencies
//!
//! ```gradle
//! dependencies {
//!     implementation 'org.tensorflow:tensorflow-lite:2.14.0'
//!     implementation 'org.tensorflow:tensorflow-lite-gpu:2.14.0'
//!     implementation 'com.qualcomm.qti.snpe:snpe:2.16.0'  // Hexagon
//! }
//! ```

use crate::backend::{AiBackend, BackendError, Result};
use half::f16;
use std::time::Duration;

/// Android NNAPI backend (Tier 3 baseline)
pub struct NnapiBackend {
    #[allow(dead_code)]
    model_path: String,
}

impl NnapiBackend {
    /// Initialize NNAPI backend
    ///
    /// **TODO**: JNI integration
    pub fn init(_model_path: String) -> Result<Self> {
        #[cfg(not(target_os = "android"))]
        return Err(BackendError::UnsupportedPlatform);

        #[cfg(target_os = "android")]
        Err(BackendError::InitializationFailed(
            "NNAPI integration not yet implemented".into(),
        ))
    }
}

/// Qualcomm Hexagon DSP backend (Tier 3 power user)
pub struct HexagonBackend {
    #[allow(dead_code)]
    model_path: String,
}

impl HexagonBackend {
    /// Initialize Hexagon DSP backend
    ///
    /// **TODO**: SNPE integration
    pub fn init(_model_path: String) -> Result<Self> {
        #[cfg(not(target_os = "android"))]
        return Err(BackendError::UnsupportedPlatform);

        #[cfg(target_os = "android")]
        Err(BackendError::InitializationFailed(
            "Hexagon DSP integration not yet implemented".into(),
        ))
    }
}

/// Check if Hexagon DSP is available
///
/// Returns true on Snapdragon devices with SNPE support.
pub fn hexagon_available() -> bool {
    #[cfg(target_os = "android")]
    {
        // TODO: Query system for Hexagon DSP availability
        // Check SoC model, SNPE library presence
        false
    }

    #[cfg(not(target_os = "android"))]
    false
}

#[async_trait::async_trait]
impl AiBackend for NnapiBackend {
    async fn update_lora(&mut self, _a: &[f16], _b: &[f16]) -> Result<Duration> {
        Err(BackendError::UnsupportedPlatform)
    }

    async fn infer(&self, _input: &[f16]) -> Result<(Vec<f16>, Duration)> {
        Err(BackendError::UnsupportedPlatform)
    }

    fn backend_name(&self) -> &str {
        "Android NNAPI (stub)"
    }

    fn backend_tier(&self) -> u8 {
        3
    }

    fn is_available(&self) -> bool {
        false
    }
}

#[async_trait::async_trait]
impl AiBackend for HexagonBackend {
    async fn update_lora(&mut self, _a: &[f16], _b: &[f16]) -> Result<Duration> {
        Err(BackendError::UnsupportedPlatform)
    }

    async fn infer(&self, _input: &[f16]) -> Result<(Vec<f16>, Duration)> {
        Err(BackendError::UnsupportedPlatform)
    }

    fn backend_name(&self) -> &str {
        "Qualcomm Hexagon DSP (stub)"
    }

    fn backend_tier(&self) -> u8 {
        3
    }

    fn is_available(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hexagon_available() {
        let available = hexagon_available();
        // Should return false on non-Android
        #[cfg(not(target_os = "android"))]
        assert!(!available);
    }

    #[test]
    fn test_nnapi_init_fails_on_non_android() {
        #[cfg(not(target_os = "android"))]
        {
            let result = NnapiBackend::init("/tmp/model.tflite".into());
            assert!(result.is_err());
        }
    }
}
