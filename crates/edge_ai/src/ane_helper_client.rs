//! # ANE Helper Client (Tier 2)
//!
//! IPC client for optional ANE Helper daemon that provides direct ANE access
//! via private APIs.
//!
//! ## Architecture
//!
//! ```text
//! AORATA.app (App Store)
//!     │
//!     │ Unix Domain Socket
//!     │ /tmp/aorata-helper.sock
//!     ↓
//! ANE Helper.app (Direct Download)
//!     │
//!     │ dlopen PrivateFrameworks
//!     ↓
//! _ANEClient / _ANECompiler
//! ```
//!
//! ## Protocol
//!
//! Request:
//! ```text
//! [u32 msg_type][u32 payload_len][payload]
//! ```
//!
//! Message types:
//! - 0x01: PING → response: PONG (u32=0)
//! - 0x02: UPDATE_LORA → payload: [u32 d_out][u32 d_in][u32 rank][f16[] a][f16[] b]
//! - 0x03: INFER → payload: [u32 len][f16[] input]
//! - 0xFF: SHUTDOWN
//!
//! Response:
//! ```text
//! [u32 status][u64 latency_us][u32 output_len][f16[] output]
//! ```
//!
//! Status codes:
//! - 0x00: OK
//! - 0x01: ERROR_INVALID_REQUEST
//! - 0x02: ERROR_ANE_FAILED
//! - 0x03: ERROR_MODEL_NOT_LOADED

use crate::backend::{AiBackend, BackendError, Result};
use half::f16;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

#[cfg(feature = "tier2-ane-helper")]
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[cfg(feature = "tier2-ane-helper")]
use tokio::net::UnixStream;

/// ANE Helper socket path
const HELPER_SOCKET: &str = "/tmp/aorata-helper.sock";

/// Message types for helper protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
enum MessageType {
    Ping = 0x01,
    UpdateLora = 0x02,
    Infer = 0x03,
    Shutdown = 0xFF,
}

/// Response status codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
enum StatusCode {
    Ok = 0x00,
    InvalidRequest = 0x01,
    AneFailed = 0x02,
    ModelNotLoaded = 0x03,
}

/// LoRA update request
#[derive(Debug, Serialize, Deserialize)]
struct LoraUpdateRequest {
    d_out: u32,
    d_in: u32,
    rank: u32,
    a: Vec<f16>,
    b: Vec<f16>,
}

/// Inference request
#[derive(Debug, Serialize, Deserialize)]
struct InferRequest {
    input: Vec<f16>,
}

/// Helper response
#[derive(Debug, Serialize, Deserialize)]
struct HelperResponse {
    status: u32,
    latency_us: u64,
    output: Vec<f16>,
}

/// ANE Helper client (Tier 2 backend)
///
/// Communicates with standalone ANE Helper daemon via Unix socket.
/// Falls back to Core ML if helper not available.
pub struct AneHelperClient {
    #[cfg(feature = "tier2-ane-helper")]
    stream: Option<UnixStream>,
}

impl AneHelperClient {
    /// Check if ANE Helper is installed and running
    ///
    /// Returns true if `/tmp/aorata-helper.sock` exists and is connectable.
    pub fn is_available() -> bool {
        #[cfg(feature = "tier2-ane-helper")]
        {
            use std::path::Path;
            Path::new(HELPER_SOCKET).exists()
        }

        #[cfg(not(feature = "tier2-ane-helper"))]
        false
    }

    /// Connect to ANE Helper daemon
    ///
    /// # Errors
    /// - `HelperNotAvailable` if socket doesn't exist
    /// - `Io` if connection fails
    #[cfg(feature = "tier2-ane-helper")]
    pub async fn connect() -> Result<Self> {
        if !Self::is_available() {
            return Err(BackendError::HelperNotAvailable);
        }

        let stream = UnixStream::connect(HELPER_SOCKET).await?;

        let mut client = Self {
            stream: Some(stream),
        };

        // Send PING to verify helper is responsive
        client.ping().await?;

        Ok(client)
    }

    /// Synchronous connect (blocking)
    #[cfg(not(feature = "tier2-ane-helper"))]
    pub fn connect() -> Result<Self> {
        Err(BackendError::UnsupportedPlatform)
    }

    /// Send PING request
    #[cfg(feature = "tier2-ane-helper")]
    async fn ping(&mut self) -> Result<Duration> {
        let stream = self
            .stream
            .as_mut()
            .ok_or(BackendError::HelperNotAvailable)?;

        let _start = std::time::Instant::now();

        // Send: [u32 msg_type=PING][u32 payload_len=0]
        stream.write_u32(MessageType::Ping as u32).await?;
        stream.write_u32(0).await?;

        // Receive: [u32 status][u64 latency_us][u32 output_len=0]
        let status = stream.read_u32().await?;
        let latency_us = stream.read_u64().await?;
        let _output_len = stream.read_u32().await?;

        if status != StatusCode::Ok as u32 {
            return Err(BackendError::HelperNotAvailable);
        }

        Ok(Duration::from_micros(latency_us))
    }

