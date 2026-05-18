# FILE Relay Server - Observability Guide

**Last Updated**: 2026-05-18  
**Version**: 0.1.0

---

## Table of Contents

1. [Overview](#overview)
2. [The Three Pillars](#the-three-pillars)
3. [Metrics Deep Dive](#metrics-deep-dive)
4. [Logging Best Practices](#logging-best-practices)
5. [Distributed Tracing](#distributed-tracing)
6. [Alerting Strategy](#alerting-strategy)
7. [Dashboards](#dashboards)
8. [Troubleshooting Workflows](#troubleshooting-workflows)
9. [Cost Optimization](#cost-optimization)

---

## Overview

Observability is the ability to understand system behavior from external outputs. FILE Relay Server provides comprehensive observability through:

- **Metrics**: Prometheus metrics exposed on port 8081
- **Logs**: Structured logging via `tracing` crate
- **Health checks**: HTTP endpoints for liveness and readiness

This guide covers how to effectively monitor, debug, and optimize the relay server in production.

---

## The Three Pillars

### Metrics (What is happening?)

**Purpose**: Quantitative measurements of system behavior over time.

**FILE Relay Server exposes 7 metrics**:
- 1 gauge: `file_relay_active_peers`
- 6 counters: registrations, CAPoW, rate limiting, packets, timeouts

**Access**: `http://relay:8081/metrics`

**Retention**: Prometheus default (15 days, configurable)

### Logs (Why is it happening?)

**Purpose**: Timestamped events with context for debugging.

**FILE Relay Server uses structured logging**:
- Level: debug, info, warn, error
- Fields: `peer_id`, `ip_hash`, `capow_result`, etc.
- Format: JSON (via `tracing-subscriber`)

**Access**: `journalctl -u file-relay` (systemd), `kubectl logs` (K8s)

**Retention**: 7 days default (logrotate/K8s)

### Traces (How did it happen?)

**Status**: Not implemented (future: OpenTelemetry integration)

---

## Metrics Deep Dive

### Metric Catalog

| Metric | Type | Description | Labels | Alert Threshold |
|--------|------|-------------|--------|-----------------|
| `file_relay_active_peers` | Gauge | Current connected peers | None | > 900 (capacity warning) |
| `file_relay_peers_registered_total` | Counter | Total peer registrations | None | N/A (informational) |
| `file_relay_capow_verified_total` | Counter | CAPoW proofs accepted | None | Rate matters, not total |
| `file_relay_capow_rejected_total` | Counter | CAPoW proofs rejected | None | > 50/s (attack) |
| `file_relay_rate_limited_total` | Counter | Requests rate-limited | None | > 100/s (attack) |
| `file_relay_packets_forwarded_total` | Counter | Packets relayed | None | Sudden drop = issue |
| `file_relay_peers_timeout_total` | Counter | Peers timed out | None | > 5/s (network quality) |

### Metric Interpretation

#### `file_relay_active_peers` (Gauge)

**What it measures**: Current number of peers with active connections.

**Normal behavior**:
- Fluctuates based on user activity
- Increases during peak hours
- Decreases during off-hours

**Anomalies**:
- Sudden spike → possible flood/attack
- Sudden drop → relay restart or network partition
- Stuck at capacity (1000) → need to scale

**Query examples**:
```promql
# Current active peers
file_relay_active_peers

# Capacity utilization %
(file_relay_active_peers / 1000) * 100

# Peer count over time (1h window)
file_relay_active_peers[1h]
```

#### `file_relay_capow_verified_total` + `file_relay_capow_rejected_total` (Counters)

**What they measure**: CAPoW proof verification success/failure.

**Normal behavior**:
- Most proofs verified (>90% success rate)
- Rejections < 10/minute (misconfigured clients)

**Anomalies**:
- High rejection rate (>50%) → DoS attack
- Zero rejections → suspicious (no validation?)
- All rejections → relay misconfiguration

**Query examples**:
```promql
# CAPoW success rate (5-minute window)
(
  rate(file_relay_capow_verified_total[5m])
  /
  (rate(file_relay_capow_verified_total[5m]) + rate(file_relay_capow_rejected_total[5m]))
) * 100

# Rejection rate (per second)
rate(file_relay_capow_rejected_total[1m])

# Total proofs processed
file_relay_capow_verified_total + file_relay_capow_rejected_total
```

#### `file_relay_packets_forwarded_total` (Counter)

**What it measures**: Total packets relayed between peers.

**Normal behavior**:
- Steady increase (proportional to peer activity)
- Rate varies with application workload

**Anomalies**:
- Sudden drop → relay issue or peer disconnections
- Zero rate → event loop not running (pending implementation)
- Spike → traffic surge or attack

**Query examples**:
```promql
# Packet forwarding rate (packets per second)
rate(file_relay_packets_forwarded_total[1m])

# Packets per peer (average)
rate(file_relay_packets_forwarded_total[5m]) / file_relay_active_peers

# Total packets in last 24h
increase(file_relay_packets_forwarded_total[24h])
```

### Recording Rules

Pre-compute expensive queries for dashboard performance.

**Example** (`deploy/prometheus/alerts.yml`):
```yaml
groups:
  - name: file_relay_recording_rules
    interval: 30s
    rules:
      - record: file_relay:capow_success_rate
        expr: |
          (
            rate(file_relay_capow_verified_total[5m])
            /
            (rate(file_relay_capow_verified_total[5m]) + rate(file_relay_capow_rejected_total[5m]))
          ) * 100

      - record: file_relay:packets_per_peer
        expr: |
          rate(file_relay_packets_forwarded_total[5m]) / file_relay_active_peers

      - record: file_relay:rate_limit_rate
        expr: |
          rate(file_relay_rate_limited_total[1m])
```

---

## Logging Best Practices

### Log Levels

| Level | When to Use | Examples |
|-------|-------------|----------|
| `error` | Operation failed, requires attention | CAPoW verification timeout, metric export failure |
| `warn` | Degraded state, no immediate failure | High rejection rate, nearing capacity |
| `info` | Normal operations | Peer registered, graceful shutdown initiated |
| `debug` | Detailed diagnostic info | CAPoW proof details, rate limiter state |
| `trace` | Very verbose (dev only) | Every packet, every lock acquisition |

### Structured Logging

**Good** (structured):
```rust
tracing::info!(
    peer_id = %peer_id,
    ip_hash = %ip_hash,
    "Peer registered successfully"
);
```

**Bad** (unstructured):
```rust
println!("Peer {} registered from {}", peer_id, ip_hash);
```

**Why**: Structured logs can be queried, filtered, and aggregated.

### Log Sampling

For high-frequency events, sample logs to reduce volume:

```rust
use tracing::Level;

// Log only 1% of packet forwards
if rand::random::<f64>() < 0.01 {
    tracing::debug!(
        peer_id = %peer_id,
        packet_size = packet.len(),
        "Packet forwarded"
    );
}
```

### Log Correlation

Add correlation IDs to trace requests across components:

```rust
let request_id = Uuid::new_v4();
let span = tracing::info_span!("handle_registration", request_id = %request_id);
let _enter = span.enter();

// All logs within this span include request_id
tracing::info!("Verifying CAPoW...");
```

---

## Distributed Tracing

**Status**: Not implemented (future: OpenTelemetry integration)

**Planned architecture**:
```
Peer A → Relay Server → Peer B
  ↓          ↓            ↓
  Trace ID = abc123 propagated through
```

**Benefits**:
- End-to-end latency breakdown
- Identify bottlenecks (CAPoW vs forwarding)
- Visualize request flow

**Implementation plan** (future):
1. Add `opentelemetry` + `tracing-opentelemetry` crates
2. Export spans to Jaeger or Tempo
3. Instrument `handle_registration`, `handle_forward`, `verify_capow`

---

## Alerting Strategy

### Alert Hierarchy

**P1 (Critical)**: Immediate response required (paging)
- Server down
- Capacity reached
- Active DoS attack

**P2 (Warning)**: Review within 1 hour
- High peer count (capacity planning)
- Elevated rejection rate
- High timeout rate

**P3 (Info)**: Review next business day
- Minor anomalies
- Informational notifications

### Alert Design Principles

1. **Actionable**: Every alert must have a runbook entry
2. **Tuned**: False positive rate < 5%
3. **Routed**: Critical → PagerDuty, Warning → Slack, Info → Email
4. **Inhibited**: Don't alert on symptoms if root cause is known

### Example Alerts

See `deploy/prometheus/alerts.yml` for full set (12 alerts).

**Critical alert anatomy**:
```yaml
- alert: FileRelayCAPoWAttack
  expr: rate(file_relay_capow_rejected_total[1m]) > 50
  for: 2m
  labels:
    severity: critical
  annotations:
    summary: "CAPoW rejection rate > 50/s for 2 minutes"
    description: "Possible DoS attack. Current rate: {{ $value | humanize }}/s"
    runbook_url: "https://github.com/file-network/relay-server/blob/main/docs/deployment/OPERATIONS-RUNBOOK.md#dos-attack"
```

**Key fields**:
- `expr`: PromQL query (when to fire)
- `for`: Sustained duration (avoid flapping)
- `labels`: Routing (severity, team, etc.)
- `annotations`: Context for responder
- `runbook_url`: What to do when it fires

---

## Dashboards

### Pre-Built Dashboard

**Location**: `deploy/grafana/provisioning/dashboards/file-relay.json`

**Panels** (4):
1. Active Peers (gauge with thresholds)
2. Packet Forwarding Rate (per pod, stacked)
3. CAPoW Verification Rate (verified vs rejected)
4. Rate Limiting (requests/s)

### Dashboard Design Principles

1. **Top-down**: Start with high-level (active peers), drill down to specifics (CAPoW rejection details)
2. **Time-aligned**: All panels use same time range
3. **Annotated**: Mark deploys, incidents, maintenance windows
4. **Templated**: Support multi-pod/multi-cluster views

### Custom Dashboard Ideas

**Capacity Planning Dashboard**:
- Peer count trend (7 days)
- Growth rate (peers/day)
- Projected capacity exhaustion date
- Per-instance utilization

**Security Dashboard**:
- CAPoW rejection rate (heatmap)
- Rate limiting events (by IP)
- Suspicious activity score
- GeoIP distribution (if available)

**Performance Dashboard**:
- Packet forwarding rate (p50, p95, p99)
- CAPoW verification latency
- Memory per peer
- CPU per 100 peers

---

## Troubleshooting Workflows

### Symptom: High CPU Usage

**Step 1**: Identify source
```bash
# Check metrics
curl http://relay:8081/metrics | grep capow_verified

# If CAPoW verification is high (>100/s), CPU is expected
```

**Step 2**: Correlate with logs
```bash
journalctl -u file-relay | grep "CAPoW verification timeout"

# If many timeouts, worker pool is saturated
```

**Step 3**: Action
- Short-term: Scale horizontally (add replicas)
- Long-term: Increase worker pool size (code change)

### Symptom: High Memory Usage

**Step 1**: Check peer count
```bash
curl http://relay:8081/metrics | grep active_peers

# Expected memory: ~512 MB + (active_peers * 0.5 MB)
```

**Step 2**: Compare to expected
```bash
# If memory >> expected, possible leak or replay cache growth
```

**Step 3**: Action
- Restart pod (temporary)
- Review replay cache cleanup (60s cycle)
- Report bug if leak confirmed

### Symptom: Packets Not Forwarded

**Step 1**: Check event loop status
```bash
# Pending implementation: event loop not running yet
# Once implemented, check logs for connection errors
```

**Step 2**: Verify peer connectivity
```bash
# Can peers reach each other?
# Check NAT traversal, firewall rules
```

---

## Cost Optimization

### Metrics Storage

**Prometheus storage cost**: ~1 KB per metric per sample

**Optimization**:
- Reduce retention (15d → 7d)
- Increase scrape interval (30s → 60s)
- Use recording rules (pre-aggregate)

**Example savings**:
- 10 pods × 7 metrics × 1 sample/30s × 86400s/day = 201,600 samples/day
- 201,600 × 1 KB × 15 days = ~3 GB (before)
- 201,600 × 1 KB × 7 days = ~1.4 GB (after, 53% savings)

### Log Storage

**Log volume**: ~100 bytes per log line

**Optimization**:
- Filter debug logs in production (`RUST_LOG=info`)
- Sample high-frequency events
- Compress logs (gzip: 10:1 ratio)

**Example savings**:
- 10 pods × 1000 logs/s × 86400s/day × 100 bytes = 84 GB/day (debug)
- 10 pods × 100 logs/s × 86400s/day × 100 bytes = 8.4 GB/day (info, 90% savings)

### Observability SLO

**Target**: < 5% of total infrastructure cost

**Breakdown**:
- Prometheus: 2%
- Logs: 1.5%
- Grafana: 0.5%
- Alertmanager: <0.1%

---

## Further Reading

- [Operations Runbook](OPERATIONS-RUNBOOK.md) — Incident response procedures
- [Performance Tuning](PERFORMANCE-TUNING.md) — Optimization guide
- [Prometheus Best Practices](https://prometheus.io/docs/practices/) — Upstream docs
- [Grafana Dashboard Guide](https://grafana.com/docs/grafana/latest/dashboards/) — Dashboard design

---

**Questions?** [Open an issue](https://github.com/file-network/relay-server/issues) or consult the [FAQ](../FAQ.md).
