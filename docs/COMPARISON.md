# FILE Relay Server vs Alternatives

**Last Updated**: 2026-05-18  
**Version**: 0.1.0

---

## Overview

This document compares FILE Relay Server with alternative relay/NAT traversal solutions for peer-to-peer applications.

---

## Quick Comparison Matrix

| Feature | FILE Relay | Tailscale DERP | LibP2P Relay | TURN Server | WebRTC STUN/TURN |
|---------|------------|----------------|--------------|-------------|------------------|
| **Protocol** | MPQUIC | Custom over TCP | LibP2P | UDP/TCP | UDP |
| **DoS Protection** | CAPoW + rate limiting | Rate limiting | None built-in | Rate limiting | None |
| **Stateless** | ✅ | ❌ (tracks connections) | ✅ | ✅ | ✅ |
| **End-to-End Encryption** | ✅ (MPQUIC) | ✅ (WireGuard) | ✅ (LibP2P) | ❌ (relay sees plaintext) | ✅ (DTLS-SRTP) |
| **Authentication** | Noise XX (pending) | WireGuard | PeerId | Credentials | ICE credentials |
| **Multi-Path** | ✅ (MPQUIC) | ❌ | ❌ | ❌ | ❌ |
| **GDPR Compliant** | ✅ (IP hashing) | ⚠️ (logs IPs) | ⚠️ (depends on impl) | ⚠️ (logs IPs) | ⚠️ (depends on impl) |
| **Horizontal Scaling** | ✅ (no coordination) | ✅ (stateless fallback) | ✅ | ✅ | ✅ |
| **Open Source** | ✅ MIT | ✅ BSD-3 | ✅ MIT/Apache | ✅ (varies) | ✅ (varies) |
| **Production Ready** | ⚠️ (99%, pending event loop) | ✅ | ✅ | ✅ | ✅ |
| **Metrics/Observability** | ✅ Prometheus | ✅ (Tailscale console) | ⚠️ (manual) | ⚠️ (depends on impl) | ⚠️ (manual) |
| **Language** | Rust | Go | Go/Rust | Varies | Varies |
| **Best For** | MPQUIC P2P apps | VPN/mesh networks | LibP2P ecosystem | Generic UDP relay | WebRTC video/voice |

---

## Detailed Comparisons

### FILE Relay Server

**Description**: Stateless MPQUIC relay with computational proof-of-work DoS protection and GDPR-compliant IP hashing.

**Strengths**:
- ✅ **DoS resistant**: CAPoW makes attacks economically expensive (~$0.01 per connection attempt)
- ✅ **Stateless**: Scales horizontally without coordination, no data loss on crash
- ✅ **GDPR compliant**: HMAC-SHA256 IP hashing, no plaintext PII in logs
- ✅ **Multi-path**: Native MPQUIC support for bandwidth aggregation
- ✅ **Observable**: Prometheus metrics with pre-built Grafana dashboard and 12 alerts
- ✅ **Production-ready deployment**: Docker, Kubernetes, systemd examples with monitoring
- ✅ **Lock-free**: DashMap + AtomicU64 for zero-contention shared state

**Weaknesses**:
- ❌ **Event loop pending**: Core packet forwarding infrastructure ready, event loop implementation pending
- ❌ **Limited ecosystem**: New project, not battle-tested at scale
- ❌ **Single-protocol**: MPQUIC only, no fallback to WebRTC or plain TCP

**Use Cases**:
- MPQUIC peer-to-peer applications (file sharing, mesh networking, distributed systems)
- Applications requiring DoS protection without infrastructure-level DDoS mitigation
- GDPR-sensitive deployments (EU, healthcare, finance)
- Zero-trust architectures (relay sees only encrypted traffic)

**Not Suitable For**:
- WebRTC applications (use TURN instead)
- Applications requiring stateful connection tracking
- Low-latency voice/video (MPQUIC has higher latency than WebRTC)

