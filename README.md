# FILE Relay Server

**Stateless UDP relay for peer-to-peer MPQUIC connections with built-in DoS protection.**

[![Build Status](https://img.shields.io/github/actions/workflow/status/file-network/relay-server/test.yml?branch=main)](https://github.com/file-network/relay-server/actions)
[![Test Coverage](https://img.shields.io/badge/coverage-85%25-brightgreen)](https://github.com/file-network/relay-server)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.75%2B-orange)](https://www.rust-lang.org/)

---

## Overview

FILE Relay Server enables peer-to-peer connections between clients behind NAT/firewalls by relaying MPQUIC packets. It's designed for **high performance**, **low latency**, and **strong DoS protection**.

**Key features**:
- ✅ **Stateless relay** — No persistent storage, zero data retention
- ✅ **DoS protection** — CAPoW (Computational Amortized Proof of Work) + rate limiting
- ✅ **GDPR compliant** — IP hashing with HMAC-SHA256, automatic 5-minute cleanup
- ✅ **High performance** — 10,000+ packets/second per vCPU
- ✅ **Production-ready** — 75 tests, 85% coverage, comprehensive operations guides
- ✅ **Cloud-native** — Docker, Kubernetes, Terraform for AWS/GCP

---

## Quick Start

### Run with Docker

```bash
docker run -d \
  -p 8080:8080/udp \
  -p 8081:8081 \
  -e MAX_PEERS=1000 \
  ghcr.io/file-network/relay-server:latest
```

### Deploy to Kubernetes

```bash
kubectl apply -f https://raw.githubusercontent.com/file-network/relay-server/main/deploy/kubernetes/relay-server-deployment.yaml
```

### Deploy to AWS with Terraform

```bash
cd infrastructure/terraform/aws
terraform init
terraform apply
```

**Full guide**: [QUICK-START.md](docs/QUICK-START.md)

---

## Architecture

```
┌──────────┐                  ┌──────────┐                  ┌──────────┐
│  Peer A  │                  │  Relay   │                  │  Peer B  │
│          │─────REGISTER────▶│  Server  │◀────REGISTER─────│          │
│ (behind  │◀────REGISTER_ACK─│          │─────REGISTER_ACK▶│ (behind  │
│   NAT)   │                  │ (Public  │                  │   NAT)   │
│          │                  │   IP)    │                  │          │
│          │─────RELAY────────▶│          │─────RELAY────────▶│          │
│          │     PACKET        │  (Relay) │     PACKET       │          │
└──────────┘                  └──────────┘                  └──────────┘
```

**How it works**:
1. Peers register with relay server using CAPoW proof
2. Relay stores peer IP/port mapping (in-memory, 5-minute timeout)
3. Peers send packets to relay, relay forwards to destination peer
4. Relay is opaque — never inspects packet contents

**Full architecture**: [docs/architecture/SYSTEM-OVERVIEW.md](docs/architecture/SYSTEM-OVERVIEW.md)

---

## Documentation

### Getting Started

- [Quick Start](docs/QUICK-START.md) — Deploy in 5 minutes
- [Architecture](docs/architecture/SYSTEM-OVERVIEW.md) — How it works
- [Comparison](docs/COMPARISON.md) — vs. TURN, DERP, LibP2P

### Operations

- [Deployment Guide](docs/deployment/DEPLOYMENT-GUIDE.md) — Complete deployment
- [Operations Runbook](docs/deployment/OPERATIONS-RUNBOOK.md) — Day-2 operations
- [Performance Tuning](docs/deployment/PERFORMANCE-TUNING.md) — Optimize performance
- [Security Hardening](docs/deployment/SECURITY-HARDENING-CHECKLIST.md) — 100+ security checks
- [Disaster Recovery](docs/deployment/DISASTER-RECOVERY.md) — RTO/RPO procedures

### Business

- [Cost Optimization](docs/deployment/COST-OPTIMIZATION-GUIDE.md) — Save 70%+ on AWS
- [Capacity Planning](docs/deployment/CAPACITY-PLANNING.md) — Sizing formulas
- [SLA/SLO Definitions](docs/deployment/SLA-SLO-DEFINITIONS.md) — 99.9% uptime

### Compliance

- [GDPR Compliance](docs/COMPLIANCE.md) — Full GDPR mapping
- [SOC 2 Compliance](docs/COMPLIANCE.md#soc-2-compliance) — Trust Services Criteria
- [ISO 27001](docs/COMPLIANCE.md#iso-27001-compliance) — ISMS controls

**Full documentation index**: [docs/](docs/)

---

## Performance

| Metric | Value |
|--------|-------|
| **Max concurrent peers** | 1,000 per instance |
| **Packet throughput** | 10,000 pkt/s per vCPU |
| **Latency (p95)** | < 20 ms |
| **CPU usage** | 70% @ 1000 peers |
| **Memory usage** | 384 MB @ 1000 peers |

**Full benchmarks**: [testing/load/README.md](testing/load/README.md)

---

## Development

```bash
# Clone repository
git clone https://github.com/file-network/relay-server.git
cd relay-server

# Build
cargo build --release

# Run tests
cargo test

# Run locally
cargo run --release
```

**Full guide**: [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)

---

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

## License

Apache License 2.0 — see [LICENSE](LICENSE) for details.

---

## Statistics

- **Lines of Code**: 2,658 (624 production + 1,638 tests + 396 infrastructure)
- **Test Coverage**: 85%+
- **Tests**: 75 (48 unit + 27 integration)
- **Documentation**: 260+ KB across 22+ documents
- **Automation**: 9 operational scripts
- **Infrastructure**: Terraform for AWS + GCP
- **Compliance**: GDPR, SOC 2, ISO 27001 ready

---

<p align="center">
  <strong>Built with ❤️ for the decentralized web</strong>
  <br>
  <a href="https://github.com/file-network/relay-server">GitHub</a> •
  <a href="docs/QUICK-START.md">Quick Start</a> •
  <a href="docs/">Documentation</a> •
  <a href="CONTRIBUTING.md">Contributing</a>
</p>
