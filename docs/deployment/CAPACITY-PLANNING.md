# FILE Relay Server - Capacity Planning Guide

Comprehensive guide for sizing FILE Relay Server infrastructure based on traffic requirements.

---

## Table of Contents

1. [Overview](#overview)
2. [Resource Formulas](#resource-formulas)
3. [Sizing Tables](#sizing-tables)
4. [Capacity Models](#capacity-models)
5. [Scaling Strategy](#scaling-strategy)
6. [Capacity Monitoring](#capacity-monitoring)
7. [Growth Planning](#growth-planning)
8. [Capacity Examples](#capacity-examples)

---

## Overview

### Key Concepts

**Capacity planning answers**:
1. How many peers can one relay instance handle?
2. How many instances do I need for N peers?
3. What CPU/memory should I allocate per instance?
4. When should I scale up/out?

**Planning approach**:
1. **Measure**: Baseline performance per instance
2. **Calculate**: Required resources for target load
3. **Buffer**: Add 20-30% headroom
4. **Monitor**: Track usage, adjust as needed

---

## Resource Formulas

### Formula 1: Maximum Peers per Instance

```
Max_Peers = min(
  CPU_Limit / CPU_Per_Peer,
  Memory_Limit / Memory_Per_Peer,
  MAX_PEERS_Config
)

Where (empirically derived):
  CPU_Per_Peer ≈ 0.0008 vCPU (0.8 milliseconds per peer-second)
  Memory_Per_Peer ≈ 512 KB (peer state + buffers)
  MAX_PEERS_Config = 1000 (configurable limit)
```

**Example** (1 vCPU, 512 MB RAM):
```
CPU-based limit:    1.0 vCPU / 0.0008 = 1,250 peers
Memory-based limit: 512 MB / 0.5 MB = 1,024 peers
Config limit:       1,000 peers

→ Max_Peers = min(1250, 1024, 1000) = 1,000 peers
```

**With headroom** (20%):
```
Recommended_Max = Max_Peers × 0.8 = 800 peers
```

---

### Formula 2: Required vCPU per Instance

```
Required_vCPU = Base_CPU + (Target_Peers × CPU_Per_Peer) + Headroom

Where:
  Base_CPU = 0.05 vCPU (baseline overhead)
  CPU_Per_Peer = 0.0008 vCPU
  Headroom = Result × 0.2 (20%)
```

**Example** (1000 peers):
```
Required_vCPU = 0.05 + (1000 × 0.0008) + 0.17
              = 0.05 + 0.8 + 0.17
              = 1.02 vCPU

→ Allocate: 1.2 vCPU (rounded up with buffer)
```

---

### Formula 3: Required Memory per Instance

```
Required_Memory = Base_Memory + (Target_Peers × Memory_Per_Peer) + Headroom

Where:
  Base_Memory = 50 MB (runtime + metrics)
  Memory_Per_Peer = 0.5 MB (512 KB)
  Headroom = Result × 0.2 (20%)
```

**Example** (1000 peers):
```
Required_Memory = 50 MB + (1000 × 0.5 MB) + 110 MB
                = 50 + 500 + 110
                = 660 MB

→ Allocate: 768 MB (rounded up to next power of 2)
```

---

### Formula 4: Required Instances

```
Required_Instances = ceil(Total_Peers / Peers_Per_Instance)

Where:
  Peers_Per_Instance = Recommended_Max (from Formula 1)
  ceil() = Round up to nearest integer
```

**Example** (5000 total peers, 800 per instance):
```
Required_Instances = ceil(5000 / 800) = ceil(6.25) = 7 instances
```

**With HA** (add 1 for redundancy):
```
HA_Instances = Required_Instances + 1 = 8 instances
```

---

### Formula 5: Network Bandwidth

```
Required_Bandwidth = Total_Peers × Packets_Per_Peer × Packet_Size × 8

Where:
  Packets_Per_Peer = User-defined (e.g., 5 packets/second)
  Packet_Size = Average packet size (e.g., 150 bytes)
  × 8 = Convert bytes to bits
```

**Example** (1000 peers, 5 pkt/s, 150 bytes):
```
Required_Bandwidth = 1000 × 5 × 150 × 8
                   = 6,000,000 bits/second
                   = 6 Mbps per instance
```

**With bidirectional** (in + out):
```
Total_Bandwidth = 6 Mbps × 2 = 12 Mbps
```

**Note**: Cloud instance network limits:
- AWS t3.medium: Up to 5 Gbps (burst)
- GCP e2-medium: Up to 10 Gbps
- **Bandwidth is typically NOT a bottleneck** for relay workloads

---

## Sizing Tables

### Table 1: Peers per Instance (by Resources)

| vCPU | Memory | Max Peers | Recommended (80%) |
|------|--------|-----------|-------------------|
| 0.5 | 256 MB | 500 | 400 |
| 1.0 | 512 MB | 1,000 | 800 |
| 1.0 | 1 GB | 1,000 | 800 |
| 2.0 | 2 GB | 2,000 | 1,600 |
| 4.0 | 4 GB | 4,000 | 3,200 |

**Bottleneck**: Usually CPU (memory is sufficient at 512 KB/peer)

---

### Table 2: Required Instances (by Total Peers)

| Total Peers | Instances (@ 800/inst) | HA Instances (+1) |
|-------------|------------------------|-------------------|
| 100 | 1 | 2 |
| 500 | 1 | 2 |
| 1,000 | 2 | 3 |
| 2,000 | 3 | 4 |
| 5,000 | 7 | 8 |
| 10,000 | 13 | 14 |
| 20,000 | 25 | 26 |
| 50,000 | 63 | 64 |
| 100,000 | 125 | 126 |

---

### Table 3: Cloud Instance Types (AWS)

| Instance | vCPU | RAM | Max Peers | Cost/Month | Cost per 1000 Peers |
|----------|------|-----|-----------|------------|---------------------|
| t3.micro | 2 | 1 GB | 1,000 | $7.59 | $7.59 |
| t3.small | 2 | 2 GB | 1,000 | $15.18 | $15.18 |
| t3.medium | 2 | 4 GB | 1,000 | $30.37 | $30.37 |
| t3.large | 2 | 8 GB | 1,000 | $60.74 | $60.74 |
| c5.large | 2 | 4 GB | 1,000 | $62.05 | $62.05 |
| c5.xlarge | 4 | 8 GB | 2,000 | $124.10 | $62.05 |

**Recommendation**: t3.medium (best balance of cost and performance)

---

### Table 4: Cloud Instance Types (GCP)

| Instance | vCPU | RAM | Max Peers | Cost/Month | Cost per 1000 Peers |
|----------|------|-----|-----------|------------|---------------------|
| e2-micro | 0.25 | 1 GB | 250 | $6.11 | $24.44 |
| e2-small | 0.5 | 2 GB | 500 | $12.21 | $24.42 |
| e2-medium | 1 | 4 GB | 1,000 | $24.27 | $24.27 |
| e2-standard-2 | 2 | 8 GB | 2,000 | $48.54 | $24.27 |
| c2-standard-4 | 4 | 16 GB | 4,000 | $144.17 | $36.04 |

**Recommendation**: e2-medium (lowest cost per peer)

**GCP Autopilot**: Pay-per-pod (even more cost-effective)
- 1000 peers @ 1 vCPU, 1 GB RAM: $11/month
- Cost per 1000 peers: **$11** (50% cheaper than e2-medium)

---

### Table 5: Packet Rate Impact

| Packet Rate | CPU Usage | Throughput | Cost Impact |
|------------|-----------|------------|-------------|
| 1 pkt/s | Low (10%) | 1k pkt/s per 1k peers | Baseline |
| 5 pkt/s | Medium (40%) | 5k pkt/s | +30% cost |
| 10 pkt/s | High (70%) | 10k pkt/s | +50% cost |
| 20 pkt/s | Very High (90%) | 20k pkt/s | +70% cost |

**Note**: Higher packet rates require more instances (CPU-bound)

---

## Capacity Models

### Model 1: Fixed Capacity (Over-Provisioned)

**Approach**: Allocate for peak traffic, leave running 24/7.

**Pros**:
- ✅ Simple
- ✅ Predictable performance
- ✅ No scaling delays

**Cons**:
- ❌ Expensive (pay for unused capacity)
- ❌ No elasticity

**When to use**: Stable, predictable traffic with no peaks

**Example** (5000 peers peak):
```
Required: 7 instances (800 peers each)
Running: 7 instances 24/7
Cost: 7 × $30/month = $210/month (AWS t3.medium)
```

---

### Model 2: Auto-Scaling (Elastic)

**Approach**: Start with minimum, scale up during peaks.

**Pros**:
- ✅ Cost-effective (pay for actual usage)
- ✅ Handles traffic spikes
- ✅ No manual intervention

**Cons**:
- ⚠️ Scaling delay (30-60 seconds)
- ⚠️ Requires monitoring

**When to use**: Variable traffic with peaks

**Example** (1000 peers average, 5000 peers peak):
```
Baseline: 2 instances (1000 peers)
Peak: 7 instances (5000 peers)
Average: 3 instances (assuming 4-hour peak daily)

Cost: 3 × $30/month = $90/month (AWS t3.medium)
Savings: $120/month vs fixed (57%)
```

**Configuration** (already in Terraform):
```yaml
HPA:
  minReplicas: 2
  maxReplicas: 10
  targetCPUUtilizationPercentage: 70
```

---

### Model 3: Hybrid (Base + Burst)

**Approach**: Fixed base capacity + auto-scale for bursts.

**Pros**:
- ✅ Balance of cost and performance
- ✅ Handles unexpected spikes
- ✅ Predictable baseline cost

**Cons**:
- ⚠️ Slightly more complex

**When to use**: Predictable base + unpredictable peaks

**Example** (2000 peers base, up to 10000 peak):
```
Base (Reserved Instances): 3 instances
Burst (Spot Instances): 0-10 instances

Cost:
  Base: 3 × $18/month (Reserved, 40% off) = $54/month
  Burst: Avg 2 × $9/month (Spot, 70% off) = $18/month
  Total: $72/month

vs Fixed: $300/month
Savings: $228/month (76%)
```

---

## Scaling Strategy

### Horizontal Scaling (Add Instances)

**When to scale out**:
- CPU usage > 70% for 5+ minutes
- Memory usage > 80% for 5+ minutes
- Active peers > 80% of `MAX_PEERS`

**HPA configuration**:
```yaml
metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
```

**Scaling behavior**:
- Scale up: Fast (add 1-2 pods immediately)
- Scale down: Slow (remove 1 pod every 5 minutes)

---

### Vertical Scaling (Bigger Instances)

**When to scale up**:
- All instances consistently at 70%+ CPU
- Horizontal scaling maxed out (10 instances)
- Cost-inefficient (many small instances)

**Process**:
1. Identify bottleneck (CPU or memory)
2. Double resources (e.g., t3.medium → t3.large)
3. Test with load
4. Monitor for 7 days
5. Adjust if needed

**Trade-off**:
- ✅ Fewer instances (simpler)
- ❌ Higher cost per instance
- ❌ Larger blast radius (if one fails)

**Recommendation**: Horizontal scaling preferred for FILE Relay (stateless)

---

### Geographic Scaling (Multi-Region)

**When to deploy multi-region**:
- Users in multiple continents
- SLA > 99.9% (survive region outage)
- Latency < 50ms required

**Capacity per region**:
```
Per_Region_Capacity = Total_Peers / Num_Regions × 1.5

Where:
  × 1.5 = Buffer for region failure (50% over-capacity)
```

**Example** (10,000 peers, 2 regions):
```
Per_Region_Capacity = 10,000 / 2 × 1.5 = 7,500 peers
Instances per region: ceil(7,500 / 800) = 10 instances

Total: 10 × 2 = 20 instances
vs Single region: 13 instances
Overhead: 54% (for HA)
```

---

## Capacity Monitoring

### Key Metrics

**1. Peer Utilization**:
```promql
# Current utilization
(file_relay_active_peers / MAX_PEERS) * 100

# Alert when > 80%
file_relay_active_peers > 800
```

**2. CPU Utilization**:
```promql
# Per-pod CPU
rate(container_cpu_usage_seconds_total{pod=~"file-relay.*"}[5m])

# Cluster CPU
avg(rate(node_cpu_seconds_total{mode!="idle"}[5m])) * 100
```

**3. Memory Utilization**:
```promql
# Per-pod memory
container_memory_usage_bytes{pod=~"file-relay.*"}

# Memory pressure
(container_memory_usage_bytes / container_spec_memory_limit_bytes) * 100
```

**4. Packet Throughput**:
```promql
# Packets per second
rate(file_relay_packets_forwarded_total[5m])

# Packets per peer
rate(file_relay_packets_forwarded_total[5m]) / file_relay_active_peers
```

### Capacity Alerts

```yaml
- alert: CapacityWarning
  expr: file_relay_active_peers > 800
  for: 10m
  labels:
    severity: warning
  annotations:
    summary: "Peer capacity at 80%"

- alert: CapacityCritical
  expr: file_relay_active_peers > 950
  for: 5m
  labels:
    severity: critical
  annotations:
    summary: "Peer capacity at 95%, scale immediately"
```

### Capacity Dashboard (Grafana)

**Panels**:
1. **Active Peers** (gauge): Current / Max
2. **CPU Usage** (graph): Per-pod, cluster average
3. **Memory Usage** (graph): Per-pod, cluster average
4. **Packet Rate** (graph): Throughput over time
5. **Capacity Forecast** (graph): Projected growth

---

## Growth Planning

### Forecasting Formula

```
Future_Peers = Current_Peers × (1 + Growth_Rate) ^ Months

Where:
  Growth_Rate = Monthly growth rate (e.g., 0.10 = 10% per month)
  Months = Planning horizon
```

**Example** (1000 peers today, 15% monthly growth):
```
Month 1:  1,000 × (1.15)^1 = 1,150 peers
Month 3:  1,000 × (1.15)^3 = 1,521 peers
Month 6:  1,000 × (1.15)^6 = 2,313 peers
Month 12: 1,000 × (1.15)^12 = 5,350 peers
```

### Capacity Planning Horizon

**Recommendation**: Plan for 6-month capacity needs

**Process**:
1. Calculate 6-month forecast
2. Size infrastructure for forecast + 20% buffer
3. Review monthly, adjust forecast
4. Provision additional capacity 2 months ahead

**Example**:
- Today: 2,000 peers (3 instances)
- 6-month forecast: 8,000 peers
- Required: ceil(8,000 / 800) = 10 instances
- With buffer: 12 instances
- **Action**: Ensure HPA `maxReplicas: 12`

---

## Capacity Examples

### Example 1: Startup (0 → 1000 Peers)

**Phase 1: Launch** (0-100 peers, Month 1)
```
Instances: 1 (t3.micro)
Cost: $7.59/month
CPU: ~10%
Memory: ~100 MB
```

**Phase 2: Growth** (100-500 peers, Months 2-3)
```
Instances: 1 (t3.small)
Cost: $15.18/month
CPU: ~40%
Memory: ~300 MB
```

**Phase 3: Scale** (500-1000 peers, Months 4-6)
```
Instances: 2 (t3.medium)
Cost: $60.74/month
CPU: ~70% per instance
Memory: ~600 MB per instance
```

**Total cost (6 months)**:
```
Month 1: $7.59
Months 2-3: $15.18 × 2 = $30.36
Months 4-6: $60.74 × 3 = $182.22
─────────────────────────────
Total: $220.17 over 6 months
Avg: $36.70/month
```

---

### Example 2: Mid-Size (5000 Peers, Steady State)

**Configuration**:
```
Instances: 7 (t3.medium)
Cost: 7 × $30.37 = $212.59/month (AWS)
     or
     7 × $3.69 = $25.83/month (GCP Autopilot)

CPU: ~70% per instance
Memory: ~650 MB per instance
Peers per instance: ~715
```

**With optimization** (AWS):
```
Instance: t3.small (sufficient for 5k peers)
Instances: 7
Cost (30% Spot): 7 × $10.63 = $74.41/month
Savings: $138.18/month (65%)
```

---

### Example 3: Enterprise (100,000 Peers, Multi-Region)

**Configuration** (2 regions):
```
Total instances: 126 (63 per region)
Instance type: t3.medium
Cost per region: 63 × $30.37 = $1,913.31/month
Total cost: $3,826.62/month (AWS)

With optimization:
  - 70% On-Demand: $2,678.63
  - 30% Spot: $343.99
  Total: $3,022.62/month
  Savings: $804/month (21%)
```

**GCP Autopilot**:
```
Total instances: 126
Cost: 126 × $3.69 = $464.94/month
Savings: $3,361.68/month (88% vs AWS)
```

---

## Capacity Planning Checklist

### Initial Planning

- [ ] Estimate peak concurrent peers (next 6 months)
- [ ] Calculate required instances (Formula 4)
- [ ] Size CPU/memory per instance (Formulas 2-3)
- [ ] Choose cloud provider (cost optimization)
- [ ] Configure auto-scaling (HPA)
- [ ] Set capacity alerts (80% warning, 95% critical)

### Monthly Review

- [ ] Review actual vs forecast
- [ ] Update growth rate
- [ ] Check resource utilization (70%+ = scale)
- [ ] Verify auto-scaling working
- [ ] Review costs vs budget
- [ ] Adjust capacity plan if needed

### Quarterly Review

- [ ] Comprehensive capacity audit
- [ ] Load testing (validate capacity)
- [ ] Review scaling strategy (horizontal vs vertical)
- [ ] Evaluate multi-region need
- [ ] Update 6-month forecast

---

## Further Reading

- [Cost Optimization Guide](COST-OPTIMIZATION-GUIDE.md) — Minimize costs
- [Performance Tuning](PERFORMANCE-TUNING.md) — Optimize per-instance performance
- [Load Testing](../../testing/load/README.md) — Validate capacity
- [Multi-Region Guide](MULTI-REGION-GUIDE.md) — Geographic scaling
- [Observability Guide](OBSERVABILITY-GUIDE.md) — Monitor capacity

---

**Questions?** [Open an issue](https://github.com/file-network/relay-server/issues) or consult the [FAQ](../FAQ.md).
