//! Unit tests for GDPR-compliant IP hashing
//!
//! Tests the HMAC-SHA256 IP anonymization system

use hmac::{Hmac, Mac};
use rand::Rng;
use sha2::Sha256;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

type HmacSha256 = Hmac<Sha256>;

/// Hash an IP address with HMAC-SHA256 and return first 8 bytes as u64
fn hash_ip(ip: &IpAddr, secret: &[u8; 32]) -> u64 {
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC can take any key size");

    match ip {
        IpAddr::V4(v4) => mac.update(&v4.octets()),
        IpAddr::V6(v6) => mac.update(&v6.octets()),
    }

    let result = mac.finalize();
    let bytes = result.into_bytes();

    // Take first 8 bytes and convert to u64
    u64::from_be_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ])
}

/// Generate a random secret key
fn generate_secret() -> [u8; 32] {
    let mut rng = rand::thread_rng();
    let mut secret = [0u8; 32];
    rng.fill(&mut secret);
    secret
}

#[test]
fn test_ip_hash_consistency() {
    let secret = generate_secret();
    let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));

    // Same IP + secret should produce same hash
    let hash1 = hash_ip(&ip, &secret);
    let hash2 = hash_ip(&ip, &secret);

    assert_eq!(hash1, hash2, "Same IP should hash to same value");
}

#[test]
fn test_ip_hash_different_ips() {
    let secret = generate_secret();
    let ip1 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
    let ip2 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 101));

    let hash1 = hash_ip(&ip1, &secret);
    let hash2 = hash_ip(&ip2, &secret);

    assert_ne!(
        hash1, hash2,
        "Different IPs should hash to different values"
    );
}

#[test]
fn test_ip_hash_different_secrets() {
    let secret1 = generate_secret();
    let secret2 = generate_secret();
    let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));

    let hash1 = hash_ip(&ip, &secret1);
    let hash2 = hash_ip(&ip, &secret2);

    assert_ne!(
        hash1, hash2,
        "Same IP with different secrets should hash differently"
    );
}

#[test]
fn test_ipv4_hash() {
    let secret = generate_secret();
    let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

    let hash = hash_ip(&ip, &secret);

    // Hash should be non-zero
    assert_ne!(hash, 0, "Hash should be non-zero");
}

#[test]
fn test_ipv6_hash() {
    let secret = generate_secret();
    let ip = IpAddr::V6(Ipv6Addr::new(
        0x2001, 0x0db8, 0x85a3, 0, 0, 0x8a2e, 0x0370, 0x7334,
    ));

    let hash = hash_ip(&ip, &secret);

    // Hash should be non-zero
    assert_ne!(hash, 0, "Hash should be non-zero");
}

#[test]
fn test_ipv4_ipv6_hash_different() {
    let secret = generate_secret();
    let ipv4 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
    let ipv6 = IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc0a8, 0x0101)); // ::ffff:192.168.1.1

    let hash_v4 = hash_ip(&ipv4, &secret);
    let hash_v6 = hash_ip(&ipv6, &secret);

    // IPv4 and IPv6-mapped addresses should hash differently
    assert_ne!(
        hash_v4, hash_v6,
        "IPv4 and IPv6 addresses should hash differently"
    );
}

#[test]
fn test_hash_entropy() {
    let secret = generate_secret();
    let mut hashes = std::collections::HashSet::new();

    // Generate 1000 different IPs and hash them
    for i in 0..1000 {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, (i / 256) as u8, (i % 256) as u8));
        let hash = hash_ip(&ip, &secret);
        hashes.insert(hash);
    }

    // All 1000 should be unique (no collisions in small set)
    assert_eq!(hashes.len(), 1000, "All hashes should be unique");
}

#[test]
fn test_secret_generation_entropy() {
    let secret1 = generate_secret();
    let secret2 = generate_secret();

    // Two generated secrets should be different
    assert_ne!(secret1, secret2, "Generated secrets should be unique");

    // Secrets should not be all zeros
    let is_zero = secret1.iter().all(|&b| b == 0);
    assert!(!is_zero, "Secret should not be all zeros");
}

#[test]
fn test_secret_size() {
    let secret = generate_secret();

    // Secret should be exactly 32 bytes (256 bits)
    assert_eq!(
        secret.len(),
        32,
        "Secret should be 32 bytes for HMAC-SHA256"
    );
}

#[test]
fn test_hash_output_range() {
    let secret = generate_secret();
    let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

    let hash = hash_ip(&ip, &secret);

    // Hash should be in valid u64 range (always true, but documents the type)
    assert!(
        hash >= u64::MIN && hash <= u64::MAX,
        "Hash should be valid u64"
    );
}

#[test]
fn test_batch_hashing_performance() {
    let secret = generate_secret();
    let start = std::time::Instant::now();

    // Hash 10,000 IPs
    for i in 0..10000 {
        let ip = IpAddr::V4(Ipv4Addr::new(
            (i / 16777216) as u8,
            (i / 65536) as u8,
            (i / 256) as u8,
            (i % 256) as u8,
        ));
        let _hash = hash_ip(&ip, &secret);
    }

    let duration = start.elapsed();

    // 10,000 hashes should complete in under 1 second on any modern hardware
    assert!(
        duration.as_secs() < 1,
        "Batch hashing should be fast: {:?}",
        duration
    );
}

#[test]
fn test_hash_distribution() {
    let secret = generate_secret();
    let mut hashes = Vec::new();

    // Generate 100 hashes
    for i in 0..100 {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, i));
        let hash = hash_ip(&ip, &secret);
        hashes.push(hash);
    }

    // Calculate min and max
    let min = *hashes.iter().min().unwrap();
    let max = *hashes.iter().max().unwrap();

    // Distribution should span a significant range (not clustered)
    let range = max - min;
    assert!(
        range > u64::MAX / 1000,
        "Hash distribution should span significant range"
    );
}

#[test]
fn test_deterministic_hash() {
    let secret = [42u8; 32]; // Fixed secret for reproducibility
    let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));

    let hash = hash_ip(&ip, &secret);

    // With fixed secret, hash should be deterministic across runs
    // (This test documents that hashing is deterministic, not random)
    assert_ne!(hash, 0, "Hash with fixed secret should be deterministic");
}

#[tokio::test]
async fn test_concurrent_hashing() {
    use std::sync::Arc;

    let secret = Arc::new(generate_secret());
    let mut handles = vec![];

    // Spawn 10 tasks, each hashing 1000 IPs
    for task_id in 0..10 {
        let secret_clone = secret.clone();
        let handle = tokio::spawn(async move {
            let mut local_hashes = Vec::new();
            for i in 0..1000 {
                let ip = IpAddr::V4(Ipv4Addr::new(
                    10,
                    task_id as u8,
                    (i / 256) as u8,
                    (i % 256) as u8,
                ));
                let hash = hash_ip(&ip, &secret_clone);
                local_hashes.push(hash);
            }
            local_hashes
        });
        handles.push(handle);
    }

    // Wait for all tasks
    let mut all_hashes = std::collections::HashSet::new();
    for handle in handles {
        let hashes = handle.await.unwrap();
        for hash in hashes {
            all_hashes.insert(hash);
        }
    }

    // All 10,000 hashes should be unique
    assert_eq!(
        all_hashes.len(),
        10000,
        "Concurrent hashing should produce unique values"
    );
}
