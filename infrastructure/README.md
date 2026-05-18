# FILE Relay Server - Infrastructure as Code

Production-ready Terraform configurations for deploying FILE Relay Server on major cloud providers.

---

## Available Deployments

| Provider | Path | Status | Difficulty | Est. Cost/Month |
|----------|------|--------|------------|-----------------|
| **AWS** | [terraform/aws/](terraform/aws/) | ✅ Production-ready | Medium | $285 |
| **GCP** | [terraform/gcp/](terraform/gcp/) | ✅ Production-ready | Easy | $108 (Autopilot) |
| **Azure** | [terraform/azure/](terraform/azure/) | 🚧 Coming soon | Medium | $250 |

---

## Quick Comparison

### AWS (EKS)

**Best for**:
- Organizations already on AWS
- Maximum flexibility and control
- Enterprise compliance (SOC 2, HIPAA)

**Pros**:
- Mature ecosystem
- Most documentation available
- Fine-grained IAM control
- Extensive third-party integrations

**Cons**:
- Higher cost ($285/month baseline)
- More complex networking (VPC, subnets, NAT)
- IRSA adds IAM complexity

**Deployment time**: 15-20 minutes

**Quick start**:
```bash
cd terraform/aws
cp terraform.tfvars.example terraform.tfvars
vim terraform.tfvars
terraform init
terraform apply
```

**Full guide**: [terraform/aws/README.md](terraform/aws/README.md)

---

### GCP (GKE)

**Best for**:
- Cost-conscious deployments
- Global user base (Anycast IP)
- Minimal operational overhead (Autopilot)

**Pros**:
- **50% cheaper than AWS** ($108/month with Autopilot)
- Global Load Balancer with Anycast (automatic geo-routing)
- GKE Autopilot = zero node management
- Simpler IAM (Workload Identity)
- Fast deployment (10-15 minutes)

**Cons**:
- Smaller ecosystem than AWS
- Less documentation for edge cases
- Autopilot enforces opinionated best practices (usually good)

**Deployment time**: 10-15 minutes

**Quick start**:
```bash
cd terraform/gcp
cp terraform.tfvars.example terraform.tfvars
vim terraform.tfvars
terraform init
terraform apply
```

**Full guide**: [terraform/gcp/README.md](terraform/gcp/README.md)

---

### Azure (AKS)

**Status**: Coming soon

**Best for**:
- Organizations on Microsoft Azure
- Windows integration needs
- Azure AD integration

**Estimated cost**: ~$250/month

---

## Feature Comparison

| Feature | AWS EKS | GCP GKE | Azure AKS |
|---------|---------|---------|-----------|
| **Managed Control Plane** | ✅ | ✅ | ✅ |
| **Managed Nodes** | ❌ | ✅ (Autopilot) | ⚠️ (Virtual Nodes) |
| **Global Load Balancer** | ⚠️ (Global Accelerator, extra cost) | ✅ (Native, included) | ⚠️ (Front Door, extra cost) |
| **IAM Integration** | ✅ (IRSA) | ✅ (Workload Identity) | ✅ (AAD Pod Identity) |
| **Auto-scaling (Pods)** | ✅ HPA | ✅ HPA | ✅ HPA |
| **Auto-scaling (Nodes)** | ✅ Cluster Autoscaler | ✅ (Built-in Autopilot) | ✅ Cluster Autoscaler |
| **Network Policy** | ✅ Calico | ✅ GKE Network Policy | ✅ Calico |
| **Monitoring** | CloudWatch | Cloud Monitoring | Azure Monitor |
| **Cost (baseline)** | $285/month | $108/month | ~$250/month |
| **Free tier** | ❌ | ✅ (control plane < 2500 vCPUs) | ✅ (control plane) |

---

## Which Cloud Should You Choose?

### Choose AWS if:
- ✅ You're already on AWS
- ✅ You need maximum control and flexibility
- ✅ You have strict compliance requirements (SOC 2, HIPAA)
- ✅ You have AWS expertise in-house
- ✅ Cost is not the primary concern

### Choose GCP if:
- ✅ You want the **lowest cost** ($108/month vs $285/month)
- ✅ You have a **global user base** (Anycast IP = best latency)
- ✅ You prefer **minimal operations** (GKE Autopilot = zero node management)
- ✅ You're starting from scratch
- ✅ You want **fast deployment** (10-15 minutes)

**Recommendation**: GCP for most users (lower cost, simpler, global routing built-in)

### Choose Azure if:
- ✅ You're on Azure
- ✅ You need Windows integration
- ✅ You use Azure AD

---

## Deployment Steps (All Providers)

### 1. Prerequisites

**All providers**:
```bash
# Install Terraform
brew install terraform  # macOS
# or: https://www.terraform.io/downloads

# Install kubectl
brew install kubectl  # macOS
```

**AWS**:
```bash
brew install awscli
aws configure
```

**GCP**:
```bash
brew install google-cloud-sdk
gcloud auth application-default login
gcloud config set project YOUR-PROJECT-ID
```

### 2. Configuration

```bash
cd terraform/<provider>
cp terraform.tfvars.example terraform.tfvars
vim terraform.tfvars  # Edit: project/region, passwords, alerts
```

**Required variables**:
- AWS: `aws_region`, `grafana_admin_password`
- GCP: `gcp_project_id`, `gcp_region`, `grafana_admin_password`

### 3. Deploy

```bash
# Initialize
terraform init

# Plan
terraform plan -out=tfplan

# Apply
terraform apply tfplan
```

