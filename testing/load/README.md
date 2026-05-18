# FILE Relay Server - Load Testing and Performance Benchmarking

Comprehensive load testing framework for validating performance, capacity, and reliability under realistic conditions.

---

## Table of Contents

1. [Overview](#overview)
2. [Quick Start](#quick-start)
3. [Test Scenarios](#test-scenarios)
4. [Understanding Results](#understanding-results)
5. [Performance Baselines](#performance-baselines)
6. [Bottleneck Identification](#bottleneck-identification)
7. [Capacity Planning](#capacity-planning)
8. [Regression Testing](#regression-testing)
9. [Advanced Usage](#advanced-usage)

---

## Overview

The load testing framework simulates realistic peer behavior including:
- **Peer registration** with CAPoW proof generation
- **Packet forwarding** through the relay server
- **Heartbeat maintenance** (keep-alive)
- **Peer churn** (joining and leaving)
- **Metrics collection** from relay server

**Tool**: `relay_load_test.py` — Python-based UDP load generator

---

## Quick Start

### Installation

```bash
cd testing/load

# Install dependencies
pip install -r requirements.txt

# Make executable
chmod +x relay_load_test.py
```

### Basic Test

```bash
# Test local relay (assumes running on localhost:8080)
./relay_load_test.py \
  --host localhost \
  --peers 10 \
  --duration 30

# Test remote relay
./relay_load_test.py \
  --host relay.example.com \
  --peers 100 \
  --duration 60 \
  --packet-rate 5.0
```

### Example Output

```
Starting load test:
  Target: localhost:8080
  Peers: 100
  Duration: 60s
  Packet rate: 5.0/s per peer
  Churn rate: 0.0/s

Phase 1: Registering peers...
  Registered: 10/100
  Registered: 20/100
  ...
  Success: 100/100

Phase 2: Sending packets...

Phase 3: Collecting final metrics...

======================================================================
LOAD TEST RESULTS
======================================================================

Duration: 60.12s

PEER METRICS:
  Registered:     100
  Rejected:       0
  Success rate:   100.0%

PACKET METRICS:
  Sent:           30000
  Relayed:        29987
  Lost:           13
  Delivery rate:  99.96%

THROUGHPUT:
  Packets/sec:    499.00
  Bytes/sec:      73.08 KB/s

SERVER METRICS:
  Active peers:   100
  Packets fwd:    29987
  CAPoW rejected: 0
  Rate limited:   0

======================================================================
```

---

## Test Scenarios

### 1. Smoke Test (Quick Validation)

**Purpose**: Verify basic functionality after deployment or code changes.

```bash
./relay_load_test.py \
  --host localhost \
  --peers 10 \
  --duration 30 \
  --packet-rate 1.0
```

**Expected**:
- Registration success rate: 100%
- Packet delivery rate: > 99%
- Duration: ~30 seconds
- No errors

**When to run**: After every deployment, before running longer tests.

---

### 2. Capacity Test (Find Maximum Peers)

**Purpose**: Determine maximum number of simultaneous peers a single relay instance can handle.

```bash
# Start low
./relay_load_test.py --host localhost --peers 100 --duration 60

# Increase gradually
./relay_load_test.py --host localhost --peers 500 --duration 60
./relay_load_test.py --host localhost --peers 1000 --duration 60
./relay_load_test.py --host localhost --peers 1500 --duration 60

# Stop when delivery rate < 95%
```

**Expected baseline** (1 vCPU, 512 MB RAM):
- Maximum peers: 1000-1200
- Packet delivery rate: > 95% at capacity
- CPU usage: < 80% at capacity

**When to run**: Before production deployment, after capacity planning changes.

---

### 3. Throughput Test (Maximum Packets/Second)

**Purpose**: Measure maximum packet forwarding throughput.

```bash
./relay_load_test.py \
  --host localhost \
  --peers 100 \
  --duration 60 \
  --packet-rate 10.0  # High packet rate
```

**Expected baseline** (1 vCPU):
- Throughput: 5,000-10,000 packets/second
- Latency p95: < 50ms
- CPU usage: 70-90%

**When to run**: Performance regression testing, after optimization work.

---

### 4. Churn Test (Peer Joining/Leaving)

**Purpose**: Test stability under high peer churn (users joining and leaving frequently).

```bash
./relay_load_test.py \
  --host localhost \
  --peers 100 \
  --duration 120 \
  --churn-rate 2.0  # 2 peers join/leave per second
```

**Expected**:
- Active peer count: fluctuates around 100
- Packet delivery rate: > 95%
- No crashes or memory leaks

**When to run**: Before production deployment, after connection management changes.

---

### 5. Stress Test (Overload)

**Purpose**: Test behavior under sustained overload conditions.

```bash
./relay_load_test.py \
  --host localhost \
  --peers 2000 \   # 2x capacity
  --duration 300 \  # 5 minutes
  --packet-rate 10.0
```

**Expected**:
- Some peers rejected (rate limiting kicks in)
- Packet delivery rate: > 90% for registered peers
- Server remains stable (no crash, no OOM)
- Graceful degradation (no cascading failure)

**When to run**: Before production deployment, after DoS protection changes.

---

### 6. Soak Test (Long Duration)

**Purpose**: Detect memory leaks, resource exhaustion, and gradual performance degradation.

```bash
./relay_load_test.py \
  --host localhost \
  --peers 500 \
  --duration 3600 \  # 1 hour
  --packet-rate 5.0 \
  --churn-rate 1.0
```

**Expected**:
- Memory usage: stable (no growth over time)
- Packet delivery rate: consistent (no degradation)
- CPU usage: stable

**When to run**: Before production deployment, quarterly as part of release validation.

---

### 7. Baseline Benchmark (Regression Detection)

**Purpose**: Establish performance baseline for detecting regressions.

```bash
# Run standardized test
./relay_load_test.py \
  --host localhost \
  --peers 500 \
  --duration 120 \
  --packet-rate 5.0 \
  --seed 42 \  # Reproducible
  --output baseline-v0.1.0.json

# Compare future versions against this baseline
```

**When to run**: After every release, store results for comparison.

---

## Understanding Results

### Key Metrics

#### 1. Registration Success Rate

```
Registration Success Rate = (Peers Registered / Total Attempts) × 100%
```

**Good**: > 99%  
**Acceptable**: 95-99%  
**Bad**: < 95%

**Troubleshooting**:
- < 95%: Check CAPoW difficulty (too high?), rate limiting (too strict?), server capacity
- 0%: Check network connectivity, firewall rules, server running?

#### 2. Packet Delivery Rate

```
Packet Delivery Rate = (Packets Relayed / Packets Sent) × 100%
```

**Good**: > 99%  
**Acceptable**: 95-99%  
**Bad**: < 95%

**Troubleshooting**:
- < 95%: Check CPU usage (overloaded?), memory (exhausted?), network (congested?)
- Gradual decline: Memory leak? Connection leak?

#### 3. Throughput (Packets/Second)

```
Throughput = Total Packets Sent / Test Duration
```

**Baseline** (1 vCPU, 512 MB RAM):
- Light load (100 peers, 1 pkt/s): ~100 pkt/s
- Medium load (500 peers, 5 pkt/s): ~2,500 pkt/s
- Heavy load (1000 peers, 10 pkt/s): ~10,000 pkt/s

**Troubleshooting**:
- Below baseline: CPU bottleneck? Check CPU usage in Prometheus
- Highly variable: Network jitter? Run test multiple times

#### 4. Latency (Packet Round-Trip Time)

**Note**: Current tool does not measure end-to-end latency (requires relay server to echo packets back). Planned for future version.

**Expected baseline**:
- p50: < 10ms
- p95: < 20ms
- p99: < 50ms

---

## Performance Baselines

### Single Instance (1 vCPU, 512 MB RAM)

| Metric | Light Load | Medium Load | Heavy Load | Overload |
|--------|-----------|-------------|------------|----------|
| **Peers** | 100 | 500 | 1000 | 2000 |
| **Packet Rate** | 1/s | 5/s | 10/s | 10/s |
| **Total Throughput** | 100 pkt/s | 2,500 pkt/s | 10,000 pkt/s | 20,000 pkt/s |
| **CPU Usage** | 5-10% | 30-40% | 70-80% | 90-100% |
| **Memory Usage** | 128 MB | 256 MB | 384 MB | 512 MB |
| **Packet Delivery** | > 99.9% | > 99.5% | > 99% | > 95% |
| **Registration Success** | 100% | 100% | > 99% | > 90% |

### Scaled Deployment (3 instances behind load balancer)

| Metric | Light Load | Medium Load | Heavy Load |
|--------|-----------|-------------|------------|
| **Total Peers** | 300 | 1,500 | 3,000 |
| **Per-Instance Peers** | 100 | 500 | 1,000 |
| **Total Throughput** | 300 pkt/s | 7,500 pkt/s | 30,000 pkt/s |
| **Packet Delivery** | > 99.9% | > 99.5% | > 99% |

---

## Bottleneck Identification

### 1. CPU Bottleneck

**Symptoms**:
- CPU usage > 90%
- Packet delivery rate dropping
- Latency increasing

**Diagnosis**:
```bash
# During load test, check CPU
kubectl top pods -n file-relay

# If CPU at limit, scale horizontally
kubectl scale deployment file-relay-server --replicas=5 -n file-relay
```

**Solutions**:
- Vertical scaling: Increase CPU requests/limits
- Horizontal scaling: Add more replicas (HPA does this automatically)
- Optimization: Profile code, optimize hot paths

### 2. Memory Bottleneck

**Symptoms**:
- Memory usage climbing over time
- OOMKilled pod restarts
- Packet delivery rate declining gradually

**Diagnosis**:
```bash
# Check memory usage
kubectl top pods -n file-relay

# Check for memory leaks
# Run soak test (1 hour), watch memory grow
./relay_load_test.py --host relay.example.com --peers 500 --duration 3600
```

**Solutions**:
- If memory stable: Increase memory limits
- If memory growing: Fix memory leak in code
- Connection leak: Check peer cleanup (5-minute timeout working?)

### 3. Network Bottleneck

**Symptoms**:
- Low CPU and memory, but poor delivery rate
- Network errors in logs
- Packet loss at network layer

**Diagnosis**:
```bash
# Check network errors on pod
kubectl exec -n file-relay <pod-name> -- netstat -s | grep error

# Check UDP buffer sizes
kubectl exec -n file-relay <pod-name> -- sysctl net.core.rmem_max
```

**Solutions**:
- Increase UDP buffer sizes (see `docs/deployment/PERFORMANCE-TUNING.md`)
- Check network CNI plugin (Calico, Cilium) configuration
- Check cloud provider network limits (AWS: network baseline vs burst)

### 4. CAPoW Bottleneck

**Symptoms**:
- Long registration times (> 1 second)
- High CAPoW rejection rate during normal load
- CPU pegged during registration phase

**Diagnosis**:
```bash
# Check CAPoW rejection rate
curl http://relay.example.com:8081/metrics | grep capow_rejected

# If rejection rate > 1% during normal operation, difficulty too high
```

**Solutions**:
- Reduce CAPoW difficulty (currently 16 bits, try 12-14)
- Increase CAPoW worker pool size
- Optimize Argon2id parameters

---

## Capacity Planning

### Formula: Peers per Instance

```
Max Peers = (Available CPU × 1000) / CPU_per_peer

Where:
  CPU_per_peer ≈ 0.8 milliseconds (800 µs) per peer-second
  Available CPU = Total CPU × 0.7 (leave 30% headroom)
```

**Example** (1 vCPU):
```
Max Peers = (1.0 × 0.7 × 1000) / 0.8 = 875 peers
```

**Recommendation**: Set `MAX_PEERS=800` (with 10% buffer)

### Formula: Required Instances

```
Required Instances = ceil(Total Peers / Max Peers per Instance)
```

**Example** (10,000 users):
```
Required Instances = ceil(10,000 / 800) = 13 instances
```

**Recommendation**: Deploy 15 instances (2 extra for redundancy)

### Cost Estimation

**AWS** (t3.medium, $0.0416/hour):
```
Cost = Instances × $30/month
     = 15 × $30
     = $450/month (for 10,000 concurrent peers)
```

**GCP** (e2-medium, ~$25/month):
```
Cost = Instances × $25/month
     = 15 × $25
     = $375/month
```

---

## Regression Testing

### Establishing Baseline

1. **Run standardized test** (reproducible seed):
```bash
./relay_load_test.py \
  --host relay.example.com \
  --peers 500 \
  --duration 120 \
  --packet-rate 5.0 \
  --seed 42 \
  --output baseline-v0.1.0.json
```

2. **Store results** in version control:
```bash
git add testing/load/baselines/baseline-v0.1.0.json
git commit -m "Add performance baseline for v0.1.0"
```

### Detecting Regressions

1. **Run same test** on new version:
```bash
./relay_load_test.py \
  --host relay.example.com \
  --peers 500 \
  --duration 120 \
  --packet-rate 5.0 \
  --seed 42 \
  --output baseline-v0.2.0.json
```

2. **Compare results**:
```bash
# Manual comparison
diff <(jq . baseline-v0.1.0.json) <(jq . baseline-v0.2.0.json)

# Or use comparison script (see below)
./compare_baselines.py baseline-v0.1.0.json baseline-v0.2.0.json
```

3. **Regression criteria**:
- Packet delivery rate: -2% = warning, -5% = fail
- Throughput: -10% = warning, -20% = fail
- Registration success: -1% = warning, -5% = fail

### CI/CD Integration

```yaml
# .github/workflows/performance-test.yml
name: Performance Test

on:
  pull_request:
    branches: [main]

jobs:
  perf-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Deploy test relay
        run: |
          docker run -d -p 8080:8080 -p 8081:8081 \
            ghcr.io/file-network/relay-server:${{ github.sha }}

      - name: Run load test
        run: |
          pip install -r testing/load/requirements.txt
          ./testing/load/relay_load_test.py \
            --host localhost \
            --peers 100 \
            --duration 60 \
            --seed 42 \
            --output results-pr-${{ github.event.pull_request.number }}.json

      - name: Compare against baseline
        run: |
          ./testing/load/compare_baselines.py \
            testing/load/baselines/baseline-main.json \
            results-pr-${{ github.event.pull_request.number }}.json \
            --fail-on-regression
```

---

## Advanced Usage

### Custom Packet Payloads

Modify `relay_load_test.py` to send custom payloads:

```python
# In send_packets() method:
payload = b'CUSTOM_DATA_HERE'
await sender.send_packet(receiver.peer_id, payload)
```

### Distributed Load Generation

Run multiple load generators against same relay:

```bash
# Machine 1
./relay_load_test.py --host relay.example.com --peers 500 --seed 1

# Machine 2
./relay_load_test.py --host relay.example.com --peers 500 --seed 2

# Machine 3
./relay_load_test.py --host relay.example.com --peers 500 --seed 3

# Total: 1500 peers
```

### Continuous Load (Background Testing)

```bash
# Run indefinitely
while true; do
  ./relay_load_test.py \
    --host relay.example.com \
    --peers 100 \
    --duration 300 \
    --output results-$(date +%s).json
  sleep 60
done
```

### Prometheus Integration

Export results to Prometheus:

```python
# Add to relay_load_test.py:
from prometheus_client import CollectorRegistry, Gauge, push_to_gateway

registry = CollectorRegistry()
g = Gauge('load_test_packet_delivery_rate', 'Packet delivery rate %', registry=registry)
g.set(results.packet_delivery_rate)

push_to_gateway('localhost:9091', job='load_test', registry=registry)
```

---

## Further Reading

- [Performance Tuning Guide](../../docs/deployment/PERFORMANCE-TUNING.md) — OS and application tuning
- [Capacity Planning](../../docs/deployment/CAPACITY-PLANNING.md) — Detailed capacity formulas
- [Operations Runbook](../../docs/deployment/OPERATIONS-RUNBOOK.md) — Production operations
- [Observability Guide](../../docs/deployment/OBSERVABILITY-GUIDE.md) — Metrics interpretation

---

**Questions?** [Open an issue](https://github.com/file-network/relay-server/issues) or consult the [FAQ](../../docs/FAQ.md).
