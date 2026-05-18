# FILE Relay Server - Risk Register

**Document Owner**: Security Team  
**Last Updated**: 2024-01-01  
**Review Frequency**: Quarterly  
**Version**: 1.0

---

## Overview

This Risk Register identifies, assesses, and tracks information security risks for FILE Relay Server.

**Risk management process**:
1. **Identify** risks through threat modeling, audits, incidents
2. **Assess** likelihood and impact
3. **Treat** with controls (mitigate, transfer, accept, avoid)
4. **Monitor** risk levels and control effectiveness
5. **Review** quarterly and after major incidents

---

## Risk Assessment Matrix

### Likelihood Scale

| Level | Likelihood | Frequency |
|-------|-----------|-----------|
| 1 | Rare | < 1% chance per year |
| 2 | Unlikely | 1-10% chance per year |
| 3 | Possible | 10-50% chance per year |
| 4 | Likely | 50-90% chance per year |
| 5 | Almost Certain | > 90% chance per year |

### Impact Scale

| Level | Impact | Description |
|-------|--------|-------------|
| 1 | Negligible | No user impact, < 1 min downtime, no data exposure |
| 2 | Minor | < 100 users affected, < 10 min downtime, limited data |
| 3 | Moderate | < 1000 users, < 1 hour downtime, some data exposure |
| 4 | Major | < 10,000 users, < 4 hours downtime, significant data |
| 5 | Severe | All users, > 4 hours downtime, major data breach |

### Risk Level Matrix

| Likelihood → <br> Impact ↓ | Rare (1) | Unlikely (2) | Possible (3) | Likely (4) | Almost Certain (5) |
|---|---|---|---|---|---|
| **Severe (5)** | Medium | High | High | Critical | Critical |
| **Major (4)** | Low | Medium | High | High | Critical |
| **Moderate (3)** | Low | Low | Medium | High | High |
| **Minor (2)** | Low | Low | Low | Medium | Medium |
| **Negligible (1)** | Low | Low | Low | Low | Medium |

**Risk levels**:
- **Critical**: Immediate action required, escalate to CTO
- **High**: Action within 7 days
- **Medium**: Action within 30 days
- **Low**: Monitor, review quarterly

---

## Current Risks

### RISK-001: DoS Attack (UDP Flood)

**Category**: Availability  
**Threat**: Attacker floods relay with UDP packets  
**Vulnerability**: UDP protocol susceptible to amplification attacks

**Inherent Risk**:
- Likelihood: 4 (Likely — common attack type)
- Impact: 4 (Major — service degradation for all users)
- **Risk Level**: HIGH

**Current Controls**:
1. ✅ CAPoW (Computational Amortized Proof of Work) — requires 100ms CPU work per registration
2. ✅ Per-IP rate limiting — 10 req/s sustained, 50 req/s burst
3. ✅ Cloud provider DDoS protection (AWS Shield / GCP Cloud Armor)
4. ✅ Auto-scaling (HPA) — scales up under load
5. ✅ Alerting — `FileRelayRateLimitAttack` alert

**Residual Risk**:
- Likelihood: 2 (Unlikely — controls effective)
- Impact: 3 (Moderate — degraded but not down)
- **Risk Level**: LOW

**Treatment Plan**:
- ✅ Current controls adequate
- Monitor: Track CAPoW rejection rate, rate limiting events

**Owner**: Security Team  
**Review Date**: 2024-04-01  
**Status**: ✅ Accepted

---

### RISK-002: IP Address Exposure

**Category**: Confidentiality (GDPR)  
**Threat**: IP addresses leaked or reverse-engineered  
**Vulnerability**: IP hashing requires secret protection

**Inherent Risk**:
- Likelihood: 3 (Possible — if secret compromised)
- Impact: 2 (Minor — only IP addresses, no sensitive data)
- **Risk Level**: MEDIUM

**Current Controls**:
1. ✅ HMAC-SHA256 hashing — not reversible without secret
2. ✅ Per-boot secret generation — secret rotated on every restart
3. ✅ Secret stored in memory only — no persistence
4. ✅ No logging of plaintext IPs — only hashed IPs in logs
5. ✅ 5-minute peer timeout — limited window of exposure

**Residual Risk**:
- Likelihood: 1 (Rare — multiple controls must fail)
- Impact: 2 (Minor — limited exposure, no sensitive data)
- **Risk Level**: LOW

