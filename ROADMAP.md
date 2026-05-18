# FILE Relay Server - Roadmap

**Vision**: Production-ready, zero-trust P2P relay with enterprise compliance and global scale.

---

## Current Status (May 2026)

### ✅ Completed

**Core Infrastructure**:
- Stateless UDP relay architecture
- CAPoW (MTP-Argon2id) DoS protection
- Per-IP rate limiting (token bucket)
- IP address hashing (GDPR compliance)
- Prometheus metrics + Grafana dashboards
- 75 tests, 85% coverage

**Deployment & Operations**:
- Docker support
- Kubernetes manifests (HPA, PDB, NetworkPolicy)
- Terraform for AWS (EKS, multi-AZ)
- Terraform for GCP (GKE Autopilot)
- 12 production alerts
- Complete operations runbook
- Disaster recovery procedures

**Compliance & Documentation**:
- GDPR compliance mapping
- SOC 2 technical controls
- ISO 27001 Annex A controls
- 260+ KB documentation (22+ guides)
- Load testing framework (7 scenarios)
- Security hardening checklist (100+ items)

### 🔄 In Progress

**SEC-C06**: Noise XX authentication (design complete, blocked by event loop)

### ⏳ Blocked

**Event Loop Implementation**: The relay cannot forward packets until the connection event loop is implemented. This is THE blocker for production use.

---

## Phase 1: Production Core (Q2 2026)

**Goal**: Relay can forward packets in production with strong security.

### 1.1 Event Loop Implementation ⚠️ BLOCKER

**Status**: Not started  
**Priority**: CRITICAL  
**Dependencies**: None  
**Effort**: 2-3 weeks

**Deliverables**:
- [ ] Tokio-based async event loop
- [ ] UDP socket receive loop
- [ ] Packet forwarding logic
- [ ] Integration tests with real traffic
- [ ] Performance benchmarks (baseline)

**Why blocked**: All infrastructure ready, but relay can't forward packets yet.

### 1.2 Noise XX Authentication

**Status**: Design complete  
**Priority**: HIGH  
**Dependencies**: Event loop  
**Effort**: 2 weeks

**Deliverables**:
- [ ] Noise XX handshake implementation
- [ ] Mutual authentication (relay ↔ peer)
- [ ] Forward secrecy
- [ ] MITM attack prevention
- [ ] Documentation updates

**Why needed**: Current CAPoW only prevents DoS, not MITM.

### 1.3 Beta Deployment

**Status**: Not started  
**Priority**: HIGH  
**Dependencies**: Event loop, Noise XX  
**Effort**: 1 week

**Deliverables**:
- [ ] Deploy to Hetzner Germany (1 VPS)
- [ ] Deploy to AWS US-East (3 instances)
- [ ] Monitor beta metrics (7 days)
- [ ] Validate SLA (99.9% uptime)
- [ ] Document lessons learned

**Success criteria**: 1000+ peers, 10k pkt/s, < 20ms p95 latency, zero security incidents.

---

## Phase 2: Global Scale (Q3 2026)

**Goal**: Multi-region deployment with GeoDNS for global low-latency.

### 2.1 Multi-Region Deployment

**Priority**: MEDIUM  
**Effort**: 2 weeks

**Deliverables**:
- [ ] Deploy to 3 regions (US-East, EU-West, AP-Southeast)
- [ ] GeoDNS routing (Route 53 or Cloud DNS)
- [ ] Cross-region monitoring (Grafana)
- [ ] Regional failover testing
- [ ] Cost analysis (3-region vs 1-region)

**Benefits**:
- Lower latency for global users (< 50ms p95)
- Survive regional outages (99.95% availability)
- GDPR data residency compliance

### 2.2 Advanced Metrics

**Priority**: MEDIUM  
**Effort**: 1 week

**Deliverables**:
- [ ] Per-peer packet rate histogram
- [ ] Latency percentiles (p50, p95, p99)
- [ ] Geographic distribution of peers
- [ ] CAPoW difficulty auto-tuning
- [ ] Cost per packet/peer

**Benefits**: Better capacity planning, cost optimization, SLA validation.

---

## Phase 3: Enterprise Features (Q4 2026)

**Goal**: Enterprise-ready with compliance certifications.

### 3.1 SOC 2 Type II Certification

**Priority**: MEDIUM  
**Effort**: 3 months (including audit)

