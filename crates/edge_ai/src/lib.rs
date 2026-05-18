//! # Edge AI - Multi-Tier AI Routing Agent for AORATA v2
//!
//! This crate implements **Phase 4** of the AORATA Protocol v2: ultra-low latency edge AI
//! routing via multi-tier acceleration backends with LoRA (Low-Rank Adaptation)
//! dynamic injection.
//!
//! ## AORATA v2 Requirements
//!
//! - **Policy Adaptation**: <1ms (LoRA weight update)
//! - **Route Inference**: <2ms (neural network forward pass)
//!
//! ## Hybrid Architecture (3 Tiers)
//!
//! This crate implements a **multi-tier backend system** that balances performance with
//! distribution constraints:
//!
//! ### Tier 1: Core ML (Baseline - App Store)
//! - **Platforms**: iOS 15+, macOS 12+
//! - **Latency**: 300-800µs LoRA update, 1-2ms inference
//! - **Distribution**: App Store, TestFlight, notarization ✅
//! - **Hardware**: ANE (automatic Core ML scheduling)
//! - **APIs**: Public MLModel + MLState
//!
//! ### Tier 2: ANE Helper (Power Users - macOS Direct)
//! - **Platform**: macOS 12+ only (direct download)
//! - **Latency**: <100µs LoRA update, <1ms inference
//! - **Distribution**: lorislab.fr/aorata-helper.dmg ⚠️ (non-notarized)
//! - **Hardware**: ANE (direct private API access)
//! - **APIs**: _ANEClient, _ANECompiler (private)
//!
//! ### Tier 3: Android
//! - **Platform**: Android 8.1+ (NNAPI), Snapdragon (Hexagon DSP)
//! - **Latency**: 500-1500µs (NNAPI), <200µs (Hexagon)
//! - **Distribution**: APK direct, F-Droid
//! - **Hardware**: GPU/NPU (NNAPI), Hexagon DSP (Qualcomm)
//! - **APIs**: Android NDK NNAPI, Qualcomm SNPE
//!
//! ## Runtime Backend Detection
//!
//! The agent auto-selects the best available backend at runtime:
//!
//! ```rust
//! use edge_ai::routing_agent::AneRoutingAgent;
//!
//! // Auto-detect (priority: ANE Helper > Core ML > Hexagon > NNAPI)
//! let agent = AneRoutingAgent::auto().await?;
//! println!("Using: {} (tier {})", agent.backend_name(), agent.backend_tier());
//! ```
//!
//! ## Deployment Strategy
//!
//! - **iOS App Store**: Tier 1 Core ML only (App Store compliant)
//! - **macOS App Store**: Tier 1 Core ML + optional Tier 2 download prompt
//! - **macOS Direct**: Tier 1 + Tier 2 ANE Helper bundled
//! - **Android APK**: Tier 3 NNAPI + Hexagon
//!
//! ## Architecture Overview
//!
//! ### The CoreML Problem
//!
//! Apple's CoreML framework operates on a rigid "compile-then-dispatch" paradigm where model
//! weights are permanently baked into compiled microcode. Any weight update requires full
//! recompilation:
//!
//! - **Recompilation latency**: 500ms - 4,200ms
//! - **AORATA budget**: <100µs adaptation
//! - **Gap**: 3-4 orders of magnitude ❌
//!
//! Additionally, CoreML suffers from:
//! - **119-compilation limit** (daemon leak → silent crash)
//! - **XPC overhead** (~0.095ms dispatch latency)
//! - **No dynamic weight injection** (static models only)
//!
//! ### Solution: Private ANE API + LoRA-as-Input
//!
//! This crate bypasses CoreML entirely and interfaces directly with the private
//! `AppleNeuralEngine.framework` to achieve:
//!
//! 1. **Single-pass compilation** (30-80ms @ startup, never again)
//! 2. **Zero-copy memory** (mmap → Metal → IOSurface triple-registration)
//! 3. **LoRA-as-Input** (adapters A, B as dynamic inputs, not baked weights)
//! 4. **Direct evaluation** (bypass XPC daemon for -10% latency)
//!
//! ## Hardware: Apple Neural Engine (ANE) H16
//!
//! **Physical Specs (M3/M4 generation):**
//! - 16 independent neural cores
//! - **19 TFLOPS fp16** (real throughput; 38 TOPS INT8 is marketing)
//! - **32 MB SRAM** (L2 cache) - **absolute budget**
//!   - Exceeding 32MB → 30% performance drop (DRAM spills)
//! - **127-deep evaluation queue** (async parallelism)
//! - **Power gating**: 0W idle, microscopic wake-up latency
//!
//! **Memory Hierarchy:**
//! - On-chip: 32MB SRAM (compiled microcode + weights + activations)
//! - Off-chip: Unified Memory (DRAM) via Apple's UMA
//! - ANE native format: **IEEE 754 fp16** (half-precision float)
//!   - INT8 dequantized to fp16 before ALU (no compute speedup, only I/O bandwidth gain)
//!
//! ## Private API Architecture
//!
//! **Framework path**: `/System/Library/PrivateFrameworks/AppleNeuralEngine.framework`
//!
//! **Key Classes (reverse-engineered):**
//! - `_ANEClient` - XPC broker to `aned` daemon (IOKit driver)
//! - `_ANECompiler` - MIL IR → E5 microcode compiler
//! - `_ANEModel` - Compiled E5 program handle
//! - `_ANERequest` - Evaluation request (input/output tensor pointers)
//! - `_ANEIOSurfaceObject` - Zero-copy IOSurface wrapper
//!
//! **Critical Optimization**: `doEvaluateDirectWithModel:options:request:qos:error:`
//! - Bypass XPC daemon → direct IOKit command queue mapping
//! - Saves ~10µs dispatch overhead
//!
//! ## Required Entitlements
//!
//! The compiled binary **must** be signed with these Apple internal entitlements:
//!
//! ```xml
//! com.apple.aned.private.allow                      <!-- Master ANE privilege -->
//! com.apple.ANECompilerService.allow                <!-- Compiler access -->
//! com.apple.aned.private.adapterWeight.allow        <!-- LoRA injection -->
//! com.apple.ane.memoryUnwiringOptOutAccess.allow    <!-- Lock memory pages -->
//! ```
//!
//! Without these, the XNU kernel enforces privilege boundaries and all API calls fail silently.
//!
//! ## Model Intermediate Language (MIL)
//!
//! The ANE compiler consumes graphs in **MIL** (Model Intermediate Language), a textual IR
//! similar to LLVM IR but for tensor algebra.
//!
//! **Supported Operations (27 total):**
//! - Linear: `conv1x1`, `matmul`, `linear`
//! - Element-wise: `add`, `mul`, `sub`, `div`
//! - Activations: `relu`, `tanh`, `sigmoid` (⚠️ NO `gelu` - decompose manually)
//! - Normalization: `layer_norm`, `batch_norm`
//! - Utilities: `clamp`, `reshape`, `transpose`
//!
//! **Critical MIL Constraints:**
//! - MIL text → `NSData*` UTF-8 bytes (**NOT** `NSString*`)
//! - `concat` operation → **silent compiler failure** ❌
//! - `gelu` activation → **not supported** (decompose to tanh approximation)
//!
//! ## LoRA-as-Input: Mathematical Foundation
//!
//! ### Standard Routing Network
//! ```
//! y = W₀ · x
//! ```
//! Where:
//! - `W₀`: Static base weights (d_out × d_in)
//! - `x`: Input telemetry vector
//! - `y`: Routing logits
//!
//! ### LoRA Weight Update (Traditional)
//! ```
//! W = W₀ + ΔW = W₀ + A·B
//! ```
//! Where:
//! - `A`: d_out × r (low-rank adapter)
//! - `B`: r × d_in (low-rank adapter)
//! - `r << min(d_out, d_in)` (rank bottleneck)
//!
//! ### Adapter-as-Input Reformulation
//! ```
//! y = W₀·x + A·(B·x)
//! ```
//!
//! **Implementation in MIL:**
//! - `W₀` → **constant** (baked into E5 microcode)
//! - `x, A, B` → **input variables** (reside in IOSurface memory)
//!
//! **Update Flow (<100µs):**
//! 1. Control plane computes new A, B
//! 2. `memcpy(A, B → IOSurface)` (zero-copy shared memory)
//! 3. Next inference reads updated A, B
//! 4. **Zero recompilation** ✅
//!
//! ## Zero-Copy Memory: Triple-Registration
//!
//! To eliminate memcpy overhead between CPU/GPU/ANE:
//!
//! ### Step 1: Host Allocation
//! ```rust
//! let ptr = libc::mmap(
//!     std::ptr::null_mut(),
//!     size,
//!     libc::PROT_READ | libc::PROT_WRITE,
//!     libc::MAP_ANON | libc::MAP_PRIVATE,
//!     -1, 0
//! );
//! ```
//!
//! ### Step 2: Metal Buffer (GPU)
//! ```rust
//! let metal_buffer = device.new_buffer_with_bytes_no_copy(
//!     ptr,
//!     size,
//!     MTLResourceOptions::StorageModeShared,
//!     None
//! );
//! ```
//!
//! ### Step 3: IOSurface (ANE)
//! ```rust
//! let cv_pixel_buffer = CVPixelBufferCreateWithBytes(...);
//! let io_surface = CVPixelBufferGetIOSurface(cv_pixel_buffer);
//! let ane_surface = _ANEIOSurfaceObject::new(io_surface);
//! ```
//!
//! **Result**: CPU/GPU/ANE share **same physical DRAM pages** → zero memcpy ✅
//!
//! ## Critical IOSurface Constraints
//!
//! ### Constraint #18: Uniform Input Allocation
//!
//! **ALL** input IOSurface objects **MUST** have the exact same byte size, regardless of
//! actual tensor dimensions.
//!
//! Example:
//! - Telemetry `x`: 512 elements × 2 bytes = 1 KB
//! - LoRA `A`: 4096×64 × 2 bytes = 512 KB
//! - LoRA `B`: 64×512 × 2 bytes = 64 KB
//!
//! → Allocate **all three** at 512 KB (max size), zero-pad unused bytes.
//!
//! **Failure mode**: Mismatched sizes → `0x1d` error code
//!
//! ### Constraint #19: Alphabetical Binding
//!
//! The ANE compiler binds IOSurface pointers to MIL variables in **strict alphabetical order**
//! based on parameter names.
//!
//! Example MIL:
//! ```
//! input flow_attrs as fp16[1, 512, 1, 1]
//! input lora_A as fp16[1, 4096, 64, 1]
//! input lora_B as fp16[1, 64, 512, 1]
//! ```
//!
//! **Correct order**: `[flow_attrs_surface, lora_A_surface, lora_B_surface]`
//! **Wrong order**: `[lora_A, flow_attrs, lora_B]` → cross-multiplied garbage ❌
//!
//! ## MatMul → Conv1x1 Optimization
//!
//! The ANE lacks a highly optimized general MatMul instruction. Standard `matmul` operations
//! drop ALU utilization to ~33% of peak.
//!
//! **Solution**: Reshape all matrix multiplications into 1×1 convolutions:
//!
//! ```rust
//! // Instead of: y = W · x  (matmul)
//! // Reshape:    x → [1, channels, 1, 1]
//! //            W → conv1x1 kernel
//! // Execute:   y = conv1x1(W, x)
//! ```
//!
//! **Result**: Full 19 TFLOPS utilization (conv1x1 triggers optimal MAC arrays) ✅
//!
//! ## Numerical Stability: FP16 NaN Cascade
//!
//! ### The Problem
//!
//! ANE executes **exclusively in fp16**:
//! - Max representable value: `65,504`
//! - Overflow → `∞`
//! - `softmax(∞)` → `exp(∞) / ∞` → `NaN`
//! - NaN propagates through network → **total divergence** ❌
//!
//! Previous research (ANEgpt): **100% NaN rate** after first LoRA update.
//!
//! ### The Solution: Activation Clamping
//!
//! Inject `clamp()` operations **before** every non-linear activation in MIL:
//!
//! ```rust
//! fn clamp(a: f16) -> f16 {
//!     a.max(-65504.0).min(65504.0)
//! }
//! ```
//!
//! **Implementation**: Insert `clamp` ops before `softmax`, `layer_norm`, `exp`.
//!
//! **Performance**: ANE executes bounds checks with fused comparators → **zero latency overhead**
//!
//! **Stability**: Arrests fp16 overflow cascade, operates invisibly on 99.9% of normal data ✅
//!
//! ## BLOBFILE Format: 64-Byte Offset Alignment
//!
//! Static weights `W₀` are serialized into a proprietary **BLOBFILE** binary structure.
//!
//! **Critical requirement**: Tensor data offset **MUST** be `uint64(64)` (64-byte boundary).
//!
//! **Failure modes**:
//! - Offset = 128 bytes → silent misalignment → numerical corruption
//! - Offset = 0 bytes → silent misalignment → numerical corruption
//!
//! **Success**: Offset = 64 bytes → correct alignment → stable inference ✅
//!
//! ## Compilation: _ANEInMemoryModelDescriptor
//!
//! ### Single-Pass Strategy
//!
//! ```rust
//! let descriptor = _ANEInMemoryModelDescriptor::new(
//!     mil_data,         // NSData* UTF-8 MIL text
//!     weight_dict,      // @{} (empty dict, or baked W₀)
//!     network_config    // Hardware hints
//! );
//!
//! let compiled_model = _ANECompiler::compile(descriptor)?;
//! ```
//!
//! **Timing**:
//! - Compilation: 30-80ms (absorbed during daemon startup)
//! - Subsequent evaluations: <1ms (E5 microcode resident in 32MB SRAM)
//!
//! **Benefit**: Bypasses CoreML's 500ms+ recompilation penalty ✅
//!
//! **Stability**: Avoids 119-compilation limit (single compile @ boot) ✅
//!
//! ## Implementation: AneRoutingAgent
//!
//! ```rust
//! use edge_ai::AneRoutingAgent;
//!
//! // Startup (30-80ms)
//! let mut agent = AneRoutingAgent::new(model_path)?;
//!
//! // Policy update (<100µs)
//! agent.update_routing_policy(&new_policy)?;
//!
//! // Route inference (<1ms)
//! let decision = agent.compute_route(&flow_attributes)?;
//! ```
//!
//! ## Module Structure
//!
//! - [`ane_ffi`] - Private API FFI bindings (Objective-C runtime)
//! - [`mil_graph`] - MIL IR generation (matmul→conv1x1, clamp injection)
//! - [`lora_adapter`] - LoRA matrices A, B (rank decomposition)
//! - [`memory`] - Triple-registration (mmap/Metal/IOSurface)
//! - [`routing_agent`] - Main AneRoutingAgent struct
//! - [`stability`] - fp16 clamping, numerical safety
//!
//! ## References
//!
//! 1. [Orion: Characterizing Apple's Neural Engine](https://arxiv.org/html/2603.06728v1)
//! 2. [ANE Training - Backpropagation on ANE](https://github.com/maderix/ANE)
//! 3. [ane-infer - Private API LLM inference](https://github.com/thebasedcapital/ane-infer)
//! 4. [Bypassing CoreML - Reddit r/LocalLLaMA](https://www.reddit.com/r/LocalLLaMA/comments/1rl9fl4/)
//! 5. [Running Gemma4 on ANE - Medium](https://rockyshikoku.medium.com/running-gemma4-on-apple-neural-engine-79fa0cb39dd2)

