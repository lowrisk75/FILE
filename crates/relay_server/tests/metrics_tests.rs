//! Unit tests for metrics tracking
//!
//! Tests the RelayMetrics structure and counter operations

use std::sync::atomic::{AtomicU64, Ordering};

/// Relay metrics structure (copied from main.rs for testing)
#[derive(Debug)]
struct RelayMetrics {
    peers_registered: AtomicU64,
    capow_verified: AtomicU64,
    capow_rejected: AtomicU64,
    rate_limited: AtomicU64,
    packets_forwarded: AtomicU64,
    peers_timeout: AtomicU64,
}

impl RelayMetrics {
    fn new() -> Self {
        Self {
            peers_registered: AtomicU64::new(0),
            capow_verified: AtomicU64::new(0),
            capow_rejected: AtomicU64::new(0),
            rate_limited: AtomicU64::new(0),
            packets_forwarded: AtomicU64::new(0),
            peers_timeout: AtomicU64::new(0),
        }
    }
}

#[test]
fn test_metrics_initialization() {
    let metrics = RelayMetrics::new();

    assert_eq!(metrics.peers_registered.load(Ordering::Relaxed), 0);
    assert_eq!(metrics.capow_verified.load(Ordering::Relaxed), 0);
    assert_eq!(metrics.capow_rejected.load(Ordering::Relaxed), 0);
    assert_eq!(metrics.rate_limited.load(Ordering::Relaxed), 0);
    assert_eq!(metrics.packets_forwarded.load(Ordering::Relaxed), 0);
    assert_eq!(metrics.peers_timeout.load(Ordering::Relaxed), 0);
}

#[test]
fn test_metrics_increment() {
    let metrics = RelayMetrics::new();

    // Increment each counter
    metrics.peers_registered.fetch_add(1, Ordering::Relaxed);
    metrics.capow_verified.fetch_add(1, Ordering::Relaxed);
    metrics.capow_rejected.fetch_add(1, Ordering::Relaxed);
    metrics.rate_limited.fetch_add(1, Ordering::Relaxed);
    metrics.packets_forwarded.fetch_add(1, Ordering::Relaxed);
    metrics.peers_timeout.fetch_add(1, Ordering::Relaxed);

    // Verify all counters incremented
    assert_eq!(metrics.peers_registered.load(Ordering::Relaxed), 1);
    assert_eq!(metrics.capow_verified.load(Ordering::Relaxed), 1);
    assert_eq!(metrics.capow_rejected.load(Ordering::Relaxed), 1);
    assert_eq!(metrics.rate_limited.load(Ordering::Relaxed), 1);
    assert_eq!(metrics.packets_forwarded.load(Ordering::Relaxed), 1);
    assert_eq!(metrics.peers_timeout.load(Ordering::Relaxed), 1);
}

#[test]
fn test_metrics_multiple_increments() {
    let metrics = RelayMetrics::new();

    // Increment peers_registered 100 times
    for _ in 0..100 {
        metrics.peers_registered.fetch_add(1, Ordering::Relaxed);
    }

    assert_eq!(metrics.peers_registered.load(Ordering::Relaxed), 100);

    // Increment packets_forwarded 1000 times
    for _ in 0..1000 {
        metrics.packets_forwarded.fetch_add(1, Ordering::Relaxed);
    }

    assert_eq!(metrics.packets_forwarded.load(Ordering::Relaxed), 1000);
}

