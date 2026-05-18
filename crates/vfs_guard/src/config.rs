//! Configuration and whitelist management

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

/// VFS Guard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardConfig {
    /// Shannon entropy threshold (bits per byte)
    ///
    /// Default: 7.8 (standard ransomware detection threshold)
    /// - Plaintext: 3.5-6.0
    /// - Compressed: 7.0-7.5
    /// - Encrypted: 7.8-8.0
    pub entropy_threshold: f64,

    /// Whitelisted file extensions (bypass entropy check)
    ///
    /// These formats are legitimately high-entropy (compressed/encrypted):
    /// - Archives: .zip, .7z, .rar
    /// - Images: .jpg, .png, .webp
    /// - Video: .mp4, .mkv
    /// - Audio: .mp3, .aac
    pub whitelist_extensions: Vec<String>,

    /// Enable SIMD optimization (AVX2)
    ///
    /// 4x speedup on supported CPUs
    pub enable_simd: bool,

    /// Minimum file size to check (bytes)
    ///
    /// Files smaller than this are always allowed
    /// Default: 4096 (4 KB)
    pub min_file_size: usize,

    /// Maximum file size to check (bytes)
    ///
    /// Files larger than this are sampled
    /// Default: 10 MB
    pub max_file_size: usize,
}

impl Default for GuardConfig {
    fn default() -> Self {
        Self {
            entropy_threshold: 7.8,
            whitelist_extensions: vec![
                // Archives
                ".zip".into(),
                ".7z".into(),
                ".rar".into(),
                ".tar.gz".into(),
                ".tgz".into(),
                // Images
                ".jpg".into(),
                ".jpeg".into(),
                ".png".into(),
                ".webp".into(),
                ".avif".into(),
                // Video
                ".mp4".into(),
                ".mkv".into(),
                ".avi".into(),
                ".mov".into(),
                // Audio
                ".mp3".into(),
                ".aac".into(),
                ".opus".into(),
                ".flac".into(),
                // Encrypted/compiled
                ".gpg".into(),
                ".asc".into(),
                ".encrypted".into(),
                ".enc".into(),
                // Binaries (often compressed)
                ".exe".into(),
                ".dll".into(),
                ".so".into(),
                ".dylib".into(),
            ],
            enable_simd: cfg!(target_feature = "avx2"),
            min_file_size: 4096,
            max_file_size: 10 * 1024 * 1024, // 10 MB
        }
    }
}

impl GuardConfig {
    /// Check if a file path is whitelisted
    pub fn is_whitelisted(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext_lower = format!(".{}", ext.to_lowercase());
            self.whitelist_extensions.contains(&ext_lower)
        } else {
            false
        }
    }

    /// Get whitelist as HashSet for O(1) lookup
    pub fn whitelist_set(&self) -> HashSet<String> {
        self.whitelist_extensions.iter().cloned().collect()
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.entropy_threshold < 0.0 || self.entropy_threshold > 8.0 {
            return Err(format!(
                "entropy_threshold must be in [0.0, 8.0], got {}",
                self.entropy_threshold
            ));
        }

        if self.min_file_size > self.max_file_size {
            return Err(format!(
                "min_file_size ({}) > max_file_size ({})",
                self.min_file_size, self.max_file_size
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GuardConfig::default();
        assert_eq!(config.entropy_threshold, 7.8);
        assert!(config.whitelist_extensions.contains(&".zip".to_string()));
    }

    #[test]
    fn test_whitelist_check() {
        let config = GuardConfig::default();

        assert!(config.is_whitelisted(Path::new("archive.zip")));
        assert!(config.is_whitelisted(Path::new("photo.jpg")));
        assert!(config.is_whitelisted(Path::new("video.mp4")));

        assert!(!config.is_whitelisted(Path::new("document.docx")));
        assert!(!config.is_whitelisted(Path::new("text.txt")));
    }

    #[test]
    fn test_validate() {
        let mut config = GuardConfig::default();
        assert!(config.validate().is_ok());

        config.entropy_threshold = 10.0;
        assert!(config.validate().is_err());

        config.entropy_threshold = 7.8;
        config.min_file_size = 1000;
        config.max_file_size = 100;
        assert!(config.validate().is_err());
    }
}
