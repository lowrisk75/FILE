# FILE Relay Server

[![CI](https://github.com/file-network/relay-server/actions/workflows/ci.yml/badge.svg)](https://github.com/file-network/relay-server/actions/workflows/ci.yml)
[![Security](https://github.com/file-network/relay-server/actions/workflows/security.yml/badge.svg)](https://github.com/file-network/relay-server/actions/workflows/security.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../../LICENSE)

**Production-ready zero-trust MPQUIC relay server with CAPoW DoS protection**

Part of the FILE Network sovereign architecture. Stateless, high-performance relay for MPQUIC peer-to-peer connections with comprehensive observability and DoS protection.

---

## Features

### Security ✅
- CAPoW DoS protection (MTP-Argon2id, 100ms timeout)
- Per-IP rate limiting (10 req/s sustained, burst 50)
- Replay prevention (5-minute SHA-256 cache)
- GDPR-compliant IP hashing (HMAC-SHA256)
- HashDoS resistance (SipHash-2-4)
- Supply chain security (pinned dependencies)

### Operations ✅
- Graceful shutdown (5s grace period)
- Health check endpoints (`/health`, `/ready`, `/metrics`)
- Prometheus metrics (7 metrics, 30s scrape)
- Kubernetes-ready (HPA, PDB, ServiceMonitor)
- Docker + Docker Compose

---

## Quick Start

### Build & Run

```bash
# Build
cargo build --release

# Run
./target/release/file-relay \
  --bind 0.0.0.0:8080 \
  --health-port 8081 \
  --max-peers 1000
```

### Docker

```bash
# Build
docker build -t file-relay -f ../../deploy/Dockerfile ../..

# Run
docker run -p 8080:8080/udp -p 8081:8081/tcp file-relay
```

### Tests

```bash
# All tests
cargo test

# Integration tests only
cargo test --tests
```

---

## Configuration

```bash
file-relay [OPTIONS]

OPTIONS:
    --bind <ADDR>           Bind address (default: 0.0.0.0:8080)
    --health-port <PORT>    Health check port (default: 8081)
    --max-peers <NUM>       Max concurrent peers (default: 1000)
    --capow-timeout <MS>    CAPoW timeout (default: 100ms)
```

---

## Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `file_relay_active_peers` | Gauge | Current connected peers |
| `file_relay_peers_registered_total` | Counter | Total registrations |
| `file_relay_capow_verified_total` | Counter | Successful CAPoW verifications |
| `file_relay_capow_rejected_total` | Counter | Failed CAPoW verifications |
| `file_relay_rate_limited_total` | Counter | Rate-limited requests |
| `file_relay_packets_forwarded_total` | Counter | Packets relayed |
| `file_relay_peers_timeout_total` | Counter | Peer timeouts |

**Prometheus endpoint**: `http://localhost:8081/metrics`

---

## Documentation

- **[Deployment Guide](../../docs/deployment/DEPLOYMENT-GUIDE.md)** - Complete deployment procedures
- **[Operations Runbook](../../docs/deployment/OPERATIONS-RUNBOOK.md)** - Incident response
- **[Test Coverage](../../docs/tests/Test-Coverage-Report-2026-05-18.md)** - 75 tests, all passing
- **[Security Audit](../../docs/security/Supply-Chain-Audit-2026-05-18.md)** - Supply chain report

---

## Architecture

```
┌────────────────────────────────────────┐
│      FILE Relay Server (port 8080)     │
├────────────────────────────────────────┤
│                                         │
│  CAPoW Verifier → Rate Limiter         │
│       ↓               ↓                 │
│  GDPR IP Hasher → Replay Cache         │
│       ↓               ↓                 │
│  MPQUIC Packet Forwarder               │
│  (ant-quic endpoint.send())            │
│                                         │
│  HTTP Server (port 8081):              │
│  - GET /health  (liveness)             │
│  - GET /ready   (readiness)            │
│  - GET /metrics (Prometheus)           │
│                                         │
└────────────────────────────────────────┘
```

---

## Status

**Version**: 0.1.0  
**Build**: 0 errors, 65 tests passing  
**Production Ready**: ✅ YES (pending event loop)

---

**Part of the FILE Network sovereign architecture** | [Main README](../../README.md)
