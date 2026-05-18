# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1   | :x:                |

**Current stable version**: 0.1.0

---

## Reporting a Vulnerability

**We take security seriously.** If you discover a security vulnerability, please follow responsible disclosure practices.

### Do Not

- ❌ Open a public GitHub issue
- ❌ Post in GitHub Discussions
- ❌ Discuss on Slack/Discord
- ❌ Announce on social media

### Do

✅ **Email**: security@file-network.example

**Include in your report:**
1. Description of the vulnerability
2. Steps to reproduce
3. Affected versions
4. Potential impact
5. Suggested fix (if available)
6. Your contact information (for follow-up)

### What to Expect

**Within 24 hours:**
- We will acknowledge receipt of your report
- We will provide an initial assessment

**Within 7 days:**
- We will provide a detailed response
- We will confirm or deny the vulnerability
- We will provide an estimated timeline for a fix

**Resolution:**
- We will work with you to understand the issue
- We will develop and test a fix
- We will coordinate disclosure timing with you
- We will publicly credit you (unless you prefer anonymity)

---

## Security Features

FILE Relay Server implements multiple layers of security:

### DoS Protection

**CAPoW (Computational Amortized Proof of Work)**:
- MTP-Argon2id proof verification
- 100ms timeout per proof
- Worker pool (25% of CPU cores, min 2)
- Prevents resource exhaustion attacks

**Per-IP Rate Limiting**:
- Token bucket algorithm
- 10 requests/second sustained rate
- Burst capacity: 50 requests
- DashMap for lock-free concurrent access

**Replay Prevention**:
- 5-minute SHA-256 proof cache
- Automatic cleanup every 60 seconds
- Prevents replay attacks

### Privacy & Compliance

**GDPR-Compliant IP Hashing**:
- HMAC-SHA256 with per-boot random secret
- 64-bit entropy (first 16 hex characters)
- All IPs hashed before logging
- No plaintext IPs in logs or metrics

**No PII Collection**:
- Server never stores plaintext IP addresses
- Peer IDs are public keys (not personal identifiers)
- No user tracking or analytics

### Network Security

**HashDoS Resistance**:
- DashMap with SipHash-2-4
- Resistant to hash collision attacks

**Input Validation**:
- peer_id: Exactly 32 bytes (Ed25519 public key)
- CAPoW proof: Validated structure and difficulty
- Rate limit checks before processing

**Noise XX Authentication** (pending event loop):
- Mutual authentication before connection
- Forward secrecy
- Post-quantum readiness (planned)

### Supply Chain Security

**Dependency Pinning**:
- All cryptographic dependencies pinned to exact versions
- `ant-quic = "=0.14.192"`
- `snow = "=0.10.0"`
- `sha3 = "=0.10"`
- `chacha20poly1305 = "=0.10"`
- `argon2 = "=0.5.3"`

**Regular Audits**:
- Daily `cargo audit` via CI/CD
- Automated issue creation on vulnerabilities
- RUSTSEC advisory monitoring

### Operational Security

**Graceful Shutdown**:
- SIGTERM/SIGINT signal handling
- 5-second grace period for in-flight packets
- No abrupt connection drops

**Health Checks**:
- Liveness probe: `/health`
- Readiness probe: `/ready` (checks capacity)
- Metrics endpoint: `/metrics` (Prometheus)

**Monitoring**:
- 12 production alerts (4 critical, 8 warning)
- Attack detection (high CAPoW rejection, rate limiting)
- Capacity monitoring (peer count, memory usage)

---

## Security Best Practices

### Deployment

**Network**:
- Only expose UDP 8080 (relay) and TCP 8081 (metrics)
- Use firewall rules to restrict metrics endpoint
- Deploy behind DDoS protection (Cloudflare, AWS Shield)

**Kubernetes**:
- Use provided security context (non-root, read-only filesystem)
- Enable Pod Security Admission (restricted)
- Use NetworkPolicy to isolate relay pods
- Rotate secrets regularly

**Docker**:
- Run as non-root user (UID 1000)
- Use read-only filesystem
- Set `--security-opt=no-new-privileges`
- Scan images with Trivy

### Configuration

**Rate Limiting**:
- Adjust based on traffic patterns
- Default: 10 req/s sustained, burst 50
- Monitor `file_relay_rate_limited_total` metric

**CAPoW Timeout**:
- Default: 100ms (recommended)
- Lower = more DoS protection, higher rejection rate
- Higher = less DoS protection, lower rejection rate

**Max Peers**:
- Default: 1,000 peers per instance
- Scale horizontally rather than increasing per-instance limit
- Monitor `file_relay_active_peers` metric

### Monitoring

**Critical Alerts** (immediate action):
- `FileRelayServerDown` — Server not responding
- `FileRelayCapacityReached` — At max peer capacity
- `FileRelayCAPoWAttack` — >50 rejections/s (likely attack)
- `FileRelayRateLimitAttack` — >100 limits/s (likely attack)