**Treatment Plan**:
- Consider: Rotate secret every 24 hours (in addition to per-boot)
- Consider: Add Noise XX authentication (eliminates need for IP hashing)

**Owner**: Security Team  
**Review Date**: 2024-04-01  
**Status**: ⚠️ Under Review

---

### RISK-003: Dependency Vulnerability

**Category**: Integrity, Confidentiality  
**Threat**: Vulnerability in third-party dependency  
**Vulnerability**: Rust crates may contain security issues

**Inherent Risk**:
- Likelihood: 4 (Likely — dependencies updated frequently)
- Impact: 4 (Major — depends on vulnerability)
- **Risk Level**: HIGH

**Current Controls**:
1. ✅ Daily security audit (`cargo audit`) — CI/CD automated
2. ✅ Dependency pinning (`Cargo.lock`) — reproducible builds
3. ✅ Minimal dependencies — only necessary crates
4. ✅ Supply chain verification — checksums verified
5. ✅ Dependabot alerts — GitHub notifications

**Residual Risk**:
- Likelihood: 2 (Unlikely — early detection)
- Impact: 3 (Moderate — patched quickly)
- **Risk Level**: LOW

**Treatment Plan**:
- ✅ Continue daily audits
- ✅ SLA: Patch critical vulnerabilities within 24 hours

**Owner**: Development Team  
**Review Date**: 2024-04-01  
**Status**: ✅ Accepted

---

### RISK-004: Insider Threat

**Category**: Confidentiality, Integrity, Availability  
**Threat**: Malicious or negligent insider  
**Vulnerability**: Personnel with production access

**Inherent Risk**:
- Likelihood: 2 (Unlikely — background checks, culture)
- Impact: 5 (Severe — full system access)
- **Risk Level**: HIGH

**Current Controls**:
1. ✅ Background checks — all production access personnel
2. ✅ Least privilege — RBAC, minimal permissions
3. ✅ MFA required — production access
4. ✅ Audit logging — all access logged (CloudTrail / Cloud Audit)
5. ✅ Code review — all changes reviewed
6. ✅ Segregation of duties — developer ≠ deployer
7. ✅ Quarterly access review — remove unnecessary access

**Residual Risk**:
- Likelihood: 1 (Rare — multiple controls)
- Impact: 4 (Major — limited by controls)
- **Risk Level**: LOW

**Treatment Plan**:
- ✅ Current controls adequate
- Monitor: Review audit logs quarterly

**Owner**: Security Team  
**Review Date**: 2024-04-01  
**Status**: ✅ Accepted

---

### RISK-005: Cloud Provider Outage

**Category**: Availability  
**Threat**: AWS/GCP region failure  
**Vulnerability**: Reliance on cloud provider

**Inherent Risk**:
- Likelihood: 2 (Unlikely — rare but happens)
- Impact: 5 (Severe — complete outage if single region)
- **Risk Level**: HIGH

**Current Controls**:
1. ✅ Multi-AZ deployment — survives AZ failure
2. ⚠️ Multi-region deployment — planned, not yet implemented
3. ✅ Auto-scaling — handles load fluctuations
4. ✅ Disaster recovery procedures — documented

**Residual Risk** (single region):
- Likelihood: 2 (Unlikely — rare)
- Impact: 4 (Major — regional outage only)
- **Risk Level**: MEDIUM

**Treatment Plan**:
- 🔄 **Action**: Deploy to 2+ regions (US-EAST, EU-WEST)
- 🔄 **Due**: 2024-03-01
- ✅ Terraform multi-region IaC ready

**Owner**: Operations Team  
**Review Date**: 2024-03-01  
**Status**: 🔄 In Progress

---

### RISK-006: CAPoW Bypass

**Category**: Availability (DoS)  
**Threat**: Attacker bypasses CAPoW verification  
**Vulnerability**: Bug in verification logic

**Inherent Risk**:
- Likelihood: 2 (Unlikely — well-tested)
- Impact: 4 (Major — DoS protection disabled)
- **Risk Level**: MEDIUM

**Current Controls**:
1. ✅ Unit tests — CAPoW verification tested
2. ✅ Integration tests — end-to-end registration tested
3. ✅ Code review — all changes reviewed
4. ✅ Fuzzing — planned (cargo-fuzz)
5. ✅ Monitoring — CAPoW rejection rate tracked