### 4. Configure kubectl

**AWS**:
```bash
aws eks update-kubeconfig --region us-east-1 --name file-relay-production
```

**GCP**:
```bash
gcloud container clusters get-credentials file-relay-production --region us-east1
```

### 5. Verify

```bash
kubectl get pods -n file-relay
kubectl get svc -n file-relay

# Get LoadBalancer address
terraform output loadbalancer_hostname  # AWS
terraform output loadbalancer_ip        # GCP

# Health check
curl http://<address>:8081/health
```

---

## Cost Optimization

### All Providers

1. **Use Spot/Preemptible Instances** (50-70% discount)
2. **Right-size resources** (start small, scale up)
3. **Single NAT Gateway** for non-production
4. **Disable monitoring** for development environments
5. **Auto-scaling** (HPA + Cluster Autoscaler) = pay only for what you use

### AWS-Specific

- **Reserved Instances**: 40% discount (1-year commit)
- **Savings Plans**: Flexible commitment
- **Spot Fleet** for non-critical workloads

### GCP-Specific

- **GKE Autopilot**: Pay-per-pod (already lowest cost)
- **Committed Use Discounts**: 57% discount (1-year)
- **Sustained Use Discounts**: Automatic (no commitment)
- **Preemptible VMs**: 80% discount

**Result**: GCP baseline is already 50% cheaper than AWS

---

## Multi-Region Deployment

Deploy to multiple regions for:
- **High availability** (99.99%+ uptime)
- **Low latency** (users routed to nearest region)
- **Disaster recovery** (survive complete region outage)

### AWS

Manual multi-region with Route 53 geolocation routing:

```bash
# Deploy to us-east-1
cd terraform/aws
terraform workspace new us-east-1
terraform apply

# Deploy to eu-west-1
terraform workspace new eu-west-1
terraform apply -var="aws_region=eu-west-1"

# Configure Route 53 (see docs/deployment/MULTI-REGION-GUIDE.md)
```

### GCP

Native global load balancing:

```bash
# Deploy to multiple regions, use same Anycast IP
# Traffic automatically routed to nearest region
# No DNS configuration needed
```

**Winner**: GCP (automatic global routing, no DNS setup)

See [docs/deployment/MULTI-REGION-GUIDE.md](../docs/deployment/MULTI-REGION-GUIDE.md) for details.

---

## Monitoring

All deployments include:
- **Prometheus**: Metrics collection
- **Grafana**: Dashboards and visualization
- **Alertmanager**: Alert routing (Slack, PagerDuty)
- **Pre-configured dashboards**: FILE Relay overview, resource usage
- **Pre-configured alerts**: 12 production alerts (4 critical, 8 warning)

Access Grafana:
```bash
kubectl port-forward -n monitoring svc/kube-prometheus-stack-grafana 3000:80
# Open: http://localhost:3000
# Login: admin / [your password from terraform.tfvars]
```

---

## Security

All deployments include:
- ✅ Pod security contexts (non-root, read-only filesystem)
- ✅ Network policies (namespace isolation)
- ✅ Resource limits (CPU, memory)
- ✅ Secrets management (Kubernetes secrets)
- ✅ IAM roles (least privilege)
- ✅ Automatic security updates (managed nodes)
- ✅ TLS for internal communication (monitoring stack)
- ✅ CAPoW DoS protection (application-level)
- ✅ Rate limiting (application-level)

See [docs/deployment/SECURITY-HARDENING-CHECKLIST.md](../docs/deployment/SECURITY-HARDENING-CHECKLIST.md)

---

## Cleanup

**Warning**: Destroys all resources including data.

```bash
cd terraform/<provider>
terraform destroy
# Confirm: yes
```

---

## Troubleshooting

### Common Issues (All Providers)

**Pods not starting**:
```bash
kubectl get pods -n file-relay
kubectl describe pod <pod-name> -n file-relay
kubectl logs <pod-name> -n file-relay
```

**LoadBalancer pending**:
```bash
kubectl describe svc file-relay-udp -n file-relay
# Check cloud provider console for load balancer status
```

**HPA not scaling**:
```bash
kubectl get hpa -n file-relay
kubectl top pods -n file-relay
# Ensure metrics-server is running
```

### Provider-Specific

- **AWS**: [terraform/aws/README.md#troubleshooting](terraform/aws/README.md#troubleshooting)
- **GCP**: [terraform/gcp/README.md#troubleshooting](terraform/gcp/README.md#troubleshooting)

---

## Further Reading

- **Deployment Guide**: [docs/deployment/DEPLOYMENT-GUIDE.md](../docs/deployment/DEPLOYMENT-GUIDE.md)
- **Operations Runbook**: [docs/deployment/OPERATIONS-RUNBOOK.md](../docs/deployment/OPERATIONS-RUNBOOK.md)
- **Multi-Region Guide**: [docs/deployment/MULTI-REGION-GUIDE.md](../docs/deployment/MULTI-REGION-GUIDE.md)
- **Alert Runbook**: [docs/deployment/ALERT-RUNBOOK.md](../docs/deployment/ALERT-RUNBOOK.md)
- **Security Checklist**: [docs/deployment/SECURITY-HARDENING-CHECKLIST.md](../docs/deployment/SECURITY-HARDENING-CHECKLIST.md)

---

**Questions?** [Open an issue](https://github.com/file-network/relay-server/issues) or consult the [FAQ](../docs/FAQ.md).
