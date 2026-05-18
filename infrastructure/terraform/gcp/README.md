# FILE Relay Server - GCP Terraform Deployment

Complete Infrastructure as Code for deploying FILE Relay Server on Google Kubernetes Engine (GKE).

---

## Quick Start

```bash
cd infrastructure/terraform/gcp

# Authenticate
gcloud auth application-default login

# Copy and edit configuration
cp terraform.tfvars.example terraform.tfvars
vim terraform.tfvars

# Initialize
terraform init

# Plan and apply
terraform plan -out=tfplan
terraform apply tfplan

# Configure kubectl
gcloud container clusters get-credentials file-relay-production --region us-east1

# Test
kubectl get pods -n file-relay
```

---

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                      GCP Project                             │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │                 VPC Network                            │ │
│  │                                                        │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌────────────┐  │ │
│  │  │  us-east1-b  │  │  us-east1-c  │  │ us-east1-d │  │ │
│  │  │              │  │              │  │            │  │ │
│  │  │  ┌────────┐  │  │  ┌────────┐  │  │ ┌────────┐ │  │ │
│  │  │  │  GKE   │  │  │  │  GKE   │  │  │ │  GKE   │ │  │ │
│  │  │  │  Node  │  │  │  │  Node  │  │  │ │  Node  │ │  │ │
│  │  │  └────────┘  │  │  └────────┘  │  │ └────────┘ │  │ │
│  │  └──────────────┘  └──────────────┘  └────────────┘  │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │           GKE Control Plane (Managed)                  │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │   Global Load Balancer (UDP - Premium Tier)           │ │
│  │          (Anycast IP, Global Routing)                  │ │
│  └────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
                            │
                            ▼
                       Internet
```

**Key differences from AWS**:
- **GKE Autopilot** option: Fully managed nodes (recommended)
- **Global Load Balancer**: Anycast IP with automatic geo-routing
- **Cloud NAT**: Managed NAT gateway per region
- **Workload Identity**: Native IAM integration (no IRSA needed)

---

## Prerequisites

### Required Tools

```bash
# Terraform
terraform --version  # >= 1.0

# gcloud CLI
gcloud --version  # >= 400.0

# kubectl
kubectl version --client  # >= 1.28

# Authenticate
gcloud auth application-default login

# Set project
gcloud config set project YOUR-PROJECT-ID
```

### GCP Permissions

Required roles:
- `roles/container.admin` (GKE cluster management)
- `roles/compute.networkAdmin` (VPC, firewall, load balancer)
- `roles/iam.serviceAccountAdmin` (Workload Identity)
- `roles/monitoring.admin` (Cloud Monitoring)
- `roles/logging.admin` (Cloud Logging)

### Enable APIs

```bash
gcloud services enable \
  container.googleapis.com \
  compute.googleapis.com \
  cloudresourcemanager.googleapis.com \
  monitoring.googleapis.com \
  logging.googleapis.com
```

### GCS Backend (Recommended)

```bash
# Create bucket for Terraform state
gsutil mb -p YOUR-PROJECT-ID -c STANDARD -l US gs://YOUR-PROJECT-terraform-state

# Enable versioning
gsutil versioning set on gs://YOUR-PROJECT-terraform-state
```

---

## Configuration

### Minimum `terraform.tfvars`

```hcl
gcp_project_id = "your-project-id"
gcp_region     = "us-east1"
environment    = "production"

grafana_admin_password = "YourSecurePassword123!"
```

### GKE Mode: Standard vs Autopilot

**GKE Autopilot** (Recommended):
- Fully managed nodes
- Pay-per-pod pricing
- Lower operational overhead
- Best practices enforced

```hcl
gke_mode = "autopilot"
```

**GKE Standard**:
- Full control over nodes
- Manual node pool management
- More flexibility

```hcl
gke_mode = "standard"
node_machine_type = "e2-medium"
node_count_min    = 3
node_count_max    = 10
```

### Network Tier: Premium vs Standard

**Premium Tier** (Recommended for production):
- Global Anycast IP
- Automatic geo-routing
- Lower latency
- Higher cost: +$0.01/GB

```hcl
network_tier = "PREMIUM"
```

**Standard Tier** (Cost-optimized):
- Regional IP
- Lower cost
- Suitable for single-region deployments

```hcl
network_tier = "STANDARD"
```

---

## Deployment

```bash
# Initialize with GCS backend
terraform init \
  -backend-config="bucket=YOUR-PROJECT-terraform-state" \
  -backend-config="prefix=file-relay/production"