**Residual Risk**:
- Likelihood: 1 (Rare — multiple layers of testing)
- Impact: 3 (Moderate — other controls remain)
- **Risk Level**: LOW

**Treatment Plan**:
- Consider: Add fuzz testing (`cargo fuzz add fuzz_capow`)
- Monitor: CAPoW rejection rate (alert if zero during attack)

**Owner**: Development Team  
**Review Date**: 2024-04-01  
**Status**: ✅ Accepted

---

### RISK-007: Memory Exhaustion Attack

**Category**: Availability  
**Threat**: Attacker registers many peers to exhaust memory  
**Vulnerability**: Peer state stored in memory

**Inherent Risk**:
- Likelihood: 3 (Possible — if capacity limits not enforced)
- Impact: 4 (Major — pod OOMKilled, restart)
- **Risk Level**: HIGH

**Current Controls**:
1. ✅ `MAX_PEERS` limit — 1000 peers per instance
2. ✅ Peer timeout — 5 minutes, automatic cleanup
3. ✅ Memory limits — Kubernetes resource limits
4. ✅ Monitoring — memory usage tracked
5. ✅ Auto-scaling — HPA adds instances

**Residual Risk**:
- Likelihood: 2 (Unlikely — limits enforced)
- Impact: 3 (Moderate — single pod affected, not cluster)
- **Risk Level**: LOW

**Treatment Plan**:
- ✅ Current controls adequate
- Monitor: Memory usage per pod (alert at 80%)

**Owner**: Operations Team  
**Review Date**: 2024-04-01  
**Status**: ✅ Accepted

---

### RISK-008: Packet Inspection / MITM

**Category**: Confidentiality  
**Threat**: Attacker inspects packet contents  
**Vulnerability**: Packets relayed in plaintext (no encryption)

**Inherent Risk**:
- Likelihood: 3 (Possible — network eavesdropping)
- Impact: 4 (Major — depends on packet content)
- **Risk Level**: HIGH

**Current Controls**:
1. ⚠️ None (relay does not encrypt packets)
2. ✅ Design: Relay is opaque (does not inspect content)
3. ✅ Documentation: Users warned to use end-to-end encryption

**Residual Risk** (without encryption):
- Likelihood: 3 (Possible — network taps exist)
- Impact: 4 (Major — confidential data exposed)
- **Risk Level**: HIGH

**Treatment Plan**:
- 🔄 **Action**: Implement Noise XX authentication + encryption
- 🔄 **Due**: 2024-06-01 (Phase 2)
- ⚠️ **Interim**: Document user responsibility for E2E encryption

**Owner**: Development Team  
**Review Date**: 2024-02-01  
**Status**: 🔄 In Progress

---

### RISK-009: Replay Attack

**Category**: Integrity  
**Threat**: Attacker replays captured CAPoW proofs  
**Vulnerability**: Proofs do not include freshness check

**Inherent Risk**:
- Likelihood: 3 (Possible — if proofs captured)
- Impact: 2 (Minor — limited impact)
- **Risk Level**: MEDIUM

**Current Controls**:
1. ✅ Timestamp in proof — proofs include generation time
2. ✅ Short validity window — proofs expire after ~5 minutes
3. ⚠️ No replay cache — same proof can be used twice

**Residual Risk**:
- Likelihood: 2 (Unlikely — limited window)
- Impact: 2 (Minor — attacker still rate-limited)
- **Risk Level**: LOW

**Treatment Plan**:
- Consider: Add replay cache (LRU cache, 5-minute TTL)
- Monitor: Duplicate peer_id registrations

**Owner**: Development Team  
**Review Date**: 2024-04-01  
**Status**: ✅ Accepted

---

### RISK-010: Configuration Error

**Category**: Availability, Security  
**Threat**: Misconfiguration causes outage or vulnerability  
**Vulnerability**: Complex deployment (Kubernetes, Terraform)

**Inherent Risk**:
- Likelihood: 3 (Possible — human error)
- Impact: 4 (Major — potential outage)
- **Risk Level**: HIGH

