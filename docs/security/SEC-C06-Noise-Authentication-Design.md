# SEC-C06: Noise XX Authentication Design

**Status:** BLOCKED - Requires connection event loop infrastructure  
**Priority:** CRITICAL  
**Created:** 2026-05-18  

## Problem

The relay server currently accepts unauthenticated connections. An attacker can:
- Register arbitrary peer_ids without proving ownership
- Impersonate legitimate peers
- Launch DoS attacks without authentication cost

## Solution: Noise XX Authentication

Require Noise XX handshake between peer and relay BEFORE allowing any peer operations.

### Architecture

```
┌─────────┐                          ┌─────────┐
│  Peer   │                          │  Relay  │
│(Client) │                          │(Server) │
└─────────┘                          └─────────┘
     │                                     │
     │  1. TCP/QUIC Connection            │
     ├────────────────────────────────────>│
     │                                     │
     │  2. Noise XX Handshake (3 msgs)    │
     │     -> e                            │
     ├────────────────────────────────────>│
     │     <- e, ee, s, es                 │
     │<────────────────────────────────────┤
     │     -> s, se                        │
     ├────────────────────────────────────>│
     │                                     │
     │  3. Authenticated! peer_id = peer's │
     │     static public key (32 bytes)    │
     │                                     │
     │  4. REGISTER_PEER message           │
     ├────────────────────────────────────>│
     │                                     │
     │  5. FORWARD_PACKET (E2E encrypted)  │
     ├────────────────────────────────────>│
     │                                     │
```

### Key Design Decisions

1. **Relay is Responder, Peer is Initiator**
   - Relay generates one static keypair on startup
   - Peers initiate Noise XX handshake when connecting
   - Relay public key can be published/distributed

2. **peer_id = Peer's Static Public Key**
   - After successful Noise XX handshake, relay extracts peer's authenticated static public key
   - This 32-byte Ed25519 public key becomes the peer_id
   - Cannot be spoofed (would require breaking Noise XX or stealing private key)

3. **E2E Encryption Preserved**
   - Relay authenticates the CONNECTION, not the PACKETS
   - Forwarded packets remain E2E encrypted between P2P peers
   - Relay cannot decrypt forwarded payload (only verifies source peer_id)

4. **Integration with crypto_fabric::noise**
   ```rust
   // Relay generates static keypair once on startup
   let relay_static_keypair = Builder::new("Noise_XX_25519_ChaChaPoly_SHA256".parse()?)
       .generate_keypair()?;
   
   // Per incoming connection:
   let mut handshake = NoiseHandshake::new_responder(&relay_static_keypair.private)?;
   
   // Process 3 handshake messages (-> e, <- e,ee,s,es, -> s,se)
   handshake.process_incoming_or_fallback(&msg1)?;
   let response = handshake.write_message(&[])?;
   // ... send response, receive msg3 ...
   
   // After handshake completes:
   let transport = handshake.into_transport()?;
   let peer_static_pub = handshake.get_remote_static_pub(); // peer_id!
   
   // Now register_peer can be called with authenticated peer_id
   state.register_peer(peer_static_pub, addr, peer_id_quic).await?;
   ```

### Implementation Requirements

**BLOCKED BY:** Connection event loop infrastructure (main.rs:236-237 TODO)

#### Required Infrastructure

1. **P2pEndpoint Connection Events**
   - Need API to accept/iterate incoming connections
   - Need connection-scoped state management
   - Need message framing for handshake vs. data packets

2. **Handshake State Management**
   ```rust
   struct PendingHandshake {
       handshake: NoiseHandshake,
       addr: SocketAddr,
       peer_id_quic: PeerId,
       timeout: Instant,  // SEC-H02: handshake timeout
   }
   
   struct RelayState {
       // ... existing fields ...
       pending_handshakes: DashMap<ConnectionId, PendingHandshake>,
   }
   ```

3. **Message Protocol**
   ```rust
   enum RelayMessage {
       NoiseHandshake(Vec<u8>),    // Noise XX handshake message
       RegisterPeer,               // After auth: register this connection
       ForwardPacket { to: Vec<u8>, data: Vec<u8> },  // E2E encrypted payload
   }
   ```

4. **Timeout Enforcement (SEC-H02)**
   - Handshake must complete within 30 seconds
   - Cleanup zombie handshake states

### Security Properties

After implementation:

1. ✅ **Authentication**: Only peers with valid Ed25519 keypairs can register
2. ✅ **No Impersonation**: peer_id cryptographically bound to private key
3. ✅ **DoS Resistance**: Handshake + CAPoW both required
4. ✅ **Forward Secrecy**: Noise XX provides perfect forward secrecy
5. ✅ **Quantum Resistance**: X25519 + ML-KEM-768 hybrid (future upgrade path)

### Testing Plan

1. **Unit Tests**
   - Valid Noise XX handshake flow
   - Reject incomplete handshakes
   - Reject expired handshake states
   - Prevent duplicate peer_id registration

2. **Integration Tests**
   - Peer connects → handshake → register → forward packet
   - Multiple peers with unique peer_ids
   - Attempt to register without handshake (should fail)
   - Attempt to forward packet before registration (should fail)

3. **Fuzzing**
   - Malformed Noise messages
   - Out-of-order handshake messages
   - Replay attacks

### Migration Path

Phase 1 (Current): Implement other critical fixes while connection loop is built  
Phase 2: Implement connection event loop + basic message framing  
Phase 3: Integrate Noise XX authentication (this design)  
Phase 4: Deploy to test VPS and validate with real peers  

### References

- `crypto_fabric::noise` module (fully implemented Noise XXfallback)
- Noise Protocol Framework: https://noiseprotocol.org/
- WireGuard (production Noise XX user)
- SEC-C06 audit finding in `FILE-Security-Analysis-Consolidated-2026-05-18.md`

---

**Next Steps:**
1. Complete SEC-H01 through SEC-H04 (can be done without connection loop)
2. Implement connection event loop infrastructure
3. Return to SEC-C06 and implement this design
4. Integration testing before VPS deployment
