# Challenge Generation and Rotation

**Status**: ⏳ NOT IMPLEMENTED (currently uses dummy challenge)  
**Priority**: HIGH (security improvement)  
**Estimated Effort**: 1 week  
**Dependencies**: Event loop (for testing)

---

## Overview

CAPoW (Computational Amortized Proof of Work) requires a **challenge** (merkle root) that clients must prove knowledge of. Currently, the relay uses a dummy challenge (`[0u8; 32]`), which is insecure for production.

This document specifies how to generate and rotate challenges securely.

---

## Current State (main.rs:448)

```rust
// Generate challenge merkle root (in production, this should be rotated periodically)
// TODO: Implement proper challenge generation and rotation
let merkle_root = [0u8; 32]; // Dummy for now - will be replaced with real challenge
```

**Why this is insecure**:
- All clients use the same challenge forever
- Attacker can pre-compute proofs offline
- No forward secrecy (past proofs remain valid)

---

## Requirements

1. **Unpredictable**: Challenge must be cryptographically random
2. **Rotated**: Challenge must change periodically (e.g., every 1 hour)
3. **Distributed**: All relay instances must use the same challenge (for load balancing)
4. **Verifiable**: Relay must verify proofs against current + previous challenge (grace period)

---

## Architecture

### Challenge Structure

```rust
struct Challenge {
    merkle_root: [u8; 32],      // Merkle root of challenge tree
    created_at: SystemTime,      // When this challenge was generated
    expires_at: SystemTime,      // When this challenge becomes invalid
}
```

### Challenge Lifecycle

```
┌─────────────────────────────────────────────────────────────┐
│                    Challenge Timeline                       │
│                                                             │
│  Challenge A            Challenge B            Challenge C  │
│  ─────────────────────  ─────────────────────  ───────────  │
│  │                   │  │                   │  │           │ │
│  │  Valid (3600s)    │  │  Valid (3600s)    │  │  Valid    │ │
│  │                   │  │                   │  │           │ │
│  ─────────────────────  ─────────────────────  ───────────  │
│         └──────┬──────┘        └──────┬──────┘              │
│                │                      │                      │
│          Grace period            Grace period                │
│          (300s overlap)         (300s overlap)               │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

**Rotation interval**: 1 hour (3600s)  
**Grace period**: 5 minutes (300s)  
**Proof validity**: 1 hour + 5 minutes = 3900s

---

## Implementation Plan

### Phase 1: Single-Instance Challenge Generation

**Goal**: Generate and rotate challenges on a single relay instance.

```rust
use rand::RngCore;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

const CHALLENGE_ROTATION_INTERVAL: Duration = Duration::from_secs(3600); // 1 hour
const CHALLENGE_GRACE_PERIOD: Duration = Duration::from_secs(300); // 5 minutes

struct ChallengeManager {
    current: Arc<RwLock<Challenge>>,
    previous: Arc<RwLock<Option<Challenge>>>,
}

