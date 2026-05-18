//! FFI exports for Noise protocol handshake with real snow implementation
//!
//! C ABI bindings for AORATA Noise XXfallback using snow 0.10.0
//!
//! **Architecture:** Real Noise handshake with 0-RTT IK pattern
//! - NoiseHandshake manages state machine (IK/XX/XXfallback)
//! - Transport provides encrypt/decrypt after handshake complete
//! - Pure X25519 + ChaCha20-Poly1305 + SHA-256
//! - Stateless nonce management (AtomicU64)

use crate::noise::{NoiseHandshake, Transport};
use std::ffi::c_int;
use std::slice;
use std::sync::Mutex;

/// FFI error codes
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseError {
    Success = 0,
    InvalidInput = -1,
    HandshakeFailed = -2,
    EncryptionFailed = -3,
    DecryptionFailed = -4,
    StateError = -5,
}

/// Opaque Noise session handle
#[repr(C)]
pub struct NoiseSession {
    _private: [u8; 0],
}

/// Internal session state (either handshaking or transport mode)
enum NoiseSessionReal {
    /// Handshake in progress
    Handshake(Box<NoiseHandshake>),

    /// Handshake complete, transport mode active
    Transport(Transport),
}

/// Wrapper for FFI-safe access
struct NoiseSessionWrapper {
    /// Current state (handshake or transport)
    /// Wrapped in Option to allow take() during conversion
    state: Mutex<Option<NoiseSessionReal>>,

    /// Local static key (32 bytes)
    _local_static: [u8; 32],

    /// Remote static key (if known, for IK 0-RTT)
    _remote_static: Option<[u8; 32]>,
}

// ============================================================================
// SESSION LIFECYCLE
// ============================================================================

/// Initialize Noise XXfallback session (initiator)
///
/// Creates a NoiseHandshake in initiator mode:
/// - If `remote_static_key` is provided (32 bytes): IK pattern (0-RTT capable)
/// - If null: XX pattern (1.5 RTT)
///
/// # Safety
/// - `local_static_key` must be 32-byte buffer
/// - `remote_static_key` may be null (XX) or 32-byte buffer (IK)
/// - Caller must free with `noise_destroy`
#[no_mangle]
pub unsafe extern "C" fn noise_create_initiator(
    local_static_key: *const u8,
    remote_static_key: *const u8,
    out_session: *mut *mut NoiseSession,
) -> c_int {
    if local_static_key.is_null() || out_session.is_null() {
        return NoiseError::InvalidInput as c_int;
    }

    // Copy local static key (32 bytes)
    let local_slice = slice::from_raw_parts(local_static_key, 32);
    let mut local_static = [0u8; 32];
    local_static.copy_from_slice(local_slice);

    // Copy remote static key if provided (IK pattern)
    let remote_static = if !remote_static_key.is_null() {
        let remote_slice = slice::from_raw_parts(remote_static_key, 32);
        let mut remote = [0u8; 32];
        remote.copy_from_slice(remote_slice);
        Some(remote)
    } else {
        None
    };

    // Create NoiseHandshake (initiator)
    let handshake = match NoiseHandshake::new_initiator(
        &local_static,
        remote_static.as_ref().map(|r| r.as_slice())
    ) {
        Ok(hs) => hs,
        Err(_) => return NoiseError::HandshakeFailed as c_int,
    };

    // Wrap in session
    let wrapper = Box::new(NoiseSessionWrapper {
        state: Mutex::new(Some(NoiseSessionReal::Handshake(Box::new(handshake)))),
        _local_static: local_static,
        _remote_static: remote_static,
    });

    *out_session = Box::into_raw(wrapper) as *mut NoiseSession;
    NoiseError::Success as c_int
}

