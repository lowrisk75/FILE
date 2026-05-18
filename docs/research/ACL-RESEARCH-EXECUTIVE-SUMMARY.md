# ACL Research Executive Summary
## For FILE Relay Server Access Control Design

**Date**: 2026-05-18  
**Full Report**: [ACL-PATTERNS-RESEARCH-2026-05-18.md](ACL-PATTERNS-RESEARCH-2026-05-18.md)

---

## TL;DR

FILE's 5-level ACL design in ACCESS-CONTROL.md is **architecturally sound** and aligns with industry best practices (Tailscale, Signal, LibP2P, TURN). The "dumb pipe" relay philosophy is **correct**—relays should authenticate peers but not enforce application-level policies.

**Immediate Action**: Fix 6 critical security findings (SEC-C01 through SEC-C06) before deploying. Estimated effort: 10-15 days.

---

## Key Findings by Category

### 1. Authentication Patterns (Industry Consensus)

**Best Practice**: Multi-layer authentication, not a single gate.

| Layer | Mechanism | Example |
|-------|-----------|---------|
| **Transport** | Mutual TLS/Noise | FILE: Noise XX handshake |
| **Registration** | Proof-of-work OR API keys | FILE: CAPoW (public) or HMAC tokens (managed) |
| **Application** | Client-side filtering | FILE: Clients decide which peers to accept |

**Recommendation**: FILE should implement Noise XX (transport) + CAPoW (registration). API keys optional for managed deployments.

### 2. Zero-Trust Design Validation

FILE's zero-trust relay matches Tailscale and Signal architectures:

✅ **Cryptographic peer authentication** (peer_id = Ed25519 public key)  
✅ **End-to-end encryption** (Noise + AONT → relay cannot decrypt)  
✅ **Minimal trust surface** (relay sees metadata, not data)  
✅ **Stateless authentication** (no session database)

**Verdict**: FILE's architecture is **superior** to centralized alternatives for sovereignty-focused users.

### 3. Rate Limiting is Critical

**Every production relay system implements rate limiting.** FILE's current design lacks this (SEC-C03).

**Industry Standard**:
- **Per-IP**: 10 req/s sustained, 50 burst
- **Per-user**: 10 peers max, 1 GB/day bandwidth
- **Per-resource**: 100 CAPoW verifications/sec globally

**Recommendation**: Implement per-IP rate limiting **immediately** (DashMap-based token bucket). Add per-user quotas if implementing API keys.

### 4. GDPR Compliance Requirements

**Critical**: IP addresses are **personal data** under GDPR. Full IP logging violates GDPR (SEC-C04).

**Industry Solution**: Hash IP addresses with HMAC-SHA256 before logging.

```rust
fn hash_ip(ip: IpAddr, secret: &[u8]) -> [u8; 32] {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).unwrap();
    mac.update(&ip.to_string().as_bytes());
    mac.finalize().into_bytes().into()
}
```

**Retention Limits**:
- Operational logs: 7 days
- Security logs: 90 days
- Prometheus metrics: 30 days

**Recommendation**: Implement IP hashing **immediately**. Document legal basis (legitimate interest in abuse prevention) in privacy policy.

### 5. API Key Patterns (for Managed Relay)

**Best Practice**: Time-limited HMAC tokens (Coturn REST API pattern), not database-backed API keys.

**Mechanism**:
1. Client requests token from backend (HTTPS)
2. Backend generates: `token = timestamp || HMAC(peer_id || timestamp, secret)`
3. Client presents token to relay
4. Relay validates HMAC + expiry (no database)

**Advantages**:
- ✅ Stateless (horizontal scaling)
- ✅ Fast (<1ms validation)
- ✅ Automatic expiration (24 hours)

