//! FFI exports for Edge AI routing with real multi-tier backends
//!
//! C ABI bindings for AORATA Edge AI using Core ML / ANE Helper / Android backends
//!
//! **Architecture:** Real AneRoutingAgent with auto-detected backend
//! - Tier 1: Core ML (iOS/macOS App Store baseline)
//! - Tier 2: ANE Helper (macOS direct, private APIs)
//! - Tier 3: Android NNAPI / Hexagon DSP
//! - LoRA dynamic weight updates
//! - <2ms inference latency

use crate::routing_agent::{AneRoutingAgent, NetworkContext};
use std::ffi::c_int;
use std::slice;

/// FFI error codes
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiError {
    Success = 0,
    InvalidInput = -1,
    BackendNotAvailable = -2,
    InferenceFailed = -3,
    RuntimeError = -4,
}

/// Network context for routing decisions
#[repr(C)]
pub struct NetworkContextFFI {
    pub avg_ttl: u8,
    pub asn: u32,
    pub iat_variance_us: u32,
    pub cpu_load: u8,
    pub num_paths: u8,
}

impl From<&NetworkContextFFI> for NetworkContext {
    fn from(ffi: &NetworkContextFFI) -> Self {
        NetworkContext {
            avg_ttl: ffi.avg_ttl,
            asn: ffi.asn,
            iat_variance: ffi.iat_variance_us as u64,
            cpu_load: ffi.cpu_load,
            num_paths: ffi.num_paths,
        }
    }
}

/// Opaque AI backend handle
#[repr(C)]
pub struct AiBackend {
    _private: [u8; 0],
}

/// Internal AI backend wrapper
struct AiBackendReal {
    /// Real AneRoutingAgent
    agent: AneRoutingAgent,

    /// Tokio runtime for async operations
    runtime: tokio::runtime::Runtime,
}

// ============================================================================
// BACKEND LIFECYCLE
// ============================================================================

/// Create AI backend (auto-detect best available tier)
///
/// Priority: ANE Helper > Core ML > Hexagon > NNAPI
///
/// Creates AneRoutingAgent with auto backend detection:
/// - macOS: ANE Helper if available, else Core ML
/// - iOS: Core ML
/// - Android: Hexagon DSP if available, else NNAPI
///
/// # Safety
/// Caller must free with `ai_destroy`
#[no_mangle]
pub unsafe extern "C" fn ai_create(out_backend: *mut *mut AiBackend) -> c_int {
    if out_backend.is_null() {
        return AiError::InvalidInput as c_int;
    }

    // Create Tokio runtime
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(_) => return AiError::RuntimeError as c_int,
    };

    // Create AneRoutingAgent (auto-detect backend)
    let agent = match runtime.block_on(async {
        AneRoutingAgent::auto()
            .await
            .map_err(|_| AiError::BackendNotAvailable)
    }) {
        Ok(a) => a,
        Err(e) => return e as c_int,
    };

    // Wrap in FFI-safe structure
    let backend_real = Box::new(AiBackendReal { agent, runtime });

    *out_backend = Box::into_raw(backend_real) as *mut AiBackend;
    AiError::Success as c_int
}

/// Destroy AI backend
///
/// # Safety
/// Backend must have been created by `ai_create`
#[no_mangle]
pub unsafe extern "C" fn ai_destroy(backend: *mut AiBackend) {
    if backend.is_null() {
        return;
    }

    let _backend_real = Box::from_raw(backend as *mut AiBackendReal);
    // Drop happens here (agent + runtime shutdown)
}

// ============================================================================
// BACKEND QUERY
// ============================================================================

/// Get backend name (Tier 1/2/3 identifier)
///
/// Returns strings like:
/// - "Core ML" (Tier 1)
/// - "ANE Helper" (Tier 2)
/// - "Android NNAPI" (Tier 3)
/// - "Qualcomm Hexagon DSP" (Tier 3)
///
/// # Safety
/// - Backend must be valid
/// - `out_name` must have capacity for at least 64 bytes
///
/// # Returns
/// Length of name written, or -1 on error
#[no_mangle]
pub unsafe extern "C" fn ai_get_backend_name(
    backend: *mut AiBackend,
    out_name: *mut u8,
    capacity: usize,
) -> c_int {
    if backend.is_null() || out_name.is_null() || capacity == 0 {
        return -1;
    }

    let backend_real = &*(backend as *const AiBackendReal);

    // Get backend name via AneRoutingAgent wrapper method
    let name = backend_real.agent.backend_name();
    let name_bytes = name.as_bytes();
    let len = name_bytes.len().min(capacity);

    let out_slice = slice::from_raw_parts_mut(out_name, capacity);
    out_slice[..len].copy_from_slice(&name_bytes[..len]);

    len as c_int
}

