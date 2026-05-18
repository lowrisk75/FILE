/// Module relay_daemon - CAPoW Anti-DDoS Asymétrique
///
/// Implémentation complète du système de Proof-of-Work asymétrique:
/// - MTP-Argon2id (Merkle Tree Proof sur Argon2id DAG)
/// - GBP (Generalized Birthday Problem via Wagner's Algorithm)
/// - ML Context Scoring (TTL, ASN, IAT, CPU saturation)
/// - Tokio/Rayon isolation stricte (async I/O + sync CPU)
///
/// Architecture:
/// - Client: Génère 2GB Argon2id + résout k-XOR (5-30s)
/// - Relay: Vérifie avec 64MB + multi-proof (<100µs)
///
/// Basé sur recherche "Rust PoW Asymétrique _ Recherche Approfondie.md"

pub mod capow;
pub mod async_verify;
pub mod s3_fifo; // Phase future (Store-and-Forward)

// Exports publics principaux
pub use capow::{
    Block1024, CapowError, GbpNode, MtpProof, NetworkFeatures, Sha256Algorithm,
    build_mtp_tree, compute_context_score, dag_to_block1024, generate_argon2_dag,
    generate_mtp_proof, map_score_to_difficulty, verify_gbp_solution, wagner_algorithm,
    CLIENT_LANES, CLIENT_MEM_KB, CLIENT_PASSES, GBP_COLLISION_BITS, GBP_K,
    RELAY_LANES, RELAY_MEM_KB, RELAY_PASSES,
};

pub use async_verify::{
    async_verify_proof, async_verify_proof_with_timeout, init_crypto_workers,
};

// FFI exports for Swift/Objective-C
pub mod ffi;
