# FILE Relay Server - Migration Guide

**Last Updated**: 2026-05-18  
**Version**: 0.1.0

---

## Table of Contents

1. [Overview](#overview)
2. [Migration Strategy](#migration-strategy)
3. [Version Compatibility](#version-compatibility)
4. [Zero-Downtime Upgrades](#zero-downtime-upgrades)
5. [Breaking Changes](#breaking-changes)
6. [Rollback Procedures](#rollback-procedures)
7. [Migration Checklists](#migration-checklists)

---

## Overview

FILE Relay Server is **stateless**, which greatly simplifies migrations. No database migrations, no data format changes, no state synchronization.

**Key characteristics**:
- **Stateless**: No persistent storage, no database
- **Backward compatible**: Old and new versions run side-by-side during rollout
- **Zero downtime**: Rolling updates without service interruption
- **Fast rollback**: Revert to previous version in < 1 minute

**Migration scenarios**:
1. Patch version upgrade (0.1.0 → 0.1.1)
2. Minor version upgrade (0.1.x → 0.2.0)
3. Major version upgrade (0.x.x → 1.0.0)
4. Emergency rollback

---

## Migration Strategy

### Semantic Versioning

FILE Relay Server follows [Semantic Versioning](https://semver.org/):

**MAJOR.MINOR.PATCH** (e.g., 1.2.3)

- **MAJOR**: Breaking changes (incompatible API/protocol changes)
- **MINOR**: New features, backward compatible
- **PATCH**: Bug fixes, backward compatible

### Upgrade Path

```
0.1.0 → 0.1.1 → 0.1.2 → 0.2.0 → 0.3.0 → 1.0.0
  ↑       ↑       ↑       ↑       ↑       ↑
Patch   Patch   Patch   Minor   Minor   Major
```

**Rules**:
- **Patch upgrades**: Always safe, zero downtime
- **Minor upgrades**: Safe, zero downtime, review CHANGELOG
- **Major upgrades**: May require coordination, review breaking changes

**Skipping versions**:
- ✅ Patch: Can skip (0.1.0 → 0.1.3)
- ✅ Minor: Can skip (0.1.0 → 0.3.0)
- ⚠️ Major: Review CHANGELOG carefully (may require intermediate steps)

---

## Version Compatibility

### Protocol Compatibility

| Server Version | Client Version | Compatible? | Notes |
|----------------|----------------|-------------|-------|
| 0.1.x | 0.1.x | ✅ Yes | Full compatibility |
| 0.2.x | 0.1.x | ✅ Yes | Backward compatible |
| 1.0.0 | 0.x.x | ⚠️ Check CHANGELOG | May have breaking changes |

**Design principle**: New server versions accept old client protocol versions.

### Metrics Compatibility

Metrics are **additive only** until a major version:
- ✅ New metrics added (minor version)
- ✅ Existing metrics unchanged
- ❌ Metrics removed/renamed (major version only)

**Impact**:
- Prometheus queries continue working across minor versions
- Dashboards work without changes
- Alerts work without changes

### Configuration Compatibility

Environment variables are **backward compatible** until a major version:
- ✅ New environment variables added
- ✅ Existing variables unchanged
- ⚠️ Variables deprecated (warning in logs, still work)
- ❌ Variables removed (major version only, after deprecation period)

---

## Zero-Downtime Upgrades

### Kubernetes Rolling Update

**Default strategy** (already configured in deployment manifests):

```yaml
strategy:
  type: RollingUpdate
  rollingUpdate:
    maxUnavailable: 1      # At most 1 pod down at a time
    maxSurge: 1            # At most 1 extra pod during update
```

**Rolling update process**:
```
Initial state:      [Pod A (v0.1.0)] [Pod B (v0.1.0)] [Pod C (v0.1.0)]
                              ↓
Update starts:      [Pod A (v0.1.0)] [Pod B (v0.1.0)] [Pod C (v0.1.0)] [Pod D (v0.2.0)]
                              ↓
Pod A replaced:     [Pod D (v0.2.0)] [Pod B (v0.1.0)] [Pod C (v0.1.0)]
                              ↓
Pod B replaced:     [Pod D (v0.2.0)] [Pod E (v0.2.0)] [Pod C (v0.1.0)]
                              ↓
Pod C replaced:     [Pod D (v0.2.0)] [Pod E (v0.2.0)] [Pod F (v0.2.0)]
                              ↓
Final state:        [Pod D (v0.2.0)] [Pod E (v0.2.0)] [Pod F (v0.2.0)]
```

**Timeline**: ~2 minutes for 3 replicas (30s per pod startup + health check)

**Zero downtime guaranteed** because:
- PodDisruptionBudget ensures min 2 pods always available
- LoadBalancer removes unhealthy pods automatically
- Peers reconnect to healthy pods if their pod goes down

### Kubernetes Upgrade Procedure

**Step 1: Review CHANGELOG**
```bash
# View changes between current and target version
git log v0.1.0..v0.2.0 --oneline
```

**Step 2: Update deployment manifest**
```yaml
# deploy/kubernetes/relay-server-deployment.yaml
spec:
  template:
    spec:
      containers:
      - name: relay-server
        image: ghcr.io/file-network/relay-server:0.2.0  # Update version
```

**Step 3: Apply update**
```bash
kubectl apply -f deploy/kubernetes/relay-server-deployment.yaml
```

**Step 4: Monitor rollout**
```bash
# Watch rollout status
kubectl rollout status deployment file-relay-server -n file-relay

# Expected output:
# Waiting for deployment "file-relay-server" rollout to finish: 1 out of 3 new replicas have been updated...
# Waiting for deployment "file-relay-server" rollout to finish: 2 out of 3 new replicas have been updated...
# deployment "file-relay-server" successfully rolled out
```

**Step 5: Verify health**
```bash
# Check all pods running
kubectl get pods -n file-relay

# Check metrics
curl http://relay:8081/metrics | grep file_relay_active_peers

# Run health check script
./scripts/ops/health-check.sh relay.example.com
```

**Step 6: Monitor for issues** (15-30 minutes)
- Watch metrics in Grafana
- Check logs for errors: `kubectl logs -n file-relay -l app=file-relay-server`
- Monitor alerts in Alertmanager

### Docker Compose Upgrade Procedure

**Blue-Green deployment** (minimal downtime):

**Step 1: Start new version (green)**
```bash
# Update docker-compose.yaml with new image version
sed -i 's/relay-server:0.1.0/relay-server:0.2.0/' docker-compose.yaml

# Start new container (green) without stopping old (blue)
docker-compose up -d --no-deps --scale relay-server=2 relay-server
```

**Step 2: Verify green is healthy**
```bash
# Check green container health
docker ps | grep relay-server

# Run health check
./scripts/ops/health-check.sh localhost
```

**Step 3: Switch traffic**
```bash
# Update load balancer to point to green
# (implementation depends on load balancer)

# Or update port mapping (if no LB)
# Stop blue, green takes over port 8080
docker-compose up -d --scale relay-server=1 relay-server
```

**Step 4: Stop blue**
```bash
# After verifying green is stable (15-30 min)
docker stop <blue-container-id>
docker rm <blue-container-id>
```

**Downtime**: ~5 seconds (DNS propagation + peer reconnect)

### Systemd Upgrade Procedure (Single Instance)

**Step 1: Download new binary**
```bash
# Download from GitHub releases
wget https://github.com/file-network/relay-server/releases/download/v0.2.0/file-relay-linux-amd64

# Or build from source
git checkout v0.2.0
cargo build --release
```

**Step 2: Stop service**
```bash
sudo systemctl stop file-relay
```

**Step 3: Replace binary**
```bash
sudo cp target/release/file-relay /usr/local/bin/file-relay
sudo chmod 755 /usr/local/bin/file-relay
```

**Step 4: Start service**
```bash
sudo systemctl start file-relay
```

**Step 5: Verify**
```bash
sudo systemctl status file-relay
./scripts/ops/health-check.sh localhost
```

**Downtime**: ~10 seconds (service restart time)

**To reduce downtime**: Deploy second instance, migrate traffic, upgrade first instance

---

## Breaking Changes

### 0.1.0 → Future Major Version

**No breaking changes yet** (initial release).

When major version changes occur, CHANGELOG will clearly document:
- What changed
- Why it changed
- Migration path
- Backward compatibility period (if any)

### Breaking Change Template (for future reference)

**Example** (hypothetical):

```markdown
## [2.0.0] - 2027-XX-XX

### Breaking Changes

**Environment variable renamed**: `MAX_PEERS` → `RELAY_MAX_PEERS`

**Reason**: Namespace collision with other tools.

**Migration**:
- Update deployment manifests: `RELAY_MAX_PEERS` instead of `MAX_PEERS`
- Old variable still works in 2.0.x with deprecation warning
- Old variable removed in 3.0.0

**Timeline**:
- v2.0.0 (released 2027-XX-XX): Deprecation warning
- v3.0.0 (release TBD): Old variable removed
```

---

## Rollback Procedures

### Kubernetes Rollback

**Automatic rollback** (if health checks fail):
```bash
# Configure in deployment
spec:
  progressDeadlineSeconds: 600  # Fail after 10 minutes
  revisionHistoryLimit: 10      # Keep last 10 versions
```

**Manual rollback**:
```bash
# Rollback to previous version
kubectl rollout undo deployment file-relay-server -n file-relay

# Rollback to specific revision
kubectl rollout history deployment file-relay-server -n file-relay
kubectl rollout undo deployment file-relay-server --to-revision=2 -n file-relay

# Monitor rollback
kubectl rollout status deployment file-relay-server -n file-relay
```

**Timeline**: ~2 minutes (same as upgrade)

### Docker Compose Rollback

```bash
# Update docker-compose.yaml to old version
sed -i 's/relay-server:0.2.0/relay-server:0.1.0/' docker-compose.yaml

# Restart with old version
docker-compose up -d
```

**Timeline**: ~30 seconds

### Systemd Rollback

```bash
# Keep old binary as backup during upgrade
sudo cp /usr/local/bin/file-relay /usr/local/bin/file-relay.bak

# Rollback
sudo systemctl stop file-relay
sudo cp /usr/local/bin/file-relay.bak /usr/local/bin/file-relay
sudo systemctl start file-relay
```

**Timeline**: ~10 seconds

---

## Migration Checklists

### Pre-Migration Checklist

- [ ] **Review CHANGELOG**
  - [ ] Read release notes for target version
  - [ ] Check for breaking changes
  - [ ] Note new features and configuration options

- [ ] **Test in staging**
  - [ ] Deploy target version to staging environment
  - [ ] Run smoke tests (peer registration, packet forwarding)
  - [ ] Verify metrics and logs
  - [ ] Load test (if significant changes)

- [ ] **Prepare rollback plan**
  - [ ] Document current version
  - [ ] Backup configuration (Git commit)
  - [ ] Verify rollback procedure

- [ ] **Schedule maintenance window** (if needed)
  - [ ] Notify users (if any downtime expected)
  - [ ] Choose low-traffic time
  - [ ] Prepare incident communication template

- [ ] **Verify monitoring**
  - [ ] Prometheus scraping working
  - [ ] Alerts configured
  - [ ] Dashboards up-to-date

### During Migration Checklist

- [ ] **Execute upgrade**
  - [ ] Apply deployment changes (kubectl apply / docker-compose up)
  - [ ] Monitor rollout progress
  - [ ] Watch for errors in logs

- [ ] **Health checks**
  - [ ] Verify all pods/containers healthy
  - [ ] Run `./scripts/ops/health-check.sh`
  - [ ] Check metrics endpoint (/metrics)
  - [ ] Verify active peers count

- [ ] **Functional validation**
  - [ ] Test peer registration (if possible)
  - [ ] Verify CAPoW verification working
  - [ ] Check rate limiting active
  - [ ] Confirm packet forwarding (once event loop implemented)

### Post-Migration Checklist

- [ ] **Monitor for 30 minutes**
  - [ ] Watch Grafana dashboards
  - [ ] Check for increased error rates
  - [ ] Monitor CAPoW rejection rate
  - [ ] Verify no alerts firing

- [ ] **Performance validation**
  - [ ] Compare metrics to baseline (pre-upgrade)
  - [ ] Check latency hasn't increased
  - [ ] Verify resource usage within limits

- [ ] **Documentation**
  - [ ] Update runbook with actual migration time
  - [ ] Document any issues encountered
  - [ ] Update internal wiki/docs with new version

- [ ] **Cleanup**
  - [ ] Remove old Docker images (if desired)
  - [ ] Archive old binaries
  - [ ] Update monitoring thresholds (if needed)

- [ ] **Post-migration communication**
  - [ ] Notify team of successful upgrade
  - [ ] Send status update to stakeholders (if applicable)

---

## Troubleshooting Migrations

### Issue: Pods stuck in pending state

**Symptom**: New pods don't start after upgrade.

**Diagnosis**:
```bash
kubectl describe pod <pod-name> -n file-relay
```

**Common causes**:
- Insufficient cluster resources (CPU/memory)
- Image pull failure (wrong tag, registry auth)
- Node selector/affinity preventing scheduling

**Solution**:
- Scale down other workloads temporarily
- Fix image tag or registry credentials
- Adjust node selector/affinity rules

### Issue: Health check failing on new version

**Symptom**: New pods start but health checks fail.

**Diagnosis**:
```bash
kubectl logs <pod-name> -n file-relay
curl http://<pod-ip>:8081/health
```

**Common causes**:
- Binary crash on startup
- Port 8081 not listening
- Health check endpoint changed (breaking change)

**Solution**:
- Check logs for crash reason
- Verify `METRICS_ADDR` environment variable
- Rollback if breaking change

### Issue: High CAPoW rejection rate after upgrade

**Symptom**: `file_relay_capow_rejected_total` increasing rapidly.

**Diagnosis**:
```bash
curl http://relay:8081/metrics | grep capow
kubectl logs -n file-relay -l app=file-relay-server | grep CAPoW
```

**Common causes**:
- CAPoW difficulty changed (breaking)
- Verification timeout changed
- Bug in new version

**Solution**:
- Review CHANGELOG for CAPoW changes
- Rollback if behavior incorrect
- Report bug if regression

---

## Version-Specific Migration Notes

### 0.1.0 → 0.1.1 (future)

**Type**: Patch release  
**Breaking changes**: None  
**Downtime**: 0  
**Special instructions**: None

### 0.1.x → 0.2.0 (future)

**Type**: Minor release  
**Breaking changes**: None (planned)  
**Downtime**: 0  
**New features**: TBD (review CHANGELOG)  
**Special instructions**: TBD

### 0.x.x → 1.0.0 (future)

**Type**: Major release  
**Breaking changes**: TBD (will be clearly documented)  
**Downtime**: TBD  
**Migration path**: TBD (multi-step if needed)  
**Special instructions**: TBD

---

## Further Reading

- [CHANGELOG](../../CHANGELOG.md) — Full version history
- [Deployment Guide](DEPLOYMENT-GUIDE.md) — Initial deployment
- [Disaster Recovery](DISASTER-RECOVERY.md) — Rollback procedures
- [Operations Runbook](OPERATIONS-RUNBOOK.md) — Operational procedures

---

**Questions?** [Open an issue](https://github.com/file-network/relay-server/issues) or consult the [FAQ](../FAQ.md).
