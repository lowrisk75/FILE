//! FFI exports for Swift/Objective-C integration
//!
//! C ABI bindings for MPQUIC transport using ant-quic 0.14.200
//!
//! **Architecture:** Real ant-quic P2pEndpoint with simple send/recv API
//! - P2pEndpoint handles all QUIC complexity internally
//! - Connect → get PeerId → send/recv with that PeerId
//! - Pure PQC (ML-KEM-768 + ML-DSA-65) automatic
//! - NAT traversal automatic via PUNCH_ME_NOW frames

use ant_quic::{P2pConfig, P2pEndpoint, PeerId};
use std::collections::HashMap;
use std::ffi::{c_char, c_int, c_uchar, CStr};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::RwLock;

/// FFI error codes
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportError {
    Success = 0,
    InvalidInput = -1,
    ConnectionFailed = -2,
    SendFailed = -3,
    RecvFailed = -4,
    RuntimeError = -5,
    AddressParseFailed = -6,
    EndpointInitFailed = -7,
    PeerNotFound = -8,
    Timeout = -9,
}

/// Opaque transport context handle
#[repr(C)]
pub struct TransportContext {
    _private: [u8; 0],
}

/// Connection handle (opaque 32-byte PeerId)
pub type ConnectionHandle = [u8; 32];

/// Internal transport context with real ant-quic endpoint
struct TransportContextReal {
    /// Real ant-quic P2pEndpoint (handles all QUIC complexity)
    endpoint: P2pEndpoint,

    /// Tokio runtime for async operations
    runtime: Runtime,

    /// Map connection handles to PeerIds
    peer_ids: Arc<RwLock<HashMap<ConnectionHandle, PeerId>>>,
}

// ============================================================================
// ENDPOINT LIFECYCLE
// ============================================================================

/// Initialize MPQUIC transport endpoint with ant-quic
///
/// Creates a real P2pEndpoint with:
/// - Pure PQC (ML-KEM-768 + ML-DSA-65) - quantum-safe crypto
/// - NAT traversal (PUNCH_ME_NOW frames) - works through Symmetric NATs
/// - Multi-path QUIC - seamless Wi-Fi/5G switching
///
/// # Safety
/// - `local_addr` must be null-terminated C string (e.g., "0.0.0.0:0")
/// - Caller must free with `transport_destroy`
#[no_mangle]
pub unsafe extern "C" fn transport_create(
    local_addr: *const c_char,
    out_context: *mut *mut TransportContext,
) -> c_int {
    if local_addr.is_null() || out_context.is_null() {
        return TransportError::InvalidInput as c_int;
    }

    // Parse address
    let addr_cstr = match CStr::from_ptr(local_addr).to_str() {
        Ok(s) => s,
        Err(_) => return TransportError::InvalidInput as c_int,
    };
    let socket_addr: SocketAddr = match addr_cstr.parse() {
        Ok(addr) => addr,
        Err(_) => return TransportError::AddressParseFailed as c_int,
    };

    // Create Tokio runtime
    let runtime = match Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return TransportError::RuntimeError as c_int,
    };

    // Create P2pEndpoint with ant-quic (real MPQUIC with Pure PQC)
    let endpoint = match runtime.block_on(async {
        let config = P2pConfig::builder()
            .bind_addr(socket_addr)
            .pqc(ant_quic::PqcConfig::default()) // Pure PQC: ML-KEM-768 + ML-DSA-65
            .build()
            .map_err(|_| TransportError::EndpointInitFailed)?;

        P2pEndpoint::new(config)
            .await
            .map_err(|_| TransportError::EndpointInitFailed)
    }) {
        Ok(ep) => ep,
        Err(e) => return e as c_int,
    };

    // Build context
    let ctx = Box::new(TransportContextReal {
        endpoint,
        runtime,
        peer_ids: Arc::new(RwLock::new(HashMap::new())),
    });

    *out_context = Box::into_raw(ctx) as *mut TransportContext;
    TransportError::Success as c_int
}

/// Destroy transport context
///
/// # Safety
/// - Context must be from `transport_create`
#[no_mangle]
pub unsafe extern "C" fn transport_destroy(context: *mut TransportContext) {
    if context.is_null() {
        return;
    }

    let ctx = Box::from_raw(context as *mut TransportContextReal);

    // Shutdown endpoint gracefully
    ctx.runtime.block_on(async {
        ctx.endpoint.shutdown().await;
    });

    // Drop happens here (all connections closed, runtime shutdown)
}

// ============================================================================
// CONNECTION MANAGEMENT
// ============================================================================

