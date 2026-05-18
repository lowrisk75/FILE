# FILE Relay Server - Cost Optimization Guide

Comprehensive guide for minimizing infrastructure costs while maintaining performance and reliability.

---

## Table of Contents

1. [Cost Overview](#cost-overview)
2. [Cost Breakdown by Provider](#cost-breakdown-by-provider)
3. [Optimization Strategies](#optimization-strategies)
4. [Cost Calculators](#cost-calculators)
5. [Right-Sizing Guide](#right-sizing-guide)
6. [Cost-Performance Trade-offs](#cost-performance-trade-offs)
7. [Cost Monitoring](#cost-monitoring)
8. [Decision Trees](#decision-trees)

---

## Cost Overview

### Baseline Costs (Monthly)

**AWS EKS** (Production, 3 nodes, us-east-1):
```
EKS Control Plane:        $73.00
Worker Nodes (3x t3.medium): $90.00
NAT Gateways (3x):        $96.00
Network Load Balancer:    $18.00
EBS Volumes:              $8.00
Data Transfer (estimate): $15.00
────────────────────────────────
Total:                    $300.00/month
```

**GCP GKE Autopilot** (Production, us-east1):
```
GKE Control Plane:        $0.00 (free tier)
Pod Resources (3 replicas): $50.00
Network Load Balancer:    $18.00
Persistent Disks:         $4.00
Data Transfer (estimate): $15.00
────────────────────────────────
Total:                    $87.00/month
```

**Cost savings**: GCP Autopilot is **71% cheaper** than AWS EKS ($87 vs $300)

---

## Cost Breakdown by Provider

### AWS Detailed Breakdown

#### Compute Costs

**EKS Control Plane**:
- Cost: $0.10/hour × 730 hours = $73/month
- **Optimization**: None (fixed cost per cluster)
- **Alternative**: Use GKE (free control plane)

**Worker Nodes** (t3.medium: 2 vCPU, 4 GB RAM):
| Count | Cost/Hour | Cost/Month | Use Case |
|-------|-----------|------------|----------|
| 1 | $0.0416 | $30.37 | Development |
| 2 | $0.0832 | $60.74 | Staging |
| 3 | $0.1248 | $91.10 | Production (HA) |
| 5 | $0.2080 | $151.84 | High traffic |
| 10 | $0.4160 | $303.68 | Very high traffic |

**Optimization strategies**:
1. **Spot Instances**: 50-70% discount
   - Production: Mix 70% On-Demand + 30% Spot
   - Staging: 100% Spot
   - Savings: ~$30/month (3 nodes, 30% Spot)

2. **Reserved Instances**: 40% discount (1-year commitment)
   - Cost: $54/month (3 nodes)
   - Savings: $37/month
   - Payback: Immediate

3. **Savings Plans**: Flexible commitment
   - Commit: $60/month
   - Save: 30-40% on usage above commitment

4. **Right-sizing**:
   - Start small (t3.small: $15/month)
   - Scale up as needed
   - Use HPA (auto-scaling)

#### Network Costs

**NAT Gateway**:
| Configuration | Cost/Month | Savings Opportunity |
|--------------|------------|---------------------|
| 3 NAT (Multi-AZ) | $96.00 | Production only |
| 1 NAT (Single-AZ) | $32.00 | **Save $64** (staging/dev) |

**Cost breakdown**:
- Per NAT: $0.045/hour = $32.85/month
- Data processing: $0.045/GB (first 10 TB)

**Optimization**:
- Production: 3 NAT (high availability)
- Staging: 1 NAT (**save $64/month**)
- Development: 1 NAT (**save $64/month**)

**Network Load Balancer**:
- Cost: $0.0225/hour = $16.43/month
- LCU cost: ~$1-2/month (low traffic)
- **Optimization**: None (required for UDP)

**Data Transfer**:
| Traffic Type | Cost |
|-------------|------|
| Inbound | FREE |
| Outbound (first 10 TB/month) | $0.09/GB |
| Outbound (next 40 TB) | $0.085/GB |
| Inter-AZ | $0.01/GB each way |

**Typical usage** (1000 concurrent peers, 5 pkt/s):
- Outbound: ~500 GB/month = $45/month
- Inter-AZ: ~200 GB/month = $2/month

**Optimization**:
- Use CloudFront for static content (lower egress)
- Single-AZ for non-production (**save inter-AZ cost**)

#### Storage Costs

**EBS Volumes** (gp3):
- Cost: $0.08/GB-month
- Typical: 20 GB per node = $1.60/node
- 3 nodes: $4.80/month

**Optimization**: Minimal (storage cost already low)

---

### GCP Detailed Breakdown

#### GKE Autopilot (Recommended)

**Control Plane**:
- Cost: FREE (clusters < 2,500 vCPUs)
- **Savings**: $73/month vs AWS

**Pod Resources** (pay-per-pod):
| Replicas | CPU | Memory | Cost/Hour | Cost/Month |
|----------|-----|--------|-----------|------------|
| 1 | 100m | 128Mi | $0.00267 | $1.95 |
| 3 | 300m | 384Mi | $0.00801 | $5.85 |
| 5 | 500m | 640Mi | $0.01335 | $9.75 |
| 10 | 1000m | 1280Mi | $0.02670 | $19.49 |

**Pricing formula**:
```
Cost/month = (vCPU × $0.0445 + GB RAM × $0.00489) × 730 hours
```

**Example** (3 replicas, 100m CPU, 128Mi RAM each):
```
CPU:    0.3 vCPU × $0.0445/hour × 730 = $9.74
Memory: 0.375 GB × $0.00489/hour × 730 = $1.34
Total:  $11.08/month
```

**Autopilot features** (included):
- ✅ Automatic node management (no node pools)
- ✅ Vertical pod autoscaling
- ✅ Pod disruption budgets
- ✅ Security best practices enforced

**Optimization**: Already optimized (pay only for pods used)

---

#### GKE Standard (Alternative)

**Control Plane**:
- Cost: FREE (same as Autopilot)

**Worker Nodes** (e2-medium: 2 vCPU, 4 GB RAM):
| Count | Cost/Month | Savings vs AWS |
|-------|------------|----------------|
| 1 | $24.27 | $6 ($30 - $24) |
| 3 | $72.82 | $18 |
| 5 | $121.37 | $30 |

**Preemptible VMs** (80% discount):
| Count | Standard | Preemptible | Savings |
|-------|----------|-------------|---------|
| 3 | $72.82 | $14.56 | $58.26 |

**When to use Standard vs Autopilot**:
- **Autopilot**: Low/variable traffic, minimal ops overhead
- **Standard**: High sustained traffic, need control over nodes

---

### Azure (Coming Soon)

**Estimated costs** (3 nodes, Standard_B2s):
```
AKS Control Plane:        FREE
Worker Nodes (3x):        $75/month
Load Balancer:            $18/month
Storage:                  $5/month
────────────────────────────────
Total:                    $98/month
```

---

## Optimization Strategies

### Strategy 1: Choose the Right Cloud Provider

**Decision matrix**:

| Factor | AWS | GCP | Azure |
|--------|-----|-----|-------|
| **Baseline cost** | $300 | $87 | ~$98 |
| **Free tier** | ❌ | ✅ Control plane | ✅ Control plane |
| **Savings opportunity** | High | Low (already optimized) | Medium |
| **Best for** | Existing AWS shops | Cost-conscious, greenfield | Existing Azure shops |

**Recommendation**:
- **New deployment**: Start with GCP Autopilot (**save 71%**)
- **Existing AWS**: Migrate or optimize (see below)
- **Enterprise**: Multi-cloud for redundancy

---

### Strategy 2: Use Spot/Preemptible Instances

**AWS Spot Instances**:
- Discount: 50-70% off On-Demand
- Risk: Can be terminated with 2-minute warning
- **Best for**: Non-critical workloads, stateless apps (perfect for relay)

**Configuration** (mixed fleet):
```hcl
# Terraform (AWS)
capacity_type = "SPOT"  # 70% discount

# Or mixed: 70% On-Demand + 30% Spot
```

**Savings** (3 nodes):
- All On-Demand: $91/month
- All Spot: $27/month (**save $64**)
- Mixed (70/30): $67/month (**save $24**)

**GCP Preemptible VMs**:
- Discount: 80% off regular price
- Risk: Max 24 hours, can be terminated anytime

**Savings** (3 nodes):
- Regular: $73/month
- Preemptible: $15/month (**save $58**)

**Recommendation**:
- **Production**: Mixed (70% On-Demand, 30% Spot) for balance
- **Staging/Dev**: 100% Spot/Preemptible (**maximum savings**)

---

### Strategy 3: Reserved Instances / Committed Use Discounts

**AWS Reserved Instances**:
| Term | Discount | Upfront |
|------|----------|---------|
| 1 year, All upfront | 40% | $655 |
| 1 year, Partial upfront | 38% | $337 |
| 3 years, All upfront | 62% | $1,181 |

**Savings** (3x t3.medium, 1 year):
- On-Demand: $1,093/year
- Reserved: $655/year
- **Save**: $438/year ($36/month)

**When to use**:
- Stable production workload (> 6 months)
- Known capacity needs
- Can commit upfront payment

**GCP Committed Use Discounts**:
| Term | Discount |
|------|----------|
| 1 year | 37% |
| 3 years | 55% |

**Savings** (3x e2-medium, 1 year):
- On-Demand: $874/year
- Committed: $551/year
- **Save**: $323/year ($27/month)

**GCP Sustained Use Discounts** (automatic):
- No commitment required
- Automatically applied for sustained usage
- Up to 30% discount

---

### Strategy 4: Right-Size Resources

**Problem**: Over-provisioned resources waste money.

**Solution**: Start small, scale up as needed.

**Sizing guide**:

| Traffic | Peers | Instances | CPU/Instance | Memory/Instance | AWS Cost | GCP Cost |
|---------|-------|-----------|--------------|-----------------|----------|----------|
| **Low** | < 300 | 1 | 1 vCPU | 512 MB | $30 | $6 |
| **Medium** | 300-1000 | 2-3 | 1 vCPU | 1 GB | $60 | $18 |
| **High** | 1000-3000 | 3-5 | 2 vCPU | 2 GB | $152 | $50 |
| **Very High** | 3000+ | 5-10 | 2 vCPU | 4 GB | $304 | $100 |

**Instance types by traffic**:

**AWS**:
| Traffic | Instance Type | vCPU | RAM | Cost/Month |
|---------|--------------|------|-----|------------|
| Low | t3.micro | 2 | 1 GB | $7.59 |
| Medium | t3.small | 2 | 2 GB | $15.18 |
| High | t3.medium | 2 | 4 GB | $30.37 |
| Very High | t3.large | 2 | 8 GB | $60.74 |

**GCP Autopilot**: Automatically right-sized (no manual tuning)

**Optimization process**:
1. Start with minimum resources
2. Monitor CPU/memory usage (Prometheus)
3. Scale up if usage > 70%
4. Scale down if usage < 30% (for 7+ days)

---

### Strategy 5: Optimize Networking

**Single NAT Gateway** (non-production):
- Production: 3 NAT = $96/month
- Staging: 1 NAT = $32/month (**save $64**)
- Development: 1 NAT = $32/month (**save $64**)

**Terraform configuration**:
```hcl
# main.tf
single_nat_gateway = var.environment != "production"
```

**Reduce data transfer**:
1. **Single-AZ** for non-production (save inter-AZ cost)
2. **CloudFront** for static content (lower egress)
3. **Compression** for metrics/logs (reduce volume)

**Savings**: $50-100/month for multi-environment setup

---

### Strategy 6: Consolidate Environments

**Problem**: Separate clusters for dev/staging/prod multiply costs.

**Solution**: Use Kubernetes namespaces instead.

**Cost comparison** (AWS):
| Configuration | Clusters | Control Plane | Worker Nodes | Total |
|--------------|----------|---------------|--------------|-------|
| **Separate** | 3 | $219 | $273 | $492 |
| **Shared** | 1 | $73 | $91 | $164 |
| **Savings** | — | $146 | $182 | **$328** |

**When to consolidate**:
- ✅ Development and staging (definitely)
- ⚠️ Production separate (recommended for isolation)

**When NOT to consolidate**:
- ❌ Different regions (latency, compliance)
- ❌ Different security zones (PCI, HIPAA)
- ❌ Very high traffic (noisy neighbor risk)

---

### Strategy 7: Use Auto-Scaling Efficiently

**HPA (Horizontal Pod Autoscaler)**:
- Scales pods based on CPU/memory
- Target: 70% CPU, 80% memory
- **Benefit**: Pay only for actual usage

**Cluster Autoscaler**:
- Scales nodes based on pending pods
- Adds nodes when pods can't schedule
- Removes nodes when utilization < 50%
- **Benefit**: No idle nodes

**Cost savings example**:
- Fixed 5 nodes: $152/month
- Auto-scaled (avg 3 nodes): $91/month
- **Save**: $61/month (40%)

**Configuration** (already included in IaC):
```yaml
# HPA
minReplicas: 3
maxReplicas: 10
targetCPUUtilizationPercentage: 70

# Cluster Autoscaler
minSize: 3
maxSize: 10
```

---

### Strategy 8: Eliminate Unused Resources

**Common waste**:
1. ❌ Stopped EC2 instances (still charged for storage)
2. ❌ Unattached EBS volumes
3. ❌ Elastic IPs not associated
4. ❌ Old snapshots/AMIs
5. ❌ Load balancers with no targets

**Audit script** (AWS):
```bash
# Find stopped instances
aws ec2 describe-instances --filters "Name=instance-state-name,Values=stopped"

# Find unattached volumes
aws ec2 describe-volumes --filters "Name=status,Values=available"

# Find unused Elastic IPs
aws ec2 describe-addresses --query 'Addresses[?AssociationId==null]'
```

**Savings**: $20-50/month (typical cleanup)

---

## Cost Calculators

### Monthly Cost Formula

**AWS EKS**:
```
Total = Control_Plane + Compute + Network + Storage + Transfer

Where:
  Control_Plane = $73
  Compute = Nodes × Instance_Cost × (1 - Spot_Percentage × 0.7)
  Network = NAT_Count × $32.85 + LB_Cost × $16.43
  Storage = Total_GB × $0.08
  Transfer = Outbound_GB × $0.09
```

**GCP Autopilot**:
```
Total = Pod_Resources + Network + Storage + Transfer

Where:
  Pod_Resources = Replicas × (vCPU × $0.0445 + GB_RAM × $0.00489) × 730
  Network = LB_Cost × $18
  Storage = Total_GB × $0.04
  Transfer = Outbound_GB × $0.12
```

---

### Interactive Calculator (Examples)

**Scenario 1: Small Deployment (1000 concurrent peers)**

**AWS**:
```
Control Plane:          $73.00
Compute (3× t3.small):  $45.54
NAT (1×):              $32.85
Load Balancer:         $16.43
Storage (60 GB):       $4.80
Transfer (50 GB):      $4.50
─────────────────────────────
Total:                 $177.12/month
```

**GCP Autopilot**:
```
Pod Resources (3×):    $11.08
Load Balancer:         $18.00
Storage (10 GB):       $0.40
Transfer (50 GB):      $6.00
─────────────────────────────
Total:                 $35.48/month
```

**Winner**: GCP (**80% cheaper**, $35 vs $177)

---

**Scenario 2: Medium Deployment (5000 concurrent peers)**

**AWS**:
```
Control Plane:          $73.00
Compute (5× t3.medium): $151.85
NAT (3×):              $98.55
Load Balancer:         $16.43
Storage (100 GB):      $8.00
Transfer (250 GB):     $22.50
─────────────────────────────
Total:                 $370.33/month
```

**GCP Autopilot**:
```
Pod Resources (5×):    $18.47
Load Balancer:         $18.00
Storage (20 GB):       $0.80
Transfer (250 GB):     $30.00
─────────────────────────────
Total:                 $67.27/month
```

**Winner**: GCP (**82% cheaper**, $67 vs $370)

---

**Scenario 3: Large Deployment (20,000 concurrent peers)**

**AWS**:
```
Control Plane:          $73.00
Compute (15× t3.large): $911.10
NAT (3×):              $98.55
Load Balancer:         $16.43
Storage (300 GB):      $24.00
Transfer (1000 GB):    $90.00
─────────────────────────────
Total:                 $1,213.08/month
```

**GCP Autopilot**:
```
Pod Resources (15×):   $55.41
Load Balancer:         $18.00
Storage (60 GB):       $2.40
Transfer (1000 GB):    $120.00
─────────────────────────────
Total:                 $195.81/month
```

**Winner**: GCP (**84% cheaper**, $196 vs $1,213)

---

## Right-Sizing Guide

### Step 1: Measure Current Usage

**Monitor for 7 days**:
```bash
# CPU usage
kubectl top nodes
kubectl top pods -n file-relay

# Prometheus query (average over 7 days)
avg_over_time(node_cpu_usage_percentage[7d])
avg_over_time(container_memory_usage_bytes[7d])
```

### Step 2: Calculate Target Resources

**Formula**:
```
Target = Current_Usage × 1.3  (30% headroom)

If Target < Current_Allocation:
  → Scale down
Else if Target > Current_Allocation × 0.7:
  → Scale up
Else:
  → No change
```

**Example**:
```
Current allocation: 2 vCPU, 4 GB RAM
Actual usage: 0.5 vCPU, 1 GB RAM
Target: 0.65 vCPU, 1.3 GB RAM

→ Can scale down to 1 vCPU, 2 GB RAM
→ Savings: 50%
```

### Step 3: Right-Size Instances

**AWS instance selection**:
| Target vCPU | Target RAM | Instance Type | Cost/Month |
|-------------|-----------|---------------|------------|
| < 1 | < 1 GB | t3.micro | $7.59 |
| < 2 | < 2 GB | t3.small | $15.18 |
| < 2 | < 4 GB | t3.medium | $30.37 |
| < 2 | < 8 GB | t3.large | $60.74 |
| < 4 | < 16 GB | t3.xlarge | $121.47 |

**GCP Autopilot**: Automatically right-sized

---

## Cost-Performance Trade-offs

### Trade-off Matrix

| Optimization | Cost Savings | Performance Impact | Risk | Recommended For |
|-------------|--------------|-------------------|------|-----------------|
| **GCP Autopilot** | 70-80% | None | Low | Everyone |
| **Spot Instances** | 50-70% | None (with proper handling) | Medium | Non-critical or mixed fleet |
| **Reserved Instances** | 30-40% | None | Low (commitment) | Stable workloads |
| **Single NAT** | $64/month | None (non-prod) | Low | Staging/dev only |
| **Consolidate clusters** | $328/month | Slight (noisy neighbor) | Medium | Dev/staging only |
| **Smaller instances** | 30-50% | Potential (if too small) | Medium | Monitor closely |
| **Single-AZ** | $2-5/month | Lower availability | High | Dev/test only |

### When to Prioritize Cost

✅ **Optimize aggressively**:
- Development environments
- Staging environments
- Proof-of-concept
- Low-traffic production (< 1000 peers)

### When to Prioritize Performance

⚠️ **Optimize carefully**:
- High-traffic production (> 5000 peers)
- SLA requirements (99.9%+)
- Customer-facing services
- Revenue-generating workloads

### Balanced Approach

**Recommended strategy**:
1. Start with GCP Autopilot (automatic optimization)
2. Use multi-AZ for production (high availability)
3. Use Spot for 30% of production fleet (cost savings with safety)
4. Monitor and adjust monthly

---

## Cost Monitoring

### Set Up Cost Alerts

**AWS**:
```bash
# Create budget
aws budgets create-budget \
  --account-id 123456789012 \
  --budget file://budget.json \
  --notifications-with-subscribers file://notifications.json
```

**budget.json**:
```json
{
  "BudgetName": "FILE-Relay-Monthly",
  "BudgetLimit": {
    "Amount": "300",
    "Unit": "USD"
  },
  "TimeUnit": "MONTHLY",
  "BudgetType": "COST"
}
```

**GCP**:
```bash
# Create budget alert
gcloud billing budgets create \
  --billing-account=BILLING_ACCOUNT_ID \
  --display-name="FILE Relay Budget" \
  --budget-amount=100USD \
  --threshold-rule=percent=80 \
  --threshold-rule=percent=100
```

### Track Cost by Tag

**AWS tags**:
```hcl
tags = {
  Project     = "FILE-Relay"
  Environment = "production"
  ManagedBy   = "Terraform"
  CostCenter  = "Infrastructure"
}
```

**Cost Explorer query**:
```bash
aws ce get-cost-and-usage \
  --time-period Start=2024-01-01,End=2024-01-31 \
  --granularity MONTHLY \
  --metrics BlendedCost \
  --group-by Type=TAG,Key=Project
```

### Monthly Cost Review Checklist

- [ ] Compare actual vs budget
- [ ] Identify cost spikes (investigate)
- [ ] Review resource utilization (right-size)
- [ ] Check for unused resources (cleanup)
- [ ] Evaluate Reserved Instance utilization
- [ ] Review Spot Instance savings
- [ ] Update capacity forecast

---

## Decision Trees

### Which Cloud Provider?

```
Start → Already on AWS?
        ├─ Yes → Migrate or Optimize?
        │        ├─ Migrate → GCP Autopilot (save 70%)
        │        └─ Optimize → Spot + Reserved + Single NAT (save 40%)
        └─ No → GCP Autopilot (lowest cost)
```

### Spot vs On-Demand?

```
Start → Production workload?
        ├─ Yes → Critical?
        │        ├─ Yes → Mixed (70% On-Demand, 30% Spot)
        │        └─ No → 100% Spot
        └─ No → 100% Spot (staging/dev)
```

### Reserved Instances?

```
Start → Stable workload (> 6 months)?
        ├─ Yes → Can pay upfront?
        │        ├─ Yes → 1-year All-Upfront (40% discount)
        │        └─ No → 1-year Partial-Upfront (38% discount)
        └─ No → On-Demand or Spot
```

### How Many NAT Gateways?

```
Start → Production?
        ├─ Yes → Multi-AZ (3 NAT, high availability)
        └─ No → Single-AZ (1 NAT, save $64/month)
```

---

## Summary: Maximum Savings

**Scenario**: 1000 concurrent peers, production

### AWS (Optimized)

**Before optimization**:
- Standard config: $300/month

**After optimization**:
```
Control Plane:         $73.00
Compute (3× t3.small, 30% Spot): $31.85
NAT (1×):             $32.85
Load Balancer:        $16.43
Storage:              $4.80
Transfer:             $4.50
───────────────────────────────
Total:                $163.43/month
Savings:              $136.57 (45%)
```

**Optimizations applied**:
1. ✅ Smaller instances (t3.medium → t3.small)
2. ✅ 30% Spot instances
3. ✅ Single NAT (was 3)
4. ✅ Right-sized storage

---

### GCP (Default)

**Already optimized** (Autopilot):
```
Total: $35.48/month
```

**No further optimization needed**.

---

### Winner

**GCP Autopilot**: $35/month  
**AWS Optimized**: $163/month

**GCP is 78% cheaper** even after aggressive AWS optimization.

---

## Further Reading

- [Capacity Planning](CAPACITY-PLANNING.md) — Resource sizing formulas
- [Performance Tuning](PERFORMANCE-TUNING.md) — Optimize performance
- [Multi-Region Guide](MULTI-REGION-GUIDE.md) — Geographic distribution costs
- [AWS Cost Optimization](https://aws.amazon.com/pricing/cost-optimization/)
- [GCP Cost Management](https://cloud.google.com/cost-management)

---

**Questions?** [Open an issue](https://github.com/file-network/relay-server/issues) or consult the [FAQ](../FAQ.md).
