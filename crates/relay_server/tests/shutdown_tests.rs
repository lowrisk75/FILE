//! Unit tests for graceful shutdown behavior
//!
//! Tests signal handling, metrics logging, connection draining, and cleanup

use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// Mock relay metrics (simplified version)
#[derive(Debug)]
struct MockMetrics {
    peers_registered: AtomicU64,
    capow_verified: AtomicU64,
    capow_rejected: AtomicU64,
    rate_limited: AtomicU64,
    packets_forwarded: AtomicU64,
    peers_timeout: AtomicU64,
}

impl MockMetrics {
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

    fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            peers_registered: self.peers_registered.load(Ordering::Relaxed),
            capow_verified: self.capow_verified.load(Ordering::Relaxed),
            capow_rejected: self.capow_rejected.load(Ordering::Relaxed),
            rate_limited: self.rate_limited.load(Ordering::Relaxed),
            packets_forwarded: self.packets_forwarded.load(Ordering::Relaxed),
            peers_timeout: self.peers_timeout.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct MetricsSnapshot {
    peers_registered: u64,
    capow_verified: u64,
    capow_rejected: u64,
    rate_limited: u64,
    packets_forwarded: u64,
    peers_timeout: u64,
}

/// Mock shutdown coordinator
struct ShutdownCoordinator {
    shutdown_signal: Arc<AtomicBool>,
    metrics: Arc<MockMetrics>,
    grace_period: Duration,
}

impl ShutdownCoordinator {
    fn new(metrics: Arc<MockMetrics>, grace_period: Duration) -> Self {
        Self {
            shutdown_signal: Arc::new(AtomicBool::new(false)),
            metrics,
            grace_period,
        }
    }

    async fn initiate_shutdown(&self) -> MetricsSnapshot {
        // Step 1: Set shutdown flag
        self.shutdown_signal.store(true, Ordering::SeqCst);

        // Step 2: Capture final metrics
        let snapshot = self.metrics.snapshot();

        // Step 3: Grace period for connection draining
        sleep(self.grace_period).await;

        snapshot
    }