/// Connect to remote peer via MPQUIC with NAT traversal
///
/// Establishes real P2P connection with:
/// - Pure PQC handshake (ML-KEM-768 key exchange + ML-DSA-65 signatures)
/// - NAT traversal (DCUtR hole punching via PUNCH_ME_NOW frames)
/// - Multi-path support (Wi-Fi + 5G simultaneous)
///
/// Returns a 32-byte connection handle (the peer's PeerId).
///
/// # Safety
/// - `remote_addr` must be null-terminated C string
/// - `out_connection_handle` must point to 32-byte buffer
#[no_mangle]
pub unsafe extern "C" fn transport_connect(
    context: *mut TransportContext,
    remote_addr: *const c_char,
    out_connection_handle: *mut c_uchar,
) -> c_int {
    if context.is_null() || remote_addr.is_null() || out_connection_handle.is_null() {
        return TransportError::InvalidInput as c_int;
    }

    let ctx = &*(context as *const TransportContextReal);

    // Parse address
    let addr_cstr = match CStr::from_ptr(remote_addr).to_str() {
        Ok(s) => s,
        Err(_) => return TransportError::InvalidInput as c_int,
    };
    let socket_addr: SocketAddr = match addr_cstr.parse() {
        Ok(addr) => addr,
        Err(_) => return TransportError::AddressParseFailed as c_int,
    };

    // Establish connection (real ant-quic P2P with NAT traversal)
    let peer_id = match ctx.runtime.block_on(async {
        let peer_conn = ctx
            .endpoint
            .connect(socket_addr)
            .await
            .map_err(|_| TransportError::ConnectionFailed)?;

        Ok::<PeerId, TransportError>(peer_conn.peer_id)
    }) {
        Ok(pid) => pid,
        Err(e) => return e as c_int,
    };

    // Convert PeerId to connection handle (32 bytes)
    let handle: ConnectionHandle = peer_id.0;

    // Store mapping
    ctx.runtime.block_on(async {
        ctx.peer_ids.write().await.insert(handle, peer_id);
    });

    // Copy to output
    std::ptr::copy_nonoverlapping(handle.as_ptr(), out_connection_handle, 32);

    TransportError::Success as c_int
}

/// Disconnect from peer
///
/// # Safety
/// - `connection_handle` must be 32-byte buffer from `transport_connect`
#[no_mangle]
pub unsafe extern "C" fn transport_disconnect(
    context: *mut TransportContext,
    connection_handle: *const c_uchar,
) -> c_int {
    if context.is_null() || connection_handle.is_null() {
        return TransportError::InvalidInput as c_int;
    }

    let ctx = &*(context as *const TransportContextReal);
    let handle_slice = std::slice::from_raw_parts(connection_handle, 32);
    let mut handle: ConnectionHandle = [0u8; 32];
    handle.copy_from_slice(handle_slice);

    // Remove peer mapping (connection auto-closed by ant-quic when endpoint shuts down)
    ctx.runtime.block_on(async {
        ctx.peer_ids.write().await.remove(&handle);
    });

    TransportError::Success as c_int
}

// ============================================================================
// SEND / RECV (Real MPQUIC with encryption)
// ============================================================================

/// Send data to peer (encrypted automatically via QUIC)
///
/// Uses ant-quic P2pEndpoint which:
/// - Opens a unidirectional QUIC stream
/// - Encrypts data with ChaCha20-Poly1305 (automatic)
/// - Sends over established MPQUIC connection
///
/// # Safety
/// - `connection_handle` must be 32-byte buffer from `transport_connect`
/// - `data` must be valid for `len` bytes
#[no_mangle]
pub unsafe extern "C" fn transport_send(
    context: *mut TransportContext,
    connection_handle: *const c_uchar,
    data: *const u8,
    len: usize,
) -> c_int {
    if context.is_null() || connection_handle.is_null() || data.is_null() {
        return TransportError::InvalidInput as c_int;
    }

    let ctx = &*(context as *const TransportContextReal);
    let handle_slice = std::slice::from_raw_parts(connection_handle, 32);
    let mut handle: ConnectionHandle = [0u8; 32];
    handle.copy_from_slice(handle_slice);

    let data_slice = std::slice::from_raw_parts(data, len);

    let result = ctx.runtime.block_on(async {
        // Get peer_id
        let peer_ids = ctx.peer_ids.read().await;
        let peer_id = peer_ids.get(&handle).ok_or(TransportError::PeerNotFound)?;

        // Send via ant-quic (opens stream, encrypts, sends)
        ctx.endpoint
            .send(peer_id, data_slice)
            .await
            .map_err(|_| TransportError::SendFailed)?;

        Ok::<(), TransportError>(())
    });

    match result {
        Ok(()) => TransportError::Success as c_int,
        Err(e) => e as c_int,
    }
}