impl ChallengeManager {
    fn new() -> Self {
        let current = Challenge::generate();
        Self {
            current: Arc::new(RwLock::new(current)),
            previous: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Generate a new random challenge
    fn generate_challenge() -> Challenge {
        let mut merkle_root = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut merkle_root);
        
        let now = SystemTime::now();
        Challenge {
            merkle_root,
            created_at: now,
            expires_at: now + CHALLENGE_ROTATION_INTERVAL + CHALLENGE_GRACE_PERIOD,
        }
    }
    
    /// Rotate challenge (call every hour)
    async fn rotate(&self) {
        let new_challenge = Self::generate_challenge();
        
        // Move current to previous
        let old_current = {
            let mut current = self.current.write().await;
            let old = current.clone();
            *current = new_challenge;
            old
        };
        
        // Store old current as previous (for grace period)
        let mut previous = self.previous.write().await;
        *previous = Some(old_current);
        
        tracing::info!(
            "Challenge rotated: new merkle_root = {}",
            hex::encode(&new_challenge.merkle_root)
        );
    }
    
    /// Verify proof against current or previous challenge
    async fn verify_proof(&self, proof: &MtpProof) -> Result<bool> {
        // Try current challenge first
        let current = self.current.read().await;
        if proof.merkle_root == current.merkle_root {
            return Ok(true);
        }
        
        // Try previous challenge (grace period)
        let previous = self.previous.read().await;
        if let Some(prev) = previous.as_ref() {
            if proof.merkle_root == prev.merkle_root {
                // Check if still in grace period
                if SystemTime::now() < prev.expires_at {
                    return Ok(true);
                }
            }
        }
        
        Ok(false) // Proof for unknown challenge
    }
    
    /// Get current challenge (for clients to fetch)
    async fn get_current(&self) -> Challenge {
        self.current.read().await.clone()
    }
}
```

**Usage in main.rs**:

```rust
// Replace line 449
let challenge_manager = Arc::new(ChallengeManager::new());

// Start challenge rotation task
let challenge_manager_clone = challenge_manager.clone();
tokio::spawn(async move {
    let mut interval = tokio::time::interval(CHALLENGE_ROTATION_INTERVAL);
    loop {
        interval.tick().await;
        challenge_manager_clone.rotate().await;
    }
});

// In handle_register()
if !challenge_manager.verify_proof(&proof).await? {
    return send_register_ack(src_addr, 0x01, "Invalid challenge").await;
}
```

**Deliverables**:
- [ ] Implement `ChallengeManager` struct
- [ ] Implement rotation task
- [ ] Update `handle_register()` to use `ChallengeManager`
- [ ] Add `/challenge` HTTP endpoint (clients fetch current challenge)
- [ ] Unit tests for rotation and grace period

### Phase 2: Distributed Challenge Synchronization

**Goal**: All relay instances use the same challenge (for load balancing).

**Problem**: With multiple relay instances, each generates its own challenge. Clients might register with instance A (challenge X) but send RELAY packets to instance B (challenge Y) → registration fails.

**Solution**: Use Redis or etcd to distribute challenges across instances.

#### Option A: Redis

```rust
use redis::AsyncCommands;

struct DistributedChallengeManager {
    redis_client: redis::Client,
    local_cache: Arc<RwLock<Challenge>>,
}

impl DistributedChallengeManager {
    async fn new(redis_url: &str) -> Result<Self> {
        let redis_client = redis::Client::open(redis_url)?;
        
        // Fetch current challenge from Redis
        let mut conn = redis_client.get_async_connection().await?;
        let challenge: Option<Vec<u8>> = conn.get("file_relay:challenge:current").await?;
        
        let local_cache = if let Some(data) = challenge {
            Challenge::deserialize(&data)?
        } else {
            // No challenge in Redis yet - generate and publish
            let challenge = Challenge::generate();
            Self::publish_challenge(&mut conn, &challenge).await?;
            challenge
        };
        
        Ok(Self {
            redis_client,
            local_cache: Arc::new(RwLock::new(local_cache)),
        })
    }
    
    async fn publish_challenge(conn: &mut impl AsyncCommands, challenge: &Challenge) -> Result<()> {
        let data = challenge.serialize();
        
        // Store current challenge
        conn.set("file_relay:challenge:current", &data).await?;
        
        // Set expiration (1 hour + grace period)
        conn.expire("file_relay:challenge:current", 3900).await?;
        
        // Publish to all subscribers (notify other instances)
        conn.publish("file_relay:challenge:rotation", &data).await?;
        
        Ok(())
    }
    
