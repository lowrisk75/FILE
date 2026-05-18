//! Error types for VFS Guard

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during VFS Guard operations
#[derive(Debug, Error)]
pub enum GuardError {
    /// High entropy detected (potential ransomware)
    #[error("High entropy detected in {path}: {entropy:.2} bits/byte (threshold: {threshold:.2})")]
    HighEntropyDetected {
        /// File path
        path: PathBuf,
        /// Calculated entropy
        entropy: f64,
        /// Configured threshold
        threshold: f64,
    },

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Invalid file path
    #[error("Invalid file path: {0}")]
    InvalidPath(PathBuf),
}

/// Result type for VFS Guard operations
pub type Result<T> = std::result::Result<T, GuardError>;