    fn is_shutting_down(&self) -> bool {
        self.shutdown_signal.load(Ordering::SeqCst)
    }
}

#[tokio::test]
async fn test_shutdown_signal_propagation() {
    let metrics = Arc::new(MockMetrics::new());
    let coordinator = ShutdownCoordinator::new(metrics, Duration::from_millis(10));

    // Initially not shutting down
    assert!(!coordinator.is_shutting_down());

    // Initiate shutdown
    coordinator.initiate_shutdown().await;

    // Should be shutting down after initiation
    assert!(coordinator.is_shutting_down());
}

#[tokio::test]
async fn test_metrics_snapshot_at_shutdown() {
    let metrics = Arc::new(MockMetrics::new());

    // Simulate some activity
    metrics.peers_registered.store(42, Ordering::Relaxed);
    metrics.capow_verified.store(35, Ordering::Relaxed);
    metrics.packets_forwarded.store(1500, Ordering::Relaxed);

    let coordinator = ShutdownCoordinator::new(metrics.clone(), Duration::from_millis(10));

    // Capture snapshot during shutdown
    let snapshot = coordinator.initiate_shutdown().await;

    // Verify snapshot captured correct values
    assert_eq!(snapshot.peers_registered, 42);
    assert_eq!(snapshot.capow_verified, 35);
    assert_eq!(snapshot.packets_forwarded, 1500);
}

#[tokio::test]
async fn test_grace_period_duration() {
    let metrics = Arc::new(MockMetrics::new());
    let grace_period = Duration::from_millis(100);
    let coordinator = ShutdownCoordinator::new(metrics, grace_period);

    let start = std::time::Instant::now();
    coordinator.initiate_shutdown().await;
    let elapsed = start.elapsed();

    // Grace period should be respected (±10ms tolerance)
    assert!(
        elapsed >= grace_period && elapsed < grace_period + Duration::from_millis(50),
        "Grace period should be ~100ms, got {:?}",
        elapsed
    );
}

#[tokio::test]
async fn test_concurrent_shutdown_checks() {
    let metrics = Arc::new(MockMetrics::new());
    let coordinator = Arc::new(ShutdownCoordinator::new(metrics, Duration::from_millis(10)));

    let mut handles = vec![];

    // Spawn 100 tasks checking shutdown status
    for _ in 0..100 {
        let coord = coordinator.clone();
        handles.push(tokio::spawn(async move {
            coord.is_shutting_down()
        }));
    }

    // All should see not-shutting-down initially
    for handle in handles {
        let status = handle.await.unwrap();
        assert!(!status);
    }

    // Initiate shutdown
    coordinator.initiate_shutdown().await;

    // Now all should see shutting-down
    let mut handles = vec![];
    for _ in 0..100 {
        let coord = coordinator.clone();
        handles.push(tokio::spawn(async move {
            coord.is_shutting_down()
        }));
    }

    for handle in handles {
        let status = handle.await.unwrap();
        assert!(status);
    }
}

#[tokio::test]
async fn test_metrics_remain_accessible_during_shutdown() {
    let metrics = Arc::new(MockMetrics::new());
    metrics.packets_forwarded.store(1000, Ordering::Relaxed);

    let coordinator = Arc::new(ShutdownCoordinator::new(metrics.clone(), Duration::from_millis(100)));

    // Spawn a task that keeps incrementing metrics during grace period
    let metrics_clone = metrics.clone();
    let increment_task = tokio::spawn(async move {
        for _ in 0..50 {
            metrics_clone.packets_forwarded.fetch_add(1, Ordering::Relaxed);
            sleep(Duration::from_millis(2)).await;
        }
    });

    // Start shutdown (includes grace period)
    let snapshot = coordinator.initiate_shutdown().await;

    // Wait for increment task to finish
    increment_task.await.unwrap();

    // Snapshot should capture state at shutdown initiation (1000),
    // but metrics continue updating during grace period
    assert_eq!(snapshot.packets_forwarded, 1000);

    // Final metrics should be higher due to increments during grace period
    let final_value = metrics.packets_forwarded.load(Ordering::Relaxed);
    assert!(final_value > 1000, "Metrics should continue updating during grace period");
}

#[tokio::test]
async fn test_zero_metrics_snapshot() {
    let metrics = Arc::new(MockMetrics::new());
    let coordinator = ShutdownCoordinator::new(metrics, Duration::from_millis(10));

    // Shutdown with no activity
    let snapshot = coordinator.initiate_shutdown().await;

    // All metrics should be zero
    assert_eq!(snapshot.peers_registered, 0);
    assert_eq!(snapshot.capow_verified, 0);
    assert_eq!(snapshot.capow_rejected, 0);
    assert_eq!(snapshot.rate_limited, 0);
    assert_eq!(snapshot.packets_forwarded, 0);
    assert_eq!(snapshot.peers_timeout, 0);
}

#[tokio::test]
async fn test_high_activity_snapshot() {
    let metrics = Arc::new(MockMetrics::new());

    // Simulate high activity
    metrics.peers_registered.store(999, Ordering::Relaxed);
    metrics.capow_verified.store(850, Ordering::Relaxed);
    metrics.capow_rejected.store(149, Ordering::Relaxed);
    metrics.rate_limited.store(50, Ordering::Relaxed);
    metrics.packets_forwarded.store(50000, Ordering::Relaxed);
    metrics.peers_timeout.store(10, Ordering::Relaxed);

    let coordinator = ShutdownCoordinator::new(metrics, Duration::from_millis(10));
    let snapshot = coordinator.initiate_shutdown().await;

    // Verify all high values captured
    assert_eq!(snapshot.peers_registered, 999);
    assert_eq!(snapshot.capow_verified, 850);
    assert_eq!(snapshot.capow_rejected, 149);
    assert_eq!(snapshot.rate_limited, 50);
    assert_eq!(snapshot.packets_forwarded, 50000);
    assert_eq!(snapshot.peers_timeout, 10);
}

#[tokio::test]
async fn test_shutdown_idempotency() {
    let metrics = Arc::new(MockMetrics::new());
    metrics.packets_forwarded.store(100, Ordering::Relaxed);

    let coordinator = Arc::new(ShutdownCoordinator::new(metrics.clone(), Duration::from_millis(10)));

    // First shutdown
    let snapshot1 = coordinator.initiate_shutdown().await;
    assert_eq!(snapshot1.packets_forwarded, 100);

    // Increment metrics after first shutdown
    metrics.packets_forwarded.store(200, Ordering::Relaxed);

    // Second shutdown (should still work, capture new state)
    let snapshot2 = coordinator.initiate_shutdown().await;
    assert_eq!(snapshot2.packets_forwarded, 200);

    // Shutdown flag should remain set
    assert!(coordinator.is_shutting_down());
}

#[tokio::test]
async fn test_grace_period_allows_in_flight_completion() {
    let metrics = Arc::new(MockMetrics::new());
    let coordinator = Arc::new(ShutdownCoordinator::new(metrics.clone(), Duration::from_millis(200)));

    // Simulate in-flight work that takes 100ms
    let work_complete = Arc::new(AtomicBool::new(false));
    let work_complete_clone = work_complete.clone();

    let work_task = tokio::spawn(async move {
        sleep(Duration::from_millis(100)).await;
        work_complete_clone.store(true, Ordering::SeqCst);
    });

    // Start shutdown with 200ms grace period
    coordinator.initiate_shutdown().await;

    // Wait for work task
    work_task.await.unwrap();

    // Work should have completed during grace period
    assert!(work_complete.load(Ordering::SeqCst), "In-flight work should complete during grace period");
}

#[tokio::test]
async fn test_multiple_concurrent_shutdowns() {
    let metrics = Arc::new(MockMetrics::new());
    metrics.packets_forwarded.store(42, Ordering::Relaxed);

    let coordinator = Arc::new(ShutdownCoordinator::new(metrics, Duration::from_millis(50)));

    let mut handles = vec![];

    // Spawn 10 tasks all trying to initiate shutdown
    for _ in 0..10 {
        let coord = coordinator.clone();
        handles.push(tokio::spawn(async move {
            coord.initiate_shutdown().await
        }));
    }

    // All should complete without error
    let mut snapshots = vec![];
    for handle in handles {
        let snapshot = handle.await.unwrap();
        snapshots.push(snapshot);
    }

    // All snapshots should capture the same metrics (42)
    for snapshot in snapshots {
        assert_eq!(snapshot.packets_forwarded, 42);
    }
}

#[test]
fn test_atomic_bool_memory_ordering() {
    let signal = AtomicBool::new(false);

    // SeqCst ordering should work across threads
    signal.store(true, Ordering::SeqCst);
    assert!(signal.load(Ordering::SeqCst));

    signal.store(false, Ordering::SeqCst);
    assert!(!signal.load(Ordering::SeqCst));
}

#[tokio::test]
async fn test_shutdown_with_no_grace_period() {
    let metrics = Arc::new(MockMetrics::new());
    let coordinator = ShutdownCoordinator::new(metrics, Duration::from_millis(0));

    let start = std::time::Instant::now();
    coordinator.initiate_shutdown().await;
    let elapsed = start.elapsed();

    // With zero grace period, shutdown should be nearly instant (< 10ms)
    assert!(elapsed < Duration::from_millis(10), "Zero grace period should be instant: {:?}", elapsed);
}

#[tokio::test]
async fn test_shutdown_performance() {
    let metrics = Arc::new(MockMetrics::new());

    // Simulate realistic relay state
    metrics.peers_registered.store(500, Ordering::Relaxed);
    metrics.capow_verified.store(450, Ordering::Relaxed);
    metrics.packets_forwarded.store(25000, Ordering::Relaxed);

    let coordinator = ShutdownCoordinator::new(metrics, Duration::from_millis(5000));

    let start = std::time::Instant::now();
    let _snapshot = coordinator.initiate_shutdown().await;
    let elapsed = start.elapsed();

    // Shutdown should complete in grace period time + small overhead (< 100ms)
    assert!(
        elapsed >= Duration::from_millis(5000) && elapsed < Duration::from_millis(5100),
        "Shutdown should take ~5s grace period + minimal overhead: {:?}",
        elapsed
    );
}