/// Initialize Noise XXfallback session (responder)
///
/// Creates a NoiseHandshake in responder mode (XX pattern always).
///
/// # Safety
/// - `local_static_key` must be 32-byte buffer
/// - Caller must free with `noise_destroy`
#[no_mangle]
pub unsafe extern "C" fn noise_create_responder(
    local_static_key: *const u8,
    out_session: *mut *mut NoiseSession,
) -> c_int {
    if local_static_key.is_null() || out_session.is_null() {
        return NoiseError::InvalidInput as c_int;
    }

    // Copy local static key
    let local_slice = slice::from_raw_parts(local_static_key, 32);
    let mut local_static = [0u8; 32];
    local_static.copy_from_slice(local_slice);

    // Create NoiseHandshake (responder)
    let handshake = match NoiseHandshake::new_responder(&local_static) {
        Ok(hs) => hs,
        Err(_) => return NoiseError::HandshakeFailed as c_int,
    };

    // Wrap in session
    let wrapper = Box::new(NoiseSessionWrapper {
        state: Mutex::new(Some(NoiseSessionReal::Handshake(Box::new(handshake)))),
        _local_static: local_static,
        _remote_static: None,
    });

    *out_session = Box::into_raw(wrapper) as *mut NoiseSession;
    NoiseError::Success as c_int
}

/// Destroy Noise session
///
/// # Safety
/// Session must have been created by `noise_create_*`
#[no_mangle]
pub unsafe extern "C" fn noise_destroy(session: *mut NoiseSession) {
    if session.is_null() {
        return;
    }

    let _wrapper = Box::from_raw(session as *mut NoiseSessionWrapper);
    // Drop happens here (Mutex + NoiseHandshake/Transport dropped)
}

// ============================================================================
// HANDSHAKE OPERATIONS
// ============================================================================

/// Perform Noise handshake (write message)
///
/// Writes a handshake message with optional payload:
/// - Initiator: First message (-> e, es, s, ss) with 0-RTT payload if IK
/// - Responder: Response message (<- e, ee, se)
///
/// # Safety
/// - `payload` may be null if `payload_len` is 0
/// - `out_message` must have capacity >= 96 bytes (handshake overhead)
/// - `out_len` receives actual message length
#[no_mangle]
pub unsafe extern "C" fn noise_handshake_write(
    session: *mut NoiseSession,
    payload: *const u8,
    payload_len: usize,
    out_message: *mut u8,
    message_capacity: usize,
    out_len: *mut usize,
) -> c_int {
    if session.is_null() || out_message.is_null() || out_len.is_null() {
        return NoiseError::InvalidInput as c_int;
    }

    let wrapper = &*(session as *const NoiseSessionWrapper);
    let mut state_guard = match wrapper.state.lock() {
        Ok(g) => g,
        Err(_) => return NoiseError::StateError as c_int,
    };

    // Get handshake (error if already in transport mode or None)
    let handshake = match state_guard.as_mut() {
        Some(NoiseSessionReal::Handshake(hs)) => hs.as_mut(),
        _ => return NoiseError::StateError as c_int,
    };

    // Prepare payload
    let payload_slice = if !payload.is_null() && payload_len > 0 {
        slice::from_raw_parts(payload, payload_len)
    } else {
        &[]
    };

    // Try 0-RTT (only valid for initiator with IK pattern)
    let message = match handshake.try_0rtt(payload_slice) {
        Ok(msg) => msg,
        Err(_) => return NoiseError::HandshakeFailed as c_int,
    };

    // Copy to output buffer
    if message.len() > message_capacity {
        return NoiseError::InvalidInput as c_int;
    }
    let out_slice = slice::from_raw_parts_mut(out_message, message_capacity);
    out_slice[..message.len()].copy_from_slice(&message);
    *out_len = message.len();

    NoiseError::Success as c_int
}

