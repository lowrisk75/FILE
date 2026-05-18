//! FFI exports for VFS anti-ransomware guard

use std::ffi::{c_char, c_double, c_int};
use std::slice;

/// FFI error codes
#[repr(C)]
pub enum GuardError {
    Success = 0,
    InvalidInput = -1,
    HighEntropyDetected = -2,
    ConfigError = -3,
}

/// Calculate Shannon entropy of byte buffer
///
/// # Safety
/// `data` must be valid for `len` bytes
///
/// # Returns
/// Entropy in bits per byte (0.0 to 8.0), or -1.0 on error
#[no_mangle]
pub unsafe extern "C" fn vfs_calculate_entropy(
    data: *const u8,
    len: usize,
    use_simd: bool,
) -> c_double {
    if data.is_null() || len == 0 {
        return -1.0;
    }

    let slice = slice::from_raw_parts(data, len);
    crate::entropy::shannon_entropy(slice, use_simd)
}

/// Check if entropy exceeds threshold (ransomware detection)
///
/// # Safety
/// `data` must be valid for `len` bytes
///
/// # Returns
/// - 1 if entropy > threshold (potential ransomware)
/// - 0 if safe
/// - -1 on error
#[no_mangle]
pub unsafe extern "C" fn vfs_is_high_entropy(
    data: *const u8,
    len: usize,
    threshold: c_double,
    use_simd: bool,
) -> c_int {
    if data.is_null() || len == 0 {
        return -1;
    }

    let slice = slice::from_raw_parts(data, len);
    if crate::entropy::is_high_entropy(slice, threshold, use_simd) {
        1
    } else {
        0
    }
}

/// Check if file extension is whitelisted
///
/// # Safety
/// `path` must be a null-terminated C string
///
/// # Returns
/// - 1 if whitelisted (e.g., .zip, .jpg)
/// - 0 if not whitelisted
/// - -1 on error
#[no_mangle]
pub unsafe extern "C" fn vfs_is_extension_whitelisted(
    path: *const c_char,
) -> c_int {
    if path.is_null() {
        return -1;
    }

    // Convert C string to Rust
    let c_str = match std::ffi::CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let path = std::path::Path::new(c_str);
    let config = crate::config::GuardConfig::default();

    if config.is_whitelisted(path) {
        1
    } else {
        0
    }
}

/// Full VFS guard check (size + whitelist + entropy)
///
/// # Safety
/// - `path` must be a null-terminated C string
/// - `data` must be valid for `len` bytes
///
/// # Returns
/// - `GuardError::Success` if write is safe
/// - `GuardError::HighEntropyDetected` if potential ransomware
#[no_mangle]
pub unsafe extern "C" fn vfs_check_write(
    path: *const c_char,
    data: *const u8,
    len: usize,
) -> c_int {
    if path.is_null() || data.is_null() {
        return GuardError::InvalidInput as c_int;
    }

    // Convert C string to Rust
    let c_str = match std::ffi::CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return GuardError::InvalidInput as c_int,
    };

    let path = std::path::Path::new(c_str);
    let slice = slice::from_raw_parts(data, len);
    let config = crate::config::GuardConfig::default();

    // Size check
    if len < config.min_file_size {
        return GuardError::Success as c_int;
    }

    // Whitelist check
    if config.is_whitelisted(path) {
        return GuardError::Success as c_int;
    }

    // Sample large files
    let sample = if len > config.max_file_size {
        &slice[..config.max_file_size]
    } else {
        slice
    };

    // Entropy check
    let entropy = crate::entropy::shannon_entropy(sample, config.enable_simd);
    if entropy > config.entropy_threshold {
        return GuardError::HighEntropyDetected as c_int;
    }

    GuardError::Success as c_int
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entropy_calculation() {
        unsafe {
            // Low entropy (repeated bytes)
            let low = vec![0xAAu8; 1024];
            let entropy = vfs_calculate_entropy(low.as_ptr(), low.len(), false);
            assert!(entropy >= 0.0 && entropy < 1.0);

            // High entropy (random-like)
            let mut high = Vec::new();
            let mut state = 0xDEADBEEFu32;
            for _ in 0..256 {
                state = state.wrapping_mul(1664525).wrapping_add(1013904223);
                high.push((state & 0xFF) as u8);
                high.push(((state >> 8) & 0xFF) as u8);
                high.push(((state >> 16) & 0xFF) as u8);
                high.push(((state >> 24) & 0xFF) as u8);
            }
            let entropy = vfs_calculate_entropy(high.as_ptr(), high.len(), false);
            assert!(entropy > 7.8);
        }
    }

    #[test]
    fn test_extension_whitelist() {
        unsafe {
            let zip = b"archive.zip\0";
            assert_eq!(vfs_is_extension_whitelisted(zip.as_ptr() as *const c_char), 1);

            let bin = b"encrypted.bin\0";
            assert_eq!(vfs_is_extension_whitelisted(bin.as_ptr() as *const c_char), 0);
        }
    }
}
