//! # ANE Routing Agent
//!
//! High-level orchestrator for AORATA Protocol v2 Edge AI routing.
//!
//! ## Overview
//!
//! The routing agent dynamically selects the best available backend and uses
//! LoRA (Low-Rank Adaptation) to adjust routing decisions based on network context:
//!
//! - **TTL analysis**: Detect CGNAT depth, ISP hops
//! - **ASN scoring**: Prefer direct peering, avoid known-bad ASNs
//! - **Inter-arrival time (IAT)**: Detect congestion patterns
//! - **CPU load**: Adaptive complexity based on device state
//!
//! ## Usage
//!
//! ```rust
//! use edge_ai::routing_agent::AneRoutingAgent;
//! use edge_ai::backend::BackendType;
//!
//! // Auto-detect best backend
//! let mut agent = AneRoutingAgent::auto().await?;
//! println!("Using: {}", agent.backend_name());
//!
//! // Or force a specific backend
//! let mut agent = AneRoutingAgent::with_backend(BackendType::CoreML).await?;
//!
//! // Update routing model based on network context
//! let latency = agent.update_from_context(&context).await?;
//! println!("LoRA update: {:?}", latency);
//!
//! // Get path scores for routing decision
//! let scores = agent.score_paths(&paths).await?;
//! let best_path = scores.iter().max_by_key(|s| s.score).unwrap();
//! ```

use crate::backend::{detect_backend, AiBackend, BackendError, BackendType, Result};
use crate::lora_adapter::{LoraAdapter, LoraRank};
use half::f16;
use std::time::Duration;

/// Network context for LoRA adaptation
#[derive(Debug, Clone)]
pub struct NetworkContext {
    /// Average TTL of recent packets (detect CGNAT depth)
    pub avg_ttl: u8,

    /// Autonomous System Number of current path
    pub asn: u32,

    /// Inter-arrival time variance (µs) - congestion indicator
    pub iat_variance: u64,

    /// CPU load percentage (0-100)
    pub cpu_load: u8,

    /// Number of active MPQUIC paths
    pub num_paths: u8,
}

impl NetworkContext {
    /// Encode context as fp16 tensor for ANE input
    ///
    /// Layout: [ttl_norm, asn_norm, iat_norm, cpu_norm, paths_norm]
    pub fn to_tensor(&self) -> Vec<f16> {
        vec![
            f16::from_f32(self.avg_ttl as f32 / 255.0),
            f16::from_f32((self.asn % 100000) as f32 / 100000.0),
            f16::from_f32((self.iat_variance.min(10000) as f32) / 10000.0),
            f16::from_f32(self.cpu_load as f32 / 100.0),
            f16::from_f32(self.num_paths as f32 / 8.0),
        ]
    }
}

/// Path score for routing decision
#[derive(Debug, Clone)]
pub struct PathScore {
    pub path_id: u64,
    pub score: f32,
    pub latency_ms: f32,
    pub packet_loss: f32,
}

/// ANE Routing Agent
///
/// Orchestrates multi-tier AI backends for adaptive MPQUIC path selection.
pub struct AneRoutingAgent {
    backend: Box<dyn AiBackend>,
    backend_type: BackendType,
    lora_adapter: LoraAdapter,
}

impl AneRoutingAgent {
    /// Create agent with auto-detected backend
    ///
    /// Priority: ANE Helper > Core ML > Hexagon > NNAPI
    pub async fn auto() -> Result<Self> {
        let backend_type = detect_backend();
        Self::with_backend(backend_type).await
    }

    /// Create agent with specific backend
    pub async fn with_backend(backend_type: BackendType) -> Result<Self> {
        let backend: Box<dyn AiBackend> = match backend_type {
            #[cfg(all(target_os = "macos", feature = "tier2-ane-helper"))]
            BackendType::AneHelper => {
                Box::new(crate::ane_helper_client::AneHelperClient::connect().await?)
            }

            #[cfg(any(target_os = "macos", target_os = "ios"))]
            BackendType::CoreML => {
                use std::path::PathBuf;
                Box::new(crate::coreml_backend::CoreMlBackend::load(PathBuf::from(
                    "/tmp/aorata_routing.mlpackage",
                ))?)
            }

            #[cfg(target_os = "android")]
            BackendType::Nnapi => {
                Box::new(crate::android_backend::NnapiBackend::init(
                    "/data/local/tmp/aorata_routing.tflite".into(),
                )?)
            }

            #[cfg(all(target_os = "android", feature = "tier3-android"))]
            BackendType::HexagonDsp => {
                Box::new(crate::android_backend::HexagonBackend::init(
                    "/data/local/tmp/aorata_routing.dlc".into(),
                )?)
            }

            #[allow(unreachable_patterns)]
            _ => return Err(BackendError::UnsupportedPlatform),
        };

        // Initialize LoRA adapter (rank=64 recommended for AORATA)
        let lora_adapter = LoraAdapter::new(
            512, // d_out (routing score dimension)
            5,   // d_in (network context features)
            LoraRank::Rank64,
        );

        Ok(Self {
            backend,
            backend_type,
            lora_adapter,
        })
    }