/// Perform Noise handshake (read message)
///
/// Processes incoming handshake message and extracts payload.
/// Automatically handles fallback if 0-RTT fails.
///
/// # Safety
/// - `message` must be valid for `message_len` bytes
/// - `out_payload` must have capacity >= `message_len`
/// - `out_len` receives actual payload length
#[no_mangle]
pub unsafe extern "C" fn noise_handshake_read(
    session: *mut NoiseSession,
    message: *const u8,
    message_len: usize,
    out_payload: *mut u8,
    payload_capacity: usize,
    out_len: *mut usize,
) -> c_int {
    if session.is_null() || message.is_null() || out_payload.is_null() || out_len.is_null() {
        return NoiseError::InvalidInput as c_int;
    }

    let wrapper = &*(session as *const NoiseSessionWrapper);
    let mut state_guard = match wrapper.state.lock() {
        Ok(g) => g,
        Err(_) => return NoiseError::StateError as c_int,
    };

    // Get handshake
    let handshake = match state_guard.as_mut() {
        Some(NoiseSessionReal::Handshake(hs)) => hs.as_mut(),
        _ => return NoiseError::StateError as c_int,
    };

    // Process incoming (handles fallback automatically)
    let message_slice = slice::from_raw_parts(message, message_len);
    let payload = match handshake.process_incoming_or_fallback(message_slice) {
        Ok(p) => p,
        Err(_) => return NoiseError::HandshakeFailed as c_int,
    };

    // Copy payload to output
    if payload.len() > payload_capacity {
        return NoiseError::InvalidInput as c_int;
    }
    let out_slice = slice::from_raw_parts_mut(out_payload, payload_capacity);
    out_slice[..payload.len()].copy_from_slice(&payload);
    *out_len = payload.len();

    NoiseError::Success as c_int
}

/// Check if handshake is complete and transition to transport mode
///
/// Returns true if ready for encrypt/decrypt operations.
/// Automatically converts NoiseHandshake → Transport.
///
/// # Safety
/// Session must be valid
#[no_mangle]
pub unsafe extern "C" fn noise_is_handshake_complete(session: *mut NoiseSession) -> bool {
    if session.is_null() {
        return false;
    }

    let wrapper = &*(session as *const NoiseSessionWrapper);
    let mut state_guard = match wrapper.state.lock() {
        Ok(g) => g,
        Err(_) => return false,
    };

    // Check if already in transport mode
    match state_guard.as_ref() {
        Some(NoiseSessionReal::Transport(_)) => return true,
        Some(NoiseSessionReal::Handshake(_)) => {}
        None => return false,
    }

    // Extract handshake (take() replaces with None)
    let handshake = match state_guard.take() {
        Some(NoiseSessionReal::Handshake(hs)) => *hs,
        _ => return false,
    };

    // Try to convert
    match handshake.into_transport() {
        Ok(transport) => {
            *state_guard = Some(NoiseSessionReal::Transport(transport));
            true
        }
        Err(_) => {
            // Handshake not ready yet
            false
        }
    }
}

// ============================================================================
// TRANSPORT OPERATIONS (after handshake complete)
// ============================================================================

/// Encrypt data with Noise transport
///
/// Uses Transport::encrypt_packet() with automatic nonce management.
/// Output includes 16-byte Poly1305 tag.
///
/// # Safety
/// - Session must have completed handshake
/// - `plaintext` must be valid for `plaintext_len` bytes
/// - `ciphertext` must have capacity for `plaintext_len + 16` (AEAD tag)
#[no_mangle]
pub unsafe extern "C" fn noise_encrypt(
    session: *mut NoiseSession,
    plaintext: *const u8,
    plaintext_len: usize,
    ciphertext: *mut u8,
    ciphertext_capacity: usize,
    out_len: *mut usize,
) -> c_int {
    if session.is_null() || plaintext.is_null() || ciphertext.is_null() || out_len.is_null() {
        return NoiseError::InvalidInput as c_int;
    }

    let wrapper = &*(session as *const NoiseSessionWrapper);
    let state_guard = match wrapper.state.lock() {
        Ok(g) => g,
        Err(_) => return NoiseError::StateError as c_int,
    };

    // Get transport (error if still in handshake or None)
    let transport = match state_guard.as_ref() {
        Some(NoiseSessionReal::Transport(t)) => t,
        _ => return NoiseError::StateError as c_int,
    };

    // Encrypt
    let plain_slice = slice::from_raw_parts(plaintext, plaintext_len);
    let encrypted = match transport.encrypt_packet(plain_slice) {
        Ok(ct) => ct,
        Err(_) => return NoiseError::EncryptionFailed as c_int,
    };

    // Copy to output
    if encrypted.len() > ciphertext_capacity {
        return NoiseError::InvalidInput as c_int;
    }
    let out_slice = slice::from_raw_parts_mut(ciphertext, ciphertext_capacity);
    out_slice[..encrypted.len()].copy_from_slice(&encrypted);
    *out_len = encrypted.len();

    NoiseError::Success as c_int
}

