# FILE Relay Client Examples

**Purpose**: Example client implementations for integrating with FILE Relay Server.

---

## Overview

FILE Relay Server uses a simple UDP-based protocol for peer-to-peer packet relaying. This directory contains reference implementations in multiple languages.

**Protocol Summary**:
1. Fetch current challenge from relay
2. Generate CAPoW proof
3. Send REGISTER packet with proof
4. Receive REGISTER_ACK
5. Send/receive RELAY packets

**Full protocol specification**: [../../docs/architecture/PROTOCOL-SPECIFICATION.md](../../docs/architecture/PROTOCOL-SPECIFICATION.md)

---

## Client Implementations

| Language | Status | Features | File |
|----------|--------|----------|------|
| **Rust** | ✅ Complete | Full protocol, async/await, error handling | [rust/](rust/) |
| **Python** | ✅ Complete | Asyncio, simple API | [python/](python/) |
| **TypeScript** | ✅ Complete | Node.js + browser, Promise-based | [typescript/](typescript/) |
| **Go** | 📝 Planned | Goroutines, channels | — |
| **Swift** | 📝 Planned | iOS/macOS native | — |

---

## Quick Start

### Rust

```bash
cd examples/clients/rust
cargo run --example register
```

```rust
use file_relay_client::{RelayClient, ClientConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let config = ClientConfig {
        relay_url: "udp://relay.example.com:8080",
        peer_id: PeerId::generate(),
    };
    
    let client = RelayClient::new(config).await?;
    
    // Register with relay
    client.register().await?;
    
    // Send packet to another peer
    let dest_peer_id = PeerId::from_hex("abcd...")?;
    let payload = b"Hello, peer!";
    client.send_to(dest_peer_id, payload).await?;
    
    // Receive packets
    while let Some((from_peer, data)) = client.recv().await {
        println!("Received from {}: {:?}", from_peer, data);
    }
    
    Ok(())
}
```

**Full example**: [rust/examples/register.rs](rust/examples/register.rs)

### Python

```bash
cd examples/clients/python
pip install -r requirements.txt
python examples/register.py
```

```python
import asyncio
from file_relay_client import RelayClient, PeerId

async def main():
    client = RelayClient(
        relay_url="udp://relay.example.com:8080",
        peer_id=PeerId.generate()
    )
    
    # Register with relay
    await client.register()
    
    # Send packet to another peer
    dest_peer_id = PeerId.from_hex("abcd...")
    await client.send_to(dest_peer_id, b"Hello, peer!")
    
    # Receive packets
    async for from_peer, data in client.recv():
        print(f"Received from {from_peer}: {data}")

asyncio.run(main())
```

**Full example**: [python/examples/register.py](python/examples/register.py)

### TypeScript (Node.js)

```bash
cd examples/clients/typescript
npm install
npm run example:register
```

```typescript
import { RelayClient, PeerId } from 'file-relay-client';

async function main() {
  const client = new RelayClient({
    relayUrl: 'udp://relay.example.com:8080',
    peerId: PeerId.generate(),
  });
  
  // Register with relay
  await client.register();
  
  // Send packet to another peer
  const destPeerId = PeerId.fromHex('abcd...');
  await client.sendTo(destPeerId, Buffer.from('Hello, peer!'));
  
  // Receive packets
  client.on('packet', (fromPeer, data) => {
    console.log(`Received from ${fromPeer}: ${data}`);
  });
}

main();
```

**Full example**: [typescript/examples/register.ts](typescript/examples/register.ts)

---

## Protocol Flow

### 1. Fetch Challenge

```
Client → Relay: GET /challenge HTTP/1.1
Relay → Client: {"merkle_root": "a3f5...", "created_at": 1705881600, "expires_at": 1705885500}
```

**Purpose**: Get current CAPoW challenge from relay.

**Endpoint**: `http://relay.example.com:8081/challenge`

**Caching**: Challenge valid for 1 hour. Cache locally and refresh on registration failure.

