# Post-Mortem Template

**Incident Date**: [YYYY-MM-DD]  
**Incident ID**: [INC-NNNN]  
**Severity**: [SEV-1 / SEV-2 / SEV-3 / SEV-4]  
**Status**: [Draft / In Review / Final]  
**Author**: [Name]  
**Reviewers**: [Names]

---

## Executive Summary

[2-3 sentences summarizing what happened, impact, and resolution]

**Example**:
> On 2026-05-18 at 14:23 UTC, the FILE Relay Server experienced a complete outage in the us-east-1 region lasting 18 minutes. The outage was caused by a Kubernetes node failure that evicted all relay pods. The issue was resolved by manual pod rescheduling. Approximately 5,000 users were affected with temporary connection loss. No data was lost due to the stateless design of the relay.

---

## Impact Assessment

| Metric | Value |
|--------|-------|
| **Severity** | [SEV-1 / SEV-2 / SEV-3 / SEV-4] |
| **Duration** | [X hours Y minutes] |
| **Start time** | [YYYY-MM-DD HH:MM UTC] |
| **End time** | [YYYY-MM-DD HH:MM UTC] |
| **Time to detect** | [X minutes] |
| **Time to resolve** | [Y minutes] |
| **Users affected** | [N users / all users / X%] |
| **Regions affected** | [us-east-1, eu-west-1, etc.] |
| **Revenue impact** | [$X / $0 / N/A] |
| **Data loss** | [None / X records / N/A] |
| **SLA compliance** | [Met / Breached by X minutes] |
| **Error budget consumed** | [X% / X minutes] |

---

## Timeline

**All times in UTC. Use 24-hour format.**

| Time | Event | Actor |
|------|-------|-------|
| 14:15 | Normal operation, 850 active peers | System |
| 14:23 | Node `ip-10-0-1-42.ec2.internal` became NotReady | AWS |
| 14:23 | Kubernetes evicted 3 relay pods from failed node | Kubernetes |
| 14:23 | Alert `FileRelayServerDown` fired | Prometheus |
| 14:24 | On-call engineer Jane paged | PagerDuty |
| 14:26 | Jane acknowledged page, began investigation | Jane |
| 14:28 | Jane identified node failure as root cause | Jane |
| 14:30 | Jane attempted automatic pod rescheduling (waited) | Jane |
| 14:35 | Pods still in Pending state, investigated further | Jane |
| 14:37 | Jane discovered cluster capacity exhausted | Jane |
| 14:38 | Jane manually scaled down non-critical workload | Jane |
| 14:39 | Relay pods successfully scheduled on healthy nodes | Kubernetes |
| 14:40 | Pods passed health checks, LoadBalancer added to rotation | Kubernetes |
| 14:41 | Peer count began recovering, users reconnecting | System |
| 14:45 | All users reconnected, incident resolved | System |
| 14:50 | Jane confirmed resolution, closed incident | Jane |

---

## Root Cause Analysis

### What Happened?

[Detailed technical explanation of the failure]

**Example**:
> The us-east-1 Kubernetes cluster node `ip-10-0-1-42.ec2.internal` experienced a hardware failure at 14:23 UTC. This node hosted 3 of the 3 total relay server pods (due to misconfigured anti-affinity rules that allowed all pods to schedule on the same node). When the node failed, all 3 pods were evicted simultaneously, causing a complete service outage.
>
> Kubernetes attempted to reschedule the pods on healthy nodes, but the cluster was at capacity (all nodes running at 90%+ CPU/memory). The pods remained in Pending state for 15 minutes until the on-call engineer manually scaled down a non-critical development workload, freeing resources for the relay pods.

### Why Did It Happen?

[Chain of causality - root cause and contributing factors]

