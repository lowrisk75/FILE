# FILE Relay Server - Operations Runbook

**Version**: 0.1.0  
**Audience**: Operations / SRE teams  
**Last Updated**: 2026-05-18

---

## Quick Reference

### Emergency Contacts

- **On-Call**: #file-relay-oncall (Slack)
- **Escalation**: ops-lead@file-network.example
- **Security**: security@file-network.example

### Service Endpoints

- **Health**: `http://<relay-ip>:8081/health`
- **Metrics**: `http://<relay-ip>:8081/metrics`
- **Prometheus**: `http://<prometheus-ip>:9090`
- **Grafana**: `http://<grafana-ip>:3000`
- **Alertmanager**: `http://<alertmanager-ip>:9093`

### Quick Commands

```bash
# Health check
curl -f http://relay-server:8081/health || echo "UNHEALTHY"

# Current metrics snapshot
curl -s http://relay-server:8081/metrics | grep file_relay

# Pod status (Kubernetes)
kubectl -n file-network get pods -l app=file-relay

# Tail logs
kubectl -n file-network logs -f deployment/file-relay-server --tail=100

# Restart deployment
kubectl -n file-network rollout restart deployment/file-relay-server
```

---

## Incident Response

### P1: Relay Server Down

**Alert**: `FileRelayServerDown`  
**Impact**: All relay traffic down, peers cannot communicate  
**SLO**: 99.9% uptime (8.76 hours/year downtime)

#### Immediate Actions (5 minutes)

1. **Verify scope**
   ```bash
   # Check all pods
   kubectl -n file-network get pods -l app=file-relay
   
   # Check health endpoints
   for pod in $(kubectl -n file-network get pods -l app=file-relay -o name); do
     kubectl -n file-network exec $pod -- curl -f http://localhost:8081/health
   done
   ```

2. **Check recent events**
   ```bash
   # Pod events (crashes, OOM, evictions)
   kubectl -n file-network get events --sort-by='.lastTimestamp' | grep file-relay
   
   # Recent logs
   kubectl -n file-network logs deployment/file-relay-server --tail=200
   ```

3. **Restore service**
   ```bash
   # If pods are CrashLooping: check resource limits
   kubectl -n file-network describe pod <pod-name> | grep -A 10 "State:"
   
   # If OOM killed: increase memory
   kubectl -n file-network set resources deployment file-relay-server \
     --limits=memory=4Gi --requests=memory=2Gi
   
   # If image pull fail: verify image exists
   kubectl -n file-network describe pod <pod-name> | grep "Image:"
   
   # Force restart
   kubectl -n file-network rollout restart deployment/file-relay-server
   ```

#### Investigation (30 minutes)

1. **Root cause analysis**
   ```bash
   # Check for OOM
   kubectl -n file-network describe pod <pod-name> | grep -i oom
   
   # Check for panics
   kubectl -n file-network logs <pod-name> | grep -i panic
   
   # Check for resource exhaustion
   kubectl -n file-network top pod <pod-name>
   ```

2. **External dependencies**
   - Network connectivity (test UDP 8080)
   - Load balancer health
   - Node health (`kubectl get nodes`)

#### Follow-up Actions

- Update postmortem in incident channel
- If recurring: increase resource limits or scale horizontally
- If bug: file GitHub issue with logs + repro steps

---

### P1: DoS Attack Detected

**Alert**: `FileRelayCAPoWAttack` or `FileRelayRateLimitAttack`  
**Impact**: Legitimate peers may be unable to connect  
**SLO**: < 5% of legitimate traffic affected

#### Immediate Actions (5 minutes)

1. **Verify attack**
   ```bash
   # Check rejection rate
   curl -s http://relay-server:8081/metrics | grep -E "capow_rejected|rate_limited"
   
   # Sample logs (IPs are hashed for GDPR)
   kubectl -n file-network logs deployment/file-relay-server --tail=100 | \
     grep -E "CAPoW rejected|Rate limited"
   ```

2. **Assess impact**
   ```bash
   # Active peers (should not drop significantly)
   curl -s http://relay-server:8081/metrics | grep active_peers
   
   # Packet forwarding (should remain stable)
   curl -s http://relay-server:8081/metrics | grep packets_forwarded
   ```

3. **Mitigate**
   ```bash
   # Rate limiting is automatic (per-IP, 10 req/s burst 50)
   # CAPoW verification timeout is 100ms (prevents CPU exhaustion)
   
   # If overwhelmed: scale horizontally
   kubectl -n file-network scale deployment file-relay-server --replicas=10
   
   # If specific IP range: block at firewall/load balancer
   # (Note: IPs in logs are hashed, use network logs for source IPs)
   ```