# Plan
terraform plan -out=tfplan

# Apply
terraform apply tfplan

# Get credentials
gcloud container clusters get-credentials \
  $(terraform output -raw cluster_name) \
  --region $(terraform output -raw region)

# Test
kubectl get pods -n file-relay
LB_IP=$(terraform output -raw loadbalancer_ip)
curl http://${LB_IP}:8081/health
```

---

## Key Features

### 1. Workload Identity

Native IAM integration (no IRSA complexity):

```yaml
serviceAccount:
  annotations:
    iam.gke.io/gcp-service-account: relay-server@PROJECT-ID.iam.gserviceaccount.com
```

### 2. Global Load Balancer

Anycast IP with automatic geo-routing:

```bash
# Users worldwide reach nearest GCP region automatically
# No DNS geolocation needed
```

### 3. GKE Autopilot

Fully managed nodes:
- No node pools to manage
- Automatic scaling
- Pay-per-pod billing
- Built-in security best practices

### 4. Cloud Monitoring

Native integration:
- Prometheus metrics auto-exported
- GKE Dashboard included
- No separate Prometheus deployment needed (optional)

---

## Cost Comparison

### GKE Autopilot (Recommended)

**Per-pod pricing**:
- 3 relay pods: ~$50/month
- Monitoring: ~$20/month
- Load Balancer: ~$18/month
- Egress (Premium): ~$20/month
- **Total: ~$108/month**

50% cheaper than AWS EKS ($285/month)

### GKE Standard

**Per-node pricing**:
- Control plane: FREE (< 2,500 vCPUs)
- 3x e2-medium nodes: ~$75/month
- Load Balancer: ~$18/month
- Egress: ~$20/month
- **Total: ~$113/month**

Still 60% cheaper than AWS EKS

---

## Monitoring

### Cloud Monitoring (Native)

```bash
# GKE Dashboard
gcloud console https://console.cloud.google.com/kubernetes/workload/overview?project=YOUR-PROJECT

# View metrics
gcloud monitoring time-series list \
  --filter='metric.type="kubernetes.io/container/cpu/core_usage_time"'
```

### Prometheus + Grafana (Optional)

```hcl
monitoring_enabled = true
```

Deploys kube-prometheus-stack (same as AWS).

### Logs

```bash
# Cloud Logging (automatic)
gcloud logging read "resource.type=k8s_container AND resource.labels.namespace_name=file-relay" --limit 100

# kubectl
kubectl logs -n file-relay -l app=file-relay-server --tail=100 -f
```

---

## Multi-Region

GCP Global Load Balancer supports multi-region natively:

```hcl
# Deploy to multiple regions
gcp_region = "us-east1"   # Primary
# Also deploy to: europe-west1, asia-southeast1

# Use same Anycast IP
# Traffic automatically routed to nearest region
```

See `docs/deployment/MULTI-REGION-GUIDE.md` for details.

---

## Troubleshooting

### Quota Exceeded

```bash
# Check quotas
gcloud compute project-info describe --project=YOUR-PROJECT

# Request increase
# Visit: https://console.cloud.google.com/iam-admin/quotas
```

### GKE Autopilot Rejected Pod

Autopilot enforces strict requirements:
- No privileged pods
- No hostPath volumes
- Resource requests required

Fix: FILE Relay Server is already Autopilot-compatible.

### LoadBalancer Pending

```bash
# Check service
kubectl describe svc file-relay-udp -n file-relay

# Check firewall rules
gcloud compute firewall-rules list --filter="name~file-relay"
```

---

## Cleanup

```bash
# Destroy all resources
terraform destroy

# Confirm: yes
```

**Warning**: Deletes all resources including monitoring data.

---

## Further Reading

- [AWS Terraform Guide](../aws/README.md) — Detailed explanations (applies to GCP)
- [GKE Best Practices](https://cloud.google.com/kubernetes-engine/docs/best-practices) — Google official guide
- [GKE Autopilot](https://cloud.google.com/kubernetes-engine/docs/concepts/autopilot-overview) — Autopilot deep dive
- [Global Load Balancing](https://cloud.google.com/load-balancing/docs/choosing-load-balancer) — Load balancer options

---

**Questions?** [Open an issue](https://github.com/file-network/relay-server/issues).