### 2. Generate CAPoW Proof

```
Input: merkle_root, peer_id, timestamp
Process: MTP-Argon2id computation (~100ms)
Output: MtpProof { merkle_root, peer_id, timestamp, iterations, hash_chain }
```

**Purpose**: Prove computational work to relay (DoS protection).

**Algorithm**: [../../docs/architecture/CAPOW.md](../../docs/architecture/CAPOW.md)

**Libraries**:
- Rust: `relay_daemon` crate (included in this repo)
- Python: `argon2-cffi` + custom MTP implementation
- TypeScript: `argon2` + custom MTP implementation

### 3. Send REGISTER Packet

```
┌──────────────────────────────────────────────────────────┐
│ Type (1 byte) │ Peer ID (32 bytes) │ CAPoW Proof (var) │
├───────────────┼────────────────────┼────────────────────┤
│      0x01     │   [peer_id]        │   [MtpProof]       │
└──────────────────────────────────────────────────────────┘
```

**Purpose**: Register with relay and start receiving packets.

**Serialization**: Use MessagePack or Protocol Buffers for MtpProof.

### 4. Receive REGISTER_ACK

```
┌─────────────────────────────────────────────────────┐
│ Type (1 byte) │ Status (1 byte) │ Message (var)   │
├───────────────┼─────────────────┼─────────────────┤
│      0x02     │  0x00 = OK      │   "OK"          │
│               │  0x01 = Error   │   "CAPoW fail"  │
└─────────────────────────────────────────────────────┘
```

**Status codes**:
- `0x00` (OK) — Registration successful
- `0x01` (Error) — CAPoW verification failed, rate limited, or capacity reached

**Error handling**:
- `CAPoW invalid` → Fetch new challenge and retry
- `Rate limited` → Exponential backoff (1s, 2s, 4s, ...)
- `Capacity reached` → Try different relay or wait

### 5. Send RELAY Packet

```
┌──────────────────────────────────────────────────────────────┐
│ Type (1 byte) │ Dest Peer ID (32 bytes) │ Payload (var)   │
├───────────────┼─────────────────────────┼─────────────────┤
│      0x03     │   [dest_peer_id]        │   [MPQUIC pkt]  │
└──────────────────────────────────────────────────────────────┘
```

**Purpose**: Send packet to another peer via relay.