**Example**:
> **Root cause**: Pod anti-affinity rule was configured with `preferredDuringSchedulingIgnoredDuringExecution` instead of `requiredDuringSchedulingIgnoredDuringExecution`. This allowed Kubernetes to schedule all pods on the same node when convenient, defeating the purpose of high availability.
>
> **Contributing factors**:
> 1. Cluster capacity planning was insufficient - running at 90% utilization left no headroom for pod rescheduling
> 2. Monitoring did not alert on pod colocation violations
> 3. Node health was not proactively monitored (node failed without warning)
> 4. PodDisruptionBudget (minAvailable: 2) was not enforced during node failure (only enforced during voluntary disruptions)

### Why Wasn't It Caught Earlier?

[Preventive measures that should have caught this]

**Example**:
> 1. **Pod placement was not validated**: No monitoring alert for "all pods on same node"
> 2. **Capacity alerts were not tuned**: Cluster capacity at 90% did not trigger preemptive scaling
> 3. **Anti-affinity testing was insufficient**: Deployment was tested only with `kubectl apply`, not with actual node failures
> 4. **Runbook testing was not performed**: Incident response procedures were documented but never rehearsed

---

## Resolution

### How Was It Fixed?

[Actions taken to resolve the incident]

**Example**:
> 1. On-call engineer identified node failure via `kubectl get nodes`
> 2. Confirmed pods in Pending state via `kubectl get pods`
> 3. Checked cluster capacity via `kubectl top nodes`
> 4. Scaled down development workload: `kubectl scale deployment dev-app --replicas=0`
> 5. Pods automatically scheduled on healthy nodes
> 6. Verified health via `./scripts/ops/health-check.sh`
> 7. Monitored metrics for 15 minutes to ensure stability

### What Worked Well?

[Positive aspects of the response]

**Example**:
> - **Fast detection**: Alert fired within 1 minute of outage
> - **Clear runbook**: Engineer followed documented procedures
> - **Effective communication**: Team notified via Slack, status updates every 10 minutes
> - **No data loss**: Stateless design prevented any data corruption
> - **Fast recovery**: Once resources available, pods recovered in < 2 minutes

### What Didn't Work Well?

[Problems encountered during response]

**Example**:
> - **Slow diagnosis**: Engineer spent 12 minutes investigating before identifying root cause
> - **Manual intervention required**: No automatic capacity scaling to handle pod rescheduling
> - **Incomplete monitoring**: Cluster capacity exhaustion was not alerted on
> - **Runbook gaps**: Runbook did not cover "cluster at capacity" scenario
> - **Communication delays**: First status update was 8 minutes after outage start (SLA: 5 minutes)

---

## Action Items

**Format**: [Action] - [Owner] - [Due Date] - [Priority] - [Status]

| # | Action | Owner | Due Date | Priority | Status |
|---|--------|-------|----------|----------|--------|
| 1 | Change pod anti-affinity from `preferred` to `required` | Jane | 2026-05-19 | P0 (Critical) | ✅ Done |
| 2 | Add monitoring alert for pod colocation violations | Bob | 2026-05-20 | P0 (Critical) | 🔄 In Progress |
| 3 | Increase cluster capacity to 70% target utilization | Alice | 2026-05-22 | P0 (Critical) | ⏳ Pending |
| 4 | Enable Cluster Autoscaler with buffer for pod rescheduling | Bob | 2026-05-25 | P1 (High) | ⏳ Pending |
| 5 | Add alert for cluster capacity > 80% | Bob | 2026-05-20 | P1 (High) | 🔄 In Progress |
| 6 | Update runbook with "cluster at capacity" scenario | Jane | 2026-05-21 | P2 (Medium) | ⏳ Pending |
| 7 | Schedule node failure drill (test pod rescheduling) | Alice | 2026-06-01 | P2 (Medium) | ⏳ Pending |
| 8 | Review PodDisruptionBudget behavior during node failures | Bob | 2026-05-23 | P2 (Medium) | ⏳ Pending |
| 9 | Document faster communication procedures (update within 5 min) | Jane | 2026-05-22 | P3 (Low) | ⏳ Pending |

---

## Lessons Learned

### What Did We Learn?