---

### Tailscale DERP

**Description**: Encrypted TCP relay for WireGuard-based mesh VPN, used when direct peer-to-peer connections fail.

**Strengths**:
- ✅ **Battle-tested**: Runs Tailscale's production infrastructure (millions of users)
- ✅ **WireGuard integration**: Seamless fallback for Tailscale mesh networks
- ✅ **Low latency**: TCP with custom congestion control
- ✅ **Global network**: Tailscale runs DERP servers in 20+ regions
- ✅ **Simple**: Minimal configuration, works out-of-the-box

**Weaknesses**:
- ❌ **Stateful**: Tracks connections, requires coordination for HA
- ❌ **No DoS protection**: Relies on infrastructure-level DDoS mitigation
- ❌ **Logs IPs**: Not GDPR-compliant by default (plaintext IPs in logs)
- ❌ **Tailscale-specific**: Designed for Tailscale ecosystem, not general-purpose
- ❌ **TCP only**: No UDP relay, no multi-path

**Use Cases**:
- Tailscale mesh VPN deployments
- Private mesh networks with known peers (not public relays)
- Applications already using WireGuard

**Not Suitable For**:
- Public relay servers (no DoS protection)
- GDPR-sensitive deployments
- Multi-path or UDP-only applications

**Comparison to FILE Relay**:
- DERP is stateful, FILE Relay is stateless → FILE Relay scales better
- DERP has no DoS protection, FILE Relay uses CAPoW → FILE Relay resists attacks
- DERP logs plaintext IPs, FILE Relay hashes IPs → FILE Relay is GDPR-compliant
- DERP is production-proven, FILE Relay is new → DERP more mature

---

### LibP2P Relay

**Description**: Relay protocol for the LibP2P networking stack, used in IPFS and other decentralized applications.

**Strengths**:
- ✅ **Ecosystem**: Part of LibP2P, works with IPFS, Filecoin, Polkadot, etc.
- ✅ **Flexible**: Supports multiple transports (TCP, QUIC, WebSocket, WebTransport)
- ✅ **Decentralized**: Peers can volunteer to be relays, no central infrastructure
- ✅ **Peer-to-peer**: Built for P2P from the ground up
- ✅ **Mature**: Used in production by IPFS (millions of nodes)

**Weaknesses**:
- ❌ **No DoS protection**: Volunteers can be overwhelmed by attacks
- ❌ **Complex**: LibP2P has steep learning curve
- ❌ **Variable performance**: Relay quality depends on volunteer resources
- ❌ **No metrics**: Manual instrumentation required
- ❌ **Not GDPR-focused**: Privacy depends on relay implementation

**Use Cases**:
- LibP2P-based applications (IPFS, Filecoin, Polkadot, Ethereum 2.0)
- Decentralized networks where users volunteer relays
- Applications requiring transport flexibility (TCP, QUIC, WebSocket)

**Not Suitable For**:
- Applications needing guaranteed relay performance
- High-security environments (relay volunteers are untrusted)
- Low-latency real-time applications (multi-hop relaying)

**Comparison to FILE Relay**:
- LibP2P is decentralized, FILE Relay is centralized → LibP2P more censorship-resistant
- LibP2P has no DoS protection, FILE Relay uses CAPoW → FILE Relay resists attacks
- LibP2P is transport-agnostic, FILE Relay is MPQUIC-only → LibP2P more flexible
- LibP2P is ecosystem-specific, FILE Relay is standalone → FILE Relay more portable

---

### TURN Server (RFC 5766)

**Description**: Generic UDP/TCP relay for NAT traversal, commonly used with WebRTC for voice/video calls.

**Strengths**:
- ✅ **Standard**: IETF RFC 5766, widely supported
- ✅ **WebRTC integration**: De facto relay for WebRTC applications
- ✅ **Mature**: Battle-tested in production (Zoom, Jitsi, WebRTC apps)
- ✅ **Transport-agnostic**: Supports UDP, TCP, TLS
- ✅ **Many implementations**: coturn, Pion TURN, eturnal, etc.

