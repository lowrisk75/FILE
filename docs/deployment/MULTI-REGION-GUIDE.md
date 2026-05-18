# FILE Relay Server - Multi-Region Deployment Guide

**Last Updated**: 2026-05-18  
**Version**: 0.1.0

---

## Table of Contents

1. [Overview](#overview)
2. [Why Multi-Region](#why-multi-region)
3. [Architecture Patterns](#architecture-patterns)
4. [Region Selection](#region-selection)
5. [Traffic Routing](#traffic-routing)
6. [Deployment Procedures](#deployment-procedures)
7. [Monitoring Multi-Region](#monitoring-multi-region)
8. [Cost Analysis](#cost-analysis)

---

## Overview

Multi-region deployment distributes FILE Relay Server across multiple geographic locations for **high availability**, **low latency**, and **disaster recovery**.

**Key characteristics**:
- **Stateless**: No data synchronization between regions
- **Independent**: Each region operates autonomously
- **Active-Active**: All regions serve traffic simultaneously
- **Geo-aware**: Route users to nearest region

---

## Why Multi-Region

### Benefits

**1. High Availability**
- Survive complete region outages
- 99.99% uptime (vs 99.9% single-region)
- Automatic failover between regions

**2. Low Latency**
- Users routed to nearest region
- Reduced round-trip time (RTT)
- Better user experience

**3. Compliance**
- Data residency requirements (EU users → EU region)
- GDPR compliance (EU data stays in EU)

**4. Disaster Recovery**
- Multiple failure domains
- Geographic diversity
- Faster recovery (no cross-region rebuild)

### Trade-offs

**Costs**:
- 2-3x infrastructure cost (multiple regions)
- Additional complexity (multi-region coordination)
- Increased monitoring/alerting overhead

**When to deploy multi-region**:
- ✅ User base > 10,000 globally distributed
- ✅ SLA requires 99.99% uptime
- ✅ Regulatory compliance needs (GDPR, data residency)
- ✅ Latency-sensitive application

**When NOT to deploy multi-region** (stick to single region + multi-AZ):
- ❌ User base < 10,000 or regionally concentrated
- ❌ SLA 99.9% acceptable (multi-AZ sufficient)
- ❌ Cost-sensitive deployment
- ❌ Early-stage product (optimize later)

---

## Architecture Patterns

### Pattern 1: Active-Active (Recommended)

All regions serve traffic simultaneously.

```
                    ┌─────────────────────┐
                    │  Global Load        │
                    │  Balancer           │
                    │  (Route 53 / GLB)   │
                    └──────────┬──────────┘
                               │
          ┌────────────────────┼────────────────────┐
          │                    │                    │
          ▼                    ▼                    ▼
    ┌──────────┐         ┌──────────┐         ┌──────────┐
    │ US-EAST  │         │ EU-WEST  │         │ AP-SOUTH │
    │ Region   │         │ Region   │         │ Region   │
    └──────────┘         └──────────┘         └──────────┘
    3 replicas           3 replicas           3 replicas
```

**Pros**:
- ✅ Highest availability (any region can fail)
- ✅ Lowest latency (always route to nearest)
- ✅ Load distributed globally

**Cons**:
- ❌ Higher cost (all regions always running)
- ❌ More complex monitoring

**When to use**: Production, high-traffic, global user base

---

### Pattern 2: Active-Passive (Simpler)

Primary region serves all traffic, secondary region on standby.

```
                    ┌─────────────────────┐
                    │  DNS Failover       │
                    │  (Route 53)         │
                    └──────────┬──────────┘
                               │
                      ┌────────┴────────┐
                      │                 │
                      ▼                 ▼ (Failover)
                ┌──────────┐      ┌──────────┐
                │ US-EAST  │      │ EU-WEST  │
                │ (Primary)│      │(Standby) │
                └──────────┘      └──────────┘
                3 replicas        1 replica (scaled down)
```

**Pros**:
- ✅ Lower cost (standby scaled down)
- ✅ Simpler to operate
- ✅ Still provides DR

**Cons**:
- ❌ Higher latency (all traffic to primary)
- ❌ Failover delay (DNS propagation)
- ❌ Wasted capacity (standby idle)

**When to use**: Cost-sensitive, regional user base, 99.9% SLA acceptable

---

### Pattern 3: Regional Isolation (Compliance)

Each region independent, no failover.

```
    ┌──────────┐         ┌──────────┐         ┌──────────┐
    │ US-EAST  │         │ EU-WEST  │         │ AP-SOUTH │
    │          │         │          │         │          │
    │ US users │         │ EU users │         │ AP users │
    │ only     │         │ only     │         │ only     │
    └──────────┘         └──────────┘         └──────────┘
```

**Pros**:
- ✅ Compliance (data never leaves region)
- ✅ Simple (no cross-region coordination)

**Cons**:
- ❌ No cross-region failover
- ❌ Regional outage = regional users down

**When to use**: Strict data residency requirements (GDPR, HIPAA)

---

## Region Selection

### Recommended Regions

**Baseline (2 regions)**:
- **US-EAST** (N. Virginia / us-east-1): Largest user base, lowest cost
- **EU-WEST** (Ireland / eu-west-1): GDPR compliance, European users

**Extended (3 regions)**:
- **US-EAST**
- **EU-WEST**
- **AP-SOUTH** (Singapore / ap-southeast-1): Asian users

**Global (4+ regions)**:
- US-EAST, US-WEST, EU-WEST, EU-CENTRAL, AP-SOUTH, AP-NORTHEAST

### Region Selection Criteria

| Criterion | Weight | Notes |
|-----------|--------|-------|
| User proximity | High | Lowest latency to users |
| Compliance | High | GDPR, data residency |
| Cost | Medium | Some regions more expensive |
| Reliability | Medium | Prefer mature regions (avoid new) |
| Feature parity | Low | All major regions support UDP LoadBalancer |

### Cloud Provider Comparison

| Provider | Recommended Regions | Notes |
|----------|---------------------|-------|
| **AWS** | us-east-1, eu-west-1, ap-southeast-1 | Most mature, lowest cost |
| **GCP** | us-east1, europe-west1, asia-southeast1 | Good global network |
| **Azure** | eastus, westeurope, southeastasia | Strong enterprise presence |

---

## Traffic Routing

### Option 1: DNS-Based (Recommended)

**AWS Route 53 Geolocation Routing**:

```yaml
# Route 53 configuration
Type: A
Routing Policy: Geolocation
Records:
  - Location: North America
    Value: us-east-lb.example.com
  - Location: Europe
    Value: eu-west-lb.example.com
  - Location: Asia
    Value: ap-south-lb.example.com
  - Default:
    Value: us-east-lb.example.com  # Fallback
```

**Pros**:
- ✅ Simple (DNS-level)
- ✅ Client-side (no proxy)
- ✅ Low cost

**Cons**:
- ❌ DNS caching (slow failover)
- ❌ Coarse geolocation (country-level)

**TTL recommendation**: 60 seconds (balance between caching and failover speed)

---

### Option 2: Global Load Balancer

**GCP Cloud Load Balancing** or **AWS Global Accelerator**:

```
Client → GLB (Anycast IP) → Nearest Region → Relay Pod
```

**Pros**:
- ✅ Fast failover (< 1 second)
- ✅ Accurate geolocation (IP-based)
- ✅ DDoS protection included

**Cons**:
- ❌ Higher cost ($18-36/month per region)
- ❌ Vendor lock-in (cloud-specific)

**When to use**: High SLA requirements (99.99%+), DDoS risk

---

### Option 3: Anycast (Advanced)

Deploy relay on Anycast IP (e.g., Cloudflare Spectrum).

**Pros**:
- ✅ Instant failover
- ✅ Best latency (BGP routing)
- ✅ DDoS protection

**Cons**:
- ❌ Complex setup (BGP peering)
- ❌ Limited to UDP (Cloudflare Spectrum)
- ❌ Expensive

**When to use**: Very high scale (100k+ concurrent users)

---

## Deployment Procedures

### Step-by-Step Multi-Region Deployment

**Prerequisites**:
- Kubernetes clusters in each region
- DNS domain (e.g., relay.example.com)
- Docker images pushed to registry (accessible from all regions)

---

**Step 1: Deploy to Primary Region**

```bash
# Context: us-east-1
kubectl config use-context us-east-1

# Deploy relay
kubectl apply -f deploy/kubernetes/relay-server-deployment.yaml
kubectl apply -f deploy/kubernetes/relay-server-monitoring.yaml

# Verify
kubectl get pods -n file-relay
./scripts/ops/health-check.sh us-east-lb.example.com
```

---

**Step 2: Deploy to Secondary Region**

```bash
# Context: eu-west-1
kubectl config use-context eu-west-1

# Deploy relay (same manifests)
kubectl apply -f deploy/kubernetes/relay-server-deployment.yaml
kubectl apply -f deploy/kubernetes/relay-server-monitoring.yaml

# Verify
kubectl get pods -n file-relay
./scripts/ops/health-check.sh eu-west-lb.example.com
```

---

**Step 3: Configure Global Load Balancer**

**Option A: AWS Route 53**
```bash
# Create hosted zone (if not exists)
aws route53 create-hosted-zone --name relay.example.com

# Create geolocation records
aws route53 change-resource-record-sets --hosted-zone-id Z1234 --change-batch file://route53-geo.json
```

**route53-geo.json**:
```json
{
  "Changes": [
    {
      "Action": "CREATE",
      "ResourceRecordSet": {
        "Name": "relay.example.com",
        "Type": "A",
        "SetIdentifier": "US",
        "GeoLocation": { "ContinentCode": "NA" },
        "TTL": 60,
        "ResourceRecords": [{ "Value": "1.2.3.4" }]
      }
    },
    {
      "Action": "CREATE",
      "ResourceRecordSet": {
        "Name": "relay.example.com",
        "Type": "A",
        "SetIdentifier": "EU",
        "GeoLocation": { "ContinentCode": "EU" },
        "TTL": 60,
        "ResourceRecords": [{ "Value": "5.6.7.8" }]
      }
    }
  ]
}
```

**Option B: GCP Global Load Balancer**
```bash
# Create backend services (per region)
gcloud compute backend-services create relay-us-east --global --protocol=UDP
gcloud compute backend-services create relay-eu-west --global --protocol=UDP

# Create global forwarding rule
gcloud compute forwarding-rules create relay-global \
  --global \
  --ip-protocol=UDP \
  --ports=8080 \
  --backend-service=relay-us-east

# Add health checks
gcloud compute health-checks create http relay-health \
  --port=8081 \
  --request-path=/health
```

---

**Step 4: Verify Multi-Region Setup**

```bash
# Test DNS resolution from different locations
dig relay.example.com @8.8.8.8  # Should return nearest region IP

# Test connectivity
./scripts/ops/health-check.sh relay.example.com

# Test failover (drain one region)
kubectl drain <node> --ignore-daemonsets -n file-relay
# Verify traffic shifts to other region
```

---

### Deployment Workflow (CI/CD)

**GitHub Actions multi-region deployment**:

```yaml
name: Deploy Multi-Region

on:
  push:
    tags:
      - 'v*'

jobs:
  deploy-us-east:
    runs-on: ubuntu-latest
    steps:
      - name: Deploy to us-east-1
        run: |
          kubectl config use-context us-east-1
          kubectl apply -f deploy/kubernetes/

  deploy-eu-west:
    needs: deploy-us-east  # Sequential deployment
    runs-on: ubuntu-latest
    steps:
      - name: Deploy to eu-west-1
        run: |
          kubectl config use-context eu-west-1
          kubectl apply -f deploy/kubernetes/

  update-dns:
    needs: [deploy-us-east, deploy-eu-west]
    runs-on: ubuntu-latest
    steps:
      - name: Update Route 53 (if IPs changed)
        run: |
          # Update DNS records
```

**Deployment order**: Primary → Secondary → Tertiary (minimize blast radius)

---

## Monitoring Multi-Region

### Metrics Aggregation

**Prometheus Federation** (aggregate metrics from all regions):

```yaml
# prometheus-global.yaml
scrape_configs:
  - job_name: 'federation-us-east'
    honor_labels: true
    metrics_path: '/federate'
    params:
      match[]:
        - '{job="file-relay"}'
    static_configs:
      - targets:
        - 'prometheus-us-east:9090'
        labels:
          region: 'us-east-1'

  - job_name: 'federation-eu-west'
    honor_labels: true
    metrics_path: '/federate'
    params:
      match[]:
        - '{job="file-relay"}'
    static_configs:
      - targets:
        - 'prometheus-eu-west:9090'
        labels:
          region: 'eu-west-1'
```

### Multi-Region Dashboard

**Grafana panels**:
1. **Active peers by region** (stacked area chart)
2. **Packet forwarding rate by region** (line chart)
3. **Regional health status** (stat panels: UP/DOWN per region)
4. **Cross-region latency** (if testing between regions)

**PromQL examples**:
```promql
# Total active peers across all regions
sum(file_relay_active_peers)

# Active peers by region
sum by (region) (file_relay_active_peers)

# Regional availability (% uptime)
avg_over_time(up{job="file-relay"}[24h]) * 100
```

### Regional Alerts

**Per-region alerts** (separate severity):

```yaml
- alert: FileRelayRegionDown
  expr: up{job="file-relay", region="us-east-1"} == 0
  for: 2m
  labels:
    severity: critical
    region: us-east-1
  annotations:
    summary: "US-EAST-1 region completely down"
    runbook_url: "..."

- alert: FileRelayMultiRegionDown
  expr: count(up{job="file-relay"} == 0) > 1
  for: 1m
  labels:
    severity: page  # Immediate escalation
  annotations:
    summary: "Multiple regions down simultaneously"
```

---

## Cost Analysis

### Single-Region vs Multi-Region Cost

**Assumptions**:
- 3 replicas per region
- AWS instance type: t3.medium ($0.0416/hour)
- LoadBalancer: $18/month per region
- Data transfer: $0.09/GB out (inter-region)

| Item | Single-Region | 2-Region | 3-Region |
|------|---------------|----------|----------|
| **Compute** (3 × t3.medium × 730h) | $91/mo | $182/mo | $273/mo |
| **LoadBalancer** | $18/mo | $36/mo | $54/mo |
| **Data transfer** (inter-region) | $0 | ~$20/mo | ~$40/mo |
| **Monitoring** (Prometheus storage) | $10/mo | $20/mo | $30/mo |
| **Total** | **$119/mo** | **$258/mo** (2.2x) | **$397/mo** (3.3x) |

**Cost optimization**:
- Use spot instances (50-70% discount)
- Reserved instances (40% discount, 1-year commit)
- Reduce replicas in low-traffic regions (1-2 instead of 3)

---

## Further Reading

- [Disaster Recovery Guide](DISASTER-RECOVERY.md) — Regional failover procedures
- [Migration Guide](MIGRATION-GUIDE.md) — Upgrading across regions
- [Deployment Guide](DEPLOYMENT-GUIDE.md) — Single-region deployment
- [Cost Optimization](PERFORMANCE-TUNING.md#cost-optimization) — Reducing expenses

---

**Questions?** [Open an issue](https://github.com/file-network/relay-server/issues) or consult the [FAQ](../FAQ.md).
