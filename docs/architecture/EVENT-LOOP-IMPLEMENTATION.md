# Event Loop Implementation Guide

**Status**: ⏳ NOT IMPLEMENTED (blocker for production)  
**Priority**: CRITICAL  
**Estimated Effort**: 2-3 weeks  
**Dependencies**: None (all infrastructure ready)

---

## Overview

The **connection event loop** is the core packet forwarding logic of FILE Relay Server. Currently, the relay has:
- ✅ CAPoW verification infrastructure
- ✅ Rate limiting
- ✅ IP hashing (GDPR compliance)
- ✅ Metrics and monitoring
- ✅ Health checks
- ❌ **Actual packet forwarding** (missing)

This document provides the technical design and implementation plan for the event loop.

---

## Current State (main.rs:603)

```rust
// Main event loop - accept connections
// TODO: Implement actual P2pEndpoint.accept() loop
// For now, just keep running
tokio::signal::ctrl_c().await?;
```

**Why this blocks everything**:
- Relay cannot forward packets
- Noise XX authentication cannot be tested
- Performance benchmarks cannot be run
- VPS deployment cannot be validated

---

## Architecture

### High-Level Flow

```
┌──────────────────────────────────────────────────────────┐
│                      Event Loop                          │
│                                                          │
│  1. Accept UDP packet                                   │
│  2. Parse packet type (REGISTER / RELAY / KEEPALIVE)   │
│  3. Apply rate limiting                                  │
│  4. Verify CAPoW proof (if REGISTER)                    │
│  5. Update peer registry                                 │
│  6. Forward packet to destination peer (if RELAY)       │
│  7. Update metrics                                       │
└──────────────────────────────────────────────────────────┘
```

### Components

1. **P2pEndpoint** (`ant_quic` crate)
   - UDP socket wrapper
   - Connection management
   - Packet send/receive

2. **Packet Parser**
   - Deserialize incoming packets
   - Validate structure
   - Extract peer_id, destination, payload

3. **CAPoW Verifier** (✅ already implemented)
   - Worker pool (25% CPU cores)
   - Async verification with timeout

4. **Rate Limiter** (✅ already implemented)
   - Per-IP token bucket
   - 10 req/s sustained, 50 burst

5. **Peer Registry** (✅ already implemented)
   - DashMap for lock-free concurrent access
   - Peer timeout cleanup task

6. **Packet Forwarder**
   - Lookup destination peer
   - Send packet via P2pEndpoint
   - Update metrics

---

## Packet Format

### REGISTER Packet

```
┌─────────────────────────────────────────────────────────────┐
│ Type (1 byte) │ Peer ID (32 bytes) │ CAPoW Proof (variable) │
├───────────────┼────────────────────┼────────────────────────┤
│      0x01     │   [peer_id]        │   [MtpProof]           │
└─────────────────────────────────────────────────────────────┘
```

**Fields**:
- `type`: 0x01 (REGISTER)
- `peer_id`: 32-byte identifier
- `proof`: Serialized MtpProof (see relay_daemon::MtpProof)

### REGISTER_ACK Packet

```
┌──────────────────────────────────────────────────────────┐
│ Type (1 byte) │ Status (1 byte) │ Message (variable)     │
├───────────────┼─────────────────┼────────────────────────┤
│      0x02     │  0x00 = OK      │   "Registered OK"      │
│               │  0x01 = Error   │   "CAPoW invalid"      │
└──────────────────────────────────────────────────────────┘
```

### RELAY Packet

```
┌──────────────────────────────────────────────────────────────────┐
│ Type (1 byte) │ Dest Peer ID (32 bytes) │ Payload (variable)   │
├───────────────┼─────────────────────────┼──────────────────────┤
│      0x03     │   [dest_peer_id]        │   [MPQUIC packet]    │
└──────────────────────────────────────────────────────────────────┘
```

**Fields**:
- `type`: 0x03 (RELAY)
- `dest_peer_id`: 32-byte destination peer ID
- `payload`: Opaque MPQUIC packet (relay does not inspect)

### KEEPALIVE Packet

```
┌─────────────────────────────┐
│ Type (1 byte) │ Timestamp   │
├───────────────┼─────────────┤
│      0x04     │  u64 (8 B)  │
└─────────────────────────────┘
```

