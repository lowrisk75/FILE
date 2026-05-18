pub mod aont;
/// Module crypto_fabric - Zero Trust Cryptography
///
/// Implémente les primitives cryptographiques:
/// - Noise Framework XXfallback (authentification 0-RTT avec fallback gracieux)
/// - AONT (All-Or-Nothing Transform) pour fragmentation asynchrone
/// - Hybrid AONT-Threshold (Shamir SSS 3-of-5 + Reed-Solomon erasure coding)
///
/// Spécifications complètes: NotebookLM "FILE Project" > crypto_fabric
pub mod noise;

pub use aont::{apply_aont_32bytes, encode_aorata_message, MPQUICPathPacket};
pub use noise::{NoiseHandshake, Transport};

// FFI exports for Swift/Objective-C
pub mod ffi;