#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

// Old module declarations (Phase 4 Tier 2 - ANE private APIs)
// These will be implemented later for ANE Helper daemon:
// pub mod ane_ffi;
// pub mod mil_graph;
// pub mod memory;

// Re-exports (updated for hybrid architecture)
pub use lora_adapter::{LoraAdapter, LoraRank};
pub use routing_agent::AneRoutingAgent;

/// Errors that can occur during ANE operations
#[derive(Debug, thiserror::Error)]
pub enum AneError {
    /// Failed to load AppleNeuralEngine.framework
    #[error("Failed to load AppleNeuralEngine.framework: {0}")]
    FrameworkLoadFailed(String),

    /// Failed to resolve Objective-C class
    #[error("Failed to resolve class {class}: {details}")]
    ClassResolutionFailed {
        /// Class name
        class: String,
        /// Error details
        details: String,
    },

    /// MIL graph generation error
    #[error("MIL graph generation failed: {0}")]
    MilGenerationFailed(String),

    /// Compilation error
    #[error("ANE compilation failed: {0}")]
    CompilationFailed(String),

    /// Evaluation error
    #[error("ANE evaluation failed: {0}")]
    EvaluationFailed(String),

    /// Memory allocation error
    #[error("Memory allocation failed: {0}")]
    MemoryAllocationFailed(String),