/// Decrypt data with Noise transport
///
/// Uses Transport::decrypt_packet() with automatic nonce verification.
/// Input must include 16-byte Poly1305 tag.
///
/// # Safety
/// - Session must have completed handshake
/// - `ciphertext` must be valid for `ciphertext_len` bytes
/// - `plaintext` must have capacity for at least `ciphertext_len - 16`
#[no_mangle]
pub unsafe extern "C" fn noise_decrypt(
    session: *mut NoiseSession,
    ciphertext: *const u8,
    ciphertext_len: usize,
    plaintext: *mut u8,
    plaintext_capacity: usize,
    out_len: *mut usize,
) -> c_int {
    if session.is_null() || ciphertext.is_null() || plaintext.is_null() || out_len.is_null() {
        return NoiseError::InvalidInput as c_int;
    }

    let wrapper = &*(session as *const NoiseSessionWrapper);
    let state_guard = match wrapper.state.lock() {
        Ok(g) => g,
        Err(_) => return NoiseError::StateError as c_int,
    };

    // Get transport
    let transport = match state_guard.as_ref() {
        Some(NoiseSessionReal::Transport(t)) => t,
        _ => return NoiseError::StateError as c_int,
    };

    // Extract nonce (first 8 bytes of ciphertext, big-endian u64)
    if ciphertext_len < 24 {
        return NoiseError::InvalidInput as c_int; // Too short (need nonce + tag + data)
    }
    let cipher_slice = slice::from_raw_parts(ciphertext, ciphertext_len);
    let nonce = u64::from_be_bytes(cipher_slice[0..8].try_into().unwrap());

    // Decrypt (nonce + AEAD tag handled by Transport)
    let decrypted = match transport.decrypt_packet(nonce, &cipher_slice[8..]) {
        Ok(pt) => pt,
        Err(_) => return NoiseError::DecryptionFailed as c_int,
    };

    // Copy to output
    if decrypted.len() > plaintext_capacity {
        return NoiseError::InvalidInput as c_int;
    }
    let out_slice = slice::from_raw_parts_mut(plaintext, plaintext_capacity);
    out_slice[..decrypted.len()].copy_from_slice(&decrypted);
    *out_len = decrypted.len();

    NoiseError::Success as c_int
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noise_handshake_initiator_responder() {
        unsafe {
            // Generate keypairs
            let initiator_static = [1u8; 32];
            let responder_static = [2u8; 32];

            // Create sessions
            let mut initiator: *mut NoiseSession = std::ptr::null_mut();
            let mut responder: *mut NoiseSession = std::ptr::null_mut();

            let result = noise_create_initiator(
                initiator_static.as_ptr(),
                std::ptr::null(), // XX pattern (no remote key)
                &mut initiator,
            );
            assert_eq!(result, NoiseError::Success as c_int);

            let result = noise_create_responder(
                responder_static.as_ptr(),
                &mut responder,
            );
            assert_eq!(result, NoiseError::Success as c_int);

            // Cleanup
            noise_destroy(initiator);
            noise_destroy(responder);
        }
    }
}