#[tokio::test]
async fn test_metrics_concurrent_increments() {
    use std::sync::Arc;

    let metrics = Arc::new(RelayMetrics::new());
    let mut handles = vec![];

    // Spawn 10 tasks, each incrementing peers_registered 100 times
    for _ in 0..10 {
        let metrics_clone = metrics.clone();
        let handle = tokio::spawn(async move {
            for _ in 0..100 {
                metrics_clone
                    .peers_registered
                    .fetch_add(1, Ordering::Relaxed);
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Should have exactly 1000 increments
    assert_eq!(
        metrics.peers_registered.load(Ordering::Relaxed),
        1000,
        "Concurrent increments should be atomic"
    );
}

#[tokio::test]
async fn test_metrics_multiple_counters_concurrent() {
    use std::sync::Arc;

    let metrics = Arc::new(RelayMetrics::new());
    let mut handles = vec![];

    // Spawn tasks for different counters concurrently
    for i in 0..6 {
        let metrics_clone = metrics.clone();
        let handle = tokio::spawn(async move {
            match i {
                0 => {
                    for _ in 0..100 {
                        metrics_clone
                            .peers_registered
                            .fetch_add(1, Ordering::Relaxed);
                    }
                }
                1 => {
                    for _ in 0..200 {
                        metrics_clone.capow_verified.fetch_add(1, Ordering::Relaxed);
                    }
                }
                2 => {
                    for _ in 0..50 {
                        metrics_clone.capow_rejected.fetch_add(1, Ordering::Relaxed);
                    }
                }
                3 => {
                    for _ in 0..75 {
                        metrics_clone.rate_limited.fetch_add(1, Ordering::Relaxed);
                    }
                }
                4 => {
                    for _ in 0..500 {
                        metrics_clone
                            .packets_forwarded
                            .fetch_add(1, Ordering::Relaxed);
                    }
                }
                5 => {
                    for _ in 0..25 {
                        metrics_clone.peers_timeout.fetch_add(1, Ordering::Relaxed);
                    }
                }
                _ => unreachable!(),
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify each counter has correct value
    assert_eq!(metrics.peers_registered.load(Ordering::Relaxed), 100);
    assert_eq!(metrics.capow_verified.load(Ordering::Relaxed), 200);
    assert_eq!(metrics.capow_rejected.load(Ordering::Relaxed), 50);
    assert_eq!(metrics.rate_limited.load(Ordering::Relaxed), 75);
    assert_eq!(metrics.packets_forwarded.load(Ordering::Relaxed), 500);
    assert_eq!(metrics.peers_timeout.load(Ordering::Relaxed), 25);
}

#[test]
fn test_metrics_no_overflow() {
    let metrics = RelayMetrics::new();

    // Set counter to near max value
    let near_max = u64::MAX - 100;
    metrics.packets_forwarded.store(near_max, Ordering::Relaxed);

    // Increment it
    metrics.packets_forwarded.fetch_add(1, Ordering::Relaxed);

    assert_eq!(
        metrics.packets_forwarded.load(Ordering::Relaxed),
        near_max + 1
    );

    // Note: In real code, we'd want to handle overflow, but u64::MAX is
    // 18,446,744,073,709,551,615 which would take centuries to reach
    // at 1M packets/second
}

#[test]
fn test_metrics_ordering_semantics() {
    let metrics = RelayMetrics::new();

    // Test different ordering semantics
    metrics.peers_registered.store(42, Ordering::Relaxed);
    assert_eq!(metrics.peers_registered.load(Ordering::Relaxed), 42);

    metrics.peers_registered.store(100, Ordering::SeqCst);
    assert_eq!(metrics.peers_registered.load(Ordering::SeqCst), 100);

    // fetch_add returns the previous value
    let prev = metrics.peers_registered.fetch_add(10, Ordering::Relaxed);
    assert_eq!(prev, 100);
    assert_eq!(metrics.peers_registered.load(Ordering::Relaxed), 110);
}

#[tokio::test]
async fn test_metrics_realistic_relay_scenario() {
    use std::sync::Arc;

    let metrics = Arc::new(RelayMetrics::new());
    let mut handles = vec![];

    // Simulate 5 peers registering
    for _ in 0..5 {
        let m = metrics.clone();
        handles.push(tokio::spawn(async move {
            m.peers_registered.fetch_add(1, Ordering::Relaxed);

            // Each peer does CAPoW verification
            m.capow_verified.fetch_add(1, Ordering::Relaxed);

            // Each peer forwards some packets
            for _ in 0..10 {
                m.packets_forwarded.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    // Simulate 2 CAPoW rejections
    for _ in 0..2 {
        let m = metrics.clone();
        handles.push(tokio::spawn(async move {
            m.capow_rejected.fetch_add(1, Ordering::Relaxed);
        }));
    }

    // Simulate 3 rate limited requests
    for _ in 0..3 {
        let m = metrics.clone();
        handles.push(tokio::spawn(async move {
            m.rate_limited.fetch_add(1, Ordering::Relaxed);
        }));
    }

    // Wait for all operations
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify realistic scenario metrics
    assert_eq!(metrics.peers_registered.load(Ordering::Relaxed), 5);
    assert_eq!(metrics.capow_verified.load(Ordering::Relaxed), 5);
    assert_eq!(metrics.capow_rejected.load(Ordering::Relaxed), 2);
    assert_eq!(metrics.rate_limited.load(Ordering::Relaxed), 3);
    assert_eq!(metrics.packets_forwarded.load(Ordering::Relaxed), 50); // 5 peers * 10 packets
    assert_eq!(metrics.peers_timeout.load(Ordering::Relaxed), 0);
}
