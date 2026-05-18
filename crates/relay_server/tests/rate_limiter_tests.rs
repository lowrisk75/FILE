//! Unit tests for rate limiting functionality
//!
//! Tests the token bucket rate limiter implementation

use std::time::{Duration, Instant};

/// Token bucket rate limiter (copied from main.rs for testing)
#[derive(Debug)]
struct RateLimiter {
    tokens: f64,
    last_refill: Instant,
    capacity: f64,
    refill_rate: f64,
}

impl RateLimiter {
    fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            tokens: capacity,
            last_refill: Instant::now(),
            capacity,
            refill_rate,
        }
    }

    fn try_consume(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    #[allow(dead_code)]
    fn available_tokens(&mut self) -> f64 {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        (self.tokens + elapsed * self.refill_rate).min(self.capacity)
    }
}

#[test]
fn test_rate_limiter_initialization() {
    let limiter = RateLimiter::new(50.0, 10.0);

    assert_eq!(limiter.capacity, 50.0, "Capacity should be 50");
    assert_eq!(limiter.refill_rate, 10.0, "Refill rate should be 10");
    assert_eq!(limiter.tokens, 50.0, "Should start with full bucket");
}

#[test]
fn test_rate_limiter_burst_capacity() {
    let mut limiter = RateLimiter::new(50.0, 10.0);

    // Should allow 50 requests immediately (burst)
    for i in 1..=50 {
        assert!(limiter.try_consume(), "Should allow request {} in burst", i);
    }

    // 51st request should be denied
    assert!(
        !limiter.try_consume(),
        "Should deny 51st request (exceeds burst)"
    );
}

#[tokio::test]
async fn test_rate_limiter_refill() {
    let mut limiter = RateLimiter::new(50.0, 10.0);

    // Drain the bucket
    for _ in 0..50 {
        assert!(limiter.try_consume());
    }

    // Should be empty now
    assert!(!limiter.try_consume(), "Bucket should be empty");

    // Wait 0.5 seconds (should refill 5 tokens at 10 tokens/sec)
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Should allow ~5 requests after refill
    let mut allowed = 0;
    for _ in 0..10 {
        if limiter.try_consume() {
            allowed += 1;
        }
    }

    // Should have refilled approximately 5 tokens (±1 for timing variance)
    assert!(
        allowed >= 4 && allowed <= 6,
        "Should refill ~5 tokens in 0.5s, got {}",
        allowed
    );
}

#[tokio::test]
async fn test_rate_limiter_sustained_rate() {
    let mut limiter = RateLimiter::new(50.0, 10.0);

    // Drain the entire bucket
    for _ in 0..50 {
        assert!(limiter.try_consume());
    }

    // Bucket should be empty
    assert!(!limiter.try_consume(), "Bucket should be empty");

    // Wait 1 second (should refill 10 tokens at 10 tokens/sec)
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Should allow ~10 requests after refill
    let mut allowed = 0;
    for _ in 0..15 {
        if limiter.try_consume() {
            allowed += 1;
        }
    }

    // Should have refilled approximately 10 tokens (allow variance for timing)
    assert!(
        allowed >= 9 && allowed <= 12,
        "Should refill ~10 tokens in 1s, got {}",
        allowed
    );
}

#[test]
fn test_rate_limiter_does_not_overfill() {
    let mut limiter = RateLimiter::new(50.0, 10.0);

    // Consume one token
    assert!(limiter.try_consume());

    // Manually advance time by setting last_refill in the past
    limiter.last_refill = Instant::now() - Duration::from_secs(100);

    // Try to consume - should refill but not exceed capacity
    assert!(limiter.try_consume());

    // Check available tokens (should be at capacity)
    let available = limiter.available_tokens();
    assert!(
        available <= limiter.capacity + 1.0,
        "Should not exceed capacity, got {}",
        available
    );
}

#[tokio::test]
async fn test_rate_limiter_per_ip_isolation() {
    use dashmap::DashMap;
    use std::net::{IpAddr, Ipv4Addr};

    let rate_limiters: DashMap<IpAddr, RateLimiter> = DashMap::new();

    let ip1 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
    let ip2 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2));

    // IP1 drains its bucket
    {
        let mut limiter1 = rate_limiters
            .entry(ip1)
            .or_insert_with(|| RateLimiter::new(50.0, 10.0));

        for _ in 0..50 {
            assert!(limiter1.try_consume());
        }

        assert!(!limiter1.try_consume(), "IP1 should be rate limited");
    }

    // IP2 should still have full bucket (isolation)
    {
        let mut limiter2 = rate_limiters
            .entry(ip2)
            .or_insert_with(|| RateLimiter::new(50.0, 10.0));

        assert!(
            limiter2.try_consume(),
            "IP2 should not be affected by IP1's limit"
        );
    }

    // Verify IP1 is still limited
    {
        let mut limiter1 = rate_limiters.get_mut(&ip1).unwrap();
        assert!(!limiter1.try_consume(), "IP1 should still be rate limited");
    }
}

#[tokio::test]
async fn test_rate_limiter_concurrent_access() {
    use dashmap::DashMap;
    use std::net::{IpAddr, Ipv4Addr};
    use std::sync::Arc;

    let rate_limiters: Arc<DashMap<IpAddr, RateLimiter>> = Arc::new(DashMap::new());
    let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));

    // Spawn 10 concurrent tasks trying to consume tokens
    let mut handles = vec![];

    for _ in 0..10 {
        let limiters = rate_limiters.clone();
        let handle = tokio::spawn(async move {
            let mut allowed = 0;
            for _ in 0..10 {
                let mut entry = limiters
                    .entry(ip)
                    .or_insert_with(|| RateLimiter::new(50.0, 10.0));

                if entry.try_consume() {
                    allowed += 1;
                }
            }
            allowed
        });
        handles.push(handle);
    }

    // Collect results
    let mut total_allowed = 0;
    for handle in handles {
        total_allowed += handle.await.unwrap();
    }

    // Should allow exactly 50 requests total (burst capacity)
    // Allow some variance due to concurrent access timing
    assert!(
        total_allowed >= 48 && total_allowed <= 52,
        "Should allow ~50 concurrent requests, got {}",
        total_allowed
    );
}

#[test]
fn test_rate_limiter_edge_cases() {
    // Test with very small capacity
    let mut small_limiter = RateLimiter::new(1.0, 1.0);
    assert!(small_limiter.try_consume());
    assert!(!small_limiter.try_consume());

    // Test with fractional capacity
    let mut fractional = RateLimiter::new(2.5, 1.0);
    assert!(fractional.try_consume());
    assert!(fractional.try_consume());
    assert!(!fractional.try_consume()); // Only 2 full tokens

    // Test with very high refill rate
    let mut fast_refill = RateLimiter::new(10.0, 1000.0);
    for _ in 0..10 {
        assert!(fast_refill.try_consume());
    }
}
