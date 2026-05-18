# FILE Relay Server - SLA/SLO Definitions

**Last Updated**: 2026-05-18  
**Version**: 0.1.0

---

## Table of Contents

1. [Overview](#overview)
2. [Service Level Indicators (SLIs)](#service-level-indicators-slis)
3. [Service Level Objectives (SLOs)](#service-level-objectives-slos)
4. [Service Level Agreements (SLAs)](#service-level-agreements-slas)
5. [Error Budgets](#error-budgets)
6. [Measurement and Reporting](#measurement-and-reporting)

---

## Overview

This document defines reliability targets for FILE Relay Server production deployments.

**Definitions**:
- **SLI (Service Level Indicator)**: Quantitative measure of service quality
- **SLO (Service Level Objective)**: Target value for an SLI (internal goal)
- **SLA (Service Level Agreement)**: Contractual commitment with consequences (external promise)

**Hierarchy**: `SLI → SLO → SLA`

**Example**:
- SLI: Availability (% of successful requests)
- SLO: 99.9% availability over 30 days (internal target)
- SLA: 99.5% availability over 30 days (contractual guarantee)

**Note**: SLA < SLO to provide error budget buffer.

---

## Service Level Indicators (SLIs)

### SLI-1: Availability

**Definition**: Percentage of time the relay server is operational and accepting connections.

**Measurement**:
```promql
# Availability % (last 30 days)
(
  sum(rate(probe_success{job="file-relay"}[30d]))
  /
  sum(rate(probe_duration_seconds_count{job="file-relay"}[30d]))
) * 100
```

**Data source**: Prometheus Blackbox Exporter probing `/health` endpoint every 30 seconds.

**Good event**: HTTP 200 response from `/health` within 5 seconds  
**Bad event**: HTTP 5xx, timeout, or connection refused

**Target**: 99.9% (43.8 minutes downtime per month)

---

### SLI-2: Success Rate

**Definition**: Percentage of peer registration requests that succeed.

**Measurement**:
```promql
# Success rate % (last 7 days)
(
  sum(rate(file_relay_peers_registered_total[7d]))
  /
  (sum(rate(file_relay_peers_registered_total[7d])) + sum(rate(file_relay_capow_rejected_total[7d])) + sum(rate(file_relay_rate_limited_total[7d])))
) * 100
```

**Data source**: Relay server metrics (Prometheus)

**Good event**: Peer registration succeeds (CAPoW verified, not rate-limited)  
**Bad event**: Registration rejected (invalid CAPoW) or rate-limited

**Target**: 95% (5% rejected due to DoS protection is acceptable)

**Note**: This SLI excludes attack traffic (by design). A high rejection rate during an attack is healthy behavior.

---

### SLI-3: Latency

**Definition**: Time to relay a packet from peer A to peer B.

**Measurement**:
```promql
# p95 latency (last 24 hours)
histogram_quantile(0.95, rate(file_relay_packet_forward_duration_seconds_bucket[24h]))
```

**Data source**: Relay server metrics (pending: instrumentation in event loop)

**Good event**: Packet forwarded in < 20ms (p95)  
**Bad event**: Packet forwarded in >= 20ms

**Target**: p95 < 20ms, p99 < 50ms

**Status**: Pending event loop implementation (instrumentation ready)

---

### SLI-4: Capacity Utilization

**Definition**: Percentage of maximum peer capacity in use.

**Measurement**:
```promql
# Capacity utilization % (current)
(file_relay_active_peers / 1000) * 100
```

**Data source**: Relay server metrics (Prometheus)

**Good event**: Utilization < 90%  
**Bad event**: Utilization >= 90%

**Target**: < 90% (maintain headroom for bursts)

---

### SLI-5: Error Rate

**Definition**: Percentage of requests resulting in errors (rate limiting, timeouts).

**Measurement**:
```promql
# Error rate % (last 1 hour)
(
  sum(rate(file_relay_rate_limited_total[1h])) + sum(rate(file_relay_peers_timeout_total[1h]))
  /
  sum(rate(file_relay_peers_registered_total[1h]))
) * 100
```

**Data source**: Relay server metrics (Prometheus)

**Good event**: Request succeeds without error  
**Bad event**: Request rate-limited or times out

**Target**: < 1% (excluding attack traffic)

---

## Service Level Objectives (SLOs)

### SLO Targets by Environment

| SLI | Production | Staging | Development |
|-----|-----------|---------|-------------|
| **Availability** | 99.9% (43.8 min downtime/mo) | 99% (7.2 hrs/mo) | 95% (36 hrs/mo) |
| **Success Rate** | 95% | 90% | N/A |
| **Latency (p95)** | < 20ms | < 50ms | N/A |
| **Latency (p99)** | < 50ms | < 100ms | N/A |
| **Capacity** | < 90% | < 95% | N/A |
| **Error Rate** | < 1% | < 5% | N/A |

### SLO Compliance Windows

| SLO | Window | Why |
|-----|--------|-----|
| Availability | 30 days rolling | Industry standard, aligns with monthly billing |
| Success Rate | 7 days rolling | Shorter window for faster detection |
| Latency | 24 hours rolling | Real-time performance indicator |
| Capacity | Real-time (5 min avg) | Immediate capacity planning |
| Error Rate | 1 hour rolling | Fast incident detection |

### Tiered SLOs (Optional)

For enterprise customers, offer tiered service levels:

| Tier | Availability | Success Rate | Latency (p95) | Price |
|------|-------------|--------------|---------------|-------|
| **Standard** | 99.5% | 90% | < 50ms | Base |
| **Premium** | 99.9% | 95% | < 20ms | +50% |
| **Enterprise** | 99.95% | 98% | < 10ms | +100% |

**Implementation**: Use Kubernetes priority classes, resource guarantees, and dedicated node pools.

---

## Service Level Agreements (SLAs)

### Production SLA (Customer-Facing)

**Availability SLA**: 99.5% uptime over 30 days

**Credits** (if SLA violated):
| Uptime % | Downtime | Credit |
|----------|----------|--------|
| 99.5% - 99.0% | 3.6 - 7.2 hours | 10% monthly credit |
| 99.0% - 95.0% | 7.2 - 36 hours | 25% monthly credit |
| < 95.0% | > 36 hours | 50% monthly credit |

**Exclusions** (do not count against SLA):
- Scheduled maintenance (with 7 days notice)
- DDoS attacks (beyond mitigation capacity)
- Third-party failures (cloud provider outage)
- Customer-caused issues (misconfiguration)

**Measurement**: Prometheus uptime monitoring with third-party validation (Pingdom, UptimeRobot)

**Claim process**: Customer opens ticket within 30 days of incident with timestamp and impact description.

---

### Internal SLO (Operations Team)

**Targets** (stricter than SLA):
- **Availability**: 99.9% (not 99.5%)
- **Buffer**: 0.4% error budget between SLO and SLA
- **Response time**: Page on-call engineer within 5 minutes of SLA violation risk

**Why stricter?**
- Provides buffer for unexpected issues
- Allows preventive action before SLA breach
- Reduces customer impact

---

## Error Budgets

### What is an Error Budget?

**Definition**: Amount of unreliability allowed within SLO.

**Calculation**:
```
Error Budget = (1 - SLO) × Time Window
```

**Example** (99.9% SLO over 30 days):
```
Error Budget = (1 - 0.999) × 30 days
             = 0.001 × 43,200 minutes
             = 43.2 minutes downtime allowed per month
```

### Error Budget Consumption

**Tracking**:
```promql
# Error budget remaining (%)
(
  1 - (
    (sum(rate(probe_success{job="file-relay"}[30d])) / sum(rate(probe_duration_seconds_count{job="file-relay"}[30d])))
    /
    0.999  # SLO target
  )
) * 100
```

**Interpretation**:
- **100% remaining**: Perfect reliability (no downtime)
- **50% remaining**: Consumed half of error budget (21.6 min downtime)
- **0% remaining**: Error budget exhausted (SLO violated)
- **Negative**: Exceeded error budget (SLO breached)

### Error Budget Policies

**When error budget is healthy (> 50% remaining)**:
- ✅ Deploy new features
- ✅ Conduct experiments
- ✅ Release frequently

**When error budget is low (< 25% remaining)**:
- ⚠️ Freeze feature releases
- ⚠️ Focus on reliability improvements
- ⚠️ Increase testing rigor

**When error budget is exhausted (< 0%)**:
- 🚨 Feature freeze (only critical fixes)
- 🚨 Postmortem required
- 🚨 Reliability work prioritized

### Error Budget Alerting

```yaml
- alert: ErrorBudgetLow
  expr: error_budget_remaining_percent < 25
  for: 30m
  labels:
    severity: warning
  annotations:
    summary: "Error budget < 25% ({{$value}}% remaining)"
    description: "Slow down releases, focus on reliability"

- alert: ErrorBudgetExhausted
  expr: error_budget_remaining_percent < 0
  for: 5m
  labels:
    severity: critical
  annotations:
    summary: "Error budget exhausted (SLO breached)"
    description: "Feature freeze in effect, postmortem required"
```

---

## Measurement and Reporting

### Data Sources

| Metric | Source | Granularity | Retention |
|--------|--------|-------------|-----------|
| Uptime | Prometheus Blackbox Exporter | 30s | 90 days |
| Success Rate | Relay server metrics | Real-time | 90 days |
| Latency | Relay server metrics (pending) | Real-time | 90 days |
| Capacity | Relay server metrics | Real-time | 90 days |
| Error Rate | Relay server metrics | Real-time | 90 days |

### Reporting Frequency

**Real-time dashboards** (Grafana):
- Current SLI values
- SLO compliance status (✓ or ✗)
- Error budget remaining

**Weekly reports** (automated):
- SLO compliance summary
- Error budget consumption trend
- Top incidents (by impact)
- Capacity utilization trend

**Monthly reports** (manual):
- SLA compliance (for customers)
- Postmortem summaries
- Reliability trends
- Recommendations

### SLO Dashboard (Grafana)

**Panels**:
1. **Availability** (stat): Current uptime %, SLO target, trend
2. **Success Rate** (graph): 7-day rolling average, SLO threshold line
3. **Latency** (heatmap): p50/p95/p99, SLO thresholds
4. **Error Budget** (gauge): % remaining, color-coded (green > 50%, yellow > 25%, red < 25%)
5. **Incidents** (table): Last 10 incidents with duration and impact

---

## Example SLO Report

**Period**: 2026-05-01 to 2026-05-31  
**Environment**: Production

| SLI | Target | Actual | Status | Error Budget |
|-----|--------|--------|--------|--------------|
| **Availability** | 99.9% | 99.92% | ✅ Pass | 34.6 min remaining (80%) |
| **Success Rate** | 95% | 96.8% | ✅ Pass | N/A |
| **Latency (p95)** | < 20ms | 12ms | ✅ Pass | N/A |
| **Latency (p99)** | < 50ms | 28ms | ✅ Pass | N/A |
| **Capacity** | < 90% | 72% | ✅ Pass | N/A |
| **Error Rate** | < 1% | 0.3% | ✅ Pass | N/A |

**Incidents**:
- **2026-05-12 14:23 UTC** (SEV-2): Node failure in us-east-1, 3 minutes downtime, auto-recovery
- **2026-05-18 09:15 UTC** (SEV-3): Brief latency spike (p99 → 85ms for 2 minutes), cause: deployment rollout

**Summary**: All SLOs met. Error budget consumption healthy (20% used). No action required.

---

## Review and Adjustment

### SLO Review Cadence

- **Monthly**: Review SLO compliance, adjust thresholds if needed
- **Quarterly**: Comprehensive SLO review, compare to industry benchmarks
- **Annually**: Major SLO revision (if service maturity changes)

### When to Adjust SLOs

**Tighten SLOs** (make more strict) when:
- Consistently exceeding targets (e.g., 99.99% actual vs 99.9% target)
- Error budget always > 80% unused
- Customer expectations increase

**Relax SLOs** (make less strict) when:
- Consistently missing targets (e.g., 99.5% actual vs 99.9% target)
- Error budget always exhausted
- Feature velocity too slow due to freezes

**Principle**: SLOs should be **achievable but challenging** (hit 90-95% of the time).

---

## Further Reading

- [Observability Guide](OBSERVABILITY-GUIDE.md) — Metrics and monitoring
- [Operations Runbook](OPERATIONS-RUNBOOK.md) — Incident response
- [Disaster Recovery](DISASTER-RECOVERY.md) — RTO/RPO definitions
- [Google SRE Book - Chapter 4: Service Level Objectives](https://sre.google/sre-book/service-level-objectives/)

---

**Questions?** [Open an issue](https://github.com/file-network/relay-server/issues) or consult the [FAQ](../FAQ.md).
