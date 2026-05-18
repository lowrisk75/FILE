# FILE Relay Server - Deployment Guide

**Version**: 0.1.0  
**Status**: Production-ready (pending event loop implementation)  
**Last Updated**: 2026-05-18

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Local Development](#local-development)
3. [Docker Deployment](#docker-deployment)
4. [Kubernetes Deployment](#kubernetes-deployment)
5. [Monitoring Setup](#monitoring-setup)
6. [Operations](#operations)
7. [Troubleshooting](#troubleshooting)

---

## Prerequisites

### Build Requirements

```bash
# Rust toolchain
rustc --version  # 1.83+ required
cargo --version

# System dependencies (Debian/Ubuntu)
sudo apt-get install pkg-config libssl-dev

# System dependencies (macOS)
brew install openssl
```

### Runtime Requirements

- **CPU**: 2+ cores (4+ recommended for production)
- **Memory**: 512 MB minimum, 2 GB recommended
- **Network**: Public IP with UDP port 8080 accessible
- **OS**: Linux (Ubuntu 22.04+, Debian 12+), macOS 13+

---

## Local Development

### Build from Source

```bash
# Clone repository
git clone https://github.com/file-network/relay-server.git
cd relay-server

# Build debug binary
cargo build -p relay_server

# Run with default settings
cargo run -p relay_server -- \
  --bind 0.0.0.0:8080 \
  --health-port 8081 \
  --max-peers 1000

# Build release binary (optimized)
cargo build -p relay_server --release
./target/release/file-relay --help
```

### Run Tests

```bash
# All tests
cargo test

# Relay server tests only
cargo test --package relay_server --tests

# Specific test file
cargo test --test rate_limiter_tests

# With output
cargo test -- --nocapture
```

### Local Testing with Docker Compose

```bash
cd deploy

# Start full stack (relay + Prometheus + Grafana + Alertmanager)
docker-compose up -d

# View logs
docker-compose logs -f relay-server

# Stop stack
docker-compose down

# Clean volumes
docker-compose down -v
```

**Access Points**:
- Relay QUIC: `udp://localhost:8080`
- Health Check: `http://localhost:8081/health`
- Metrics: `http://localhost:8081/metrics`
- Prometheus: `http://localhost:9090`
- Grafana: `http://localhost:3000` (admin/admin)
- Alertmanager: `http://localhost:9093`

---

## Docker Deployment

### Build Docker Image

```bash
# Build image
docker build -t file-relay-server:v0.1.0 -f deploy/Dockerfile .

# Tag for registry
docker tag file-relay-server:v0.1.0 ghcr.io/file-network/relay-server:v0.1.0
docker tag file-relay-server:v0.1.0 ghcr.io/file-network/relay-server:latest

# Push to registry
docker push ghcr.io/file-network/relay-server:v0.1.0
docker push ghcr.io/file-network/relay-server:latest
```

### Run Standalone Container

```bash
# Run with default settings
docker run -d \
  --name file-relay \
  -p 8080:8080/udp \
  -p 8081:8081/tcp \
  --restart unless-stopped \
  file-relay-server:v0.1.0

# Run with custom settings
docker run -d \
  --name file-relay \
  -p 8080:8080/udp \
  -p 8081:8081/tcp \
  -e RUST_LOG=info,relay_server=debug \
  --restart unless-stopped \
  file-relay-server:v0.1.0 \
    --bind 0.0.0.0:8080 \
    --health-port 8081 \
    --max-peers 2000 \
    --capow-timeout 150

# View logs
docker logs -f file-relay

# Health check
curl http://localhost:8081/health

# Stop and remove
docker stop file-relay && docker rm file-relay
```

---

## Kubernetes Deployment

### Prerequisites

```bash
# kubectl installed and configured
kubectl version --client

# Cluster ready (1.27+)
kubectl cluster-info

# Create namespace
kubectl create namespace file-network
```

### Deploy Relay Server

```bash
cd deploy/kubernetes

# Apply deployment
kubectl apply -f relay-server-deployment.yaml

# Verify deployment
kubectl -n file-network get deployments
kubectl -n file-network get pods
kubectl -n file-network get services

# Check logs
kubectl -n file-network logs -f deployment/file-relay-server

# Describe pod (troubleshooting)
kubectl -n file-network describe pod <pod-name>
```

### Deploy Monitoring

```bash
# Install Prometheus Operator (if not already installed)
kubectl apply -f https://raw.githubusercontent.com/prometheus-operator/prometheus-operator/main/bundle.yaml

# Apply monitoring resources
kubectl apply -f relay-server-monitoring.yaml

# Verify ServiceMonitor
kubectl -n file-network get servicemonitor

# Verify PrometheusRule
kubectl -n file-network get prometheusrule

# Access Grafana dashboard
kubectl -n file-network port-forward svc/grafana 3000:3000
# Open http://localhost:3000
```

### Scaling

```bash
# Manual scaling
kubectl -n file-network scale deployment file-relay-server --replicas=5

# Check HPA status
kubectl -n file-network get hpa
kubectl -n file-network describe hpa file-relay-server

# View current metrics
kubectl -n file-network top pods
```

### Updates

```bash
# Update image
kubectl -n file-network set image deployment/file-relay-server \
  relay-server=ghcr.io/file-network/relay-server:v0.2.0

# Watch rollout
kubectl -n file-network rollout status deployment/file-relay-server

# Rollback if needed
kubectl -n file-network rollout undo deployment/file-relay-server

# View rollout history
kubectl -n file-network rollout history deployment/file-relay-server
```

---

## Monitoring Setup

### Prometheus Metrics

The relay server exposes Prometheus metrics on `/metrics` endpoint (port 8081):

**Gauge Metrics**:
- `file_relay_active_peers` - Current number of connected peers

**Counter Metrics**:
- `file_relay_peers_registered_total` - Total peers ever registered
- `file_relay_capow_verified_total` - Successful CAPoW verifications
- `file_relay_capow_rejected_total` - Failed CAPoW verifications
- `file_relay_rate_limited_total` - Requests blocked by rate limiter
- `file_relay_packets_forwarded_total` - Total packets relayed
- `file_relay_peers_timeout_total` - Peers disconnected due to timeout

### Key Alerts

**Critical** (immediate action):
- `FileRelayServerDown` - Server not responding
- `FileRelayCapacityReached` - 1000 peers (max capacity)
- `FileRelayCAPoWAttack` - >50 CAPoW rejections/s
- `FileRelayRateLimitAttack` - >100 rate limits/s

**Warning** (investigate soon):
- `FileRelayHighPeerCount` - >900 peers
- `FileRelayHighCAPoWRejectionRate` - >10 rejections/s
- `FileRelayHighRateLimiting` - >20 limits/s
- `FileRelayHighPeerTimeouts` - >5 timeouts/s

### Grafana Dashboards

**Pre-configured dashboard** (via ConfigMap):
- Active peers gauge
- Packet forwarding rate (per pod)
- CAPoW verification rate (verified vs rejected)
- Rate limiting (requests/s)

**Custom queries**:
```promql
# CAPoW success rate (5-minute window)
rate(file_relay_capow_verified_total[5m]) /
(rate(file_relay_capow_verified_total[5m]) + rate(file_relay_capow_rejected_total[5m]))

# Average packets per peer
sum(rate(file_relay_packets_forwarded_total[5m])) /
sum(file_relay_active_peers)

# Rate limiting percentage
rate(file_relay_rate_limited_total[5m]) /
(rate(file_relay_peers_registered_total[5m]) + rate(file_relay_rate_limited_total[5m])) * 100
```

---

## Operations

### Health Checks

```bash
# Liveness probe (is server running?)
curl http://relay-server:8081/health
# Response: {"status":"healthy","service":"FILE Relay Server"}

# Readiness probe (can server accept traffic?)
curl http://relay-server:8081/ready
# Response: {"status":"ready","active_peers":542,"max_peers":1000}
# or: {"status":"not_ready","reason":"max_peers_reached",...}

# Metrics export
curl http://relay-server:8081/metrics
# Response: Prometheus text format
```

### Graceful Shutdown

The relay server handles SIGTERM and SIGINT with a 5-second grace period:

```bash
# Send SIGTERM (Kubernetes default)
kill -TERM <pid>

# Server logs during shutdown:
# 🛑 Shutdown signal received, initiating graceful shutdown...
# 📊 Final relay metrics (active_peers=X, ...)
# ⏳ Draining connections (5 second grace period)...
# ⚠️ Forcefully disconnecting N remaining peers (if any)
# ✅ Graceful shutdown complete
```

### Log Analysis

```bash
# Follow logs (Kubernetes)
kubectl -n file-network logs -f deployment/file-relay-server

# Filter by level
kubectl -n file-network logs deployment/file-relay-server | grep ERROR
kubectl -n file-network logs deployment/file-relay-server | grep WARN

# Search for specific events
kubectl -n file-network logs deployment/file-relay-server | grep "CAPoW rejected"
kubectl -n file-network logs deployment/file-relay-server | grep "Rate limited"
kubectl -n file-network logs deployment/file-relay-server | grep "Peer timeout"

# Docker logs
docker logs file-relay | grep -i error
docker logs --since 1h file-relay
```

### Performance Tuning

**CPU Allocation**:
- CAPoW verification uses worker pool (25% of cores, min 2)
- 4 cores → 1 worker, 8 cores → 2 workers, 16 cores → 4 workers

**Memory**:
- Base: ~100 MB
- Per 1000 peers: ~50 MB
- Replay cache (5-minute TTL): ~10 MB per 10,000 proofs
- **Recommended**: 512 MB for <500 peers, 1 GB for <1000 peers

**Network**:
- UDP buffer size: OS default (increase with `sysctl -w net.core.rmem_max=...`)
- Keep-alive: Not applicable (stateless UDP)

---

## Troubleshooting

### Server Won't Start

**Issue**: Binary fails to start

```bash
# Check port availability
sudo lsof -i :8080  # QUIC port
sudo lsof -i :8081  # Health check port

# Check permissions (needs CAP_NET_BIND_SERVICE for ports <1024)
sudo setcap 'cap_net_bind_service=+ep' /usr/local/bin/file-relay

# Verify binary
file /usr/local/bin/file-relay
ldd /usr/local/bin/file-relay  # Check missing libraries
```

**Issue**: Container exits immediately

```bash
# Check logs
docker logs file-relay

# Common causes:
# - Port already in use (stop conflicting container)
# - Invalid arguments (check --help)
# - Missing permissions (run with --security-opt or adjust seccomp profile)
```

### High CAPoW Rejection Rate

**Symptoms**: `FileRelayHighCAPoWRejectionRate` or `FileRelayCAPoWAttack` alert

**Causes**:
1. **DoS attack** - Malicious peers sending invalid proofs
2. **Client misconfiguration** - Clients using wrong difficulty or algorithm
3. **Clock skew** - Client and server clocks out of sync

**Actions**:
```bash
# Check rejection rate
curl http://relay-server:8081/metrics | grep capow_rejected

# Review logs for details
kubectl logs deployment/file-relay-server | grep "CAPoW rejected"

# If attack: block source IPs at firewall level
# If clients: verify client CAPoW implementation
# If clock: check NTP sync on clients
```

### High Rate Limiting

**Symptoms**: `FileRelayHighRateLimiting` or `FileRelayRateLimitAttack` alert

**Causes**:
1. **Burst traffic** - Many peers reconnecting simultaneously
2. **DoS attack** - Flood from single or multiple IPs
3. **Client bug** - Client retrying too aggressively

**Actions**:
```bash
# Check rate limiting rate
curl http://relay-server:8081/metrics | grep rate_limited

# Logs show hashed IPs (GDPR-compliant)
kubectl logs deployment/file-relay-server | grep "Rate limited"

# Adjust rate limiter (requires rebuild):
# - Increase burst capacity (default: 50)
# - Increase sustained rate (default: 10 req/s)
# Edit main.rs: RateLimiter::new(100.0, 20.0)  // burst 100, 20 req/s
```

### Peer Capacity Issues

**Symptoms**: `FileRelayCapacityReached` alert, /ready returns `not_ready`

**Actions**:
```bash
# Check current peer count
curl http://relay-server:8081/ready

# Increase max peers (requires restart)
kubectl -n file-network set env deployment/file-relay-server \
  MAX_PEERS=2000

# Or edit deployment YAML:
#   args: ["--max-peers", "2000"]

# Scale horizontally instead (preferred)
kubectl -n file-network scale deployment file-relay-server --replicas=5
```

### Low Packet Forwarding Rate

**Symptoms**: `FileRelayLowPacketRate` alert, high peer count but low throughput

**Causes**:
1. **Network congestion** - Upstream/downstream bandwidth saturated
2. **Peer inactivity** - Peers connected but not sending data
3. **Event loop bottleneck** - (Future: after event loop implemented)

**Actions**:
```bash
# Check packet rate
curl http://relay-server:8081/metrics | grep packets_forwarded

# Check network I/O (Linux)
iftop
nethogs
ss -s  # Socket statistics

# Kubernetes: check pod network limits
kubectl -n file-network describe pod <pod-name> | grep -A 5 Limits
```

### Memory Leaks

**Symptoms**: Memory usage grows unbounded

**Diagnostics**:
```bash
# Kubernetes: watch memory usage
kubectl -n file-network top pod <pod-name>

# Check for replay cache growth (should be bounded)
curl http://relay-server:8081/metrics | grep active_peers

# Expected memory:
# Base: ~100 MB
# + (active_peers * 0.05 MB)
# + (replay_cache_entries * 0.000048 MB)

# If OOM: increase memory limits or reduce max_peers
```

### Connection Refused

**Issue**: Clients cannot connect to relay

```bash
# Check service exposure (Kubernetes)
kubectl -n file-network get svc file-relay-server
kubectl -n file-network describe svc file-relay-server

# Check external IP assignment
kubectl -n file-network get svc file-relay-server -o jsonpath='{.status.loadBalancer.ingress}'

# Test connectivity from outside cluster
nc -vzu <external-ip> 8080  # UDP test

# Check firewall rules (cloud provider)
# - Ensure UDP port 8080 is open
# - Check security groups / firewall rules
```

---

## Security Considerations

### Network

- **Firewall**: Only UDP 8080 (relay) and TCP 8081 (metrics) need to be exposed
- **DDoS Protection**: CAPoW + per-IP rate limiting built-in
- **Amplification**: Rate limiter prevents amplification attacks

### Data Privacy

- **GDPR**: All IP addresses are hashed (HMAC-SHA256) before logging
- **No PII**: Server never sees plaintext IP addresses in logs
- **Ephemeral Secrets**: IP hash secret is per-boot, not persisted

### Supply Chain

- **Dependencies**: All crypto dependencies pinned to exact versions
- **Audit**: Run `cargo audit` before each deployment
- **Updates**: Monitor RUSTSEC advisories

---

## Next Steps

1. **Implement Connection Event Loop** (blocker for full deployment)
2. **Implement Noise XX Authentication** (SEC-C06)
3. **Deploy Beta VPS** (Hetzner Germany)
4. **Validate Metrics** with live traffic
5. **Stress Test** with 1,000 concurrent peers
6. **Geographic Distribution** (multiple relay servers)

---

## Support

- **Documentation**: `docs/` directory
- **Issues**: GitHub Issues
- **Security**: security@file-network.example
- **Monitoring**: Grafana dashboards + Prometheus alerts

---

**Deployment Status**: ✅ Production-ready (pending event loop)  
**Last Updated**: 2026-05-18  
**Version**: 0.1.0