**Deliverables**:
- [ ] Complete Information Security Policy
- [ ] Complete Risk Register
- [ ] Vendor Management Policy
- [ ] Business Continuity Plan
- [ ] Third-party SOC 2 audit
- [ ] Certification report

**Why needed**: Enterprise customers require SOC 2 for vendor approval.

### 3.2 ISO 27001 Certification

**Priority**: LOW  
**Effort**: 6 months (including audit)

**Deliverables**:
- [ ] ISMS Manual
- [ ] Risk Assessment (complete)
- [ ] Statement of Applicability
- [ ] Internal audit schedule
- [ ] Third-party ISO 27001 audit
- [ ] Certification

**Why needed**: Global enterprise compliance (especially EU).

### 3.3 Managed Service Offering

**Priority**: LOW  
**Effort**: 2 months

**Deliverables**:
- [ ] Multi-tenant isolation (namespace per customer)
- [ ] Per-customer metrics and billing
- [ ] Self-service portal (web UI)
- [ ] API for provisioning
- [ ] Customer support runbook

**Business case**: Recurring revenue vs one-time OSS deployment.

---

## Phase 4: Advanced Features (2027+)

**Goal**: Next-generation relay features.

### 4.1 Post-Quantum Cryptography

**Priority**: LOW  
**Effort**: 1 month

**Deliverables**:
- [ ] Kyber or NIST PQC KEM integration
- [ ] Hybrid PQC + classical crypto (fallback)
- [ ] Performance benchmarks (PQC overhead)

**Why needed**: Future-proof against quantum computers.

### 4.2 WebSocket Support

**Priority**: LOW  
**Effort**: 2 weeks

**Deliverables**:
- [ ] WebSocket endpoint (wss://relay:8443)
- [ ] Browser client compatibility
- [ ] Performance comparison (WebSocket vs UDP)

**Trade-offs**:
- ✅ Browser compatibility
- ❌ Protocol complexity
- ❌ Performance overhead (TCP vs UDP)

**Decision**: Evaluate demand before implementation.

### 4.3 MPQUIC Connection Multiplexing

**Priority**: LOW  
**Effort**: 1 month

**Deliverables**:
- [ ] Relay multiplexes multiple MPQUIC connections per peer
- [ ] Per-connection metrics
- [ ] Load balancing across paths

**Benefits**: Better bandwidth utilization, failover.

---

## Community Requests

Vote on features at: [GitHub Discussions](https://github.com/file-network/relay-server/discussions)

### Top Requests

1. **IPv6 support** (8 votes)
2. **HTTP/3 compatibility** (5 votes)
3. **Relay federation** (multiple relays coordinate) (4 votes)
4. **WebRTC fallback** (TURN compatibility) (3 votes)

---

## Non-Goals

What we will **NOT** build:

❌ **Packet inspection** — Relay is opaque by design  
❌ **Data retention** — Stateless, GDPR-compliant  
❌ **User authentication** — Relay doesn't know users (only peer IDs)  
❌ **Billing/metering** — Use cloud provider billing or build on top  
❌ **Web dashboard** — Use Grafana or build separately

---

## How to Influence the Roadmap

1. **Vote on features**: [GitHub Discussions](https://github.com/file-network/relay-server/discussions)
2. **Report bugs**: [GitHub Issues](https://github.com/file-network/relay-server/issues)
3. **Contribute code**: See [CONTRIBUTING.md](CONTRIBUTING.md)
4. **Sponsor development**: Contact dev@file-network.example

---

## Release Schedule

| Version | Target Date | Focus |
|---------|------------|-------|
| **v0.1.0** | Q2 2026 | Event loop + Noise XX + Beta |
| **v0.2.0** | Q3 2026 | Multi-region + Advanced metrics |
| **v1.0.0** | Q4 2026 | SOC 2 certification + Production-ready |
| **v1.1.0** | Q1 2027 | Post-quantum crypto |
| **v2.0.0** | Q2 2027+ | WebSocket + MPQUIC multiplexing |

---

## Get Involved

- **Documentation**: [docs/](docs/)
- **Quick Start**: [docs/QUICK-START.md](docs/QUICK-START.md)
- **GitHub**: https://github.com/file-network/relay-server
- **Discussions**: https://github.com/file-network/relay-server/discussions
- **Email**: dev@file-network.example

---

**Last updated**: 2026-05-18