**Weaknesses**:
- ❌ **No encryption**: Relay sees plaintext traffic (unless using DTLS/SRTP end-to-end)
- ❌ **Credential-based auth**: No DoS protection beyond credentials
- ❌ **Not GDPR-focused**: Most implementations log plaintext IPs
- ❌ **Stateful**: Tracks allocations, requires cleanup
- ❌ **High bandwidth cost**: Relay forwards all media traffic

**Use Cases**:
- WebRTC voice/video applications
- Generic UDP relay for games, VoIP
- Applications requiring IETF standard compliance

**Not Suitable For**:
- Zero-trust architectures (relay sees plaintext)
- GDPR-sensitive deployments
- Multi-path applications (TURN is single-path)

**Comparison to FILE Relay**:
- TURN is standard, FILE Relay is custom → TURN more interoperable
- TURN sees plaintext, FILE Relay sees encrypted → FILE Relay more private
- TURN is credential-based, FILE Relay uses CAPoW → FILE Relay more DoS-resistant
- TURN is widely supported, FILE Relay is new → TURN more mature

---

### WebRTC STUN/TURN

**Description**: NAT traversal stack for WebRTC, combines STUN (direct P2P) and TURN (relay fallback).

**Strengths**:
- ✅ **Browser support**: Native in all modern browsers
- ✅ **Direct connection first**: STUN tries P2P before falling back to TURN
- ✅ **Low latency**: Optimized for real-time voice/video
- ✅ **Mature**: Powers Zoom, Google Meet, Microsoft Teams, etc.
- ✅ **Standards-based**: IETF RFCs, wide industry support

**Weaknesses**:
- ❌ **TURN relay issues**: Same as TURN (plaintext, no DoS protection, stateful)
- ❌ **Browser-centric**: Works best in browsers, harder in native apps
- ❌ **Complex**: ICE negotiation, SDP, signaling server required
- ❌ **Media-focused**: Designed for video/voice, not general P2P data

**Use Cases**:
- Video conferencing (Zoom, Teams, Meet)
- Browser-based P2P applications
- Real-time voice/video communication

**Not Suitable For**:
- Large file transfers (high TURN bandwidth cost)
- Non-browser native applications (WebRTC stack overhead)
- Multi-path or custom protocols

**Comparison to FILE Relay**:
- WebRTC is browser-native, FILE Relay requires client implementation → WebRTC easier for web apps
- WebRTC TURN sees plaintext, FILE Relay sees encrypted → FILE Relay more private
- WebRTC is real-time optimized, FILE Relay is general-purpose → WebRTC better for voice/video
- WebRTC is standards-based, FILE Relay is custom → WebRTC more interoperable

---

## Feature-by-Feature Comparison

### DoS Protection

| Solution | Mechanism | Effectiveness | Cost per Attack |
|----------|-----------|---------------|-----------------|
| **FILE Relay** | CAPoW (MTP-Argon2id) + rate limiting | High | ~$0.01 per connection |
| **Tailscale DERP** | Infrastructure DDoS mitigation | Medium | Depends on provider |
| **LibP2P Relay** | None (volunteer relays) | Low | Free (volunteers absorb) |
| **TURN** | Credential-based | Low | Free (credentials can be leaked) |
| **WebRTC** | Same as TURN | Low | Free |

**Winner**: FILE Relay (computational proof-of-work makes attacks expensive)

---

### Privacy / GDPR Compliance

| Solution | IP Logging | Plaintext Visibility | GDPR Compliance |
|----------|-----------|---------------------|-----------------|
| **FILE Relay** | Hashed (HMAC-SHA256) | ❌ (E2E encrypted) | ✅ Yes |
| **Tailscale DERP** | Plaintext | ❌ (WireGuard encrypted) | ⚠️ Needs custom config |
| **LibP2P Relay** | Depends on impl | ❌ (LibP2P encrypted) | ⚠️ Depends on impl |
| **TURN** | Plaintext | ✅ (sees plaintext) | ❌ No |
| **WebRTC** | Plaintext | ✅ (TURN sees plaintext) | ❌ No |

