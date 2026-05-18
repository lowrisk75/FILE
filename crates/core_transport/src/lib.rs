//! AORATA Protocol v2 - Core Transport Layer
//!
//! Provides MPQUIC fabric with NAT traversal, Hash DoS mitigation,
//! and defensive validation for Zero Trust networking.
//!
//! # Modules
//! - `mpquic`: MPQUIC endpoint with SipHash-2-4 routing
//! - `nat_punch`: NAT hole punching (DCUtR, TTL manipulation)
//!
//! # Example
//! ```no_run
//! use core_transport::mpquic::MpQuicFabric;
//! use std::net::SocketAddr;
//!
//! #[tokio::main]
//! async fn main() -> std::io::Result<()> {
//!     let addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
//!     let fabric = MpQuicFabric::new(addr, 10_000).await?;
//!
//!     println!("AORATA MPQUIC listening on {}", fabric.local_addr);
//!     Ok(())
//! }
//! ```

pub mod mpquic;
pub mod nat_punch;

// Re-exports for convenience
pub use mpquic::{ConnectionId, MpQuicFabric, SipHashMap};

// FFI exports for Swift/Objective-C
pub mod ffi;