**Purpose**: Reset peer timeout (5-minute inactivity)

---

## Implementation Plan

### Phase 1: Basic Event Loop (Week 1)

**Goal**: Accept packets and route them based on type.

```rust
// Pseudo-code
loop {
    // Accept incoming packet
    let (packet, src_addr) = endpoint.recv().await?;
    
    // Parse packet type
    match packet[0] {
        0x01 => handle_register(packet, src_addr).await?,
        0x03 => handle_relay(packet, src_addr).await?,
        0x04 => handle_keepalive(packet, src_addr).await?,
        _ => {
            warn!("Unknown packet type: {}", packet[0]);
            continue;
        }
    }
}
```

**Deliverables**:
- [ ] Implement `P2pEndpoint::recv()` wrapper
- [ ] Implement packet type parsing
- [ ] Implement `handle_register()` stub
- [ ] Implement `handle_relay()` stub
- [ ] Implement `handle_keepalive()` stub
- [ ] Unit tests for packet parsing

### Phase 2: REGISTER Handler (Week 1-2)

**Goal**: Process REGISTER packets with CAPoW verification.

```rust
async fn handle_register(
    packet: &[u8],
    src_addr: SocketAddr,
    state: Arc<RelayState>,
) -> Result<()> {
    // 1. Rate limiting
    if !state.rate_limiters.check_and_consume(src_addr.ip()) {
        state.metrics.rate_limited.fetch_add(1, Ordering::Relaxed);
        return send_register_ack(src_addr, 0x01, "Rate limited").await;
    }
    
    // 2. Parse packet
    let peer_id = PeerId::from_bytes(&packet[1..33])?;
    let proof = MtpProof::deserialize(&packet[33..])?;
    
    // 3. Verify CAPoW proof
    let challenge = state.challenge.load();
    match async_verify_proof_with_timeout(proof, challenge, CAPOW_TIMEOUT).await {
        Ok(true) => {
            state.metrics.capow_verified.fetch_add(1, Ordering::Relaxed);
        }
        _ => {
            state.metrics.capow_rejected.fetch_add(1, Ordering::Relaxed);
            return send_register_ack(src_addr, 0x01, "CAPoW invalid").await;
        }
    }
    
    // 4. Check capacity
    if state.peers.len() >= state.max_peers {
        return send_register_ack(src_addr, 0x01, "Capacity reached").await;
    }
    
    // 5. Register peer
    let peer_info = PeerInfo {
        addr: src_addr,
        last_seen: Instant::now(),
    };
    state.peers.insert(peer_id, peer_info);
    state.metrics.peers_registered.fetch_add(1, Ordering::Relaxed);
    
    // 6. Send ACK
    send_register_ack(src_addr, 0x00, "OK").await
}
```

**Deliverables**:
- [ ] Implement REGISTER packet deserialization
- [ ] Integrate CAPoW verification (already exists in relay_daemon)
- [ ] Implement peer registry insertion
- [ ] Implement REGISTER_ACK response
- [ ] Integration test: REGISTER flow

### Phase 3: RELAY Handler (Week 2)

**Goal**: Forward packets between peers.

```rust
async fn handle_relay(
    packet: &[u8],
    src_addr: SocketAddr,
    state: Arc<RelayState>,
) -> Result<()> {
    // 1. Parse destination peer ID
    let dest_peer_id = PeerId::from_bytes(&packet[1..33])?;
    let payload = &packet[33..];
    
    // 2. Lookup destination peer
    let dest_addr = match state.peers.get(&dest_peer_id) {
        Some(peer_info) => {
            peer_info.last_seen = Instant::now(); // Update timestamp
            peer_info.addr
        }
        None => {
            warn!("Unknown destination peer: {:?}", dest_peer_id);
            return Ok(()); // Drop packet silently (no ACK for RELAY)
        }
    };
    
    // 3. Forward packet
    state.endpoint.send_to(payload, dest_addr).await?;
    state.metrics.packets_forwarded.fetch_add(1, Ordering::Relaxed);
    
    Ok(())
}
```

**Deliverables**:
- [ ] Implement RELAY packet parsing
- [ ] Implement peer lookup
- [ ] Implement packet forwarding via P2pEndpoint
- [ ] Integration test: End-to-end RELAY flow

### Phase 4: KEEPALIVE Handler (Week 2)