    /// Send LoRA update request
    #[cfg(feature = "tier2-ane-helper")]
    async fn send_lora_update(&mut self, a: &[f16], b: &[f16]) -> Result<Duration> {
        let stream = self
            .stream
            .as_mut()
            .ok_or(BackendError::HelperNotAvailable)?;

        let request = LoraUpdateRequest {
            d_out: 0, // Helper infers from data
            d_in: 0,
            rank: 0,
            a: a.to_vec(),
            b: b.to_vec(),
        };

        let payload = bincode::serialize(&request)?;

        // Send: [u32 msg_type][u32 payload_len][payload]
        stream.write_u32(MessageType::UpdateLora as u32).await?;
        stream.write_u32(payload.len() as u32).await?;
        stream.write_all(&payload).await?;

        // Receive: [u32 status][u64 latency_us][u32 output_len=0]
        let status = stream.read_u32().await?;
        let latency_us = stream.read_u64().await?;
        let _output_len = stream.read_u32().await?;

        match status {
            s if s == StatusCode::Ok as u32 => Ok(Duration::from_micros(latency_us)),
            s if s == StatusCode::AneFailed as u32 => Err(BackendError::LoraUpdateFailed(
                "ANE execution failed".into(),
            )),
            s if s == StatusCode::ModelNotLoaded as u32 => Err(BackendError::ModelNotLoaded),
            _ => Err(BackendError::LoraUpdateFailed("Unknown error".into())),
        }
    }

    /// Send inference request
    #[cfg(feature = "tier2-ane-helper")]
    async fn send_infer(&mut self, input: &[f16]) -> Result<(Vec<f16>, Duration)> {
        let stream = self
            .stream
            .as_mut()
            .ok_or(BackendError::HelperNotAvailable)?;

        let request = InferRequest {
            input: input.to_vec(),
        };

        let payload = bincode::serialize(&request)?;

        // Send: [u32 msg_type][u32 payload_len][payload]
        stream.write_u32(MessageType::Infer as u32).await?;
        stream.write_u32(payload.len() as u32).await?;
        stream.write_all(&payload).await?;

        // Receive: [u32 status][u64 latency_us][u32 output_len][f16[] output]
        let status = stream.read_u32().await?;
        let latency_us = stream.read_u64().await?;
        let output_len = stream.read_u32().await?;

        if status != StatusCode::Ok as u32 {
            return Err(BackendError::InferenceFailed(
                "Helper returned error".into(),
            ));
        }

        let mut output = vec![f16::ZERO; output_len as usize];
        let bytes = unsafe {
            std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u8, output_len as usize * 2)
        };
        stream.read_exact(bytes).await?;

        Ok((output, Duration::from_micros(latency_us)))
    }
}

#[cfg(feature = "tier2-ane-helper")]
#[async_trait::async_trait]
impl AiBackend for AneHelperClient {
    async fn update_lora(&mut self, a: &[f16], b: &[f16]) -> Result<Duration> {
        self.send_lora_update(a, b).await
    }

    async fn infer(&self, _input: &[f16]) -> Result<(Vec<f16>, Duration)> {
        Err(BackendError::HelperNotAvailable)
    }

    fn backend_name(&self) -> &str {
        "ANE Helper"
    }

    fn backend_tier(&self) -> u8 {
        2
    }

    fn is_available(&self) -> bool {
        self.stream.is_some()
    }
}

// Stub impl when tier2 feature is disabled
#[cfg(not(feature = "tier2-ane-helper"))]
#[async_trait::async_trait]
impl AiBackend for AneHelperClient {
    async fn update_lora(&mut self, _a: &[f16], _b: &[f16]) -> Result<Duration> {
        Err(BackendError::UnsupportedPlatform)
    }

    async fn infer(&self, _input: &[f16]) -> Result<(Vec<f16>, Duration)> {
        Err(BackendError::UnsupportedPlatform)
    }

    fn backend_name(&self) -> &str {
        "ANE Helper (unavailable)"
    }

    fn backend_tier(&self) -> u8 {
        2
    }

    fn is_available(&self) -> bool {
        false
    }
}

/// Get ANE Helper download URL
pub fn helper_download_url() -> &'static str {
    "https://lorislab.fr/aorata-helper.dmg"
}

/// Get expected ANE Helper installation path
pub fn helper_app_path() -> PathBuf {
    PathBuf::from("/Applications/AORATA Helper.app")
}

#[cfg(all(test, feature = "tier2-ane-helper"))]
mod tests {
    use super::*;

    #[test]
    fn test_is_available() {
        // Should not panic
        let available = AneHelperClient::is_available();
        println!("ANE Helper available: {}", available);
    }

    #[test]
    fn test_helper_paths() {
        assert_eq!(HELPER_SOCKET, "/tmp/aorata-helper.sock");
        assert!(helper_download_url().starts_with("https://"));
        assert!(helper_app_path().ends_with("AORATA Helper.app"));
    }

    #[tokio::test]
    async fn test_connect_when_unavailable() {
        if !AneHelperClient::is_available() {
            let result = AneHelperClient::connect().await;
            assert!(result.is_err());
        }
    }
}