/// Get backend tier (1 = Core ML, 2 = ANE Helper, 3 = Android)
///
/// # Safety
/// Backend must be valid
#[no_mangle]
pub unsafe extern "C" fn ai_get_tier(backend: *mut AiBackend) -> u8 {
    if backend.is_null() {
        return 0;
    }

    let backend_real = &*(backend as *const AiBackendReal);
    backend_real.agent.backend_tier()
}

// ============================================================================
// LORA OPERATIONS
// ============================================================================

/// Update LoRA weights from network context
///
/// Dynamically adapts routing model based on network state:
/// - TTL analysis (CGNAT depth)
/// - ASN scoring (peering quality)
/// - Inter-arrival time variance (congestion)
/// - CPU load (device state)
///
/// # Safety
/// - Backend must be valid
/// - Context must be valid
///
/// # Returns
/// Update latency in microseconds, or -1 on error
#[no_mangle]
pub unsafe extern "C" fn ai_update_lora(
    backend: *mut AiBackend,
    context: *const NetworkContextFFI,
) -> i64 {
    if backend.is_null() || context.is_null() {
        return -1;
    }

    let backend_real = &mut *(backend as *mut AiBackendReal);
    let ffi_context = &*context;
    let network_context = NetworkContext::from(ffi_context);

    // Update LoRA weights (async operation)
    let duration = match backend_real.runtime.block_on(async {
        backend_real
            .agent
            .update_from_context(&network_context)
            .await
            .map_err(|_| AiError::InferenceFailed)
    }) {
        Ok(d) => d,
        Err(_) => return -1,
    };

    duration.as_micros() as i64
}

// ============================================================================
// INFERENCE OPERATIONS
// ============================================================================

/// Perform inference (routing decision)
///
/// Runs neural network forward pass to score MPQUIC paths.
///
/// # Input Format
/// - `input`: Path features (f32), e.g., [latency_ms, packet_loss, jitter, ...]
/// - `input_len`: Number of features per path × number of paths
///
/// # Output Format
/// - `output`: Path scores (f32), one score per path
/// - `out_len`: Number of scores (= number of paths)
///
/// # Safety
/// - Backend must be valid
/// - `input` must be valid for `input_len` floats (f32)
/// - `output` must have capacity for at least `output_capacity` floats
/// - `out_len` will be set to actual output length
///
/// # Returns
/// Inference latency in microseconds, or -1 on error
#[no_mangle]
pub unsafe extern "C" fn ai_infer(
    backend: *mut AiBackend,
    input: *const f32,
    input_len: usize,
    output: *mut f32,
    output_capacity: usize,
    out_len: *mut usize,
) -> i64 {
    if backend.is_null() || input.is_null() || output.is_null() || out_len.is_null() {
        return -1;
    }

    let backend_real = &*(backend as *const AiBackendReal);

    // Convert f32 input to f16 for backend
    let in_slice = slice::from_raw_parts(input, input_len);
    let input_f16: Vec<half::f16> = in_slice.iter().map(|&f| half::f16::from_f32(f)).collect();

    // Run inference (async)
    let (output_f16, duration): (Vec<half::f16>, std::time::Duration) =
        match backend_real.runtime.block_on(async {
            backend_real
                .agent
                .infer(&input_f16)
                .await
                .map_err(|_| AiError::InferenceFailed)
        }) {
            Ok(result) => result,
            Err(_) => return -1,
        };

    // Convert f16 output back to f32
    let len = output_f16.len().min(output_capacity);
    let out_slice = slice::from_raw_parts_mut(output, output_capacity);
    for (i, &val) in output_f16.iter().take(len).enumerate() {
        out_slice[i] = val.to_f32();
    }
    *out_len = len;

    duration.as_micros() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_context_conversion() {
        let ffi_ctx = NetworkContextFFI {
            avg_ttl: 64,
            asn: 15169, // Google
            iat_variance_us: 5000,
            cpu_load: 30,
            num_paths: 3,
        };

        let network_ctx = NetworkContext::from(&ffi_ctx);
        assert_eq!(network_ctx.avg_ttl, 64);
        assert_eq!(network_ctx.asn, 15169);
        assert_eq!(network_ctx.iat_variance, 5000);
        assert_eq!(network_ctx.cpu_load, 30);
        assert_eq!(network_ctx.num_paths, 3);
    }

    #[test]
    fn test_ffi_struct_size() {
        assert_eq!(std::mem::size_of::<NetworkContextFFI>(), 16);
    }
}
