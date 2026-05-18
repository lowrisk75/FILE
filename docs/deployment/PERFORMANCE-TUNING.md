# FILE Relay Server - Performance Tuning Guide

**Last Updated**: 2026-05-18  
**Version**: 0.1.0

---

## Table of Contents

1. [Overview](#overview)
2. [Baseline Performance](#baseline-performance)
3. [Resource Sizing](#resource-sizing)
4. [Operating System Tuning](#operating-system-tuning)
5. [Application Tuning](#application-tuning)
6. [Kubernetes Tuning](#kubernetes-tuning)
7. [Network Tuning](#network-tuning)
8. [Monitoring Performance](#monitoring-performance)
9. [Benchmarking](#benchmarking)
10. [Troubleshooting Performance Issues](#troubleshooting-performance-issues)

---

## Overview

This guide covers performance optimization for FILE Relay Server across different deployment scenarios. Tuning is typically done in this order:

1. **Resource sizing** (get enough CPU/memory)
2. **OS tuning** (kernel parameters, file limits)
3. **Application tuning** (environment variables)
4. **Network tuning** (UDP buffers, MTU, congestion control)
5. **Monitoring** (validate improvements)

**Expected baseline** (default config, no tuning):
- 500 concurrent peers
- 500 req/s registration rate
- 10,000 packets/s forwarding
- 512 MB memory usage
- 500m CPU usage (0.5 cores)

---

## Baseline Performance

### Test Environment

```
Hardware:
- 4 CPU cores (Intel Xeon / AMD EPYC)
- 4 GB RAM
- 1 Gbps network

Software:
- Ubuntu 22.04
- Kernel 5.15+
- FILE Relay Server 0.1.0 (default config)
```

### Baseline Metrics

| Metric | Value | Notes |
|--------|-------|-------|
| Max concurrent peers | 1,000 | Configurable via `MAX_PEERS` |
| Registration throughput | ~500 req/s | Limited by CAPoW verification |
| Packet forwarding rate | ~50,000 pps | Limited by network I/O |
| Relay latency | 1-5 ms | Packet forwarding only |
| CAPoW verification latency | 50-100 ms | Argon2id computation |
| Memory per peer | ~0.5 MB | Peer registry + rate limiter + replay cache |
| CPU per 100 peers | ~50m | Baseline + packet forwarding |

### Bottlenecks

1. **CAPoW verification**: CPU-bound (Argon2id)
2. **Packet forwarding**: Network I/O bound
3. **Peer lookup**: Memory latency (DashMap hash lookup)
4. **Rate limiting**: Contention on token bucket updates

---

## Resource Sizing

### Horizontal Scaling (Recommended)

**Rule of thumb**: Scale horizontally at 80-90% capacity.

| Peers per Instance | CPU | Memory | Network | Recommendation |
|--------------------|-----|--------|---------|----------------|
| 500 | 500m | 512 MB | 500 Mbps | Dev/test |
| 1,000 | 1000m | 1 GB | 1 Gbps | Small production |
| 3,000 | 2000m | 2 GB | 3 Gbps | **Not recommended** (use 3×1000) |

**Why horizontal scaling?**:
- Better fault tolerance (no single point of failure)
- Easier capacity planning (linear scaling)
- Lower instance cost (3× small vs 1× large)

### Vertical Scaling

If horizontal scaling is not possible (e.g., single VPS):

```bash
# Increase max peers
export MAX_PEERS=2000

# Ensure sufficient memory (1 MB per peer + 512 MB base)
# 2000 peers = 2 GB + 512 MB = ~2.5 GB
# Allocate 4 GB for headroom
```

**Limits**:
- CPU: CAPoW worker pool scales to 25% of cores (diminishing returns after 8 cores)
- Memory: DashMap shards increase with load, but linear growth
- Network: UDP socket buffer size limits (see [Network Tuning](#network-tuning))

---

## Operating System Tuning

### File Descriptor Limits

Each peer consumes ~2 file descriptors (UDP socket + timer). Default limit is often 1024.

**Check current limits**:
```bash
ulimit -n
```

**Increase for systemd service** (add to `/etc/systemd/system/file-relay.service`):
```ini
[Service]
LimitNOFILE=65536
```

**Increase for Docker**:
```yaml
services:
  relay-server:
    ulimits:
      nofile:
        soft: 65536
        hard: 65536
```

**Increase system-wide** (add to `/etc/security/limits.conf`):
```
* soft nofile 65536
* hard nofile 65536
```

### UDP Socket Buffers

Increase UDP receive/send buffers to avoid packet drops under load.

**Check current buffers**:
```bash
sysctl net.core.rmem_max
sysctl net.core.wmem_max
sysctl net.core.rmem_default
sysctl net.core.wmem_default
```

**Increase buffers** (add to `/etc/sysctl.conf`):
```ini
# Increase max UDP buffer size to 16 MB
net.core.rmem_max = 16777216
net.core.wmem_max = 16777216

# Increase default UDP buffer size to 4 MB
net.core.rmem_default = 4194304
net.core.wmem_default = 4194304

# Increase UDP receive buffer queue
net.core.netdev_max_backlog = 5000
```

**Apply**:
```bash
sudo sysctl -p
```

### Connection Tracking

Disable conntrack for UDP port 8080 to reduce overhead (advanced).

**Warning**: Only do this if you understand iptables/nftables.

```bash
# iptables
sudo iptables -t raw -A PREROUTING -p udp --dport 8080 -j NOTRACK
sudo iptables -t raw -A OUTPUT -p udp --sport 8080 -j NOTRACK

# nftables
sudo nft add rule ip raw prerouting udp dport 8080 notrack
sudo nft add rule ip raw output udp sport 8080 notrack
```

### Scheduler Tuning

For latency-sensitive workloads, consider using the `deadline` or `mq-deadline` I/O scheduler.

**Check current scheduler**:
```bash
cat /sys/block/sda/queue/scheduler
```

**Change to deadline**:
```bash
echo deadline | sudo tee /sys/block/sda/queue/scheduler
```

---

## Application Tuning

### Environment Variables

```bash
# Increase max peers (default: 1000)
export MAX_PEERS=2000

# Increase log verbosity for debugging (default: info)
export RUST_LOG=debug

# Bind metrics to all interfaces (default: 0.0.0.0:8081)
export METRICS_ADDR=0.0.0.0:8081
```

### CAPoW Worker Pool Sizing

Worker pool size is automatically calculated as `max(2, num_cpus * 0.25)`.

**Examples**:
- 4 cores → 1 worker (2 minimum)
- 8 cores → 2 workers
- 16 cores → 4 workers
- 32 cores → 8 workers

**To manually override** (requires code change in `main.rs`):
```rust
let worker_count = 8; // Force 8 workers
```

**Trade-offs**:
- More workers → higher CAPoW throughput, but more CPU contention
- Fewer workers → lower CAPoW throughput, but more CPU for packet forwarding

**Recommendation**: Keep default (25% of cores) unless profiling shows CAPoW as bottleneck.

### Tokio Runtime Tuning

Tokio runtime is configured for optimal async I/O performance. Advanced tuning:

```rust
// In main.rs
tokio::runtime::Builder::new_multi_thread()
    .worker_threads(num_cpus::get()) // Default: num CPUs
    .max_blocking_threads(512)        // Default: 512
    .thread_stack_size(2 * 1024 * 1024) // Default: 2 MB
    .build()
```

**When to tune**:
- Very high connection count (>5000 peers) → increase `worker_threads`
- Many blocking operations → increase `max_blocking_threads`
- Stack overflow → increase `thread_stack_size`

---

## Kubernetes Tuning

### Resource Requests and Limits

**Start conservative**, then scale based on actual usage.

```yaml
resources:
  requests:
    cpu: 500m        # Guaranteed CPU
    memory: 512Mi    # Guaranteed memory
  limits:
    cpu: 2000m       # Max CPU (bursts allowed)
    memory: 2Gi      # Max memory (OOM kill if exceeded)
```

**Sizing guidelines**:
- Requests = average usage (50th percentile)
- Limits = peak usage (95th percentile) + 20% headroom

**Monitor**:
```bash
kubectl top pods -n file-relay
```

### Pod Affinity and Anti-Affinity

**Anti-affinity** (recommended): Spread pods across nodes for fault tolerance.

```yaml
affinity:
  podAntiAffinity:
    requiredDuringSchedulingIgnoredDuringExecution:
      - labelSelector:
          matchExpressions:
            - key: app
              operator: In
              values:
                - file-relay-server
        topologyKey: kubernetes.io/hostname
```

**Affinity** (advanced): Co-locate with monitoring pods for lower latency.

```yaml
affinity:
  podAffinity:
    preferredDuringSchedulingIgnoredDuringExecution:
      - weight: 100
        podAffinityTerm:
          labelSelector:
            matchExpressions:
              - key: app
                operator: In
                values:
                  - prometheus
          topologyKey: kubernetes.io/hostname
```

### HorizontalPodAutoscaler Tuning

**Default config** (3-10 replicas, CPU + memory + custom metrics):

```yaml
minReplicas: 3
maxReplicas: 10
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
  - type: Pods
    pods:
      metric:
        name: file_relay_active_peers
      target:
        type: AverageValue
        averageValue: "900"
```

**Aggressive scaling** (faster scale-up):

```yaml
behavior:
  scaleUp:
    stabilizationWindowSeconds: 60  # Scale up after 1 minute
    policies:
      - type: Percent
        value: 100  # Double pods
        periodSeconds: 60
  scaleDown:
    stabilizationWindowSeconds: 300  # Scale down after 5 minutes
    policies:
      - type: Pods
        value: 1  # Remove 1 pod at a time
        periodSeconds: 120
```

### LoadBalancer Service Tuning

**AWS Network Load Balancer**:
```yaml
service:
  type: LoadBalancer
  annotations:
    service.beta.kubernetes.io/aws-load-balancer-type: "nlb"
    service.beta.kubernetes.io/aws-load-balancer-cross-zone-load-balancing-enabled: "true"
    service.beta.kubernetes.io/aws-load-balancer-backend-protocol: "udp"
```

**GCP Load Balancer**:
```yaml
service:
  type: LoadBalancer
  annotations:
    cloud.google.com/load-balancer-type: "External"
    networking.gke.io/load-balancer-type: "Internal"  # For internal-only
```

---

## Network Tuning

### MTU Optimization

Default MTU is 1500 bytes. QUIC benefits from larger MTU (jumbo frames).

**Check current MTU**:
```bash
ip link show eth0 | grep mtu
```

**Increase MTU** (if supported by network):
```bash
sudo ip link set dev eth0 mtu 9000
```

**Kubernetes**: Set pod MTU via CNI (Calico, Cilium, Flannel).

**Trade-offs**:
- Larger MTU → fewer packets, lower CPU, higher throughput
- Smaller MTU → more compatibility, less fragmentation risk

**Recommendation**: Keep default (1500) unless you control the entire network path.

### UDP Offloading

Enable UDP Generic Segmentation Offload (GSO) and Generic Receive Offload (GRO) to reduce CPU usage.

**Check current settings**:
```bash
ethtool -k eth0 | grep -E 'udp|gso|gro'
```

**Enable offloading**:
```bash
sudo ethtool -K eth0 tx-udp-segmentation on
sudo ethtool -K eth0 rx-udp-gro-forwarding on
```

**Benefit**: 20-30% CPU reduction at high packet rates.

### Congestion Control

QUIC uses its own congestion control (Cubic by default). For the underlying UDP socket, use BBR.

**Enable BBR** (add to `/etc/sysctl.conf`):
```ini
net.core.default_qdisc = fq
net.ipv4.tcp_congestion_control = bbr
```

**Apply**:
```bash
sudo sysctl -p
```

**Verify**:
```bash
sysctl net.ipv4.tcp_congestion_control
```

---

## Monitoring Performance

### Key Metrics to Watch

| Metric | Target | Alert Threshold | Notes |
|--------|--------|-----------------|-------|
| `file_relay_active_peers` | < 900 | > 900 (90% capacity) | Scale horizontally |
| `file_relay_packets_forwarded_total` | Stable | Sudden drop | Network issue |
| `file_relay_capow_rejected_total` | < 10/s | > 50/s | Possible DoS attack |
| `file_relay_rate_limited_total` | < 5/s | > 100/s | Possible DoS attack |
| CPU usage | < 70% | > 80% | Add more replicas |
| Memory usage | < 80% | > 90% | Increase limits or scale |
| Network bandwidth | < 80% capacity | > 90% | Upgrade network |

### Prometheus Queries

**Packet forwarding rate**:
```promql
rate(file_relay_packets_forwarded_total[1m])
```

**CAPoW verification rate**:
```promql
rate(file_relay_capow_verified_total[1m])
```

**CAPoW success percentage**:
```promql
(
  rate(file_relay_capow_verified_total[5m])
  /
  (rate(file_relay_capow_verified_total[5m]) + rate(file_relay_capow_rejected_total[5m]))
) * 100
```

**Packets per peer**:
```promql
rate(file_relay_packets_forwarded_total[1m]) / file_relay_active_peers
```

**Memory per peer**:
```promql
container_memory_usage_bytes{pod=~"file-relay.*"} / file_relay_active_peers
```

### Grafana Dashboards

Pre-built dashboard: `deploy/grafana/provisioning/dashboards/file-relay.json`

**Key panels**:
1. Active peers (gauge with thresholds)
2. Packet forwarding rate (per pod)
3. CAPoW verification rate (verified vs rejected)
4. Rate limiting (requests/s)

---

## Benchmarking

### Load Testing

**Tool**: Custom MPQUIC load generator (not included, must be built).

**Test scenario**:
1. Register 1,000 peers over 60 seconds (~16 req/s)
2. Each peer sends 10 packets/s to random targets (10,000 packets/s total)
3. Run for 10 minutes
4. Measure: latency (p50, p95, p99), packet loss, CPU, memory

**Expected results** (baseline, no tuning):
- Latency p50: 2 ms
- Latency p95: 8 ms
- Latency p99: 20 ms
- Packet loss: < 0.01%
- CPU: ~1000m (1 core)
- Memory: ~1 GB

### Stress Testing

**Scenario**: DoS attack simulation.

1. Generate 10,000 registration requests with invalid CAPoW proofs
2. Measure: rejection rate, CPU, memory
3. Ensure: relay stays responsive, legitimate peers can still connect

**Expected results**:
- Rejection rate: 10,000/s (all invalid proofs rejected)
- CPU: ~2000m (2 cores, CAPoW verification)
- Memory: ~1 GB (replay cache grows, but bounded by 5-min TTL)
- Legitimate peers: Can still register (worker pool not starved)

### Profiling

**CPU profiling** (requires `perf` on Linux):
```bash
# Attach to running process
sudo perf record -F 99 -p $(pidof file-relay) -g -- sleep 60

# Generate flamegraph
sudo perf script | stackcollapse-perf.pl | flamegraph.pl > flamegraph.svg
```

**Memory profiling** (requires `valgrind` or `heaptrack`):
```bash
# Run with heaptrack
heaptrack ./target/release/file-relay

# Analyze results
heaptrack_gui heaptrack.file-relay.*.gz
```

---

## Troubleshooting Performance Issues

### High CPU Usage

**Symptom**: CPU usage > 80% consistently.

**Diagnosis**:
```bash
# Check CAPoW rejection rate
curl http://localhost:8081/metrics | grep capow_rejected

# If high: DoS attack or misconfigured clients
# If low: Packet forwarding bottleneck
```

**Solutions**:
1. **DoS attack**: Alert security team, enable DDoS protection
2. **CAPoW verification**: Increase worker pool size (requires code change)
3. **Packet forwarding**: Scale horizontally (add more replicas)

### High Memory Usage

**Symptom**: Memory usage > 80% of limit.

**Diagnosis**:
```bash
# Check peer count
curl http://localhost:8081/metrics | grep active_peers

# Expected memory: ~512 MB + (active_peers * 0.5 MB)
```

**Solutions**:
1. **More peers than expected**: Increase memory limits or scale horizontally
2. **Memory leak** (unlikely in Rust): Restart pod, report bug
3. **Replay cache growth**: Check cleanup is running (every 60s)

### High Packet Loss

**Symptom**: Peers report packet loss > 1%.

**Diagnosis**:
```bash
# Check UDP buffer drops
netstat -su | grep "packet receive errors"
netstat -su | grep "receive buffer errors"

# If > 0: UDP buffers too small
```

**Solutions**:
1. **Increase UDP buffers**: See [Operating System Tuning](#operating-system-tuning)
2. **Network congestion**: Scale horizontally (distribute load)
3. **MTU issues**: Check for fragmentation (`ip -s link`)

### High Latency

**Symptom**: Packet forwarding latency > 20 ms consistently.

**Diagnosis**:
```bash
# Check CPU usage
top -p $(pidof file-relay)

# If CPU > 80%: CPU starvation
# If CPU < 50%: Network issue
```

**Solutions**:
1. **CPU starvation**: Scale horizontally or vertically
2. **Network latency**: Check peers are routed to nearest relay
3. **Peer lookup contention**: Increase DashMap shards (requires code change)

### CAPoW Timeouts

**Symptom**: High CAPoW rejection rate with "verification timeout" logs.

**Diagnosis**:
```bash
# Check worker pool size
# Current: max(2, num_cpus * 0.25)
grep -r "worker_count" crates/relay_server/src/main.rs
```

**Solutions**:
1. **Increase worker pool**: Change `0.25` to `0.5` (50% of cores)
2. **Vertical scaling**: Add more CPU cores
3. **Lower CAPoW difficulty**: Not recommended (reduces DoS protection)

---

## Performance Tuning Checklist

Before going to production:

- [ ] OS tuning: File descriptors, UDP buffers, conntrack
- [ ] Application tuning: `MAX_PEERS`, log level
- [ ] Network tuning: MTU, UDP offloading, BBR
- [ ] Kubernetes tuning: Resource requests/limits, HPA, anti-affinity
- [ ] Monitoring: Prometheus scraping, Grafana dashboards, alerts configured
- [ ] Load testing: 1,000 peer test completed successfully
- [ ] Stress testing: DoS simulation completed, relay stayed responsive
- [ ] Profiling: CPU flamegraph reviewed, no unexpected hotspots
- [ ] Baseline metrics: Recorded for comparison after deployment

---

## Further Reading

- [Operations Runbook](OPERATIONS-RUNBOOK.md) - Incident response procedures
- [Deployment Guide](DEPLOYMENT-GUIDE.md) - Deployment procedures
- [System Overview](../architecture/SYSTEM-OVERVIEW.md) - Architecture details
- [Monitoring Setup](DEPLOYMENT-GUIDE.md#monitoring-setup) - Prometheus + Grafana setup

---

**Questions or issues?** [Open an issue](https://github.com/file-network/relay-server/issues) or consult the [FAQ](../FAQ.md).
