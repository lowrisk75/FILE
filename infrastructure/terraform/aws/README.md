# FILE Relay Server - AWS Terraform Deployment

Complete Infrastructure as Code for deploying FILE Relay Server on AWS EKS with production-grade configuration.

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Prerequisites](#prerequisites)
4. [Quick Start](#quick-start)
5. [Configuration](#configuration)
6. [Deployment](#deployment)
7. [Post-Deployment](#post-deployment)
8. [Monitoring](#monitoring)
9. [Scaling](#scaling)
10. [Cost Management](#cost-management)
11. [Troubleshooting](#troubleshooting)
12. [Cleanup](#cleanup)

---

## Overview

This Terraform configuration deploys a complete FILE Relay Server stack on AWS:

- **VPC**: Multi-AZ networking with public/private subnets
- **EKS**: Managed Kubernetes cluster with worker nodes
- **FILE Relay Server**: Deployed as Kubernetes Deployment with HPA and PDB
- **Monitoring**: Prometheus, Grafana, and Alertmanager (optional)
- **Load Balancing**: Network Load Balancer for UDP traffic
- **Autoscaling**: Horizontal Pod Autoscaler and Cluster Autoscaler
- **Security**: Pod security contexts, network policies, IRSA

**Estimated deployment time**: 15-20 minutes

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                           AWS Region                            │
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │                      VPC (10.0.0.0/16)                   │ │
│  │                                                           │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │ │
│  │  │   AZ-1       │  │   AZ-2       │  │   AZ-3       │  │ │
│  │  │              │  │              │  │              │  │ │
│  │  │  ┌────────┐  │  │  ┌────────┐  │  │  ┌────────┐  │  │ │
│  │  │  │ Public │  │  │  │ Public │  │  │  │ Public │  │  │ │
│  │  │  │ Subnet │  │  │  │ Subnet │  │  │  │ Subnet │  │  │ │
│  │  │  └───┬────┘  │  │  └───┬────┘  │  │  └───┬────┘  │  │ │
│  │  │      │       │  │      │       │  │      │       │  │ │
│  │  │  ┌───▼────┐  │  │  ┌───▼────┐  │  │  ┌───▼────┐  │  │ │
│  │  │  │  NAT   │  │  │  │  NAT   │  │  │  │  NAT   │  │  │ │
│  │  │  │Gateway │  │  │  │Gateway │  │  │  │Gateway │  │  │ │
│  │  │  └───┬────┘  │  │  └───┬────┘  │  │  └───┬────┘  │  │ │
│  │  │      │       │  │      │       │  │      │       │  │ │
│  │  │  ┌───▼────┐  │  │  ┌───▼────┐  │  │  ┌───▼────┐  │  │ │
│  │  │  │Private │  │  │  │Private │  │  │  │Private │  │  │ │
│  │  │  │ Subnet │  │  │  │ Subnet │  │  │  │ Subnet │  │  │ │
│  │  │  │        │  │  │  │        │  │  │  │        │  │  │ │
│  │  │  │ ┌────┐ │  │  │  │ ┌────┐ │  │  │  │ ┌────┐ │  │  │ │
│  │  │  │ │EKS │ │  │  │  │ │EKS │ │  │  │  │ │EKS │ │  │  │ │
│  │  │  │ │Node│ │  │  │  │ │Node│ │  │  │  │ │Node│ │  │  │ │
│  │  │  │ └────┘ │  │  │  │ └────┘ │  │  │  │ └────┘ │  │  │ │
│  │  │  └────────┘  │  │  └────────┘  │  │  └────────┘  │  │ │
│  │  └──────────────┘  └──────────────┘  └──────────────┘  │ │
│  └───────────────────────────────────────────────────────────┘ │
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │                   EKS Control Plane                       │ │
│  └───────────────────────────────────────────────────────────┘ │
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │                 Network Load Balancer                     │ │
│  │                  (UDP Port 8080)                          │ │
│  └───────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
                         Internet
```

**Inside EKS**:
- `file-relay` namespace: FILE Relay Server pods (3-10 replicas with HPA)
- `monitoring` namespace: Prometheus, Grafana, Alertmanager
- `kube-system` namespace: AWS Load Balancer Controller, Cluster Autoscaler

---

## Prerequisites

### Required Tools

```bash
# Terraform
terraform --version  # >= 1.0

# AWS CLI
aws --version  # >= 2.0
aws configure  # Set credentials

# kubectl
kubectl version --client  # >= 1.28

# Helm (for manual chart management)
helm version  # >= 3.0
```

### AWS Permissions

Your IAM user/role needs permissions for:
- VPC, EC2, NAT Gateway
- EKS, IAM (IRSA roles)
- ELB/NLB
- CloudWatch Logs
- S3 (for Terraform state)

Recommended managed policies:
- `AmazonEKSClusterPolicy`
- `AmazonEKSWorkerNodePolicy`
- `AmazonEC2ContainerRegistryReadOnly`
- `AmazonEKS_CNI_Policy`
- `AmazonEKSVPCResourceController`

### S3 Backend (Recommended)

Create an S3 bucket for Terraform state:

```bash
aws s3api create-bucket \
  --bucket my-terraform-state-file-relay \
  --region us-east-1

aws s3api put-bucket-versioning \
  --bucket my-terraform-state-file-relay \
  --versioning-configuration Status=Enabled

aws s3api put-bucket-encryption \
  --bucket my-terraform-state-file-relay \
  --server-side-encryption-configuration '{
    "Rules": [{
      "ApplyServerSideEncryptionByDefault": {
        "SSEAlgorithm": "AES256"
      }
    }]
  }'
```

---

## Quick Start

### 1. Clone and Configure

```bash
cd infrastructure/terraform/aws

# Copy example variables
cp terraform.tfvars.example terraform.tfvars

# Edit variables
vim terraform.tfvars
```

**Minimum `terraform.tfvars`**:

```hcl
aws_region  = "us-east-1"
environment = "production"

grafana_admin_password = "YourSecurePassword123!"

# Optional: Alerting
alert_slack_webhook_url      = "https://hooks.slack.com/services/YOUR/WEBHOOK/URL"
alert_pagerduty_service_key  = "your-pagerduty-integration-key"
```

### 2. Initialize Terraform

```bash
terraform init \
  -backend-config="bucket=my-terraform-state-file-relay" \
  -backend-config="key=file-relay/production/terraform.tfstate" \
  -backend-config="region=us-east-1"
```

### 3. Plan

```bash
terraform plan -out=tfplan
```

Review the plan carefully. Expected resources:
- ~80-100 resources (VPC, EKS, K8s resources, monitoring)

### 4. Apply

```bash
terraform apply tfplan
```

**Wait**: 15-20 minutes for EKS cluster and node group provisioning.

### 5. Configure kubectl

```bash
aws eks update-kubeconfig --region us-east-1 --name file-relay-production

kubectl get nodes
kubectl get pods -n file-relay
```

### 6. Get LoadBalancer Hostname

```bash
terraform output relay_loadbalancer_hostname
```

### 7. Test Health

```bash
LB=$(terraform output -raw relay_loadbalancer_hostname)
curl http://${LB}:8081/health
```

Expected: `{"status":"ok"}`

---

## Configuration

### Variable Reference

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `aws_region` | AWS region | `us-east-1` | No |
| `environment` | Environment (production/staging/development) | `production` | No |
| `project_name` | Project name for resource naming | `file-relay` | No |
| `vpc_cidr` | VPC CIDR block | `10.0.0.0/16` | No |
| `availability_zones` | AZs for multi-AZ deployment | `[us-east-1a, us-east-1b, us-east-1c]` | No |
| `cluster_version` | Kubernetes version | `1.28` | No |
| `node_instance_types` | EC2 instance types | `[t3.medium]` | No |
| `node_desired_size` | Desired number of worker nodes | `3` | No |
| `node_min_size` | Minimum number of worker nodes | `3` | No |
| `node_max_size` | Maximum number of worker nodes | `10` | No |
| `relay_image_repository` | Docker image repository | `ghcr.io/file-network/relay-server` | No |
| `relay_image_tag` | Docker image tag | `latest` | No |
| `relay_replicas` | Number of relay replicas | `3` | No |
| `relay_max_peers` | Max peers per instance | `1000` | No |
| `relay_port` | UDP port | `8080` | No |
| `relay_metrics_port` | Metrics HTTP port | `8081` | No |
| `hpa_enabled` | Enable HPA | `true` | No |
| `hpa_min_replicas` | HPA min replicas | `3` | No |
| `hpa_max_replicas` | HPA max replicas | `10` | No |
| `monitoring_enabled` | Deploy Prometheus/Grafana | `true` | No |
| `alertmanager_enabled` | Deploy Alertmanager | `true` | No |
| `grafana_admin_password` | Grafana admin password | `changeme` | **Yes** |
| `alert_slack_webhook_url` | Slack webhook for alerts | `""` | No |
| `alert_pagerduty_service_key` | PagerDuty integration key | `""` | No |

### Environment-Specific Configurations

**Production**:
```hcl
environment         = "production"
node_desired_size   = 3
node_min_size       = 3
node_max_size       = 10
relay_replicas      = 3
hpa_min_replicas    = 3
hpa_max_replicas    = 10
monitoring_enabled  = true
```

**Staging**:
```hcl
environment         = "staging"
node_desired_size   = 2
node_min_size       = 2
node_max_size       = 5
relay_replicas      = 2
hpa_min_replicas    = 2
hpa_max_replicas    = 5
monitoring_enabled  = true
```

**Development**:
```hcl
environment         = "development"
node_desired_size   = 1
node_min_size       = 1
node_max_size       = 3
relay_replicas      = 1
hpa_min_replicas    = 1
hpa_max_replicas    = 3
monitoring_enabled  = false  # Reduce cost
```

---

## Deployment

### Initial Deployment

```bash
# 1. Plan
terraform plan -out=tfplan

# 2. Apply
terraform apply tfplan

# 3. Verify
kubectl get pods -n file-relay -w
```

### Updating Configuration

```bash
# Edit variables
vim terraform.tfvars

# Plan changes
terraform plan -out=tfplan

# Apply (rolling update)
terraform apply tfplan
```

### Upgrading Relay Server Version

```bash
# Update image tag
relay_image_tag = "v0.2.0"

# Apply (triggers rolling update)
terraform apply

# Watch rollout
kubectl rollout status deployment/file-relay-server -n file-relay
```

### Manual Rollback

```bash
kubectl rollout undo deployment/file-relay-server -n file-relay
```

---

## Post-Deployment

### DNS Configuration

Point your domain to the LoadBalancer:

```bash
# Get hostname
LB_HOSTNAME=$(terraform output -raw relay_loadbalancer_hostname)
echo $LB_HOSTNAME

# Create Route 53 CNAME record
aws route53 change-resource-record-sets \
  --hosted-zone-id Z1234567890ABC \
  --change-batch "{
    \"Changes\": [{
      \"Action\": \"UPSERT\",
      \"ResourceRecordSet\": {
        \"Name\": \"relay.example.com\",
        \"Type\": \"CNAME\",
        \"TTL\": 60,
        \"ResourceRecords\": [{\"Value\": \"$LB_HOSTNAME\"}]
      }
    }]
  }"
```

### Verify Health

```bash
# Health endpoint
curl http://relay.example.com:8081/health

# Readiness endpoint
curl http://relay.example.com:8081/ready

# Metrics
curl http://relay.example.com:8081/metrics
```

### Access Grafana

```bash
# Port-forward
kubectl port-forward -n monitoring svc/kube-prometheus-stack-grafana 3000:80

# Open browser
open http://localhost:3000

# Login
Username: admin
Password: [from terraform.tfvars]
```

**Dashboards**:
- FILE Relay Server - Overview
- Kubernetes / Compute Resources / Cluster
- Kubernetes / Compute Resources / Namespace (Pods)

---

## Monitoring

### Metrics

All FILE Relay metrics are automatically scraped by Prometheus.

**Key metrics**:
```promql
# Active peers
file_relay_active_peers

# Packet forwarding rate
rate(file_relay_packets_forwarded_total[5m])

# Success rate
(rate(file_relay_peers_registered_total[5m]) / 
 (rate(file_relay_peers_registered_total[5m]) + 
  rate(file_relay_capow_rejected_total[5m]) + 
  rate(file_relay_rate_limited_total[5m]))) * 100
```

### Alerts

All production alerts are configured automatically:

**Critical (P1)**:
- FileRelayServerDown
- FileRelayCapacityReached
- FileRelayCAPoWAttack
- FileRelayRateLimitAttack

**Warning (P2)**:
- FileRelayHighPeerCount
- FileRelayHighCAPoWRejectionRate
- FileRelayHighPeerTimeouts
- FileRelayPodRestarting
- FileRelayHighMemoryUsage
- FileRelayHighCPUUsage

Alerts are routed to:
- **Slack**: All warnings and critical alerts
- **PagerDuty**: Critical alerts only

### Logs

```bash
# All relay pods
kubectl logs -n file-relay -l app=file-relay-server --tail=100 -f

# Specific pod
kubectl logs -n file-relay file-relay-server-abc123 -f

# Previous container (after crash)
kubectl logs -n file-relay file-relay-server-abc123 --previous
```

### Access Prometheus

```bash
kubectl port-forward -n monitoring svc/kube-prometheus-stack-prometheus 9090:9090
open http://localhost:9090
```

### Access Alertmanager

```bash
kubectl port-forward -n monitoring svc/kube-prometheus-stack-alertmanager 9093:9093
open http://localhost:9093
```

---

## Scaling

### Horizontal Scaling (Pods)

**Automatic** (HPA):
- Scales based on CPU and memory utilization
- Default targets: 70% CPU, 80% memory
- Min replicas: 3, Max replicas: 10

**Manual**:
```bash
kubectl scale deployment file-relay-server --replicas=5 -n file-relay
```

### Vertical Scaling (Resources)

Update resource limits in `terraform.tfvars`:

```hcl
relay_cpu_request    = "200m"
relay_cpu_limit      = "2000m"
relay_memory_request = "256Mi"
relay_memory_limit   = "1Gi"
```

Apply:
```bash
terraform apply
```

### Node Scaling

**Automatic** (Cluster Autoscaler):
- Scales nodes based on pending pods
- Min nodes: 3, Max nodes: 10

**Manual** (increase max nodes):
```hcl
node_max_size = 15
```

Apply:
```bash
terraform apply
```

### Capacity Planning

See `docs/deployment/CAPACITY-PLANNING.md` for detailed guidelines.

**Rule of thumb**:
- 1 relay pod (100m CPU, 128Mi RAM) = ~1000 peers
- 1 t3.medium node = ~8 relay pods
- For 10,000 peers: 10 pods across 2 nodes (with headroom)

---

## Cost Management

### Estimated Costs

See `terraform output estimated_monthly_cost` for current breakdown.

**Production baseline** (us-east-1):
- EKS Control Plane: $73/month
- 3x t3.medium nodes: ~$90/month
- 3x NAT Gateways: ~$96/month
- NLB: ~$18/month
- EBS volumes: ~$8/month
- Total: **~$285/month**

### Cost Optimization

**1. Use Spot Instances** (50-70% discount):

In `eks.tf`, change:
```hcl
capacity_type = "SPOT"
```

**2. Single NAT Gateway** (non-production):

In `terraform.tfvars`:
```hcl
environment = "staging"  # Automatically uses single NAT
```

Or in `main.tf`:
```hcl
single_nat_gateway = true
```

Saves: ~$64/month

**3. Reduce Monitoring** (development):

```hcl
monitoring_enabled = false
```

Saves: ~$10/month

**4. Reserved Instances** (1-year commit, 40% discount):

Purchase Reserved Instances for predictable workloads:
```bash
aws ec2 purchase-reserved-instances-offering \
  --instance-count 3 \
  --reserved-instances-offering-id <offering-id>
```

**5. Right-size Nodes**:

Monitor actual usage and adjust `node_instance_types`:
- `t3.small`: ~$15/month (suitable for < 5 relay pods)
- `t3.medium`: ~$30/month (default)
- `t3.large`: ~$60/month (for high CPU workloads)

### Cost Monitoring

Enable AWS Cost Explorer tags:

```bash
aws ce get-tags --time-period Start=2024-01-01,End=2024-12-31 \
  --tag-key Environment
```

---

## Troubleshooting

### Pods Not Starting

```bash
# Check pod status
kubectl get pods -n file-relay

# Describe pod
kubectl describe pod <pod-name> -n file-relay

# Check events
kubectl get events -n file-relay --sort-by='.lastTimestamp'
```

Common issues:
- **ImagePullBackOff**: Check `relay_image_repository` and `relay_image_tag`
- **CrashLoopBackOff**: Check logs with `kubectl logs`
- **Pending**: Node capacity issue, check `kubectl describe node`

### LoadBalancer Stuck in Pending

```bash
# Check service
kubectl describe svc file-relay-udp -n file-relay

# Check AWS Load Balancer Controller
kubectl logs -n kube-system -l app.kubernetes.io/name=aws-load-balancer-controller
```

Fix:
- Ensure AWS Load Balancer Controller is running
- Check IAM permissions for IRSA role
- Verify subnet tags (should be set automatically by VPC module)

### HPA Not Scaling

```bash
# Check HPA status
kubectl get hpa -n file-relay
kubectl describe hpa file-relay-hpa -n file-relay

# Check metrics-server
kubectl top nodes
kubectl top pods -n file-relay
```

Fix:
- Ensure metrics-server is running (deployed by kube-prometheus-stack)
- Check resource requests/limits are set on pods

### Terraform State Lock

```bash
# If stuck on "Acquiring state lock"
terraform force-unlock <lock-id>
```

### Connection Timeouts

```bash
# Check security groups
aws ec2 describe-security-groups \
  --filters "Name=tag:Name,Values=*file-relay*"

# Check NLB target health
aws elbv2 describe-target-health \
  --target-group-arn <arn>
```

### High Costs

```bash
# Identify cost drivers
aws ce get-cost-and-usage \
  --time-period Start=2024-01-01,End=2024-01-31 \
  --granularity MONTHLY \
  --metrics BlendedCost \
  --group-by Type=TAG,Key=Project
```

---

## Cleanup

### Destroy All Resources

```bash
# Plan destroy
terraform plan -destroy -out=destroy.tfplan

# Review carefully
terraform show destroy.tfplan

# Destroy
terraform destroy

# Confirm: yes
```

**Warning**: This deletes:
- All relay server data (stateless, OK)
- Prometheus metrics history
- Grafana dashboards
- EKS cluster
- VPC and networking

### Preserve Monitoring Data

Before destroying, export Prometheus data:

```bash
# Port-forward
kubectl port-forward -n monitoring svc/kube-prometheus-stack-prometheus 9090:9090

# Snapshot (requires promtool)
promtool tsdb create-blocks-from openmetrics \
  http://localhost:9090/api/v1/query \
  --output-dir ./prometheus-backup
```

### Cleanup S3 Terraform State (Optional)

```bash
# Remove state file
aws s3 rm s3://my-terraform-state-file-relay/file-relay/production/terraform.tfstate

# Delete bucket
aws s3api delete-bucket \
  --bucket my-terraform-state-file-relay \
  --region us-east-1
```

---

## Further Reading

- [Deployment Guide](../../../docs/deployment/DEPLOYMENT-GUIDE.md) — General deployment guide
- [Operations Runbook](../../../docs/deployment/OPERATIONS-RUNBOOK.md) — Day-2 operations
- [Multi-Region Guide](../../../docs/deployment/MULTI-REGION-GUIDE.md) — Multi-region deployment
- [Alert Runbook](../../../docs/deployment/ALERT-RUNBOOK.md) — Alert response procedures
- [AWS EKS Best Practices](https://aws.github.io/aws-eks-best-practices/) — AWS official guide

---

**Questions?** [Open an issue](https://github.com/file-network/relay-server/issues) or consult the [FAQ](../../../docs/FAQ.md).
