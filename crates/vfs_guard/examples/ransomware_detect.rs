//! # VFS Guard Ransomware Detection Demo
//!
//! Demonstrates Shannon entropy-based ransomware detection.

use vfs_guard::{VfsGuard, GuardConfig};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== VFS Guard — Anti-Ransomware Detection Demo ===\n");

    // Initialize guard with default config
    let guard = VfsGuard::new(GuardConfig::default());
    let config = guard.config().await;

    println!("Configuration:");
    println!("  • Entropy threshold: {} bits/byte", config.entropy_threshold);
    println!("  • Min file size: {} bytes", config.min_file_size);
    println!("  • SIMD enabled: {}\n", config.enable_simd);

    // Test 1: Safe plaintext
    println!("Test 1: Safe plaintext");
    let plaintext = b"The quick brown fox jumps over the lazy dog. ".repeat(20);
    let entropy = guard.calculate_entropy(&plaintext).await;
    println!("  • Data: ASCII text (repeated)");
    println!("  • Entropy: {:.2} bits/byte", entropy);
    match guard.check_write(&PathBuf::from("document.txt"), &plaintext).await {
        Ok(()) => println!("  ✅ ALLOWED\n"),
        Err(e) => println!("  ❌ BLOCKED: {}\n", e),
    }

    // Test 2: Simulated ransomware (high entropy)
    println!("Test 2: Simulated ransomware encryption");
    let mut encrypted = Vec::new();
    let mut state = 0xDEADBEEFu32;
    for _ in 0..1280 {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        encrypted.push((state & 0xFF) as u8);
        encrypted.push(((state >> 8) & 0xFF) as u8);
        encrypted.push(((state >> 16) & 0xFF) as u8);
        encrypted.push(((state >> 24) & 0xFF) as u8);
    }
    let entropy = guard.calculate_entropy(&encrypted).await;
    println!("  • Data: Pseudo-random (simulated AES)");
    println!("  • Entropy: {:.2} bits/byte", entropy);
    match guard.check_write(&PathBuf::from("encrypted.bin"), &encrypted).await {
        Ok(()) => println!("  ✅ ALLOWED\n"),
        Err(e) => println!("  ❌ BLOCKED: {}\n", e),
    }

    // Test 3: Whitelisted high-entropy format
    println!("Test 3: Legitimate compressed file (ZIP)");
    let entropy = guard.calculate_entropy(&encrypted).await;
    println!("  • Data: Same high-entropy data");
    println!("  • Extension: .zip (whitelisted)");
    println!("  • Entropy: {:.2} bits/byte", entropy);
    match guard.check_write(&PathBuf::from("archive.zip"), &encrypted).await {
        Ok(()) => println!("  ✅ ALLOWED (whitelisted)\n"),
        Err(e) => println!("  ❌ BLOCKED: {}\n", e),
    }

    // Test 4: Small file bypass
    println!("Test 4: Small file (below threshold)");
    let small_encrypted = vec![0xFFu8; 100]; // < 4096 bytes
    let entropy = guard.calculate_entropy(&small_encrypted).await;
    println!("  • Size: {} bytes (< {} threshold)", small_encrypted.len(), config.min_file_size);
    println!("  • Entropy: {:.2} bits/byte", entropy);
    match guard.check_write(&PathBuf::from("small.bin"), &small_encrypted).await {
        Ok(()) => println!("  ✅ ALLOWED (too small to threat)\n"),
        Err(e) => println!("  ❌ BLOCKED: {}\n", e),
    }

    println!("=== Summary ===\n");
    println!("VFS Guard uses Shannon entropy (H = -Σ p(x) log₂ p(x)) to detect");
    println!("encryption at write-time. Ransomware encryption produces near-uniform");
    println!("byte distributions with entropy > 7.8 bits/byte.\n");

    println!("✅ Safe writes: plaintext, compressed (whitelisted), small files");
    println!("❌ Blocked writes: high-entropy non-whitelisted files\n");

    println!("✨ Demo complete!");

    Ok(())
}
