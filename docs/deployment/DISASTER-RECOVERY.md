# FILE Relay Server - Disaster Recovery Guide

**Last Updated**: 2026-05-18  
**Version**: 0.1.0

---

## Table of Contents

1. [Overview](#overview)
2. [Failure Scenarios](#failure-scenarios)
3. [Recovery Procedures](#recovery-procedures)
4. [Data Backup Strategy](#data-backup-strategy)
5. [RTO and RPO](#rto-and-rpo)
6. [Testing DR Plans](#testing-dr-plans)
7. [Communication Plan](#communication-plan)

---

## Overview

FILE Relay Server is **stateless by design**, which simplifies disaster recovery significantly. No persistent data = no data loss scenarios.

**Key characteristics**:
- **Stateless**: No database, no persistent storage
- **Ephemeral state**: Peer registry, rate limiters, replay cache (all in-memory)
- **Zero data loss**: Peers reconnect after failure, no historical data stored
- **Fast recovery**: Restart = full recovery (< 30 seconds)

**Disaster scenarios**:
1. Single pod/container failure
2. Node/host failure
3. Availability zone failure
4. Region failure
5. Complete infrastructure failure

---

## Failure Scenarios

### Scenario 1: Single Pod/Container Failure

**Impact**: Minimal (other pods continue serving)

**Detection**:
- Kubernetes liveness probe fails
- Alert: `FileRelayServerDown` (if all pods down)
- Metrics stop updating for that pod

**Automatic recovery** (Kubernetes):
1. Liveness probe fails after 3 attempts (30s)
2. Kubernetes kills pod
3. New pod starts automatically
4. Readiness probe passes (ready in ~10s)
5. LoadBalancer adds pod to rotation

**Manual recovery** (Docker/systemd):
```bash
# Docker
docker restart file-relay-production

# systemd
sudo systemctl restart file-relay
```

**RTO**: ~30 seconds (Kubernetes), ~5 seconds (manual restart)  
**RPO**: 0 (no data loss)

**User impact**:
- Peers on failed pod disconnect
- Reconnect to another pod automatically (< 5 seconds)
- No message loss (application-layer retries)

---

### Scenario 2: Node/Host Failure

**Impact**: All pods on that node fail

**Detection**:
- Kubernetes node becomes `NotReady`
- Alert: `KubeNodeNotReady` (from kube-state-metrics)
- Multiple relay pods down simultaneously

**Automatic recovery** (Kubernetes):
1. Node marked `NotReady` after 40 seconds
2. Pods evicted after 5 minutes (default)
3. Scheduler places pods on healthy nodes
4. PodDisruptionBudget ensures min 2 pods always available

**Manual recovery** (if node can be recovered):
```bash
# SSH to node (if accessible)
ssh node-name

# Check node status
kubectl get node node-name

# Drain node (graceful eviction)
kubectl drain node-name --ignore-daemonsets --delete-emptydir-data

# Fix node issue (reboot, etc.)
sudo reboot

# Uncordon node
kubectl uncordon node-name
```

**RTO**: ~5 minutes (pod eviction timeout)  
**RPO**: 0 (no data loss)

**User impact**:
- Peers on failed node disconnect
- Reconnect to pods on healthy nodes
- Brief connection interruption (< 1 minute)

---

### Scenario 3: Availability Zone Failure

**Impact**: All nodes in that AZ fail

**Detection**:
- Multiple node failures simultaneously
- Cloud provider status page (AWS, GCP, Azure)
- Alert: `KubeNodeNotReady` (multiple nodes)

**Automatic recovery** (Kubernetes with multi-AZ):
1. Pods on failed AZ nodes evicted (5 minutes)
2. Scheduler places pods on nodes in healthy AZs
3. LoadBalancer routes traffic to healthy AZs

**Prerequisites**:
- Cluster spans multiple AZs (3+ AZs recommended)
- Pod anti-affinity spreads pods across AZs
- LoadBalancer distributes traffic across AZs

**Kubernetes config**:
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
        topologyKey: topology.kubernetes.io/zone  # AZ anti-affinity
```

**RTO**: ~5 minutes (pod eviction timeout)  
**RPO**: 0 (no data loss)

**User impact**:
- Peers in failed AZ disconnect
- Reconnect to pods in healthy AZs
- Connection interruption (< 1 minute)

---

### Scenario 4: Region Failure

**Impact**: Entire region unavailable

**Detection**:
- All nodes in region unreachable
- Cloud provider status page
- Alert: All relay pods down

**Recovery** (requires multi-region deployment):

**Option A: Manual DNS failover**
1. Update DNS records to point to backup region
2. Wait for TTL (5-60 minutes depending on DNS config)
3. Traffic flows to backup region

**Option B: Global load balancer (AWS Route 53, GCP Cloud Load Balancing)**
1. Health check fails for primary region
2. Load balancer automatically routes to backup region
3. Traffic shifted in ~1 minute

**Prerequisites**:
- Multi-region deployment (see [Multi-Region Guide](MULTI-REGION-GUIDE.md))
- Shared DNS name across regions
- Global load balancer or low DNS TTL

**RTO**: 1-60 minutes (depends on DNS/load balancer)  
**RPO**: 0 (no data loss)

**User impact**:
- Brief global outage (DNS propagation time)
- All peers reconnect to backup region
- No message loss (application retries)

---

### Scenario 5: Complete Infrastructure Failure

**Impact**: All infrastructure unavailable (extremely rare)

**Examples**:
- Cloud provider complete outage
- Kubernetes control plane failure
- Network partition (all regions unreachable)

**Recovery**:

**Phase 1: Deploy to new infrastructure** (1-4 hours)
1. Provision new Kubernetes cluster (or VMs)
2. Deploy relay server from Docker image (GHCR)
3. Update DNS to point to new infrastructure
4. Validate deployment

**Phase 2: Restore configuration** (30 minutes)
1. Configuration in Git (already versioned)
2. No data to restore (stateless)
3. Metrics restored from Prometheus backups (if needed)

**Deployment from scratch**:
```bash
# 1. Provision Kubernetes cluster
# (cloud provider specific, e.g., eksctl, gcloud, az aks create)

# 2. Deploy relay server
kubectl apply -f deploy/kubernetes/relay-server-deployment.yaml
kubectl apply -f deploy/kubernetes/relay-server-monitoring.yaml

# 3. Verify
kubectl get pods -n file-relay
kubectl logs -n file-relay -l app=file-relay-server

# 4. Update DNS
# (cloud provider console or CLI)
```

**RTO**: 1-4 hours (infrastructure provisioning time)  
**RPO**: 0 (no data loss, but historical metrics lost if Prometheus not backed up)

**User impact**:
- Extended global outage (hours)
- All peers reconnect after recovery
- Historical metrics lost (if Prometheus not backed up)

---

## Recovery Procedures

### Restart Single Pod (Kubernetes)

```bash
# Find failing pod
kubectl get pods -n file-relay

# Delete pod (will be recreated)
kubectl delete pod file-relay-server-abc123 -n file-relay

# Verify new pod is running
kubectl get pods -n file-relay -w
```

### Restart All Pods (Rolling)

```bash
# Rolling restart (zero downtime)
kubectl rollout restart deployment file-relay-server -n file-relay

# Watch rollout status
kubectl rollout status deployment file-relay-server -n file-relay
```

### Scale Up Quickly (Emergency Capacity)

```bash
# Manual scale
kubectl scale deployment file-relay-server --replicas=10 -n file-relay

# Verify
kubectl get pods -n file-relay
```

### Rollback to Previous Version

```bash
# View rollout history
kubectl rollout history deployment file-relay-server -n file-relay

# Rollback to previous version
kubectl rollout undo deployment file-relay-server -n file-relay

# Rollback to specific revision
kubectl rollout undo deployment file-relay-server --to-revision=2 -n file-relay
```

### Restore from Backup (Configuration Only)

```bash
# Configuration is in Git
git checkout v0.1.0
kubectl apply -f deploy/kubernetes/

# Metrics (if Prometheus backed up)
# Restore Prometheus data directory from backup
```

---

## Data Backup Strategy

### What to Backup

Since the relay is stateless, backups focus on **configuration and observability data**.

| Item | Backup Frequency | Retention | Purpose |
|------|------------------|-----------|---------|
| **Git repository** | Continuous (GitHub) | Infinite | Source code, config, docs |
| **Docker images** | On release | Infinite (GHCR) | Rollback to old versions |
| **Kubernetes manifests** | Continuous (Git) | Infinite | Infrastructure as code |
| **Prometheus data** | Daily | 30 days | Historical metrics, incident investigation |
| **Grafana dashboards** | Continuous (Git) | Infinite | Provisioned from JSON |
| **Logs** | Continuous (aggregator) | 7-30 days | Incident investigation |

### Prometheus Backup

**Manual snapshot**:
```bash
# Create snapshot
curl -XPOST http://prometheus:9090/api/v1/admin/tsdb/snapshot

# Snapshot saved to: /prometheus/snapshots/<timestamp>

# Copy snapshot
kubectl cp prometheus-pod:/prometheus/snapshots/<timestamp> ./prometheus-backup-<date>.tar.gz
```

**Automated backup** (Kubernetes CronJob):
```yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: prometheus-backup
spec:
  schedule: "0 2 * * *"  # Daily at 2 AM
  jobTemplate:
    spec:
      template:
        spec:
          containers:
          - name: backup
            image: alpine:latest
            command:
            - sh
            - -c
            - |
              # Create snapshot via API
              # Upload to S3/GCS/Azure
              # (implementation depends on cloud provider)
```

### Log Backup

**Log aggregators automatically backup**:
- Elasticsearch: Automatic replicas + snapshots
- Loki: Retention policy (7-30 days)
- CloudWatch Logs: Automatic retention

**No manual backup needed** if using log aggregator.

---

## RTO and RPO

### Recovery Time Objective (RTO)

Maximum tolerable downtime.

| Scenario | RTO | Notes |
|----------|-----|-------|
| Single pod failure | 30 seconds | Kubernetes automatic restart |
| Node failure | 5 minutes | Pod eviction timeout |
| AZ failure | 5 minutes | Multi-AZ deployment |
| Region failure | 1-60 minutes | Depends on DNS/load balancer |
| Complete infrastructure failure | 1-4 hours | Manual rebuild |

**Target RTO**: < 5 minutes for common failures (99.9% of cases)

### Recovery Point Objective (RPO)

Maximum tolerable data loss.

| Data Type | RPO | Notes |
|-----------|-----|-------|
| Peer connections | 0 | Peers reconnect automatically |
| Message data | 0 | End-to-end at application layer |
| Configuration | 0 | Git versioned, no loss possible |
| Metrics | 1 day | If Prometheus not backed up |
| Logs | 0 | If log aggregator used |

**Target RPO**: 0 for all operational data (stateless design)

---

## Testing DR Plans

### DR Test Schedule

| Test Type | Frequency | Duration | Purpose |
|-----------|-----------|----------|---------|
| Pod restart | Weekly | 5 min | Verify automatic recovery |
| Node drain | Monthly | 30 min | Verify pod eviction works |
| Region failover | Quarterly | 2 hours | Verify multi-region setup |
| Complete rebuild | Annually | 4 hours | Verify from-scratch deployment |

### DR Test Procedure Template

**Test**: [Test name, e.g., "Pod restart test"]  
**Date**: [YYYY-MM-DD]  
**Tester**: [Name]  
**Environment**: [Staging/Production]

**Pre-test checklist**:
- [ ] Announce test to team (if production)
- [ ] Verify backups current
- [ ] Verify monitoring active
- [ ] Verify rollback plan ready

**Test steps**:
1. [Step 1]
2. [Step 2]
3. ...

**Expected outcome**: [What should happen]

**Actual outcome**: [What actually happened]

**Recovery time**: [Measured RTO]

**Issues found**: [Problems encountered]

**Action items**: [Follow-up tasks]

**Conclusion**: ☐ Pass  ☐ Fail (reason: ________________)

---

## Communication Plan

### Incident Severity Levels

| Level | Impact | Response Time | Communication |
|-------|--------|---------------|---------------|
| **SEV-1** | Complete outage, all users affected | Immediate | Status page + email + Slack |
| **SEV-2** | Partial outage, some users affected | 15 minutes | Status page + Slack |
| **SEV-3** | Degraded performance, no user impact | 1 hour | Slack |
| **SEV-4** | Minimal, informational | Next business day | Internal only |

### Incident Communication Template

**Initial notification** (within 5 minutes):
```
INCIDENT: FILE Relay Server [SEV-X]

Status: Investigating
Impact: [Brief description of user impact]
Start time: [YYYY-MM-DD HH:MM UTC]

We are investigating an issue with the FILE Relay Server.
Updates will be posted every 15 minutes.

Next update: [HH:MM UTC]
```

**Status update** (every 15 minutes):
```
INCIDENT UPDATE: FILE Relay Server [SEV-X]

Status: [Investigating / Identified / Monitoring / Resolved]
Impact: [Current user impact]
Root cause: [If known]

Actions taken:
- [Action 1]
- [Action 2]

Next update: [HH:MM UTC]
```

**Resolution notification**:
```
INCIDENT RESOLVED: FILE Relay Server [SEV-X]

Status: Resolved
Impact: None (service fully restored)
Resolution time: [YYYY-MM-DD HH:MM UTC]
Duration: [X hours Y minutes]

Root cause: [Summary]

Actions taken:
- [Action 1]
- [Action 2]

Post-mortem: [Link to document, if applicable]
```

### Incident Roles

| Role | Responsibilities |
|------|------------------|
| **Incident Commander** | Overall coordination, decision-making |
| **Technical Lead** | Diagnose and fix technical issues |
| **Communications Lead** | Status updates, customer communication |
| **Scribe** | Document timeline, actions taken |

### Escalation Path

```
L1: On-call engineer (PagerDuty)
  ↓ (if not resolved in 30 min)
L2: Senior engineer / Team lead
  ↓ (if not resolved in 2 hours)
L3: Engineering manager / CTO
  ↓ (if business-critical)
L4: Executive team
```

---

## Post-Incident Review

### Post-Mortem Template

**Incident**: [Brief title]  
**Date**: [YYYY-MM-DD]  
**Severity**: [SEV-1/2/3/4]  
**Duration**: [X hours Y minutes]  
**Impact**: [Users affected, data loss, etc.]

**Timeline**:
- HH:MM UTC - [Event 1]
- HH:MM UTC - [Event 2]
- ...

**Root Cause**: [Detailed explanation]

**Contributing Factors**:
- [Factor 1]
- [Factor 2]

**What Went Well**:
- [Success 1]
- [Success 2]

**What Went Wrong**:
- [Problem 1]
- [Problem 2]

**Action Items**:
| Action | Owner | Due Date | Status |
|--------|-------|----------|--------|
| [Action 1] | [Name] | [YYYY-MM-DD] | [Open/In Progress/Done] |
| [Action 2] | [Name] | [YYYY-MM-DD] | [Open/In Progress/Done] |

**Lessons Learned**: [Summary]

---

## Further Reading

- [Operations Runbook](OPERATIONS-RUNBOOK.md) — Operational procedures
- [Security Hardening Checklist](SECURITY-HARDENING-CHECKLIST.md) — Security review
- [Multi-Region Deployment Guide](MULTI-REGION-GUIDE.md) — Geographic distribution
- [Observability Guide](OBSERVABILITY-GUIDE.md) — Monitoring best practices

---

**Questions?** [Open an issue](https://github.com/file-network/relay-server/issues) or consult the [FAQ](../FAQ.md).