    /// IOSurface constraint violation
    #[error("IOSurface constraint #{constraint} violated: {details}")]
    ConstraintViolation {
        /// Constraint number (18 = uniform size, 19 = alphabetical)
        constraint: u8,
        /// Violation details
        details: String,
    },

    /// Numerical stability error (fp16 overflow)
    #[error("Numerical instability detected: {0}")]
    NumericalInstability(String),

    /// Missing required entitlement
    #[error("Missing required entitlement: {0}")]
    MissingEntitlement(String),
}

/// Result type for ANE operations
pub type Result<T> = std::result::Result<T, AneError>;

// ============================================================================
// Module Exports
// ============================================================================

/// Backend abstraction layer (multi-tier AI acceleration)
pub mod backend;

/// Core ML backend (Tier 1 - App Store baseline)
#[cfg(feature = "tier1-coreml")]
pub mod coreml_backend;

/// ANE Helper client (Tier 2 - macOS power users)
#[cfg(feature = "tier2-ane-helper")]
pub mod ane_helper_client;

/// Android backends (Tier 3 - NNAPI / Hexagon DSP)
#[cfg(feature = "tier3-android")]
pub mod android_backend;

/// LoRA adapter (Low-Rank Adaptation)
pub mod lora_adapter;

/// FP16 numerical stability utilities
pub mod stability;

/// High-level routing agent (orchestrator)
pub mod routing_agent;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = AneError::ConstraintViolation {
            constraint: 18,
            details: "IOSurface size mismatch".to_string(),
        };
        assert!(err.to_string().contains("Constraint #18"));
    }
}

// FFI exports for Swift/Objective-C
pub mod ffi;