/// Receive data from any peer (decrypted automatically)
///
/// Blocks until data arrives or timeout expires.
///
/// # Safety
/// - `out_peer_handle` must point to 32-byte buffer (receives sender's PeerId)
/// - `buffer` must have `capacity` bytes
/// - `out_len` receives actual data length
#[no_mangle]
pub unsafe extern "C" fn transport_recv(
    context: *mut TransportContext,
    timeout_ms: u64,
    out_peer_handle: *mut c_uchar,
    buffer: *mut u8,
    capacity: usize,
    out_len: *mut usize,
) -> c_int {
    if context.is_null() || out_peer_handle.is_null() || buffer.is_null() || out_len.is_null() {
        return TransportError::InvalidInput as c_int;
    }

    let ctx = &*(context as *const TransportContextReal);
    let buffer_slice = std::slice::from_raw_parts_mut(buffer, capacity);

    let result = ctx.runtime.block_on(async {
        // Receive via ant-quic (blocks until data or timeout)
        let timeout = Duration::from_millis(timeout_ms);
        let (peer_id, data) = ctx
            .endpoint
            .recv(timeout)
            .await
            .map_err(|_| TransportError::Timeout)?;

        // Copy data
        let len = data.len().min(capacity);
        buffer_slice[..len].copy_from_slice(&data[..len]);
        *out_len = len;

        // Return sender's peer_id
        let handle: ConnectionHandle = peer_id.0;
        std::ptr::copy_nonoverlapping(handle.as_ptr(), out_peer_handle, 32);

        Ok::<(), TransportError>(())
    });

    match result {
        Ok(()) => TransportError::Success as c_int,
        Err(e) => e as c_int,
    }
}

// ============================================================================
// LEGACY TEST API (simplified for Swift tests)
// ============================================================================

/// Test function: encrypt buffer (for testing only)
///
/// **Note:** Real encryption happens in `transport_send()`.
///
/// # Safety
/// - For testing Swift bridge only
#[no_mangle]
pub unsafe extern "C" fn transport_encrypt(
    context: *mut TransportContext,
    plaintext: *const u8,
    plaintext_len: usize,
    ciphertext: *mut u8,
    ciphertext_capacity: usize,
    out_len: *mut usize,
) -> c_int {
    if context.is_null() || plaintext.is_null() || ciphertext.is_null() || out_len.is_null() {
        return TransportError::InvalidInput as c_int;
    }

    // For testing: copy data (real encryption in transport_send via QUIC)
    let plain = std::slice::from_raw_parts(plaintext, plaintext_len);
    let cipher = std::slice::from_raw_parts_mut(ciphertext, ciphertext_capacity);

    if ciphertext_capacity < plaintext_len {
        return TransportError::SendFailed as c_int;
    }

    cipher[..plaintext_len].copy_from_slice(plain);
    *out_len = plaintext_len;

    TransportError::Success as c_int
}

/// Test function: decrypt buffer (for testing only)
///
/// # Safety
/// - For testing only
#[no_mangle]
pub unsafe extern "C" fn transport_decrypt(
    context: *mut TransportContext,
    ciphertext: *const u8,
    ciphertext_len: usize,
    plaintext: *mut u8,
    plaintext_capacity: usize,
    out_len: *mut usize,
) -> c_int {
    if context.is_null() || ciphertext.is_null() || plaintext.is_null() || out_len.is_null() {
        return TransportError::InvalidInput as c_int;
    }

    // For testing: copy data
    let cipher = std::slice::from_raw_parts(ciphertext, ciphertext_len);
    let plain = std::slice::from_raw_parts_mut(plaintext, plaintext_capacity);

    if plaintext_capacity < ciphertext_len {
        return TransportError::RecvFailed as c_int;
    }

    plain[..ciphertext_len].copy_from_slice(cipher);
    *out_len = ciphertext_len;

    TransportError::Success as c_int
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_lifecycle() {
        unsafe {
            let mut context: *mut TransportContext = std::ptr::null_mut();

            let result = transport_create(b"127.0.0.1:0\0".as_ptr() as *const c_char, &mut context);
            assert_eq!(result, TransportError::Success as c_int);
            assert!(!context.is_null());

            transport_destroy(context);
        }
    }
}