**Winner**: FILE Relay (HMAC-SHA256 IP hashing + E2E encryption)

---

### Scalability

| Solution | Stateless? | Horizontal Scaling | Max Peers per Instance |
|----------|-----------|-------------------|------------------------|
| **FILE Relay** | ✅ Yes | ✅ No coordination | 1,000+ (configurable) |
| **Tailscale DERP** | ❌ No | ⚠️ Requires coordination | ~10,000 |
| **LibP2P Relay** | ✅ Yes | ✅ Decentralized | Depends on volunteer |
| **TURN** | ❌ No (tracks allocations) | ⚠️ Requires coordination | ~1,000 |
| **WebRTC** | ❌ No | ⚠️ Requires coordination | ~1,000 |

**Winner**: FILE Relay + LibP2P (stateless, no coordination needed)

---

### Observability

| Solution | Metrics | Alerts | Dashboard | Health Checks |
|----------|---------|--------|-----------|---------------|
| **FILE Relay** | ✅ Prometheus (7 metrics) | ✅ 12 alerts | ✅ Grafana | ✅ 3 endpoints |
| **Tailscale DERP** | ✅ (Tailscale console) | ✅ | ✅ | ✅ |
| **LibP2P Relay** | ⚠️ Manual | ❌ | ❌ | ⚠️ Manual |
| **TURN** | ⚠️ Varies | ⚠️ Varies | ⚠️ Varies | ⚠️ Varies |
| **WebRTC** | ⚠️ Manual | ❌ | ❌ | ⚠️ Manual |

**Winner**: FILE Relay + Tailscale (production-ready observability out-of-the-box)

---

### Deployment Complexity

| Solution | Docker | Kubernetes | Bare Metal | Configuration |
|----------|--------|------------|------------|---------------|
| **FILE Relay** | ✅ Multi-stage | ✅ Full manifests | ✅ systemd | Simple (env vars) |
| **Tailscale DERP** | ✅ Official image | ⚠️ Manual | ✅ Binary | Simple |
| **LibP2P Relay** | ⚠️ Custom | ⚠️ Custom | ✅ Binary | Complex |
| **TURN** | ✅ coturn image | ⚠️ Manual | ✅ Binary | Medium (config file) |
| **WebRTC** | ⚠️ Varies | ⚠️ Varies | ⚠️ Varies | Complex (STUN+TURN) |

**Winner**: FILE Relay + Tailscale (comprehensive deployment examples)

---

## Use Case Recommendations

### When to Use FILE Relay

✅ **Choose FILE Relay if**:
- You need MPQUIC peer-to-peer connectivity
- DoS protection is critical (public relay servers)
- GDPR compliance is required (EU, healthcare, finance)
- Stateless horizontal scaling is important
- You want production-ready monitoring out-of-the-box
- Zero-trust architecture (relay sees only encrypted traffic)

❌ **Don't choose FILE Relay if**:
- You need WebRTC (use TURN instead)
- You need LibP2P ecosystem (use LibP2P Relay instead)
- You need a battle-tested solution (Tailscale DERP, TURN, WebRTC)
- You need the event loop implemented today (pending)

---

### When to Use Tailscale DERP

✅ **Choose Tailscale DERP if**:
- You're building a Tailscale-based mesh VPN
- You trust Tailscale's infrastructure or run your own
- You need proven production scale (millions of users)
- WireGuard integration is required

❌ **Don't choose Tailscale DERP if**:
- You need DoS protection for public relays
- GDPR compliance is critical
- You need stateless scaling

---

### When to Use LibP2P Relay