**Recommendation**: Use this for Level 3 (Registration ACLs) in managed deployments. Avoid OAuth (overkill) and database-backed keys (doesn't scale).

### 6. P2P ACLs: Client-Side, Not Relay-Side

**Industry Consensus** (LibP2P, Signal, Tailscale): Relays do **NOT** enforce peer-to-peer ACLs.

**Rationale**:
- Relay learns social graph (privacy leak)
- Adds latency (ACL lookup per packet)
- Adds complexity (ACL state management)
- Clients can filter more effectively (application context)

**Exception**: TURN servers implement "permissions" to prevent relay abuse. Relevant only if spam becomes a **proven problem**.

**Recommendation**: File's Level 4 (P2P ACLs) should be **client-side filtering** by default. Only implement relay-side ACLs if spam is demonstrated in production.

### 7. Multi-Tenancy Strategies

**For FILE**:

| Deployment | Tenants | Strategy | Cost | Isolation |
|------------|---------|----------|------|-----------|
| **Personal** | 1 | Single relay | €5/month | N/A |
| **Small Group** | 2-10 | Tenant ID in peer_id | €5/month | Medium |
| **Enterprise** | 10-100 | Separate relay instances | €5-50/month | **Strong** |
| **SaaS** | 100+ | VLAN/namespace isolation | €50+/month | **Strong** |

**Recommendation**: Start with single relay (personal). Use separate instances for B2B customers (strong isolation justifies cost).

### 8. SOC 2 Requirements

**Current Gaps**:
- ❌ No authentication on metrics endpoint (CC6.1 violation)
- ❌ No admin access controls documented (CC6.3 gap)
- ❌ No audit logging for admin actions (CC7.2 gap)

**Remediation**:
1. Secure metrics endpoint: Network isolation OR HTTP Basic Auth (Level 1 ACLs)
2. Document admin access: SSH key management, MFA requirements
3. Enable SSH audit logging: Centralized syslog
4. Create incident response playbook: DDoS, breach, outage procedures

**Timeline**: 3-5 days after critical findings fixed.

---

## Implementation Roadmap

### Phase 1: Critical Security (Week 1-2) — 🔴 **BLOCKING**

Fix 6 critical findings from threat-modeler audit:

1. **SEC-C06**: Noise XX authentication (2 days)
2. **SEC-C01**: Enable CAPoW verification (2 days)
3. **SEC-C05**: Peer ID validation (1 day)
4. **SEC-C03**: Per-IP rate limiting (1 day)
5. **SEC-C04**: IP address hashing (1 day)
6. **SEC-C02**: Packet forwarding (3 days)

**Deliverable**: Functional relay with minimal viable security (ready for beta testing).

### Phase 2: Observability (Week 3) — 🟡 **HIGH PRIORITY**

7. Admin ACLs (Level 1): Network isolation for port 8081 (1 day)
8. Structured logging: JSON logs with hashed IDs (2 days)
9. Metrics expansion: Auth failures, bandwidth per peer (1 day)

**Deliverable**: Observable, securable relay (SOC 2 foundation).

### Phase 3: Advanced ACLs (Week 4+) — 🟢 **OPTIONAL**

10. API Key Authentication (Level 3): Time-limited HMAC tokens (1 week)
11. P2P ACLs (Level 4): Only if spam is proven problem (1 week)
12. Multi-tenancy: Separate relay instances for B2B (1 week)

**Deliverable**: Enterprise-ready relay (B2B/SaaS deployments).

---

## Architectural Validation

FILE's 5-level ACL design matches industry best practices:

| Level | FILE's Approach | Industry Analog | Verdict |
|-------|----------------|----------------|---------|
| **Level 1** | Network isolation or HTTP Basic Auth | Prometheus, Grafana | ✅ Correct |
| **Level 2** | Open to 0.0.0.0/0 + CAPoW | TURN, Tailscale DERP | ✅ Correct |
| **Level 3** | CAPoW-only (public) or API keys (managed) | Coturn REST API | ✅ Correct |
| **Level 4** | Client-side filtering | LibP2P, Signal | ✅ Correct |
| **Level 5** | Client responsibility | Universal | ✅ Correct |

**No architectural changes needed.** Focus on implementation quality.

---

## Decision Matrix

| Question | Public Relay | Managed Relay | Enterprise B2B |
|----------|--------------|---------------|----------------|
| **Level 3 Auth** | CAPoW only | HMAC tokens | HMAC tokens + quotas |
| **Level 4 P2P ACLs** | No (client-side) | No (unless spam proven) | Optional (per-tenant) |
| **Multi-tenancy** | Single relay | Single relay (tenant ID) | Separate instances |
| **Rate limits** | Per-IP (10/s) | Per-IP + per-user | Per-tenant configs |
| **Audit logging** | Minimal (GDPR) | Standard (90 days) | Full (SOC 2) |
| **External audit** | Not needed | Optional | **Required** ($80k-150k) |

---

## Open Questions for Kevin

1. **Deployment Model**: Public relay (CAPoW-only) or managed relay (API keys)?

2. **Multi-Tenancy**: Single relay for all users, or separate instances per org?

3. **P2P ACLs**: Implement relay-enforced ACLs (Level 4)?  
   → Recommend: **No** (client-side filtering). Only implement if spam is proven in beta.

4. **Audit Timeline**: When to commission external security audit?  
   → Recommend: **After 4-6 week beta** (validate architecture viability first).

5. **Mesh Deployment**: Deploy 3 relays immediately, or start with 1?  
   → Recommend: **Start with 1** (simpler testing). Add 2 more after beta validation.

---

## Next Steps

1. **Review this research** with FILE project team
2. **Prioritize critical findings** (SEC-C01 through SEC-C06)
3. **Implement Phase 1** (10-15 days, 1 senior Rust engineer)
4. **Deploy beta relay** (1 VPS, 4-6 weeks validation)
5. **Scale to production** (3 VPS, mesh authentication, monitoring)

---

**Research Completed**: 2026-05-18  
**Full Report**: 9,800 words, 10+ hours research  
**Sources**: 30+ RFCs, implementations, security guides, compliance frameworks
