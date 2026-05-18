# FILE Relay Server - Alert Runbook

**Last Updated**: 2026-05-18  
**Version**: 0.1.0

---

## Overview

This runbook provides detailed response procedures for each production alert. All alerts are defined in `deploy/prometheus/alerts.yml`.

**Alert severity levels**:
- **Critical (P1)**: Immediate response required, page on-call engineer
- **Warning (P2)**: Review within 1 hour, Slack notification

---

## Table of Contents

### Critical Alerts (P1)
1. [FileRelayServerDown](#1-filerelayserverdown)
2. [FileRelayCapacityReached](#2-filerelayCapacityReached)
3. [FileRelayCAPoWAttack](#3-filerelaycapowattack)
4. [FileRelayRateLimitAttack](#4-filerelayRatelimitAttack)

### Warning Alerts (P2)
5. [FileRelayHighPeerCount](#5-filerelayhighpeercount)
6. [FileRelayHighCAPoWRejectionRate](#6-filerelayhighcapowrejectionrate)
7. [FileRelayHighPeerTimeouts](#7-filerelayhighpeertimeouts)
8. [FileRelayPodRestarting](#8-filerelayPodRestarting)
9. [FileRelayHighMemoryUsage](#9-filerelayhighmemoryusage)
10. [FileRelayHighCPUUsage](#10-filerelayhighcpuusage)
11. [FileRelayMetricsDown](#11-filerelaymetricsdown)
12. [FileRelayNoActiveConnections](#12-filerelaynoactiveconnections)

---

## Critical Alerts (P1)

### 1. FileRelayServerDown

**Alert definition**:
```yaml
alert: FileRelayServerDown
expr: up{job="file-relay"} == 0
for: 2m
severity: critical
```

**What it means**: Prometheus cannot scrape metrics from the relay server for 2 minutes. Server is likely down or unreachable.

---

#### Immediate Actions (< 5 minutes)

**Step 1: Verify scope**
```bash
# Check how many pods are down
kubectl get pods -n file-relay

# Check metrics from other pods
curl http://relay:8081/metrics
```

**Step 2: Check pod status**
```bash
# Describe failing pod
kubectl describe pod <pod-name> -n file-relay

# Check recent events
kubectl get events -n file-relay --sort-by='.lastTimestamp' | head -20
```

**Step 3: Review logs**
```bash
# Last 100 lines
kubectl logs <pod-name> -n file-relay --tail=100

# Search for errors
kubectl logs <pod-name> -n file-relay | grep -i error
```

**Possible causes**:
- Container crashed (OOM, panic, segfault)
- Node failure (all pods on node affected)
- Network partition (pod running but unreachable)
- Image pull failure (new deployment)

---

#### Recovery Procedures

**If single pod crashed**:
```bash
# Kubernetes will auto-restart
# Wait 30 seconds, then verify
kubectl get pods -n file-relay

# If stuck, force delete
kubectl delete pod <pod-name> -n file-relay --force --grace-period=0
```

**If all pods down**:
```bash
# Check deployment status
kubectl get deployment file-relay-server -n file-relay

# Check for recent changes
kubectl rollout history deployment file-relay-server -n file-relay

# Rollback if recent deploy caused issue
kubectl rollout undo deployment file-relay-server -n file-relay
```

**If node failure**:
```bash
# Check node status
kubectl get nodes

# Cordon failed node
kubectl cordon <node-name>

# Drain node (pods will reschedule)
kubectl drain <node-name> --ignore-daemonsets --delete-emptydir-data
```

---

#### Post-Incident

- [ ] Verify all pods healthy
- [ ] Run health check: `./scripts/ops/health-check.sh`
- [ ] Review metrics for 15 minutes (watch for recurring issues)
- [ ] Create incident report if downtime > 5 minutes
- [ ] Update runbook if new failure mode discovered

---

### 2. FileRelayCapacityReached

**Alert definition**:
```yaml
alert: FileRelayCapacityReached
expr: file_relay_active_peers >= 1000
for: 5m
severity: critical
```

**What it means**: Active peer count at or above configured maximum (1000). New peers will be rejected.

---

#### Immediate Actions (< 5 minutes)

**Step 1: Verify capacity**
```bash
# Check current peer count
curl http://relay:8081/metrics | grep active_peers

# Check all pods
kubectl get pods -n file-relay -o wide
for pod in $(kubectl get pods -n file-relay -o name); do
  echo "$pod:"
  kubectl exec $pod -n file-relay -- curl -s localhost:8081/metrics | grep active_peers
done
```

**Step 2: Scale horizontally**
```bash
# Manual scale (immediate relief)
kubectl scale deployment file-relay-server --replicas=5 -n file-relay

# Verify new pods starting
kubectl get pods -n file-relay -w
```

**Step 3: Monitor redistribution**
```bash
# Peers should redistribute to new pods
# Watch metrics for 2-3 minutes
curl http://relay:8081/metrics | grep active_peers
```

---

#### Root Cause Analysis

**Check for spike or gradual growth**:
```bash
# Use Grafana dashboard or Prometheus
# Query: file_relay_active_peers[24h]
# Look for:
# - Sudden spike = possible attack or traffic surge
# - Gradual growth = organic growth, need capacity planning
```

**Determine if attack**:
```bash
# Check registration rate
curl http://relay:8081/metrics | grep peers_registered_total

# Check rejection rate
curl http://relay:8081/metrics | grep capow_rejected_total

# High rejection rate = likely attack
# Low rejection rate = legitimate growth
```

---

#### Long-Term Solutions

**If organic growth**:
- Update HPA targets: `maxReplicas: 10` → `maxReplicas: 15`
- Increase per-instance capacity: `MAX_PEERS=1000` → `MAX_PEERS=1500`
- Review capacity planning: `./scripts/ops/capacity-calculator.sh 5000`

**If recurring attacks**:
- Review CAPoW difficulty (may need increase)
- Implement additional rate limiting (network edge)
- Consider DDoS protection service (Cloudflare Spectrum)

---

#### Post-Incident

- [ ] Verify capacity now < 90%
- [ ] Update capacity planning documents
- [ ] Schedule capacity review meeting
- [ ] Update HPA configuration if needed

---

### 3. FileRelayCAPoWAttack

**Alert definition**:
```yaml
alert: FileRelayCAPoWAttack
expr: rate(file_relay_capow_rejected_total[1m]) > 50
for: 2m
severity: critical
```

**What it means**: CAPoW rejection rate > 50/second for 2 minutes. Likely DoS attack with invalid proofs.

---

#### Immediate Actions (< 5 minutes)

**Step 1: Confirm attack**
```bash
# Check rejection rate
curl http://relay:8081/metrics | grep capow_rejected

# Check if all pods affected
kubectl get pods -n file-relay -o wide
for pod in $(kubectl get pods -n file-relay -o name); do
  echo "$pod:"
  kubectl exec $pod -n file-relay -- curl -s localhost:8081/metrics | grep capow_rejected
done
```

**Step 2: Identify attack source (if logs available)**
```bash
# Check recent logs for rejected proofs
kubectl logs -n file-relay -l app=file-relay-server --tail=100 | grep "CAPoW rejected"

# Look for patterns in IP hashes
kubectl logs -n file-relay -l app=file-relay-server --tail=1000 | grep "CAPoW rejected" | awk '{print $X}' | sort | uniq -c | sort -rn | head -20
```

**Step 3: Verify legitimate users not affected**
```bash
# Check success rate
curl http://relay:8081/metrics | grep capow_verified

# If verified count still increasing = legitimate users OK
# If verified count flat = attack blocking legitimate traffic
```

---

#### Mitigation Options

**Option 1: Monitor and wait**
- If success rate still > 90%, CAPoW is working
- Attack is being mitigated automatically
- Monitor CPU usage, scale if needed

**Option 2: Scale up (if CPU saturated)**
```bash
# CAPoW verification is CPU-intensive
# Scale horizontally to distribute load
kubectl scale deployment file-relay-server --replicas=8 -n file-relay
```

**Option 3: Network-level blocking (if attack source identified)**
```bash
# If all attacks from specific ASN/country:
# - Add firewall rules at cloud provider level
# - Contact DDoS protection provider
# - Consider Cloudflare Spectrum
```

**Option 4: Emergency rate limiting (last resort)**
```bash
# Temporarily reduce rate limits
kubectl set env deployment/file-relay-server RATE_LIMIT=5 -n file-relay
# (reduces from 10 req/s to 5 req/s)

# Revert after attack subsides
kubectl set env deployment/file-relay-server RATE_LIMIT- -n file-relay
```

---

#### Post-Incident

- [ ] Document attack timeline
- [ ] Analyze attack pattern (IPs, proof types)
- [ ] Review CAPoW difficulty (increase if needed)
- [ ] Test DDoS protection provider response
- [ ] Update incident response plan

---

### 4. FileRelayRateLimitAttack

**Alert definition**:
```yaml
alert: FileRelayRateLimitAttack
expr: rate(file_relay_rate_limited_total[1m]) > 100
for: 2m
severity: critical
```

**What it means**: Rate limiting triggering > 100/second. Likely distributed attack or flash crowd.

---

#### Immediate Actions (< 5 minutes)

**Step 1: Determine attack type**
```bash
# Check rejection pattern
curl http://relay:8081/metrics | grep rate_limited_total

# Compare to CAPoW rejections
curl http://relay:8081/metrics | grep capow_rejected_total

# If BOTH high = coordinated attack (invalid proofs + flooding)
# If ONLY rate limiting = flash crowd or distributed attack
```

**Step 2: Assess legitimate user impact**
```bash
# Check success rate
curl http://relay:8081/metrics | grep peers_registered_total

# Calculate rejection rate:
# rejection_rate = rate_limited / (registered + rate_limited)
# If > 50% = significant user impact
```

**Step 3: Scale up**
```bash
# Rate limiting is per-instance
# More instances = more total capacity
kubectl scale deployment file-relay-server --replicas=10 -n file-relay
```

---

#### Mitigation Options

**Option 1: Increase per-IP limits (temporary)**
```bash
# If legitimate flash crowd (product launch, etc.)
# Temporarily increase burst capacity
# (requires code change, not runtime configurable)
```

**Option 2: Network-level rate limiting**
```bash
# Offload rate limiting to:
# - Cloud provider (AWS WAF, GCP Cloud Armor)
# - CDN/DDoS protection (Cloudflare)
# - Edge load balancer

# Advantage: Block attack before reaching relay
```

**Option 3: Geoblock (if attack from specific regions)**
```bash
# Use cloud provider firewall rules
# Block entire countries (if users not there)
```

---

#### Post-Incident

- [ ] Review rate limiting thresholds
- [ ] Evaluate need for network-level protection
- [ ] Analyze attack source distribution
- [ ] Update capacity planning

---

## Warning Alerts (P2)

### 5. FileRelayHighPeerCount

**Alert definition**:
```yaml
alert: FileRelayHighPeerCount
expr: file_relay_active_peers > 900
for: 15m
severity: warning
```

**What it means**: Peer count approaching capacity (90%). Time to scale or plan capacity.

---

#### Actions (within 1 hour)

**Step 1: Review trend**
```bash
# Check growth rate over last 24h
# Use Grafana or Prometheus query:
# deriv(file_relay_active_peers[24h])
```

**Step 2: Determine if scaling needed**
```bash
# Calculate time to capacity
# growth_rate = peers/hour
# time_to_capacity = (1000 - current) / growth_rate

# If < 4 hours = scale now
# If 4-24 hours = schedule scaling
# If > 24 hours = monitor
```

**Step 3: Scale proactively**
```bash
# Add 1-2 replicas
kubectl scale deployment file-relay-server --replicas=4 -n file-relay
```

---

#### Post-Response

- [ ] Update capacity planning doc
- [ ] Review HPA thresholds
- [ ] Schedule capacity review if recurring

---

### 6. FileRelayHighCAPoWRejectionRate

**Alert definition**:
```yaml
alert: FileRelayHighCAPoWRejectionRate
expr: rate(file_relay_capow_rejected_total[5m]) > 10
for: 10m
severity: warning
```

**What it means**: CAPoW rejection rate > 10/second for 10 minutes. Possible minor attack or misconfigured clients.

---

#### Actions (within 1 hour)

**Step 1: Check if spike or sustained**
```bash
# View last hour trend
curl http://relay:8081/metrics | grep capow_rejected_total

# If flat = misconfigured clients
# If increasing = possible attack ramp-up
```

**Step 2: Review logs for patterns**
```bash
# Sample rejected proofs
kubectl logs -n file-relay -l app=file-relay-server --tail=500 | grep "CAPoW rejected" | head -20

# Look for:
# - Same error repeated = client bug
# - Different errors = diverse attack
```

**Step 3: Monitor success rate**
```bash
# If success rate still > 90% = not urgent
# If success rate dropping = escalate to P1
```

---

### 7. FileRelayHighPeerTimeouts

**Alert definition**:
```yaml
alert: FileRelayHighPeerTimeouts
expr: rate(file_relay_peers_timeout_total[5m]) > 5
for: 10m
severity: warning
```

**What it means**: Peers timing out (5-minute idle) at > 5/second. Network quality issue or client disconnects.

---

#### Actions (within 1 hour)

**Step 1: Determine cause**
```bash
# Check if correlated with other events
# - Recent deployment? (rollout issue)
# - Network issues? (check node network metrics)
# - Client-side issues? (widespread internet outage)
```

**Step 2: Review logs**
```bash
# Check for connection errors
kubectl logs -n file-relay -l app=file-relay-server | grep -i timeout
```

**Step 3: Monitor for escalation**
```bash
# If timeout rate keeps increasing = escalate
# If stable = log and monitor
```

---

### 8. FileRelayPodRestarting

**Alert definition**:
```yaml
alert: FileRelayPodRestarting
expr: rate(kube_pod_container_status_restarts_total{pod=~"file-relay.*"}[15m]) > 0
severity: warning
```

**What it means**: Pod restarted in last 15 minutes. Crash, OOM, or liveness probe failure.

---

#### Actions (within 1 hour)

**Step 1: Check restart reason**
```bash
# Get pod status
kubectl get pods -n file-relay

# Describe pod
kubectl describe pod <pod-name> -n file-relay

# Look for:
# - OOMKilled = memory limit too low
# - CrashLoopBackOff = application crash
# - Error = other issue
```

**Step 2: Review logs before crash**
```bash
# Previous pod logs
kubectl logs <pod-name> -n file-relay --previous
```

**Step 3: Take action based on cause**
- **OOM**: Increase memory limits
- **Crash**: Review stack trace, file bug
- **Liveness probe**: Review probe config

---

### 9-12. Additional Warning Alerts

*(Abbreviated for space - follow similar structure)*

**9. FileRelayHighMemoryUsage**: Memory > 80%, check for leak or scale up  
**10. FileRelayHighCPUUsage**: CPU > 80%, scale up or optimize  
**11. FileRelayMetricsDown**: Prometheus scrape failing, check network/firewall  
**12. FileRelayNoActiveConnections**: Zero peers, investigate registration issues

---

## Alert Response Checklist

For every alert response:

- [ ] Acknowledge alert in Alertmanager/PagerDuty
- [ ] Notify team in incident Slack channel
- [ ] Document actions taken (incident timeline)
- [ ] Escalate if not resolved in SLA window
- [ ] Create post-mortem for critical incidents
- [ ] Update runbook if new learnings

---

## Escalation Path

**L1**: On-call engineer (PagerDuty, responds within 5 min)  
**L2**: Senior engineer / Team lead (escalate if not resolved in 30 min)  
**L3**: Engineering manager (escalate if not resolved in 2 hours)  
**L4**: CTO / Executive team (business-critical only)

---

## Further Reading

- [Operations Runbook](OPERATIONS-RUNBOOK.md) — General procedures
- [Disaster Recovery](DISASTER-RECOVERY.md) — Major incident response
- [Observability Guide](OBSERVABILITY-GUIDE.md) — Metrics interpretation
- [SLA/SLO Definitions](SLA-SLO-DEFINITIONS.md) — Service targets

---

**Questions or improvements?** [Open an issue](https://github.com/file-network/relay-server/issues).