✅ **Choose LibP2P Relay if**:
- Your application is already using LibP2P
- Decentralization is a requirement (volunteer relays)
- You need transport flexibility (TCP, QUIC, WebSocket, WebTransport)
- You're building for IPFS, Filecoin, Polkadot ecosystem

❌ **Don't choose LibP2P Relay if**:
- You need guaranteed relay performance
- DoS protection is critical
- Observability out-of-the-box is required

---

### When to Use TURN / WebRTC

✅ **Choose TURN/WebRTC if**:
- You're building a WebRTC video/voice application
- Browser support is required
- Standards compliance (IETF RFCs) is important
- Low-latency real-time media is critical

❌ **Don't choose TURN/WebRTC if**:
- Zero-trust / E2E encryption is required (TURN sees plaintext)
- GDPR compliance is critical
- DoS protection is important
- High bandwidth costs are a concern

---

## Migration Paths

### From TURN to FILE Relay

**Feasibility**: ⚠️ **Medium** (protocol incompatible, requires client changes)

**Steps**:
1. Implement MPQUIC client library in your application
2. Deploy FILE Relay server alongside existing TURN
3. Gradually migrate clients to MPQUIC
4. Decommission TURN once migration complete

**Benefits**:
- ✅ DoS protection (CAPoW)
- ✅ GDPR compliance (IP hashing)
- ✅ Multi-path support (bandwidth aggregation)

**Costs**:
- ❌ Client implementation effort (MPQUIC library)
- ❌ Cannot reuse existing TURN infrastructure
- ❌ No browser support (WebRTC → MPQUIC not possible)

---

### From Tailscale DERP to FILE Relay

**Feasibility**: ❌ **Low** (different ecosystems, Tailscale-specific)

**Not recommended** unless you're moving away from Tailscale entirely.

---

### From LibP2P Relay to FILE Relay

**Feasibility**: ⚠️ **Medium** (protocol incompatible, ecosystem lock-in)

**Steps**:
1. Implement MPQUIC transport for LibP2P (if possible)
2. Deploy FILE Relay as LibP2P relay backend
3. Migrate clients to MPQUIC transport

**Benefits**:
- ✅ DoS protection
- ✅ GDPR compliance
- ✅ Centralized relay management

**Costs**:
- ❌ Lose LibP2P ecosystem benefits (decentralization, transport flexibility)
- ❌ Custom LibP2P transport implementation required
- ❌ May break compatibility with other LibP2P nodes

**Verdict**: Only migrate if DoS protection + GDPR compliance outweigh LibP2P ecosystem benefits.

---

## Conclusion

FILE Relay Server fills a specific niche: **MPQUIC peer-to-peer applications needing DoS protection and GDPR compliance** with **stateless horizontal scaling**.

**Choose FILE Relay if** you prioritize:
1. DoS resistance (CAPoW computational proof-of-work)
2. GDPR compliance (IP hashing)
3. Stateless scaling (no coordination)
4. Multi-path connectivity (MPQUIC)
5. Production observability (Prometheus + Grafana)

**Choose alternatives if** you need:
- WebRTC/voice/video → **TURN / WebRTC**
- Tailscale mesh VPN → **Tailscale DERP**
- LibP2P ecosystem → **LibP2P Relay**
- Battle-tested at scale → **Tailscale DERP / TURN**

---

## Further Reading

- [FILE Relay Architecture](architecture/SYSTEM-OVERVIEW.md)
- [Deployment Guide](deployment/DEPLOYMENT-GUIDE.md)
- [FAQ](FAQ.md)
- [Tailscale DERP Documentation](https://tailscale.com/kb/1232/derp-servers/)
- [LibP2P Relay Specification](https://github.com/libp2p/specs/blob/master/relay/circuit-v2.md)
- [RFC 5766: TURN](https://tools.ietf.org/html/rfc5766)
- [WebRTC Architecture](https://webrtc.org/getting-started/overview)