#### Investigation (1 hour)

1. **Attack pattern**
   ```bash
   # CAPoW attack indicators:
   # - High capow_rejected rate (>50/s)
   # - Low capow_verified rate
   # - Active peers stable or growing slowly
   
   # Rate limit attack indicators:
   # - High rate_limited rate (>100/s)
   # - Many unique hashed IPs in logs
   # - Burst pattern (spikes every N seconds)
   ```

2. **Source identification**
   ```bash
   # Check load balancer / firewall logs for source IPs
   # (relay logs only have hashed IPs for GDPR compliance)
   
   # Cloud provider: check VPC flow logs
   # Load balancer: check access logs
   ```

3. **Block at perimeter**
   ```bash
   # Example: AWS Security Group
   aws ec2 revoke-security-group-ingress \
     --group-id sg-xxxxx \
     --ip-permissions IpProtocol=udp,FromPort=8080,ToPort=8080,IpRanges='[{CidrIp=<attacker-ip>/32}]'
   
   # Example: iptables
   sudo iptables -A INPUT -s <attacker-ip> -p udp --dport 8080 -j DROP
   ```

#### Follow-up Actions

- Document attack pattern in incident report
- If widespread: consider adjusting rate limits or CAPoW difficulty
- If ASN-based: contact ISP abuse desk

---

### P2: High Peer Count

**Alert**: `FileRelayHighPeerCount` or `FileRelayCapacityReached`  
**Impact**: Near capacity, may reject new peers  
**SLO**: < 90% capacity for 95% of time

#### Immediate Actions (15 minutes)

1. **Check current capacity**
   ```bash
   # Readiness endpoint
   curl -s http://relay-server:8081/ready | jq
   
   # Metrics
   curl -s http://relay-server:8081/metrics | grep active_peers
   ```

2. **Determine cause**
   ```bash
   # Natural growth vs. sudden spike
   # Check Grafana: Active Peers dashboard (last 24 hours)
   
   # Registration rate
   curl -s http://relay-server:8081/metrics | grep peers_registered_total
   ```

3. **Scale if needed**
   ```bash
   # Horizontal scaling (preferred)
   kubectl -n file-network scale deployment file-relay-server --replicas=5
   
   # OR increase max-peers per pod (requires restart)
   kubectl -n file-network set env deployment/file-relay-server MAX_PEERS=2000
   kubectl -n file-network rollout restart deployment/file-relay-server
   ```

#### Investigation (1 hour)

1. **Growth pattern**
   - Natural: gradual increase over days/weeks → plan capacity expansion
   - Spike: sudden increase in hours → investigate registration source
   - Flatline at max: hitting capacity limit → immediate scaling needed

2. **Peer behavior**
   ```bash
   # Timeout rate (high = peers not staying connected)
   curl -s http://relay-server:8081/metrics | grep peers_timeout
   
   # Packet rate per peer (low = idle peers)
   # packets_forwarded / active_peers
   ```

#### Follow-up Actions

- Update capacity planning docs
- If HPA exists: verify autoscaling thresholds
- If recurring: add capacity proactively

---

### P2: High Peer Timeout Rate

**Alert**: `FileRelayHighPeerTimeouts`  
**Impact**: Poor peer connectivity, degraded service quality  
**SLO**: < 1% timeout rate

#### Immediate Actions (10 minutes)

1. **Check timeout rate**
   ```bash
   # Timeout rate
   curl -s http://relay-server:8081/metrics | grep peers_timeout
   
   # Compare to registration rate
   curl -s http://relay-server:8081/metrics | grep peers_registered
   ```

2. **Review recent logs**
   ```bash
   # Timeout events
   kubectl -n file-network logs deployment/file-relay-server --tail=200 | \
     grep "Peer timeout"
   ```

#### Investigation (30 minutes)

1. **Possible causes**
   - **Network instability**: Packet loss, latency spikes
   - **Client bugs**: Clients not sending keep-alive
   - **Server overload**: CPU/memory exhaustion delaying timeout checks
   - **Geographic distance**: High RTT causing perceived inactivity

2. **Diagnostics**
   ```bash
   # Check server resources
   kubectl -n file-network top pod <pod-name>
   
   # Check network metrics (if available)
   # - Packet loss rate
   # - RTT to clients
   # - UDP buffer overruns
   
   # Check timeout threshold (default: 5 minutes)
   # If too aggressive: increase in main.rs and redeploy
   ```

#### Follow-up Actions

