//! Main VFS Guard orchestrator

use crate::config::GuardConfig;
use crate::entropy::shannon_entropy;
use crate::error::{GuardError, Result};
use std::path::Path;
use tokio::sync::RwLock;

/// VFS Guard — Anti-Ransomware filesystem monitor
///
/// Intercepts file writes and blocks high-entropy content (potential ransomware).
pub struct VfsGuard {
    config: RwLock<GuardConfig>,
}

impl VfsGuard {
    /// Create new VFS Guard with configuration
    ///
    /// # Example
    ///
    /// ```
    /// use vfs_guard::{VfsGuard, GuardConfig};
    ///
    /// let config = GuardConfig::default();
    /// let guard = VfsGuard::new(config);
    /// ```
    pub fn new(config: GuardConfig) -> Self {
        Self {
            config: RwLock::new(config),
        }
    }

    /// Check if a file write should be allowed
    ///
    /// # Arguments
    /// - `path`: File path being written
    /// - `data`: File data to write
    ///
    /// # Returns
    /// - `Ok(())` if write is safe
    /// - `Err(GuardError::HighEntropyDetected)` if potential ransomware
    ///
    /// # Example
    ///
    /// ```
    /// use vfs_guard::{VfsGuard, GuardConfig};
    /// use std::path::PathBuf;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let guard = VfsGuard::new(GuardConfig::default());
    ///
    /// let safe_data = b"Hello, world!";
    /// assert!(guard.check_write(&PathBuf::from("test.txt"), safe_data).await.is_ok());
    ///
    /// // Simulate encrypted data (high entropy)
    /// let mut encrypted = Vec::new();
    /// let mut state = 0xDEADBEEFu32;
    /// for _ in 0..256 {
    ///     state = state.wrapping_mul(1664525).wrapping_add(1013904223);
    ///     encrypted.push((state & 0xFF) as u8);
    ///     encrypted.push(((state >> 8) & 0xFF) as u8);
    ///     encrypted.push(((state >> 16) & 0xFF) as u8);
    ///     encrypted.push(((state >> 24) & 0xFF) as u8);
    /// }
    /// assert!(guard.check_write(&PathBuf::from("encrypted.bin"), &encrypted).await.is_err());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn check_write(&self, path: &Path, data: &[u8]) -> Result<()> {
        let config = self.config.read().await;

        // Step 1: Size check
        if data.len() < config.min_file_size {
            // Too small to be ransomware threat
            return Ok(());
        }

        // Step 2: Whitelist check
        if config.is_whitelisted(path) {
            // Compressed/encrypted formats are OK
            return Ok(());
        }

        // Step 3: Sample large files
        let sample = if data.len() > config.max_file_size {
            // Sample first max_file_size bytes
            &data[..config.max_file_size]
        } else {
            data
        };

        // Step 4: Entropy calculation
        let entropy = shannon_entropy(sample, config.enable_simd);

        // Step 5: Threshold check
        if entropy > config.entropy_threshold {
            return Err(GuardError::HighEntropyDetected {
                path: path.to_path_buf(),
                entropy,
                threshold: config.entropy_threshold,
            });
        }

