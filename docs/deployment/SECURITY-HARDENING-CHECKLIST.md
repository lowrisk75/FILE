# FILE Relay Server - Security Hardening Checklist

**Last Updated**: 2026-05-18  
**Version**: 0.1.0

---

## Overview

This checklist ensures FILE Relay Server is deployed securely. Review before production deployment and after security incidents.

**Time required**: 30-60 minutes for initial review

---

## Pre-Deployment Checklist

### Infrastructure Security

- [ ] **Network segmentation**
  - [ ] Relay server on dedicated subnet/VLAN
  - [ ] Firewall rules: allow only UDP 8080 (public), TCP 8081 (monitoring only)
  - [ ] DDoS protection enabled (Cloudflare Spectrum, AWS Shield, etc.)
  - [ ] Rate limiting at network edge (if available)

- [ ] **TLS/encryption**
  - [ ] QUIC provides transport encryption (built-in)
  - [ ] Metrics endpoint (TCP 8081) restricted to internal network
  - [ ] No plaintext credentials in environment variables

- [ ] **Access control**
  - [ ] Metrics endpoint NOT exposed to public internet
  - [ ] SSH access restricted to bastion/jump host
  - [ ] No root login on relay server
  - [ ] Service runs as non-root user (UID 1000 in Docker, `file-relay` user in systemd)

- [ ] **Patch management**
  - [ ] OS up-to-date (security patches applied)
  - [ ] Rust toolchain up-to-date (1.83+)
  - [ ] Dependencies audited (`cargo audit` passing)

---

### Application Security

- [ ] **Binary hardening**
  - [ ] Built in release mode (`cargo build --release`)
  - [ ] PIE (Position Independent Executable) enabled (default in Rust)
  - [ ] Stack canaries enabled (default in Rust)
  - [ ] NX bit set (non-executable stack, default in Rust)

- [ ] **Supply chain security**
  - [ ] All dependencies pinned to exact versions (Cargo.lock committed)
  - [ ] `cargo audit` run and passing
  - [ ] No HIGH or CRITICAL vulnerabilities
  - [ ] Unmaintained dependencies documented and justified

- [ ] **Configuration security**
  - [ ] `MAX_PEERS` set appropriately (default 1000)
  - [ ] `RUST_LOG` set to `info` or `warn` in production (not `debug` or `trace`)
  - [ ] No secrets in environment variables (use secrets management)

- [ ] **CAPoW tuning**
  - [ ] Worker pool size appropriate (default: 25% cores, min 2)
  - [ ] Verification timeout enforced (100ms)
  - [ ] Replay cache TTL set (5 minutes)

- [ ] **Rate limiting tuning**
  - [ ] Per-IP rate limit appropriate (default: 10 req/s, burst 50)
  - [ ] Tested under load (legitimate clients not blocked)

---

### Docker Security

- [ ] **Image security**
  - [ ] Base image up-to-date (`debian:bookworm-slim` or newer)
  - [ ] Image scanned with Trivy (no HIGH/CRITICAL)
  - [ ] Image built from source (not third-party)
  - [ ] Multi-stage build used (minimal final image)

- [ ] **Runtime security**
  - [ ] Container runs as non-root (UID 1000)
  - [ ] Read-only root filesystem (`read_only: true`)
  - [ ] No new privileges (`security_opt: no-new-privileges:true`)
  - [ ] Resource limits set (CPU, memory)
  - [ ] Capabilities dropped (if using Docker >= 20.10)

- [ ] **Docker Compose**
  - [ ] Secrets via Docker secrets (not environment variables)
  - [ ] Networks isolated (no `network_mode: host`)
  - [ ] Volumes minimal (only `/tmp` if needed)

---

### Kubernetes Security

- [ ] **Pod security**
  - [ ] Pod runs as non-root (`runAsNonRoot: true`, `runAsUser: 1000`)
  - [ ] Read-only root filesystem (`readOnlyRootFilesystem: true`)
  - [ ] No privilege escalation (`allowPrivilegeEscalation: false`)
  - [ ] Capabilities dropped (`drop: [ALL]`)
  - [ ] seccomp profile applied (`type: RuntimeDefault`)