    async fn subscribe_rotations(&self) -> Result<()> {
        let mut pubsub = self.redis_client.get_async_connection().await?.into_pubsub();
        pubsub.subscribe("file_relay:challenge:rotation").await?;
        
        let local_cache = self.local_cache.clone();
        
        tokio::spawn(async move {
            while let Some(msg) = pubsub.on_message().next().await {
                let data: Vec<u8> = msg.get_payload().unwrap();
                if let Ok(new_challenge) = Challenge::deserialize(&data) {
                    let mut cache = local_cache.write().await;
                    *cache = new_challenge;
                    tracing::info!("Challenge updated from Redis pubsub");
                }
            }
        });
        
        Ok(())
    }
}
```

**Redis schema**:
```
file_relay:challenge:current → [Challenge binary blob, TTL 3900s]
file_relay:challenge:rotation → [Pubsub channel for rotation notifications]
```

**Deployment**:
1. Deploy Redis cluster (3 nodes, persistent)
2. Update relay config: `--redis-url redis://localhost:6379`
3. All relay instances connect to same Redis

**Deliverables**:
- [ ] Implement `DistributedChallengeManager`
- [ ] Redis pub/sub for rotation notifications
- [ ] Failover handling (Redis down → use local challenge)
- [ ] Integration tests with Redis
- [ ] Update Kubernetes manifests (add Redis StatefulSet)

#### Option B: Etcd (Alternative)

Etcd provides strong consistency (Raft consensus), better for distributed systems than Redis.

```rust
use etcd_client::{Client, PutOptions};

struct EtcdChallengeManager {
    etcd_client: Client,
    local_cache: Arc<RwLock<Challenge>>,
}

// Similar implementation to Redis, but using etcd
```

**When to use Etcd**:
- Need strong consistency guarantees
- Already using etcd in infrastructure (e.g., Kubernetes)

**When to use Redis**:
- Simpler deployment
- Lower latency (in-memory)
- Already using Redis for other features

---

## Challenge Distribution Protocol

### Client Flow

```
1. Client starts up
2. GET /challenge → Relay returns current challenge
3. Client generates CAPoW proof with challenge
4. Client sends REGISTER with proof
5. Relay verifies proof against current or previous challenge
6. Relay accepts registration
```

### HTTP Endpoint: `/challenge`

```rust
async fn get_challenge(
    AxumState(state): AxumState<Arc<RelayState>>,
) -> impl IntoResponse {
    let challenge = state.challenge_manager.get_current().await;
    
    Json(json!({
        "merkle_root": hex::encode(&challenge.merkle_root),
        "created_at": challenge.created_at.duration_since(UNIX_EPOCH).unwrap().as_secs(),
        "expires_at": challenge.expires_at.duration_since(UNIX_EPOCH).unwrap().as_secs(),
    }))
}
```

**Response**:
```json
{
  "merkle_root": "a3f5b8c1d2e4f678...",
  "created_at": 1705881600,
  "expires_at": 1705885500
}
```

**Client caching**:
- Clients should cache challenge for 1 hour
- Fetch new challenge if registration fails with "Invalid challenge"

---

## Security Considerations

### Attack: Pre-Computation

**Threat**: Attacker generates proofs for future challenges offline.

**Mitigation**: Challenges are cryptographically random (256-bit entropy). Attacker cannot predict future challenges.

### Attack: Replay

**Threat**: Attacker captures valid proof and replays it.

**Mitigation**: Relay has anti-replay cache (5-minute TTL). See `relay_daemon::ReplayCache`.

### Attack: Challenge Exhaustion

**Threat**: Attacker fetches `/challenge` endpoint 1M times per second to DoS relay.

**Mitigation**: Rate limit `/challenge` endpoint (10 req/s per IP).

### Attack: Distributed DoS

**Threat**: Attacker controls multiple relay instances, each generating different challenges → clients can't register.

**Mitigation**: Use distributed challenge synchronization (Redis/etcd). All instances use same challenge.

---

## Monitoring

### Metrics

```rust
struct ChallengeMetrics {
    rotations_total: AtomicU64,            // Total challenge rotations
    rotation_failures: AtomicU64,          // Failed rotations (Redis down, etc.)
    grace_period_verifications: AtomicU64, // Proofs verified against previous challenge
}
```

**Prometheus queries**:
```promql
# Rotation rate (should be 1 per hour)
rate(file_relay_challenge_rotations_total[1h])

# Failure rate (should be 0)
rate(file_relay_challenge_rotation_failures[5m])

# Grace period usage (normal: 0-5% of verifications)
file_relay_challenge_grace_period_verifications / file_relay_capow_verified_total
```

### Alerts

```yaml
- alert: ChallengeRotationStuck
  expr: time() - file_relay_challenge_last_rotation_timestamp > 3900
  for: 5m
  labels:
    severity: critical
  annotations:
    summary: "Challenge rotation stuck for > 65 minutes"

- alert: ChallengeRotationFailing
  expr: rate(file_relay_challenge_rotation_failures[5m]) > 0
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "Challenge rotation failing (Redis down?)"
```

---

## Testing Strategy

### Unit Tests

1. Challenge generation (randomness)
2. Rotation logic (current → previous)
3. Grace period verification
4. Expiration checks

### Integration Tests

1. **Single instance**:
   - Generate challenge
   - Client fetches challenge via `/challenge`
   - Client generates proof
   - Relay verifies proof

2. **Rotation**:
   - Start relay
   - Wait 1 hour
   - Verify challenge rotated
   - Verify old challenge still valid (grace period)
   - Wait 6 minutes
   - Verify old challenge expired

3. **Distributed (with Redis)**:
   - Start 3 relay instances
   - Instance A rotates challenge
   - Verify instances B and C receive new challenge
   - Client registers on A, sends RELAY to B
   - Verify B accepts RELAY

---

## Deployment Checklist

Before marking challenge generation as "DONE":

1. **Implementation**:
   - [ ] `ChallengeManager` implemented
   - [ ] Rotation task running
   - [ ] Grace period verification working
   - [ ] `/challenge` HTTP endpoint added

2. **Distributed (if using Redis)**:
   - [ ] Redis deployment (StatefulSet)
   - [ ] Pub/sub rotation notifications
   - [ ] Failover handling (Redis down)

3. **Tests pass**:
   - [ ] Unit tests (generation, rotation, grace period)
   - [ ] Integration tests (single + distributed)
   - [ ] Load tests (challenge fetch rate limiting)

4. **Documentation**:
   - [ ] CLIENT-INTEGRATION-GUIDE.md updated (challenge fetching)
   - [ ] PROTOCOL-SPECIFICATION.md updated (challenge format)
   - [ ] OPERATIONS-RUNBOOK.md updated (challenge rotation monitoring)

5. **Production validation**:
   - [ ] Deploy to staging
   - [ ] Validate rotation (wait 1 hour, check logs)
   - [ ] Validate grace period (register during rotation)
   - [ ] Validate distributed sync (if using Redis)

---

## FAQ

### Why rotate challenges?

**Security**: Rotating challenges prevents attackers from pre-computing proofs offline. Each challenge is only valid for 1 hour + grace period.

### Why a grace period?

**Availability**: Without a grace period, clients that generate proofs just before rotation would fail to register. The 5-minute grace period ensures clients have time to retry.

### Why not rotate more frequently?

**Trade-off**: More frequent rotation improves security but increases client complexity (must fetch challenge more often). 1 hour is a balance.

### What if Redis goes down?

**Failover**: Each relay instance uses its local cached challenge. New instances cannot start (no challenge to fetch), but running instances continue with stale challenge. Alert fires after 65 minutes (rotation stuck).

### Can I disable challenge rotation for testing?

**Yes**: Set `--challenge-rotation-interval 0` to disable rotation (use fixed dummy challenge). Only for testing, never production.

---

## References

- [CAPOW.md](CAPOW.md) - CAPoW algorithm specification
- [relay_daemon](../../crates/relay_daemon/) - CAPoW verification implementation
- [main.rs:448](../../crates/relay_server/src/main.rs) - Current TODO location

---

**Status**: ⏳ Awaiting implementation  
**Priority**: HIGH (after event loop)  
**Last Updated**: 2026-05-18