**Goal**: Reset peer timeout on keepalive.

```rust
async fn handle_keepalive(
    packet: &[u8],
    src_addr: SocketAddr,
    state: Arc<RelayState>,
) -> Result<()> {
    // 1. Find peer by source address
    for mut peer_entry in state.peers.iter_mut() {
        if peer_entry.value().addr == src_addr {
            peer_entry.value_mut().last_seen = Instant::now();
            break;
        }
    }
    
    Ok(())
}
```

**Deliverables**:
- [ ] Implement KEEPALIVE packet parsing
- [ ] Implement peer timestamp update
- [ ] Integration test: Peer timeout prevention

### Phase 5: Error Handling & Edge Cases (Week 3)

**Goal**: Robust error handling for production.

**Edge cases**:
1. **Malformed packets** → Drop silently, log at debug level
2. **Unknown packet types** → Drop silently, increment metric
3. **Peer timeout during RELAY** → Drop packet, remove peer
4. **Rate limit exceeded** → Send REGISTER_ACK with error
5. **CAPoW timeout** → Reject registration
6. **Capacity reached** → Send REGISTER_ACK with error
7. **Duplicate peer_id** → Overwrite (last registration wins)

**Deliverables**:
- [ ] Error handling for all edge cases
- [ ] Negative integration tests (malformed packets, etc.)
- [ ] Fuzzing setup (`cargo fuzz`)
- [ ] Performance benchmarks (baseline)

---

## Integration Points

### Existing Infrastructure (Already Working)

1. **CAPoW Verification** (`relay_daemon::async_verify_proof_with_timeout`)
   - Worker pool initialized in `main()`
   - Async verification with 60s timeout
   - Returns `Result<bool>`

2. **Rate Limiting** (`RateLimiter` struct)
   - Per-IP token bucket
   - `check_and_consume()` method
   - Already integrated in DashMap

3. **Peer Registry** (`DashMap<PeerId, PeerInfo>`)
   - Lock-free concurrent access
   - Timeout cleanup task running every 30s

4. **Metrics** (`RelayMetrics` struct)
   - Atomic counters
   - Exposed via `/metrics` endpoint

5. **Health Checks** (Axum HTTP server)
   - `/health` endpoint
   - `/ready` endpoint (checks capacity)

### New Dependencies Needed

**None**. All required crates already in `Cargo.toml`:
- `ant_quic` ✅ (P2pEndpoint, PeerId)
- `tokio` ✅ (async runtime)
- `dashmap` ✅ (concurrent HashMap)
- `anyhow` ✅ (error handling)
- `tracing` ✅ (logging)

---

## Testing Strategy

### Unit Tests

1. Packet parsing (REGISTER, RELAY, KEEPALIVE)
2. Error handling (malformed packets)
3. Peer registry operations

### Integration Tests

1. **REGISTER flow**:
   - Client sends REGISTER with valid CAPoW
   - Relay verifies and sends REGISTER_ACK
   - Peer appears in registry

2. **RELAY flow**:
   - Register 2 peers (A, B)
   - Peer A sends RELAY packet to B
   - Relay forwards packet to B's address

3. **KEEPALIVE flow**:
   - Register peer
   - Send KEEPALIVE every 60s
   - Verify peer doesn't timeout

4. **Timeout flow**:
   - Register peer
   - Wait 5 minutes (no KEEPALIVE)
   - Verify peer removed from registry

### Load Tests (After Implementation)

Use existing load testing framework (`testing/load/relay_load_test.py`):
1. Smoke test (10 peers, 10 seconds)
2. Capacity test (1000 peers)
3. Throughput test (10k pkt/s)
4. Soak test (1 hour)

---

## Performance Considerations

### Packet Processing Pipeline

```
Recv → Parse → Rate Limit → Verify CAPoW → Update Registry → Forward → Metrics
 └─── ~1µs ──┘  └── ~10µs ──┘  └─── 100ms ───┘  └─ ~100µs ─┘  └ ~1µs ┘
```

**Bottlenecks**:
1. **CAPoW verification** (100ms) → Worker pool with timeout
2. **Peer registry lookup** (~100µs) → DashMap (lock-free)
3. **UDP send** (~1µs) → Async I/O

**Optimization opportunities**:
- CAPoW verification is already async (non-blocking)
- Use `DashMap` for concurrent peer access (no locks)
- Batch metrics updates (every 1s instead of per-packet)