- [ ] **Network policies**
  - [ ] NetworkPolicy created (allow UDP 8080 ingress, TCP 8081 from Prometheus only)
  - [ ] Egress restricted (if applicable)

- [ ] **RBAC**
  - [ ] ServiceAccount created (not `default`)
  - [ ] Minimal permissions (relay doesn't need K8s API access)
  - [ ] No `cluster-admin` or `admin` roles

- [ ] **Secrets management**
  - [ ] No secrets in ConfigMaps or environment variables
  - [ ] Use Kubernetes Secrets or external secrets manager (Vault, AWS Secrets Manager)

- [ ] **Pod disruption budget**
  - [ ] PDB configured (minAvailable: 2)
  - [ ] Ensures availability during node drains

---

### Monitoring & Alerting

- [ ] **Metrics collection**
  - [ ] Prometheus scraping relay metrics (30s interval)
  - [ ] Metrics endpoint NOT public (internal only)
  - [ ] TLS on metrics endpoint (if exposed outside cluster)

- [ ] **Alerting**
  - [ ] Critical alerts configured (server down, capacity, attacks)
  - [ ] Alert routing configured (PagerDuty, Slack, Email)
  - [ ] Runbook URLs in alert annotations
  - [ ] Alert testing performed (simulate conditions)

- [ ] **Logging**
  - [ ] Structured logging enabled (`tracing`)
  - [ ] Log level appropriate (`info` or `warn` in production)
  - [ ] Logs aggregated (ELK, Loki, CloudWatch)
  - [ ] Log retention policy defined (7-30 days)

- [ ] **Audit logging**
  - [ ] IP hashing enabled (GDPR compliance)
  - [ ] Audit trail for administrative actions
  - [ ] Logs immutable (write-only, no deletion)

---

### Incident Response

- [ ] **Runbook prepared**
  - [ ] Operations runbook reviewed ([OPERATIONS-RUNBOOK.md](OPERATIONS-RUNBOOK.md))
  - [ ] Incident response team identified
  - [ ] Escalation path defined

- [ ] **Backups**
  - [ ] Metrics backed up (Prometheus snapshots)
  - [ ] Logs backed up (off-site storage)
  - [ ] Configuration backed up (Git)

- [ ] **Rollback plan**
  - [ ] Previous version available (Docker image, binary)
  - [ ] Rollback tested (can revert in <5 minutes)
  - [ ] Database migrations reversible (N/A for stateless relay)

---

## Security Testing Checklist

### Vulnerability Scanning

- [ ] **Static analysis**
  - [ ] `cargo clippy` passing
  - [ ] `cargo audit` passing
  - [ ] No `unsafe` code (or justified)

- [ ] **Container scanning**
  - [ ] Trivy scan passing (no HIGH/CRITICAL)
  - [ ] Base image CVEs reviewed
  - [ ] Outdated dependencies identified

- [ ] **Network scanning**
  - [ ] `nmap` scan shows only UDP 8080, TCP 8081
  - [ ] No unnecessary services running

### Penetration Testing

- [ ] **DoS resilience**
  - [ ] CAPoW flood test (1000 invalid proofs/s) → relay stays responsive
  - [ ] Connection flood test (1000 registration attempts/s) → rate limiting works
  - [ ] Packet flood test (10,000 packets/s) → relay does not crash

- [ ] **Authentication bypass**
  - [ ] Invalid CAPoW proofs rejected
  - [ ] Replay attacks blocked (5-minute cache)
  - [ ] Rate limiting cannot be bypassed (per-IP enforced)

- [ ] **Input validation**
  - [ ] Malformed peer_id rejected (must be 32 bytes)
  - [ ] Oversized packets rejected (if size limit exists)
  - [ ] Invalid QUIC frames rejected (handled by quinn)

---

## Post-Deployment Validation

### First 24 Hours

- [ ] **Health checks**
  - [ ] All replicas healthy (`/health` returning 200)
  - [ ] Capacity available (`/ready` returning 200)
  - [ ] Metrics exposed (`/metrics` returning data)

- [ ] **Functional testing**
  - [ ] Peers can register successfully
  - [ ] CAPoW verification working (high success rate)
  - [ ] Rate limiting working (bad actors blocked)
  - [ ] Packet forwarding working (once event loop implemented)

- [ ] **Security monitoring**
  - [ ] No unexpected alerts fired
  - [ ] CAPoW rejection rate < 10%
  - [ ] Rate limiting rate < 5%
  - [ ] No crashes or restarts

### First Week

- [ ] **Capacity planning**
  - [ ] Peer count trending as expected
  - [ ] Resource usage within limits
  - [ ] No capacity warnings

- [ ] **Performance validation**
  - [ ] Latency within SLA (< 10ms p95)
  - [ ] Packet loss < 0.1%
  - [ ] No CPU throttling

- [ ] **Incident review**
  - [ ] Any incidents documented
  - [ ] Runbook updated
  - [ ] Alerts tuned (reduce false positives)

---

## Continuous Security

### Weekly

- [ ] Review security alerts (Prometheus Alertmanager)
- [ ] Check for abnormal metrics (CAPoW rejection, rate limiting)
- [ ] Review audit logs for suspicious activity

### Monthly

- [ ] Run `cargo audit` (check for new vulnerabilities)
- [ ] Review and update dependencies
- [ ] Scan Docker images with Trivy
- [ ] Review firewall rules (ensure least privilege)

### Quarterly

- [ ] Full security review (this checklist)
- [ ] Penetration testing (internal or external)
- [ ] Disaster recovery drill (simulate relay failure)
- [ ] Update runbook and documentation

### Annually

- [ ] Third-party security audit (recommended for critical deployments)
- [ ] Review and update security policy ([SECURITY.md](../SECURITY.md))
- [ ] Team security training

---

## Compliance Checklist

### GDPR

- [ ] **Data minimization**
  - [ ] Only necessary data collected (peer_id, IP hash, timestamps)
  - [ ] No PII in logs (IP addresses hashed)

- [ ] **Right to erasure**
  - [ ] Peer data deleted after timeout (5 minutes)
  - [ ] No long-term peer data storage

- [ ] **Data portability**
  - [ ] Metrics available via Prometheus API
  - [ ] Logs available via query interface

- [ ] **Security measures**
  - [ ] IP hashing with HMAC-SHA256
  - [ ] Per-boot secret (historical logs cannot be correlated)
  - [ ] Encryption in transit (QUIC)

### SOC 2

- [ ] **Access control**
  - [ ] RBAC enforced (Kubernetes)
  - [ ] Audit logs enabled
  - [ ] MFA for administrative access

- [ ] **Change management**
  - [ ] Code review required (GitHub PRs)
  - [ ] CI/CD pipeline (automated testing)
  - [ ] Canary deployments (gradual rollout)

- [ ] **Incident response**
  - [ ] Runbook documented
  - [ ] Incident response team identified
  - [ ] Post-mortem process defined

---

## Security Review Sign-Off

**Reviewer**: ___________________________  
**Date**: ___________________________  
**Version**: 0.1.0  
**Findings**: ___________________________  
**Risk level**: ☐ Low  ☐ Medium  ☐ High  
**Approved for production**: ☐ Yes  ☐ No (reasons: ________________)  

---

## Further Reading

- [Security Policy](../../SECURITY.md) — Vulnerability disclosure, features
- [Supply Chain Audit](../security/Supply-Chain-Audit-2026-05-18.md) — Dependency audit
- [Operations Runbook](OPERATIONS-RUNBOOK.md) — Incident response
- [OWASP Top 10](https://owasp.org/www-project-top-ten/) — Common vulnerabilities

---

**Questions?** [Open an issue](https://github.com/file-network/relay-server/issues) or email security@file-network.example.