- If network issue: work with network team
- If client bug: notify client developers
- If threshold issue: adjust IDLE_TIMEOUT and redeploy
- If server overload: scale resources

---

### P3: Low Packet Forwarding Rate

**Alert**: `FileRelayLowPacketRate`  
**Impact**: Degraded performance, peers not communicating  
**SLO**: > 1 packet/s when active_peers > 10

#### Investigation (1 hour)

1. **Verify problem**
   ```bash
   # Packet rate
   curl -s http://relay-server:8081/metrics | grep packets_forwarded
   
   # Active peers
   curl -s http://relay-server:8081/metrics | grep active_peers
   
   # Rate (packets/s) = packets_forwarded_total / uptime
   ```

2. **Possible causes**
   - **Idle peers**: Peers connected but not sending data (expected)
   - **Network congestion**: Upstream/downstream bottleneck
   - **Event loop bottleneck**: (Future: after event loop implemented)

3. **Diagnostics**
   ```bash
   # Check network I/O
   kubectl -n file-network exec <pod-name> -- ss -s
   
   # Check pod network limits
   kubectl -n file-network describe pod <pod-name> | grep -A 5 "Limits:"
   ```

#### Follow-up Actions

- If idle peers: expected behavior, no action
- If network issue: check bandwidth limits, QoS policies
- If event loop: (Future) profile and optimize

---

## Routine Maintenance

### Weekly Tasks

**Monday Morning** (15 minutes):

```bash
# 1. Review alerts from past week
# Check Alertmanager: http://alertmanager:9093/#/alerts

# 2. Check resource utilization trends
# Grafana: Resource Usage dashboard (7-day view)

# 3. Verify backup/DR readiness
# (Future: when state is persisted)

# 4. Review capacity planning
curl -s http://relay-server:8081/metrics | grep active_peers
# Compare to max_peers, plan for growth
```

### Monthly Tasks

**First Monday of Month** (1 hour):

```bash
# 1. Dependency audit
cd /path/to/relay-server
cargo audit

# 2. Update dependencies (if vulnerabilities found)
# Review RUSTSEC advisories
# Test updates in staging before prod

# 3. Review operational metrics
# - Uptime (target: 99.9%)
# - P50/P90/P99 packet latency (future metric)
# - Attack frequency and severity
# - Resource utilization trends

# 4. Capacity planning review
# Project growth for next 3 months
# Plan scaling events
```

### Quarterly Tasks

**Beginning of Quarter** (half day):

1. **Performance review**
   - Review SLO compliance
   - Analyze incident trends
   - Update runbooks based on learnings

2. **Cost optimization**
   - Review resource utilization
   - Right-size deployments
   - Evaluate autoscaling policies

3. **Security review**
   - Review access logs (if applicable)
   - Audit firewall rules
   - Update dependency pins

4. **DR test**
   - Simulate failover
   - Verify backup restoration (future)
   - Test rollback procedures

---

## Deployment Procedures

### Standard Deployment (Low Risk)

**Trigger**: New feature, dependency update, config change  
**Downtime**: Zero (rolling update)  
**Duration**: 10-15 minutes

```bash
# 1. Pre-deployment checks
# - All tests passing in CI
# - Staging validation complete
# - Change approved in review

# 2. Tag new version
git tag v0.2.0
git push origin v0.2.0

# 3. Build and push image
docker build -t ghcr.io/file-network/relay-server:v0.2.0 -f deploy/Dockerfile .
docker push ghcr.io/file-network/relay-server:v0.2.0

# 4. Update deployment
kubectl -n file-network set image deployment/file-relay-server \
  relay-server=ghcr.io/file-network/relay-server:v0.2.0

# 5. Watch rollout
kubectl -n file-network rollout status deployment/file-relay-server

# 6. Verify health
curl http://<new-pod-ip>:8081/health
curl http://<new-pod-ip>:8081/metrics | grep active_peers

# 7. Monitor for 30 minutes
# Watch Grafana dashboards for anomalies
# Check error rate, timeout rate, packet rate

# 8. If issues: rollback
kubectl -n file-network rollout undo deployment/file-relay-server
```

### Emergency Hotfix (High Risk)

**Trigger**: Security vulnerability, critical bug  
**Downtime**: Possible (if restart required)  
**Duration**: 30-60 minutes

