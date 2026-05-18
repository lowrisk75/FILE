# FILE Relay Server - Client Integration Guide

**Last Updated**: 2026-05-18  
**Version**: 0.1.0  
**Audience**: Application developers building clients that use FILE Relay Server

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Protocol Specification](#protocol-specification)
4. [Client Implementation](#client-implementation)
5. [CAPoW Generation](#capow-generation)
6. [Error Handling](#error-handling)
7. [Best Practices](#best-practices)
8. [Testing](#testing)
9. [Example Code](#example-code)

---

## Overview

This guide explains how to integrate your application with FILE Relay Server for peer-to-peer connectivity through NAT/firewalls.

**Prerequisites**:
- Understanding of QUIC protocol
- MPQUIC library (e.g., `ant-quic` for Rust, `quic-go` for Go)
- Familiarity with peer-to-peer networking

**What you'll learn**:
- How to register with the relay
- How to forward packets through the relay
- How to generate CAPoW proofs
- How to handle errors and reconnections

---

## Architecture

### Client-Relay-Peer Communication

```
┌─────────┐                ┌─────────────┐                ┌─────────┐
│ Peer A  │◄──────────────►│    Relay    │◄──────────────►│ Peer B  │
│(Client) │   QUIC/MPQUIC  │   Server    │   QUIC/MPQUIC  │(Client) │
└─────────┘                └─────────────┘                └─────────┘
     │                            │                            │
     │  1. REGISTER(peer_id,      │                            │
     │     capow_proof)            │                            │
     ├───────────────────────────>│                            │
     │                            │                            │
     │  2. REGISTER_ACK           │                            │
     │<───────────────────────────┤                            │
     │                            │                            │
     │                            │  3. REGISTER(peer_id,      │
     │                            │     capow_proof)           │
     │                            │<───────────────────────────┤
     │                            │                            │
     │                            │  4. REGISTER_ACK           │
     │                            ├───────────────────────────>│
     │                            │                            │
     │  5. RELAY_PACKET(          │                            │
     │     target=B, data)        │                            │
     ├───────────────────────────>│                            │
     │                            │                            │
     │                            │  6. PACKET(source=A, data) │
     │                            ├───────────────────────────>│
     │                            │                            │
     │  7. RELAY_ACK              │                            │
     │<───────────────────────────┤                            │
```

### Message Flow

1. **Registration**: Peer connects to relay, proves work (CAPoW), relay stores peer info
2. **Heartbeat**: Peer sends heartbeat every 2 minutes to stay alive (5-minute timeout)
3. **Packet relay**: Peer A sends packet for Peer B, relay forwards, Peer B receives
4. **Direct connection** (future): Peers attempt hole-punching for direct P2P

---

## Protocol Specification

### Message Types

**Note**: Detailed protocol specification is pending event loop implementation. This section describes the planned protocol.

#### REGISTER

**Purpose**: Register peer with relay

**Format**:
```rust
struct RegisterMessage {
    peer_id: [u8; 32],           // Ed25519 public key
    capow_proof: Vec<u8>,        // MTP-Argon2id proof
    protocol_version: u8,        // Protocol version (currently 1)
}
```

**CAPoW proof** must satisfy:
- Algorithm: MTP-Argon2id
- Difficulty: Server-configured (check `/health` endpoint for current difficulty)
- Freshness: Timestamp within 5 minutes
- Uniqueness: Not seen before (replay prevention)

**Response**: `REGISTER_ACK` or `REGISTER_REJECT`

---

#### REGISTER_ACK

**Purpose**: Confirm successful registration

**Format**:
```rust
struct RegisterAck {
    success: bool,
    relay_peer_id: [u8; 32],     // Relay's peer ID (for verification)
    keepalive_interval_sec: u32, // How often to send heartbeat (120s)
}
```

---

#### REGISTER_REJECT

**Purpose**: Registration failed

**Format**:
```rust
struct RegisterReject {
    reason: RejectReason,
}

enum RejectReason {
    InvalidCAPoW,
    RateLimited,
    CapacityReached,
    InvalidPeerId,
    ReplayAttack,
}
```

**Client action**: 
- `InvalidCAPoW`: Regenerate proof, retry
- `RateLimited`: Exponential backoff (1s, 2s, 4s, 8s...)
- `CapacityReached`: Try another relay
- `InvalidPeerId`: Fix peer ID format
- `ReplayAttack`: Generate fresh proof

---

#### HEARTBEAT

**Purpose**: Keep registration alive

**Format**:
```rust
struct Heartbeat {
    peer_id: [u8; 32],
}
```

**Frequency**: Every 120 seconds (as indicated in `REGISTER_ACK`)

**Timeout**: Relay removes peer after 300 seconds (5 minutes) of no heartbeat

---

#### RELAY_PACKET

**Purpose**: Send packet to another peer

**Format**:
```rust
struct RelayPacket {
    source_peer_id: [u8; 32],    // Your peer ID
    target_peer_id: [u8; 32],    // Destination peer ID
    payload: Vec<u8>,            // Encrypted payload (relay cannot decrypt)
}
```

**Payload**:
- **End-to-end encrypted** (relay cannot decrypt)
- **Application-defined format** (relay just forwards bytes)
- **Max size**: TBD (check relay configuration)

**Response**: `RELAY_ACK` or `RELAY_REJECT`

---

#### PACKET

**Purpose**: Receive packet from another peer (via relay)

**Format**:
```rust
struct Packet {
    source_peer_id: [u8; 32],    // Sender's peer ID
    payload: Vec<u8>,            // Encrypted payload
}
```

**Client action**: Decrypt payload using shared secret with source peer

---

## Client Implementation

### Connection Lifecycle

**Phase 1: Connect**
```rust
// 1. Establish QUIC connection to relay
let relay_addr = "relay.example.com:8080";
let connection = quinn::connect(relay_addr)?;

// 2. Generate CAPoW proof
let capow_proof = generate_capow_proof(peer_id)?;

// 3. Send REGISTER message
let register_msg = RegisterMessage {
    peer_id: my_peer_id,
    capow_proof,
    protocol_version: 1,
};
send_message(&connection, &register_msg)?;

// 4. Wait for REGISTER_ACK
let ack = receive_message::<RegisterAck>(&connection)?;
if !ack.success {
    return Err("Registration failed");
}

// 5. Store keepalive interval
let keepalive_interval = Duration::from_secs(ack.keepalive_interval_sec as u64);
```

**Phase 2: Maintain Connection**
```rust
// Spawn heartbeat task
tokio::spawn(async move {
    let mut interval = tokio::time::interval(keepalive_interval);
    loop {
        interval.tick().await;
        let heartbeat = Heartbeat { peer_id: my_peer_id };
        if let Err(e) = send_message(&connection, &heartbeat) {
            log::error!("Heartbeat failed: {}", e);
            break;
        }
    }
});
```

**Phase 3: Send/Receive Packets**
```rust
// Send packet to peer
async fn send_to_peer(
    connection: &Connection,
    target_peer_id: [u8; 32],
    payload: Vec<u8>,
) -> Result<()> {
    let relay_msg = RelayPacket {
        source_peer_id: my_peer_id,
        target_peer_id,
        payload,
    };
    send_message(connection, &relay_msg)?;
    
    // Wait for ACK
    let ack = receive_message::<RelayAck>(connection)?;
    if !ack.success {
        return Err("Relay rejected packet");
    }
    Ok(())
}

// Receive packets from peers
async fn receive_loop(connection: &Connection) {
    loop {
        match receive_message::<Packet>(connection) {
            Ok(packet) => {
                log::info!("Received packet from {:?}", packet.source_peer_id);
                handle_packet(packet).await;
            }
            Err(e) => {
                log::error!("Receive error: {}", e);
                break;
            }
        }
    }
}
```

**Phase 4: Disconnect**
```rust
// Graceful shutdown
connection.close(0u32.into(), b"goodbye");
```

---

## CAPoW Generation

### MTP-Argon2id Proof

**Purpose**: Prove computational work to prevent DoS attacks

**Algorithm**: MTP-Argon2id (Memory-hard, ASIC-resistant)

**Parameters** (check relay server for current values):
- Memory cost: 64 MB
- Time cost: 3 iterations
- Parallelism: 4 threads
- Difficulty: Target hash starts with N zero bits

**Implementation** (Rust):
```rust
use argon2::{Argon2, Version};
use sha2::{Sha256, Digest};

fn generate_capow_proof(peer_id: [u8; 32]) -> Result<Vec<u8>> {
    let mut nonce = 0u64;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs();
    
    loop {
        // Build input: peer_id || timestamp || nonce
        let mut input = Vec::new();
        input.extend_from_slice(&peer_id);
        input.extend_from_slice(&timestamp.to_be_bytes());
        input.extend_from_slice(&nonce.to_be_bytes());
        
        // Compute Argon2id hash
        let mut output = [0u8; 32];
        Argon2::default().hash_password_into(
            &input,
            b"FILE-relay-salt", // Fixed salt
            &mut output,
        )?;
        
        // Check if hash meets difficulty target
        let hash_value = u32::from_be_bytes([output[0], output[1], output[2], output[3]]);
        if hash_value < DIFFICULTY_TARGET {
            // Build proof
            let mut proof = Vec::new();
            proof.extend_from_slice(&peer_id);
            proof.extend_from_slice(&timestamp.to_be_bytes());
            proof.extend_from_slice(&nonce.to_be_bytes());
            proof.extend_from_slice(&output);
            return Ok(proof);
        }
        
        nonce += 1;
        
        // Give up after 1 million attempts (should never happen with proper difficulty)
        if nonce > 1_000_000 {
            return Err("CAPoW generation timeout");
        }
    }
}
```

**Performance**: Should take ~50-100ms on modern CPUs. If taking > 1 second, difficulty is too high (report to relay operator).

---

## Error Handling

### Connection Errors

| Error | Cause | Action |
|-------|-------|--------|
| **Connection refused** | Relay down or firewall | Retry with exponential backoff (1s, 2s, 4s, 8s, 16s, 32s max) |
| **TLS handshake failed** | Certificate issue | Verify relay certificate, check system time |
| **Timeout** | Network issue | Retry, try alternate relay |

### Registration Errors

| Error | Cause | Action |
|-------|-------|--------|
| **InvalidCAPoW** | Proof incorrect | Regenerate proof, check difficulty |
| **RateLimited** | Too many requests | Exponential backoff (1s, 2s, 4s...) |
| **CapacityReached** | Relay at max peers | Try another relay |
| **InvalidPeerId** | Peer ID format wrong | Fix peer ID (must be 32 bytes, Ed25519 public key) |
| **ReplayAttack** | Proof already used | Generate fresh proof (include new timestamp) |

### Relay Errors

| Error | Cause | Action |
|-------|-------|--------|
| **TargetNotFound** | Peer B not registered | Wait and retry (peer may reconnect) |
| **PacketTooLarge** | Payload exceeds limit | Split into smaller packets |
| **Timeout** | Relay overloaded | Retry once, then try alternate relay |

---

## Best Practices

### 1. Implement Exponential Backoff

```rust
async fn connect_with_backoff(relay_addr: &str) -> Result<Connection> {
    let mut delay = Duration::from_secs(1);
    let max_delay = Duration::from_secs(32);
    
    loop {
        match quinn::connect(relay_addr) {
            Ok(conn) => return Ok(conn),
            Err(e) => {
                log::warn!("Connection failed: {}, retrying in {:?}", e, delay);
                tokio::time::sleep(delay).await;
                delay = std::cmp::min(delay * 2, max_delay);
            }
        }
    }
}
```

### 2. Cache CAPoW Proofs (Short-Term)

```rust
// CAPoW proofs valid for ~5 minutes
// Cache to avoid recomputation on reconnect
struct ProofCache {
    proof: Vec<u8>,
    expires_at: Instant,
}

impl ProofCache {
    fn get_or_generate(&mut self, peer_id: [u8; 32]) -> Result<Vec<u8>> {
        if Instant::now() < self.expires_at {
            return Ok(self.proof.clone());
        }
        
        let proof = generate_capow_proof(peer_id)?;
        self.proof = proof.clone();
        self.expires_at = Instant::now() + Duration::from_secs(240); // 4 minutes
        Ok(proof)
    }
}
```

### 3. Handle Peer Disconnects Gracefully

```rust
// Peers may disconnect and reconnect with new relay address
// Store peer_id → last_seen mapping
struct PeerRegistry {
    peers: HashMap<[u8; 32], Instant>,
}

impl PeerRegistry {
    fn is_alive(&self, peer_id: &[u8; 32]) -> bool {
        self.peers.get(peer_id)
            .map(|last_seen| last_seen.elapsed() < Duration::from_secs(600))
            .unwrap_or(false)
    }
}
```

### 4. Encrypt All Payloads End-to-End

```rust
// Relay cannot decrypt packets
// Use Noise XX or similar authenticated encryption
use noise_protocol::HandshakeState;

fn encrypt_packet(payload: &[u8], shared_secret: &[u8; 32]) -> Vec<u8> {
    // ChaCha20-Poly1305 or AES-256-GCM
    // ... encryption code ...
}

fn decrypt_packet(encrypted: &[u8], shared_secret: &[u8; 32]) -> Result<Vec<u8>> {
    // ... decryption code ...
}
```

### 5. Monitor Connection Health

```rust
// Track metrics for debugging
struct ConnectionMetrics {
    packets_sent: AtomicU64,
    packets_received: AtomicU64,
    errors: AtomicU64,
    last_heartbeat: Mutex<Instant>,
}

impl ConnectionMetrics {
    fn is_healthy(&self) -> bool {
        self.last_heartbeat.lock().unwrap().elapsed() < Duration::from_secs(180)
    }
}
```

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capow_generation() {
        let peer_id = [0u8; 32];
        let proof = generate_capow_proof(peer_id).unwrap();
        assert!(proof.len() > 32); // Contains nonce + hash
    }

    #[test]
    fn test_register_message_serialization() {
        let msg = RegisterMessage {
            peer_id: [1u8; 32],
            capow_proof: vec![2u8; 64],
            protocol_version: 1,
        };
        let bytes = bincode::serialize(&msg).unwrap();
        let deserialized: RegisterMessage = bincode::deserialize(&bytes).unwrap();
        assert_eq!(msg.peer_id, deserialized.peer_id);
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_registration_flow() {
    // Start local relay server for testing
    let relay = spawn_test_relay().await;
    
    // Connect client
    let client = Client::connect(&relay.addr()).await.unwrap();
    
    // Register
    client.register().await.unwrap();
    
    // Verify registered
    assert!(client.is_registered());
    
    // Cleanup
    relay.shutdown().await;
}
```

### Load Testing

```bash
# Use tool like `ab` or custom script
# Test relay with 100 concurrent clients
cargo run --example load_test -- --relay relay.example.com:8080 --clients 100
```

---

## Example Code

### Complete Client (Rust)

See `examples/simple-client/` in the repository for a full working example.

**Quick start**:
```bash
cd examples/simple-client
cargo run -- --relay relay.example.com:8080 --peer-id <your-peer-id>
```

### Other Languages

**Go**: [github.com/file-network/relay-client-go](https://github.com/file-network/relay-client-go) (TBD)  
**TypeScript**: [github.com/file-network/relay-client-ts](https://github.com/file-network/relay-client-ts) (TBD)  
**Python**: [github.com/file-network/relay-client-py](https://github.com/file-network/relay-client-py) (TBD)

---

## Troubleshooting

### "CAPoW verification failed"

- **Check difficulty**: Relay may have increased difficulty
- **Check timestamp**: Ensure system clock is accurate (NTP sync)
- **Check algorithm**: Must be MTP-Argon2id (not plain Argon2)

### "Rate limited"

- **Slow down**: Implement exponential backoff
- **Check burst**: Ensure not sending > 50 requests in quick succession
- **Contact operator**: If legitimate use case, request rate limit increase

### "Peer not found"

- **Wait and retry**: Peer may be reconnecting
- **Check peer_id**: Ensure correct 32-byte Ed25519 public key
- **Out-of-band communication**: Coordinate with peer to ensure both registered

---

## Further Reading

- [Architecture Overview](docs/architecture/SYSTEM-OVERVIEW.md) — Relay server design
- [Security Policy](SECURITY.md) — Security features and guarantees
- [FAQ](docs/FAQ.md) — Frequently asked questions
- [MPQUIC Specification](https://datatracker.ietf.org/doc/draft-deconinck-quic-multipath/) — MPQUIC protocol

---

**Questions?** [Open an issue](https://github.com/file-network/relay-server/issues) or join [GitHub Discussions](https://github.com/file-network/relay-server/discussions).