**Payload**: Opaque byte array (relay doesn't inspect). Typically encrypted MPQUIC packet.

**No ACK**: RELAY packets are not acknowledged by relay. Use application-level ACKs if needed.

### 6. Receive RELAY Packet

```
Client receives UDP packet from relay:
┌──────────────────────────────────────────────────────────────┐
│ Type (1 byte) │ Src Peer ID (32 bytes) │ Payload (var)    │
├───────────────┼────────────────────────┼──────────────────┤
│      0x03     │   [src_peer_id]        │   [MPQUIC pkt]   │
└──────────────────────────────────────────────────────────────┘
```

**Purpose**: Receive packet from another peer.

**Verification**: Client should verify src_peer_id matches expected peer (prevent spoofing).

### 7. Send KEEPALIVE (Optional)

```
┌─────────────────────────────────┐
│ Type (1 byte) │ Timestamp (8 B) │
├───────────────┼─────────────────┤
│      0x04     │  [unix_time]    │
└─────────────────────────────────┘
```

**Purpose**: Reset peer timeout (5-minute inactivity).

**When to send**: If no RELAY packets sent in last 4 minutes.

**Alternative**: Just send RELAY packets regularly (even empty) instead of KEEPALIVE.

---

## Common Integration Patterns

### Pattern 1: Simple P2P Chat

```
Peer A                  Relay                  Peer B
  │                       │                       │
  │─────REGISTER────────▶│                       │
  │◀────REGISTER_ACK─────│                       │
  │                       │◀────REGISTER─────────│
  │                       │─────REGISTER_ACK────▶│
  │                       │                       │
  │──RELAY(msg:"Hello")─▶│                       │
  │                       │──RELAY(msg:"Hello")─▶│
  │                       │                       │
  │                       │◀─RELAY(msg:"Hi!")────│
  │◀─RELAY(msg:"Hi!")────│                       │
```

**Use case**: Real-time text chat between two peers.

**Implementation**: [rust/examples/p2p_chat.rs](rust/examples/p2p_chat.rs)

### Pattern 2: Group Broadcast

```
Peer A sends message to N peers:
1. For each peer in group:
   2. client.send_to(peer_id, payload)
3. Relay forwards to each peer
```

**Use case**: Group chat, multiplayer game state sync.

**Optimization**: Use multicast if relay supports it (future).

**Implementation**: [python/examples/group_broadcast.py](python/examples/group_broadcast.py)

### Pattern 3: File Transfer

```
Sender:
1. Split file into chunks (64 KB each)
2. For each chunk:
   3. send_to(receiver_id, chunk_data)
   4. Wait for ACK (application-level)
5. send_to(receiver_id, EOF_marker)

Receiver:
1. Receive chunks
2. Send ACK for each chunk
3. Reassemble file
```

**Use case**: Large file transfer over unreliable network.

**Implementation**: [typescript/examples/file_transfer.ts](typescript/examples/file_transfer.ts)

### Pattern 4: MPQUIC Connection

```
Client A:
1. Establish MPQUIC connection to relay
2. Send MPQUIC packets via RELAY protocol
3. Relay forwards to Client B
4. Client B receives MPQUIC packets
5. MPQUIC connection established end-to-end

Note: Relay is opaque transport layer for MPQUIC
```

**Use case**: Full MPQUIC connection over relay (FILE network use case).

**Implementation**: Requires MPQUIC library integration (not yet available).

---

## Best Practices

### 1. Challenge Caching

**DO**:
```rust
let mut cached_challenge = None;

loop {
    if cached_challenge.is_none() || challenge_expired() {
        cached_challenge = Some(fetch_challenge().await?);
    }
    
    let proof = generate_proof(cached_challenge.unwrap()).await?;
    match register(proof).await {
        Ok(_) => break,
        Err(e) if e.is_invalid_challenge() => {
            cached_challenge = None; // Fetch new challenge
            continue;
        }
        Err(e) => return Err(e),
    }
}
```

**DON'T**:
```rust
// BAD: Fetch challenge on every registration attempt (wasteful)
let challenge = fetch_challenge().await?;
let proof = generate_proof(challenge).await?;
register(proof).await?;
```

### 2. Exponential Backoff

**DO**:
```python
async def register_with_backoff(client):
    delay = 1  # Start with 1 second
    max_delay = 60  # Cap at 60 seconds
    
    while True:
        try:
            await client.register()
            return
        except RateLimitError:
            await asyncio.sleep(delay)
            delay = min(delay * 2, max_delay)
```

**DON'T**:
```python
# BAD: Immediate retry on rate limit (hammers relay)
while True:
    try:
        await client.register()
        return
    except RateLimitError:
        continue  # Instant retry
```

### 3. Packet Validation

**DO**:
```typescript
client.on('packet', (fromPeer, data) => {
  // Verify sender is expected peer
  if (!expectedPeers.has(fromPeer)) {
    console.warn(`Unexpected packet from ${fromPeer}`);
    return;
  }
  
  // Decrypt and verify signature
  const decrypted = decrypt(data, sessionKey);
  if (!verify(decrypted)) {
    console.warn(`Invalid signature from ${fromPeer}`);
    return;
  }
  
  handlePacket(decrypted);
});
```

**DON'T**:
```typescript
// BAD: Trust all packets blindly
client.on('packet', (fromPeer, data) => {
  handlePacket(data); // No validation!
});
```

### 4. Resource Cleanup

**DO**:
```rust
let client = RelayClient::new(config).await?;

// Use tokio::select! for graceful shutdown
tokio::select! {
    _ = client.run() => {},
    _ = tokio::signal::ctrl_c() => {
        client.shutdown().await?;
    }
}
```

**DON'T**:
```rust
// BAD: Leak resources on exit
let client = RelayClient::new(config).await?;
client.run().await?; // No cleanup on ctrl+c
```

---

## Testing Your Client

### Unit Tests

Test packet serialization/deserialization:

```rust
#[test]
fn test_register_packet_serialization() {
    let peer_id = PeerId::generate();
    let proof = MtpProof::mock();
    let packet = RegisterPacket { peer_id, proof };
    
    let bytes = packet.serialize();
    assert_eq!(bytes[0], 0x01); // Type = REGISTER
    
    let deserialized = RegisterPacket::deserialize(&bytes).unwrap();
    assert_eq!(deserialized.peer_id, peer_id);
}
```

### Integration Tests

Test against local relay:

```python
async def test_register():
    # Start local relay
    relay_proc = await start_local_relay()
    
    # Create client
    client = RelayClient("udp://localhost:8080", PeerId.generate())
    
    # Should register successfully
    await client.register()
    
    # Cleanup
    await client.close()
    relay_proc.terminate()
```

### Load Tests

Test with many concurrent clients:

```typescript
async function loadTest() {
  const clients = [];
  
  // Create 1000 clients
  for (let i = 0; i < 1000; i++) {
    const client = new RelayClient({
      relayUrl: 'udp://localhost:8080',
      peerId: PeerId.generate(),
    });
    clients.push(client);
  }
  
  // Register all in parallel
  await Promise.all(clients.map(c => c.register()));
  
  console.log('1000 clients registered successfully');
}
```

---

## Troubleshooting

### Registration Fails with "CAPoW invalid"

**Cause**: Challenge expired or proof generation incorrect.

**Solution**:
1. Fetch fresh challenge: `GET /challenge`
2. Verify proof generation matches algorithm in [CAPOW.md](../../docs/architecture/CAPOW.md)
3. Check system time is synchronized (NTP)

### Packets Not Received

**Cause**: Firewall blocking UDP, or peer not registered.

**Solution**:
1. Verify both peers registered successfully
2. Test UDP connectivity: `nc -vzu relay-ip 8080`
3. Check relay logs for dropped packets
4. Verify dest_peer_id is correct (32 bytes hex)

### High Latency

**Cause**: Network latency or relay overload.

**Solution**:
1. Measure round-trip time: `ping relay-ip`
2. Check relay metrics: `curl http://relay-ip:8081/metrics`
3. If relay CPU > 80%, scale horizontally
4. Consider multi-region deployment (lower latency)

### Memory Leak

**Cause**: Not closing clients properly.

**Solution**:
```rust
// WRONG: Never call client.close()
let client = RelayClient::new(config).await?;
client.run().await?;
// Client never closed → socket leak

// RIGHT: Always close
let client = RelayClient::new(config).await?;
tokio::select! {
    _ = client.run() => {},
    _ = shutdown_signal() => {
        client.close().await?; // Cleanup
    }
}
```

---

## Contributing

Want to add a client implementation in another language?

1. Follow protocol specification: [PROTOCOL-SPECIFICATION.md](../../docs/architecture/PROTOCOL-SPECIFICATION.md)
2. Implement packet serialization (REGISTER, RELAY, KEEPALIVE)
3. Implement CAPoW proof generation (see [CAPOW.md](../../docs/architecture/CAPOW.md))
4. Add tests (unit + integration)
5. Add example usage
6. Submit PR!

**See**: [CONTRIBUTING.md](../../CONTRIBUTING.md)

---

## References

- [Protocol Specification](../../docs/architecture/PROTOCOL-SPECIFICATION.md)
- [CAPoW Algorithm](../../docs/architecture/CAPOW.md)
- [Client Integration Guide](../../docs/CLIENT-INTEGRATION-GUIDE.md)
- [FAQ](../../docs/FAQ.md)

---

**Status**: Reference implementations pending event loop completion  
**Last Updated**: 2026-05-18
