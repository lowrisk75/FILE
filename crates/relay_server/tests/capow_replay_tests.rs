//! Unit tests for CAPoW proof replay prevention
//!
//! Tests the SHA-256 proof hashing and TTL-based replay cache

use std::time::{Duration, Instant};
use sha2::{Sha256, Digest};
use dashmap::DashMap;

/// Hash a CAPoW proof using SHA-256
fn hash_proof(proof: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(proof);
    let result = hasher.finalize();

    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

/// Replay cache entry with timestamp
#[derive(Debug, Clone)]
struct CacheEntry {
    timestamp: Instant,
}

impl CacheEntry {
    fn new() -> Self {
        Self {
            timestamp: Instant::now(),
        }
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        self.timestamp.elapsed() > ttl
    }
}

#[test]
fn test_proof_hash_consistency() {
    let proof = b"test_proof_data_12345";

    let hash1 = hash_proof(proof);
    let hash2 = hash_proof(proof);

    assert_eq!(hash1, hash2, "Same proof should hash to same value");
}

#[test]
fn test_proof_hash_different_proofs() {
    let proof1 = b"proof_1";
    let proof2 = b"proof_2";

    let hash1 = hash_proof(proof1);
    let hash2 = hash_proof(proof2);

    assert_ne!(hash1, hash2, "Different proofs should hash to different values");
}

#[test]
fn test_proof_hash_length() {
    let proof = b"any_proof";

    let hash = hash_proof(proof);

    // SHA-256 always produces 32-byte (256-bit) hash
    assert_eq!(hash.len(), 32, "Hash should be 32 bytes");
}

#[test]
fn test_proof_hash_empty_input() {
    let proof = b"";

    let hash = hash_proof(proof);

    // Empty proof should still produce valid hash
    assert_eq!(hash.len(), 32, "Empty proof should produce 32-byte hash");

    // Hash of empty input should be deterministic
    let hash2 = hash_proof(b"");
    assert_eq!(hash, hash2, "Empty proof hash should be deterministic");
}

#[test]
fn test_proof_hash_large_input() {
    // 10KB proof
    let proof = vec![42u8; 10 * 1024];

    let hash = hash_proof(&proof);

    // Large proof should still produce 32-byte hash
    assert_eq!(hash.len(), 32, "Large proof should produce 32-byte hash");
}

#[test]
fn test_cache_entry_creation() {
    let entry = CacheEntry::new();

    // Entry should have recent timestamp
    assert!(entry.timestamp.elapsed() < Duration::from_millis(10));
}

#[test]
fn test_cache_entry_expiry() {
    let mut entry = CacheEntry::new();

    // Fresh entry should not be expired with 5 minute TTL
    assert!(!entry.is_expired(Duration::from_secs(300)));

    // Manually set old timestamp
    entry.timestamp = Instant::now() - Duration::from_secs(400);

    // Old entry should be expired
    assert!(entry.is_expired(Duration::from_secs(300)));
}

#[test]
fn test_replay_cache_basic() {
    let cache: DashMap<[u8; 32], CacheEntry> = DashMap::new();
    let proof = b"test_proof";
    let hash = hash_proof(proof);

    // First submission should be new
    assert!(!cache.contains_key(&hash), "Cache should be empty initially");

    // Insert proof
    cache.insert(hash, CacheEntry::new());

    // Second submission should be detected as replay
    assert!(cache.contains_key(&hash), "Cache should contain proof after insertion");
}

#[tokio::test]
async fn test_replay_detection_timing() {
    let cache: DashMap<[u8; 32], CacheEntry> = DashMap::new();
    let proof = b"timing_test_proof";
    let hash = hash_proof(proof);

    // Insert proof
    cache.insert(hash, CacheEntry::new());

    // Immediate replay should be detected
    assert!(cache.contains_key(&hash), "Immediate replay should be detected");

    // Wait 100ms and check again
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(cache.contains_key(&hash), "Proof should still be cached after 100ms");
}

#[test]
fn test_cache_cleanup_expired_entries() {
    let cache: DashMap<[u8; 32], CacheEntry> = DashMap::new();
    let ttl = Duration::from_secs(300); // 5 minutes

    // Insert 10 proofs with old timestamps
    for i in 0..10u8 {
        let proof = vec![i; 32];
        let hash = hash_proof(&proof);
        let mut entry = CacheEntry::new();
        entry.timestamp = Instant::now() - Duration::from_secs(400); // 6+ minutes ago
        cache.insert(hash, entry);
    }

    // Insert 5 proofs with recent timestamps
    for i in 10..15u8 {
        let proof = vec![i; 32];
        let hash = hash_proof(&proof);
        cache.insert(hash, CacheEntry::new());
    }

    // Cleanup: remove expired entries
    cache.retain(|_hash, entry| !entry.is_expired(ttl));

    // Only 5 recent entries should remain
    assert_eq!(cache.len(), 5, "Cleanup should remove 10 expired entries, leave 5 recent");
}

#[tokio::test]
async fn test_concurrent_replay_detection() {
    use std::sync::Arc;

    let cache: Arc<DashMap<[u8; 32], CacheEntry>> = Arc::new(DashMap::new());
    let proof = b"concurrent_proof";
    let hash = hash_proof(proof);

    // Insert proof once
    cache.insert(hash, CacheEntry::new());

    let mut handles = vec![];

    // Spawn 100 tasks trying to check the same proof
    for _ in 0..100 {
        let cache_clone = cache.clone();
        let handle = tokio::spawn(async move {
            cache_clone.contains_key(&hash)
        });
        handles.push(handle);
    }

    // All 100 checks should detect the replay
    for handle in handles {
        let detected = handle.await.unwrap();
        assert!(detected, "All concurrent checks should detect replay");
    }
}

#[tokio::test]
async fn test_concurrent_insertions() {
    use std::sync::Arc;

    let cache: Arc<DashMap<[u8; 32], CacheEntry>> = Arc::new(DashMap::new());
    let mut handles = vec![];

    // Spawn 100 tasks, each inserting a unique proof
    for i in 0..100 {
        let cache_clone = cache.clone();
        let handle = tokio::spawn(async move {
            let proof = format!("proof_{}", i);
            let hash = hash_proof(proof.as_bytes());
            cache_clone.insert(hash, CacheEntry::new());
        });
        handles.push(handle);
    }

    // Wait for all insertions
    for handle in handles {
        handle.await.unwrap();
    }

    // Cache should contain exactly 100 entries
    assert_eq!(cache.len(), 100, "Concurrent insertions should all succeed");
}

#[test]
fn test_proof_hash_collision_resistance() {
    let mut hashes = std::collections::HashSet::new();

    // Generate 10,000 sequential proofs
    for i in 0..10000 {
        let proof = format!("proof_{}", i);
        let hash = hash_proof(proof.as_bytes());

        // Convert to Vec for HashSet storage
        let hash_vec: Vec<u8> = hash.to_vec();
        hashes.insert(hash_vec);
    }

    // All 10,000 should be unique (no collisions)
    assert_eq!(hashes.len(), 10000, "All proof hashes should be unique");
}

#[test]
fn test_cache_memory_efficiency() {
    let cache: DashMap<[u8; 32], CacheEntry> = DashMap::new();

    // Insert 1,000 proofs
    for i in 0..1000 {
        let proof = format!("memory_test_{}", i);
        let hash = hash_proof(proof.as_bytes());
        cache.insert(hash, CacheEntry::new());
    }

    // Cache should contain exactly 1,000 entries
    assert_eq!(cache.len(), 1000);

    // Each entry is small: 32 bytes (hash) + ~16 bytes (Instant) = ~48 bytes per entry
    // 1,000 entries ≈ 48KB (acceptable for replay prevention)
}

#[tokio::test]
async fn test_ttl_boundary_cases() {
    let ttl_5min = Duration::from_secs(300);

    // Entry at exactly TTL boundary
    let mut entry_exact = CacheEntry::new();
    entry_exact.timestamp = Instant::now() - Duration::from_secs(300);

    // Should be expired (> TTL, not >=)
    assert!(entry_exact.is_expired(ttl_5min), "Entry at exact TTL should be expired");

    // Entry just before TTL
    let mut entry_before = CacheEntry::new();
    entry_before.timestamp = Instant::now() - Duration::from_secs(299);

    // Should not be expired
    assert!(!entry_before.is_expired(ttl_5min), "Entry just before TTL should not be expired");
}

#[test]
fn test_proof_hash_deterministic() {
    let proof = b"deterministic_test";

    // Hash 100 times
    let mut hashes = Vec::new();
    for _ in 0..100 {
        let hash = hash_proof(proof);
        hashes.push(hash);
    }

    // All hashes should be identical
    let first = hashes[0];
    for hash in hashes.iter() {
        assert_eq!(*hash, first, "Proof hashing should be deterministic");
    }
}

#[tokio::test]
async fn test_cleanup_performance() {
    let cache: DashMap<[u8; 32], CacheEntry> = DashMap::new();
    let ttl = Duration::from_secs(300);

    // Insert 10,000 proofs (half old, half recent)
    for i in 0..10000 {
        let proof = format!("cleanup_perf_{}", i);
        let hash = hash_proof(proof.as_bytes());
        let mut entry = CacheEntry::new();

        if i < 5000 {
            // First half: old entries
            entry.timestamp = Instant::now() - Duration::from_secs(400);
        }

        cache.insert(hash, entry);
    }

    let start = Instant::now();

    // Cleanup expired entries
    cache.retain(|_hash, entry| !entry.is_expired(ttl));

    let duration = start.elapsed();

    // Cleanup of 10,000 entries should be fast (< 100ms)
    assert!(duration.as_millis() < 100, "Cleanup should be fast: {:?}", duration);

    // Verify correct count
    assert_eq!(cache.len(), 5000, "Cleanup should remove 5,000 old entries");
}