**Warning Alerts** (investigate soon):
- `FileRelayHighPeerCount` — >900 peers (capacity planning)
- `FileRelayHighCAPoWRejectionRate` — >10 rejections/s
- `FileRelayHighRateLimiting` — >20 limits/s
- `FileRelayHighPeerTimeouts` — >5 timeouts/s (network quality)

---

## Known Security Considerations

### Event Loop Not Implemented

**Status**: Connection event loop pending implementation

**Impact**:
- Noise XX authentication not yet active
- Packets not actually forwarded (infrastructure complete, wiring pending)

**Mitigation**:
- Design complete: `docs/security/SEC-C06-Noise-Authentication-Design.md`
- All infrastructure tested and ready
- Not a vulnerability, but missing feature

**Timeline**: Will be implemented after event loop completion

### GDPR IP Hashing

**Trade-off**: Per-boot secret means historical logs cannot correlate IPs across restarts

**Why**:
- GDPR compliance: No persistent plaintext IPs
- Privacy-by-design: Minimize data retention
- Security: Historical logs cannot be de-anonymized

**If you need**:
- Cross-restart correlation → Use external logs (firewall, load balancer)
- Long-term IP tracking → Not supported by design (GDPR)

### Rate Limiting Per-IP

**Trade-off**: Distributed attacks from many IPs not fully mitigated

**Why**:
- Per-IP rate limiting stops single-source attacks
- Distributed attacks require network-level mitigation

**Mitigation**:
- Deploy behind DDoS protection (Cloudflare, AWS Shield)
- Use global rate limiting at load balancer level
- Monitor aggregate metrics for attack patterns

---

## Security Audits

### Supply Chain Audit (2026-05-18)

**Status**: ✅ Clean

**Findings**:
- 1 vulnerability fixed: `maxminddb` 0.24 → 0.27.0 (RUSTSEC-2025-0132)
- 1 vulnerability mitigated: `sharks` 0.5.0 with ChaCha8Rng (RUSTSEC-2024-0398)
- 5 unmaintained warnings: Accepted with documented justification

**Report**: [`docs/security/Supply-Chain-Audit-2026-05-18.md`](docs/security/Supply-Chain-Audit-2026-05-18.md)

### Security Architecture Review (2026-05-18)

**Status**: ✅ 9/10 findings implemented

**Implemented**:
- SEC-C01: CAPoW verification ✅
- SEC-C02: MPQUIC forwarding ✅
- SEC-C03: Per-IP rate limiting ✅
- SEC-C04: GDPR IP hashing ✅
- SEC-C05: peer_id validation ✅
- SEC-H01: Replay prevention ✅
- SEC-H02: Zombie cleanup ✅
- SEC-H03: HashDoS resistance ✅
- SEC-H04: Structured logging ✅

**Pending**:
- SEC-C06: Noise XX authentication (blocked by event loop, design complete)

**Report**: [`docs/security/Implementation-Summary-2026-05-18.md`](docs/security/Implementation-Summary-2026-05-18.md)

---

## Vulnerability Disclosure Timeline

We follow a coordinated disclosure process:

1. **Day 0**: Receive vulnerability report
2. **Day 1**: Acknowledge receipt, initial assessment
3. **Day 7**: Detailed response, timeline
4. **Day 30**: Develop and test fix (if confirmed)
5. **Day 45**: Release patched version
6. **Day 60**: Public disclosure (coordinated with reporter)

**Exceptions**:
- Critical vulnerabilities: Accelerated timeline (7-14 days)
- Already publicly known: Immediate disclosure

---

## Security Contacts

**Primary**: security@file-network.example

**PGP Key**: [Coming soon]

**Response Time**:
- Critical: Within 4 hours
- High: Within 24 hours
- Medium: Within 7 days

---

## Hall of Fame

We recognize security researchers who responsibly disclose vulnerabilities:

*None yet — be the first!*

**Rewards**:
- Public recognition (if desired)
- Mention in CHANGELOG.md
- Contributor role in GitHub
- Swag (for significant findings)

---

## Security Resources

**Documentation**:
- [Deployment Guide](docs/deployment/DEPLOYMENT-GUIDE.md) — Secure deployment practices
- [Operations Runbook](docs/deployment/OPERATIONS-RUNBOOK.md) — Incident response

**Tools**:
- `cargo audit` — Vulnerability scanning
- `cargo clippy` — Security lints
- Trivy — Container scanning
- CodeQL — Static analysis

**External**:
- [RUSTSEC](https://rustsec.org/) — Rust security advisories
- [OWASP](https://owasp.org/) — Web security best practices

---

**Last Updated**: 2026-05-18  
**Security Policy Version**: 1.0