        Ok(())
    }

    /// Check if a file write would be blocked (non-async, for sync contexts)
    ///
    /// **Note**: This is a convenience wrapper that blocks the current thread.
    /// Prefer `check_write()` in async contexts.
    pub fn check_write_blocking(&self, path: &Path, data: &[u8]) -> Result<()> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            GuardError::ConfigError(format!("Failed to create runtime: {}", e))
        })?;

        rt.block_on(self.check_write(path, data))
    }

    /// Get current configuration (read-only)
    pub async fn config(&self) -> GuardConfig {
        self.config.read().await.clone()
    }

    /// Update configuration
    ///
    /// # Arguments
    /// - `new_config`: New configuration to apply
    ///
    /// # Errors
    /// Returns error if validation fails
    pub async fn update_config(&self, new_config: GuardConfig) -> Result<()> {
        new_config
            .validate()
            .map_err(GuardError::ConfigError)?;

        let mut config = self.config.write().await;
        *config = new_config;

        Ok(())
    }

    /// Calculate entropy of data without checking against threshold
    ///
    /// Useful for analysis/debugging.
    pub async fn calculate_entropy(&self, data: &[u8]) -> f64 {
        let config = self.config.read().await;
        shannon_entropy(data, config.enable_simd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_guard_allows_safe_write() {
        let guard = VfsGuard::new(GuardConfig::default());

        let safe_data = b"The quick brown fox jumps over the lazy dog.";
        let result = guard.check_write(&PathBuf::from("test.txt"), safe_data).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_guard_blocks_high_entropy() {
        let guard = VfsGuard::new(GuardConfig::default());

        // Simulate encrypted data
        let mut encrypted = Vec::new();
        let mut state = 0xCAFEBABEu32;
        for _ in 0..1024 {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            encrypted.push((state & 0xFF) as u8);
            encrypted.push(((state >> 8) & 0xFF) as u8);
            encrypted.push(((state >> 16) & 0xFF) as u8);
            encrypted.push(((state >> 24) & 0xFF) as u8);
        }

        let result = guard.check_write(&PathBuf::from("encrypted.bin"), &encrypted).await;

        assert!(result.is_err());
        match result {
            Err(GuardError::HighEntropyDetected { entropy, .. }) => {
                assert!(entropy > 7.8);
            }
            _ => panic!("Expected HighEntropyDetected error"),
        }
    }

    #[tokio::test]
    async fn test_guard_allows_whitelisted() {
        let guard = VfsGuard::new(GuardConfig::default());

        // High entropy data, but whitelisted extension
        let mut encrypted = Vec::new();
        let mut state = 0xDEADBEEFu32;
        for _ in 0..1024 {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            encrypted.push((state & 0xFF) as u8);
            encrypted.push(((state >> 8) & 0xFF) as u8);
            encrypted.push(((state >> 16) & 0xFF) as u8);
            encrypted.push(((state >> 24) & 0xFF) as u8);
        }

        let result = guard.check_write(&PathBuf::from("archive.zip"), &encrypted).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_guard_allows_small_files() {
        let guard = VfsGuard::new(GuardConfig::default());

        // High entropy but small file
        let small_encrypted = vec![0xAAu8; 100]; // < 4096 default min_file_size

        let result = guard.check_write(&PathBuf::from("small.bin"), &small_encrypted).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_guard_samples_large_files() {
        let mut config = GuardConfig::default();
        config.max_file_size = 1024; // Sample only first 1KB
        let guard = VfsGuard::new(config);

        // First 1KB is safe, rest is encrypted
        let mut data = vec![0x20u8; 1024]; // Safe ASCII space
        let mut state = 0x12345678u32;
        for _ in 0..1024 {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            data.push((state & 0xFF) as u8);
            data.push(((state >> 8) & 0xFF) as u8);
            data.push(((state >> 16) & 0xFF) as u8);
            data.push(((state >> 24) & 0xFF) as u8);
        }

        let result = guard.check_write(&PathBuf::from("partial.bin"), &data).await;

        // Should pass (only first 1KB checked, which is safe)
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_config() {
        let guard = VfsGuard::new(GuardConfig::default());

        let mut new_config = GuardConfig::default();
        new_config.entropy_threshold = 7.5;

        assert!(guard.update_config(new_config).await.is_ok());

        let config = guard.config().await;
        assert_eq!(config.entropy_threshold, 7.5);
    }

    #[tokio::test]
    async fn test_calculate_entropy() {
        let guard = VfsGuard::new(GuardConfig::default());

        let low_entropy = vec![0xAAu8; 1024];
        let entropy = guard.calculate_entropy(&low_entropy).await;
        assert!(entropy < 1.0);

        let mut high_entropy = Vec::new();
        let mut state = 0xBEEFCAFEu32;
        for _ in 0..256 {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            high_entropy.push((state & 0xFF) as u8);
            high_entropy.push(((state >> 8) & 0xFF) as u8);
            high_entropy.push(((state >> 16) & 0xFF) as u8);
            high_entropy.push(((state >> 24) & 0xFF) as u8);
        }
        let entropy = guard.calculate_entropy(&high_entropy).await;
        assert!(entropy > 7.8);
    }
}