```bash
# 1. Assess impact
# - Is this security-critical? (follow sec team process)
# - Can we wait for standard deployment window?
# - What's the blast radius if fix fails?

# 2. Prepare rollback
# - Document current state
# - Save current deployment YAML
kubectl -n file-network get deployment file-relay-server -o yaml > rollback.yaml

# 3. Deploy hotfix
# (Same as standard deployment, but watch more closely)

# 4. Immediate validation
# - Test vulnerable code path
# - Verify fix effectiveness
# - Monitor all metrics closely

# 5. Incident report
# Document root cause, fix, and lessons learned
```

### Configuration Change Only

**Trigger**: Change max-peers, rate limits, logging level  
**Downtime**: Zero (environment variable update)

```bash
# 1. Update environment variable
kubectl -n file-network set env deployment/file-relay-server \
  MAX_PEERS=2000 \
  RUST_LOG=info,relay_server=trace

# 2. Pods restart automatically
kubectl -n file-network rollout status deployment/file-relay-server

# 3. Verify new config
kubectl -n file-network exec <pod-name> -- env | grep -E "MAX_PEERS|RUST_LOG"

# 4. Check logs for new level
kubectl -n file-network logs <pod-name> --tail=50
```

---

## Monitoring and Alerting

### Key Metrics to Watch

**Availability**:
- `up{job="file-relay-server"}` - Should be 1 (server responding)

**Capacity**:
- `file_relay_active_peers` - Track against max (default 1000)
- `file_relay_peers_registered_total` - Growth rate

**Security**:
- `rate(file_relay_capow_rejected_total[5m])` - Rejection rate (attack indicator)
- `rate(file_relay_rate_limited_total[5m])` - Rate limiting (abuse indicator)

**Performance**:
- `rate(file_relay_packets_forwarded_total[5m])` - Throughput
- `rate(file_relay_peers_timeout_total[5m])` - Connection quality

### Alert Fatigue Prevention

1. **Tune thresholds** based on actual traffic patterns
2. **Use inhibition rules** to suppress redundant alerts
3. **Group alerts** by severity and component
4. **Adjust timing**: warning vs. critical timing

### On-Call Rotation

**Responsibilities**:
- Respond to P1 alerts within 5 minutes
- Respond to P2 alerts within 30 minutes
- Daily health check (5 minutes)
- Handoff notes at rotation change

**Handoff Checklist**:
- [ ] Any ongoing incidents?
- [ ] Planned maintenance this week?
- [ ] Any known issues to watch?
- [ ] Recent changes deployed?
- [ ] Runbook updates needed?

---

## Common Issues

### Issue: "Rate limited" log spam

**Cause**: Client retry logic too aggressive, or actual attack  
**Fix**: Check if legitimate traffic or attack, adjust client or block IPs

### Issue: High memory usage

**Cause**: Replay cache growth, peer count increase  
**Fix**: Verify cache cleanup running (60s interval), increase memory limits

### Issue: UDP packets dropped

**Cause**: Kernel buffer too small, network congestion  
**Fix**: Increase `net.core.rmem_max`, check network capacity

### Issue: "CAPoW rejected" for valid proofs

**Cause**: Clock skew, wrong difficulty, algorithm mismatch  
**Fix**: Verify client/server clocks in sync, check CAPoW parameters

---

## Glossary

- **CAPoW**: Computational Amortized Proof of Work (DoS protection)
- **MPQUIC**: Multi-Path QUIC (transport protocol)
- **Relay**: Stateless packet forwarder between peers
- **peer_id**: 32-byte Ed25519 public key (peer identifier)
- **Hashed IP**: HMAC-SHA256(IP, per-boot-secret) for GDPR compliance
- **Zombie peer**: Connected peer with no activity for >5 minutes

---

## Appendix: Useful Commands

### Quick Diagnostics

```bash
# Full metrics dump
curl -s http://relay-server:8081/metrics | grep file_relay

# Health status
curl -s http://relay-server:8081/health | jq

# Readiness status
curl -s http://relay-server:8081/ready | jq

# Pod status (Kubernetes)
kubectl -n file-network get pods -l app=file-relay -o wide

# Recent logs with timestamps
kubectl -n file-network logs deployment/file-relay-server --since=1h --timestamps

# Follow logs for specific pattern
kubectl -n file-network logs -f deployment/file-relay-server | grep -E "ERROR|WARN|Rate limited"
```

### Performance Profiling (Future)

```bash
# CPU profile (requires pprof endpoint)
curl http://relay-server:6060/debug/pprof/profile?seconds=30 > cpu.prof

# Memory profile
curl http://relay-server:6060/debug/pprof/heap > heap.prof

# Analyze with pprof
go tool pprof cpu.prof
```

---

**Runbook Version**: 1.0  
**Last Updated**: 2026-05-18  
**Next Review**: 2026-06-18