    /// Update LoRA weights from network context
    ///
    /// Adjusts routing model based on current network conditions.
    ///
    /// # Returns
    /// Duration of the LoRA update operation
    pub async fn update_from_context(&mut self, context: &NetworkContext) -> Result<Duration> {
        // Compute LoRA matrices from context
        // In production, this would use a pre-trained encoder
        let (a, b) = self.compute_lora_from_context(context);

        // Update backend
        self.backend.update_lora(&a, &b).await
    }

    /// Compute LoRA matrices from network context
    ///
    /// **TODO**: Replace with trained encoder model
    fn compute_lora_from_context(&self, context: &NetworkContext) -> (Vec<f16>, Vec<f16>) {
        // Placeholder: Identity-like LoRA (no adaptation)
        // In production, this would be a trained neural network that maps
        // NetworkContext → (LoRA_A, LoRA_B)
        let _context_tensor = context.to_tensor();

        let mut a = self.lora_adapter.a.clone();
        let b = self.lora_adapter.b.clone();

        // Simple heuristic: scale LoRA by congestion indicator
        let scale = f16::from_f32(1.0 + (context.iat_variance as f32 / 10000.0));
        for val in a.iter_mut() {
            *val = *val * scale;
        }

        (a, b)
    }

    /// Score multiple paths for routing decision
    ///
    /// **TODO**: Real inference implementation
    #[cfg(feature = "async")]
    pub async fn score_paths(&self, _paths: &[PathCandidate]) -> Result<Vec<PathScore>> {
        // Placeholder
        Ok(vec![])
    }

    #[cfg(not(feature = "async"))]
    pub fn score_paths(&self, _paths: &[PathCandidate]) -> Result<Vec<PathScore>> {
        Ok(vec![])
    }

    /// Get backend name
    pub fn backend_name(&self) -> &str {
        self.backend.backend_name()
    }

    /// Get backend type
    pub fn backend_type(&self) -> BackendType {
        self.backend_type
    }

    /// Get backend tier
    pub fn backend_tier(&self) -> u8 {
        self.backend.backend_tier()
    }

    /// Run inference directly on backend
    ///
    /// Wrapper for backend.infer() to avoid exposing private backend field.
    pub async fn infer(&self, input: &[f16]) -> Result<(Vec<f16>, Duration)> {
        self.backend.infer(input).await
    }
}

/// Path candidate for scoring
#[derive(Debug, Clone)]
pub struct PathCandidate {
    pub path_id: u64,
    pub remote_addr: String,
    pub rtt_ms: f32,
    pub recent_loss: f32,
}

#[cfg(all(test, feature = "async"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_auto_detect() {
        let agent = AneRoutingAgent::auto().await;

        // Should succeed on supported platforms
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            let agent = agent.unwrap();
            println!("Detected: {}", agent.backend_name());
            assert!(agent.backend_tier() >= 1 && agent.backend_tier() <= 3);
        }
    }

    #[tokio::test]
    async fn test_context_encoding() {
        let context = NetworkContext {
            avg_ttl: 64,
            asn: 15169, // Google
            iat_variance: 500,
            cpu_load: 25,
            num_paths: 3,
        };

        let tensor = context.to_tensor();
        assert_eq!(tensor.len(), 5);

        // All values should be normalized [0, 1]
        for val in tensor {
            let f = val.to_f32();
            assert!(f >= 0.0 && f <= 1.0, "Value {} out of range", f);
        }
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    #[tokio::test]
    async fn test_update_from_context() {
        let mut agent = AneRoutingAgent::auto().await.unwrap();

        let context = NetworkContext {
            avg_ttl: 64,
            asn: 15169,
            iat_variance: 500,
            cpu_load: 25,
            num_paths: 3,
        };

        let latency = agent.update_from_context(&context).await.unwrap();

        println!("LoRA update latency: {:?}", latency);

        // Check backend-specific latency targets
        match agent.backend_type() {
            BackendType::CoreML => {
                assert!(latency.as_micros() >= 300 && latency.as_micros() <= 2000);
            }
            BackendType::AneHelper => {
                assert!(latency.as_micros() <= 100);
            }
            _ => {}
        }
    }
}