1. **Pod anti-affinity must be `required`, not `preferred`**: "Soft" rules provide no guarantee of high availability
2. **Cluster capacity planning must account for failures**: Running at 90% utilization leaves no headroom for rescheduling
3. **Monitoring must validate configuration, not just metrics**: Anti-affinity violations went undetected
4. **Runbooks must be tested regularly**: Documented procedures had gaps revealed only during real incident
5. **Stateless design validated**: Zero data loss despite complete outage proved architectural choice

### Knowledge Gaps Identified

- **Team knowledge**: Only 1 engineer familiar with Kubernetes cluster scaling
- **Tool knowledge**: `kubectl top nodes` not widely known for capacity diagnosis
- **Process knowledge**: PodDisruptionBudget behavior during involuntary disruptions unclear

### Training Needs

- [ ] Kubernetes capacity management training for all engineers
- [ ] Incident response drill: node failure scenario
- [ ] Runbook walkthrough session

---

## Supporting Data

### Metrics Snapshots

**Before incident** (14:15 UTC):
```
file_relay_active_peers: 850
file_relay_packets_forwarded_total: 1,245,832
up{job="file-relay"}: 1 (all pods)
```

**During incident** (14:30 UTC):
```
file_relay_active_peers: 0
up{job="file-relay"}: 0 (all pods down)
```

**After resolution** (14:45 UTC):
```
file_relay_active_peers: 847 (users reconnected)
up{job="file-relay"}: 1 (all pods healthy)
```

### Logs

**Pod eviction** (14:23:18 UTC):
```
Pod file-relay-server-abc123 evicted from node ip-10-0-1-42.ec2.internal: NodeNotReady
```

**Pending pod** (14:30:42 UTC):
```
Pod file-relay-server-xyz789: 0/3 nodes available: 3 Insufficient cpu
```

**Successful scheduling** (14:39:12 UTC):
```
Pod file-relay-server-xyz789 successfully assigned to node ip-10-0-2-15.ec2.internal
```

### Related Links

- **Alert**: https://prometheus.example.com/alerts/FileRelayServerDown
- **Grafana dashboard**: https://grafana.example.com/d/relay-server
- **Slack incident channel**: #incident-2026-05-18-relay-outage
- **Jira ticket**: FILE-1234

---

## Customer Communication

### Initial Notification (14:25 UTC)

> **INCIDENT: FILE Relay Server Outage**
>
> We are investigating a service disruption affecting the FILE Relay Server in the us-east-1 region. Users may experience connection failures. We are actively working to resolve the issue.
>
> Next update: 14:35 UTC

### Resolution Notice (14:50 UTC)

> **RESOLVED: FILE Relay Server Outage**
>
> The service disruption has been resolved. All systems are operational. Users affected by the outage have automatically reconnected.
>
> **Incident summary**:
> - Start: 14:23 UTC
> - End: 14:45 UTC
> - Duration: 22 minutes
> - Cause: Infrastructure failure (node outage)
> - Impact: ~5,000 users temporarily unable to connect
> - Data loss: None
>
> We apologize for the disruption. A full post-mortem will be published within 72 hours.

---

## Review and Sign-Off

| Role | Name | Date | Signature |
|------|------|------|-----------|
| **Incident Commander** | Jane Smith | 2026-05-19 | ✓ |
| **Technical Lead** | Bob Johnson | 2026-05-19 | ✓ |
| **Engineering Manager** | Alice Chen | 2026-05-20 | ✓ |
| **CTO** | David Lee | 2026-05-21 | ✓ |

**Status**: ✅ **Approved for publication**

---

## Appendix

### Cost Impact

- **Infrastructure**: No additional cost (unused capacity consumed)
- **Engineer time**: 3 hours (Jane: 1.5h, Bob: 1h, Alice: 0.5h)
- **Customer credits**: $250 (SLA breach for 5 premium customers)
- **Total**: ~$750

### Related Incidents

- **INC-1042** (2026-03-12): Similar pod colocation issue on staging
- **INC-0987** (2026-01-05): Node failure in eu-west-1, but pods rescheduled successfully (cluster had capacity)

---

**Document Version**: 1.0  
**Last Updated**: 2026-05-21  
**Next Review**: 2026-08-21 (quarterly)