### Memory Usage

**Per peer**: ~512 KB
- Peer registry entry: 100 bytes
- Rate limiter: 48 bytes
- OS UDP buffers: ~300 KB

**Max 1000 peers** = ~512 MB RAM

---

## Metrics to Add

After event loop implementation, add these new metrics:

```rust
packets_received_total: AtomicU64,      // Total packets received (all types)
packets_dropped_total: AtomicU64,       // Packets dropped (malformed, unknown dest)
relay_latency_microseconds: Histogram,  // Time from recv to send
```

**Prometheus queries**:
```promql
# Packet rate
rate(file_relay_packets_received_total[5m])

# Drop rate (should be < 0.1%)
rate(file_relay_packets_dropped_total[5m]) / rate(file_relay_packets_received_total[5m])

# Latency p95 (target: < 20ms)
histogram_quantile(0.95, file_relay_relay_latency_microseconds)
```

---

## Noise XX Integration (Phase 2 - After Event Loop)

Once event loop is stable, add Noise XX authentication:

1. **Handshake flow**:
   - Client sends `HANDSHAKE_INIT` (before REGISTER)
   - Relay responds with `HANDSHAKE_RESPONSE`
   - Client sends `HANDSHAKE_FINAL` (authenticated)
   - Relay accepts REGISTER

2. **Encrypted RELAY packets**:
   - Payload encrypted with Noise session keys
   - Relay forwards encrypted payload (still opaque)

3. **Session management**:
   - Store Noise session state in peer registry
   - Rotate session keys every 1 hour

**Deliverables** (separate task):
- [ ] Noise XX handshake implementation
- [ ] Session key rotation
- [ ] Encrypted RELAY packet forwarding
- [ ] Integration tests

---

## Deployment Checklist (After Implementation)

Before marking event loop as "DONE":

1. **Tests pass**:
   - [ ] `cargo test` (all unit tests)
   - [ ] `cargo test --test integration` (all integration tests)
   - [ ] Load test smoke scenario passes

2. **Code quality**:
   - [ ] `cargo clippy` (no warnings)
   - [ ] `cargo fmt` (formatted)
   - [ ] `cargo audit` (no vulnerabilities)

3. **Documentation updated**:
   - [ ] PROTOCOL-SPECIFICATION.md (packet formats)
   - [ ] QUICK-START.md (example client code)
   - [ ] FAQ.md (update "When will event loop be implemented?")
   - [ ] ROADMAP.md (mark Phase 1.1 complete)

4. **Benchmarks established**:
   - [ ] Baseline performance recorded (throughput, latency)
   - [ ] Compared against targets (10k pkt/s, < 20ms p95)

5. **VPS deployment tested**:
   - [ ] Deploy to Hetzner Germany (1 VPS)
   - [ ] Monitor metrics for 24 hours
   - [ ] Validate SLA (99.9% uptime)

---

## FAQ

### Why is this a blocker?

Without the event loop, the relay **cannot forward packets**. All infrastructure (monitoring, deployment, compliance) is ready, but the relay is a no-op until this is implemented.

### Why wasn't this implemented first?

Production-readiness requires **infrastructure + code**. We built infrastructure first (tests, docs, deployment) so that once the event loop is done, we can deploy immediately.

### How long will this take?

**Estimate**: 2-3 weeks for one experienced Rust developer.
- Week 1: Basic loop + REGISTER handler
- Week 2: RELAY + KEEPALIVE handlers
- Week 3: Error handling, edge cases, benchmarks

### Can I contribute?

**Yes!** See [CONTRIBUTING.md](../../CONTRIBUTING.md). Key skills:
- Rust async/await (Tokio)
- UDP socket programming
- Concurrent data structures (DashMap)

---

## References

- [PROTOCOL-SPECIFICATION.md](PROTOCOL-SPECIFICATION.md) - Full protocol spec
- [CAPOW.md](CAPOW.md) - CAPoW algorithm
- [relay_daemon crate](../../crates/relay_daemon/) - CAPoW verification implementation
- [main.rs](../../crates/relay_server/src/main.rs) - Current relay code (TODO at line 603)

---

**Status**: ⏳ Awaiting implementation  
**Last Updated**: 2026-05-18