**Current Controls**:
1. ✅ Infrastructure as Code — Terraform, version-controlled
2. ✅ Configuration review — PR review process
3. ✅ Automated testing — Terraform validate, plan
4. ✅ Security hardening checklist — pre-deployment
5. ✅ Staging environment — test before production

**Residual Risk**:
- Likelihood: 2 (Unlikely — multiple checks)
- Impact: 3 (Moderate — staging catches most)
- **Risk Level**: LOW

**Treatment Plan**:
- ✅ Current controls adequate
- Consider: Add automated configuration scanning (tfsec, checkov)

**Owner**: Operations Team  
**Review Date**: 2024-04-01  
**Status**: ✅ Accepted

---

## Risk Summary

### By Risk Level

| Risk Level | Count | Risks |
|-----------|-------|-------|
| **Critical** | 0 | — |
| **High** | 1 | RISK-008 (Packet inspection) |
| **Medium** | 1 | RISK-005 (Cloud outage, single region) |
| **Low** | 8 | All others |

### By Status

| Status | Count | Risks |
|--------|-------|-------|
| ✅ **Accepted** | 8 | Most risks adequately controlled |
| 🔄 **In Progress** | 2 | RISK-005 (Multi-region), RISK-008 (Noise XX) |
| ⚠️ **Under Review** | 1 | RISK-002 (IP exposure) |

### Top Risks (Requiring Action)

1. **RISK-008**: Packet inspection — Implement Noise XX encryption (Due: 2024-06-01)
2. **RISK-005**: Cloud provider outage — Deploy multi-region (Due: 2024-03-01)
3. **RISK-002**: IP address exposure — Review secret rotation frequency

---

## Risk Treatment Plan Summary

| Risk ID | Action | Owner | Due Date | Status |
|---------|--------|-------|----------|--------|
| RISK-005 | Deploy to 2+ regions | Operations | 2024-03-01 | 🔄 In Progress |
| RISK-008 | Implement Noise XX | Development | 2024-06-01 | 🔄 In Progress |
| RISK-002 | Review secret rotation | Security | 2024-02-01 | ⚠️ Under Review |

---

## Risk Monitoring

### Key Risk Indicators (KRIs)

| KRI | Target | Threshold | Frequency |
|-----|--------|-----------|-----------|
| CAPoW rejection rate | < 1% | > 5% = investigate | Real-time |
| Rate limiting events | < 10/hour | > 100/hour = alert | Real-time |
| Dependency vulnerabilities | 0 critical | 1+ critical = patch | Daily |
| Failed authentication | < 5/day | > 20/day = investigate | Daily |
| Uptime | > 99.9% | < 99.5% = escalate | Monthly |

### Monitoring Dashboard

**Prometheus queries**:
```promql
# CAPoW rejection rate
rate(file_relay_capow_rejected_total[5m])

# Rate limiting events
rate(file_relay_rate_limited_total[5m])

# Availability
avg_over_time(up{job="file-relay"}[30d]) * 100
```

---

## Review and Approval

### Quarterly Review Checklist

- [ ] Review all existing risks (likelihood, impact, controls)
- [ ] Identify new risks (threat landscape changes, new features)
- [ ] Update treatment plans (progress, new actions)
- [ ] Review KRIs (threshold breaches, trends)
- [ ] Update risk register (this document)
- [ ] Report to management (summary, top risks)

### Annual Review Checklist

- [ ] Comprehensive threat modeling (entire system)
- [ ] Penetration testing (external firm)
- [ ] Control effectiveness review (are controls working?)
- [ ] Risk appetite review (acceptable risk levels)
- [ ] Risk register update (major revision)

---

## Document Control

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2024-01-01 | Security Team | Initial version |

**Next review**: 2024-04-01 (Quarterly)

---

## Contact

**Risk Owner**: Security Team  
**Email**: security@file-network.example  
**Phone**: +XX XXX XXX XXXX

**Escalation**: CTO (Critical risks only)

---

## Related Documents

- [Information Security Policy](INFORMATION-SECURITY-POLICY.md)
- [Compliance Documentation](COMPLIANCE.md)
- [Threat Model](architecture/THREAT-MODEL.md)
- [Security Hardening Checklist](deployment/SECURITY-HARDENING-CHECKLIST.md)

---

**Approved by**:

**Security Lead**: ______________________________  
**Date**: ______________________________

**CTO**: ______________________________  
**Date**: ______________________________
