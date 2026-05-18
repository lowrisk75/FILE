# Quick Start Guide

Get FILE Relay Server running in 5 minutes.

---

## Prerequisites

Choose one deployment method:

| Method | Requirements |
|--------|-------------|
| **Docker** | Docker 20.10+ |
| **Kubernetes** | kubectl + cluster access |
| **Terraform (AWS)** | Terraform 1.0+, AWS CLI, AWS account |
| **Terraform (GCP)** | Terraform 1.0+, gcloud CLI, GCP project |

---

## Option 1: Docker (Fastest)

### Step 1: Run the relay

```bash
docker run -d \
  --name file-relay \
  -p 8080:8080/udp \
  -p 8081:8081 \
  -e MAX_PEERS=1000 \
  -e RUST_LOG=info \
  ghcr.io/file-network/relay-server:latest
```

### Step 2: Verify it's running

```bash
# Check logs
docker logs file-relay

# Check metrics
curl http://localhost:8081/metrics
```

You should see:
```
# HELP file_relay_peers_total Total number of registered peers
# TYPE file_relay_peers_total gauge
file_relay_peers_total 0
```

### Step 3: Test registration

```bash
# Send a REGISTER packet (example - replace with your client)
echo -n "REGISTER|peer123|proof_data_here" | nc -u localhost 8080
```

**Done!** Your relay is running at `udp://localhost:8080`.

---

## Option 2: Kubernetes

### Step 1: Apply manifests

```bash
# Create namespace
kubectl create namespace file-relay

# Deploy relay
kubectl apply -f https://raw.githubusercontent.com/file-network/relay-server/main/deploy/kubernetes/relay-server-deployment.yaml
```

### Step 2: Wait for pods

```bash
kubectl wait --for=condition=ready pod -l app=file-relay -n file-relay --timeout=60s
```

### Step 3: Get service endpoint

```bash
# For LoadBalancer (cloud)
kubectl get svc file-relay -n file-relay

# For NodePort (on-premise)
kubectl get nodes -o wide
kubectl get svc file-relay -n file-relay -o jsonpath='{.spec.ports[0].nodePort}'
```

### Step 4: Verify metrics

```bash
kubectl port-forward -n file-relay svc/file-relay 8081:8081 &
curl http://localhost:8081/metrics
```

**Done!** Your relay is running in Kubernetes.

---

## Option 3: Terraform (AWS)

### Step 1: Clone repository

```bash
git clone https://github.com/file-network/relay-server.git
cd relay-server/infrastructure/terraform/aws
```

### Step 2: Configure variables

Create `terraform.tfvars`:

```hcl
region             = "us-east-1"
cluster_name       = "file-relay-prod"
max_peers          = 1000
min_replicas       = 2
max_replicas       = 10
environment        = "production"
```

### Step 3: Deploy

```bash
# Initialize Terraform
terraform init

# Preview changes
terraform plan

# Deploy (takes ~10 minutes)
terraform apply
```

### Step 4: Get relay endpoint

```bash
# Get LoadBalancer endpoint
terraform output relay_endpoint
```

**Done!** Your relay is running on AWS EKS.

---

## Option 4: Terraform (GCP)

### Step 1: Clone repository

```bash
git clone https://github.com/file-network/relay-server.git
cd relay-server/infrastructure/terraform/gcp
```

### Step 2: Configure variables

Create `terraform.tfvars`:

```hcl
project_id         = "your-gcp-project"
region             = "us-central1"
cluster_name       = "file-relay-prod"
max_peers          = 1000
min_replicas       = 2
max_replicas       = 10
environment        = "production"
```

### Step 3: Deploy

```bash
# Initialize Terraform
terraform init

# Preview changes
terraform plan

# Deploy (takes ~5 minutes with Autopilot)
terraform apply
```

### Step 4: Get relay endpoint

```bash
# Get LoadBalancer endpoint
terraform output relay_endpoint
```

**Done!** Your relay is running on GCP GKE Autopilot.

---

## Verify Your Deployment

### Check relay health

```bash
# Replace RELAY_IP with your relay's public IP
curl http://RELAY_IP:8081/metrics
```

Expected output:
```
file_relay_peers_total 0
file_relay_packets_relayed_total 0
file_relay_uptime_seconds 42
```

### Check Prometheus metrics

```bash
# For Kubernetes deployments with ServiceMonitor
kubectl port-forward -n monitoring svc/prometheus 9090:9090 &
open http://localhost:9090
```

Query: `file_relay_peers_total`

### Check logs

**Docker**:
```bash
docker logs file-relay
```

**Kubernetes**:
```bash
kubectl logs -f -n file-relay -l app=file-relay
```

Expected output:
```
2024-01-01T00:00:00Z INFO file_relay: Starting FILE Relay Server
2024-01-01T00:00:00Z INFO file_relay: Listening on 0.0.0.0:8080 (UDP)
2024-01-01T00:00:00Z INFO file_relay: Metrics server listening on 0.0.0.0:8081
```

---

## Test Peer Registration

### Using netcat (manual test)

```bash
# This is a simplified example - your client must generate valid CAPoW proofs
echo -n "REGISTER|test-peer-123|mock-proof-data" | nc -u RELAY_IP 8080
```

### Using the load tester (automated test)

```bash
# Clone repo if not already done
git clone https://github.com/file-network/relay-server.git
cd relay-server

# Install dependencies
pip install -r testing/load/requirements.txt

# Run smoke test (10 peers, 10 seconds)
./testing/load/relay_load_test.py \
  --host RELAY_IP \
  --peers 10 \
  --duration 10 \
  --scenario smoke
```

Expected output:
```
✅ Smoke test passed
   Registered: 10/10 peers (100.0%)
   Packets sent: 50
   Success rate: 100.0%
```

---

## Environment Variables

Common configuration options:

| Variable | Default | Description |
|----------|---------|-------------|
| `MAX_PEERS` | 1000 | Maximum concurrent peers per instance |
| `PEER_TIMEOUT_SECONDS` | 300 | Peer inactivity timeout (5 minutes) |
| `RATE_LIMIT_BURST` | 50 | Rate limit burst capacity |
| `RATE_LIMIT_SUSTAINED` | 10 | Rate limit sustained capacity (req/s) |
| `CAPOW_DIFFICULTY` | 100000 | CAPoW difficulty (iterations) |
| `RUST_LOG` | info | Log level (trace, debug, info, warn, error) |

**Example with custom settings**:

```bash
docker run -d \
  -p 8080:8080/udp \
  -p 8081:8081 \
  -e MAX_PEERS=500 \
  -e PEER_TIMEOUT_SECONDS=600 \
  -e RUST_LOG=debug \
  ghcr.io/file-network/relay-server:latest
```

---

## Next Steps

### 1. Production Hardening

- [ ] Review [Security Hardening Checklist](deployment/SECURITY-HARDENING-CHECKLIST.md)
- [ ] Configure TLS for metrics endpoint
- [ ] Set up Prometheus alerting
- [ ] Configure log aggregation

### 2. Monitoring

- [ ] Deploy Prometheus + Grafana
- [ ] Import Grafana dashboard (JSON in `deploy/grafana/`)
- [ ] Configure PagerDuty/Opsgenie alerts

### 3. Capacity Planning

- [ ] Review [Capacity Planning Guide](deployment/CAPACITY-PLANNING.md)
- [ ] Calculate expected peer count
- [ ] Right-size your deployment

### 4. Cost Optimization

- [ ] Review [Cost Optimization Guide](deployment/COST-OPTIMIZATION-GUIDE.md)
- [ ] Consider Spot/Preemptible instances
- [ ] Evaluate GCP Autopilot (71% cheaper than AWS)

### 5. Compliance

- [ ] Review [GDPR Compliance](../COMPLIANCE.md)
- [ ] Customize [Information Security Policy](../INFORMATION-SECURITY-POLICY.md)
- [ ] Complete [Risk Register](../RISK-REGISTER.md)

---

## Troubleshooting

### Problem: Pods stuck in Pending

**Cause**: Insufficient cluster resources

**Solution**:
```bash
# Check node resources
kubectl describe nodes

# Check pod events
kubectl describe pod -n file-relay -l app=file-relay

# Scale up cluster (if using Cluster Autoscaler)
# - It will auto-scale
# - Or manually add nodes
```

### Problem: Peers not registering

**Cause 1**: Firewall blocking UDP 8080

**Solution**:
```bash
# AWS: Check security group rules
aws ec2 describe-security-groups --group-ids sg-xxxxx

# GCP: Check firewall rules
gcloud compute firewall-rules list --filter="name~file-relay"

# Allow UDP 8080 inbound
```

**Cause 2**: Invalid CAPoW proof

**Solution**: Check client proof generation. See [Protocol Specification](architecture/PROTOCOL-SPECIFICATION.md) for CAPoW algorithm.

### Problem: High CPU usage

**Cause**: Too many peers per instance

**Solution**:
```bash
# Check current peer count
curl http://RELAY_IP:8081/metrics | grep file_relay_peers_total

# If > 1000 peers per instance, scale horizontally
kubectl scale deployment file-relay -n file-relay --replicas=5
```

### Problem: Metrics endpoint 404

**Cause**: Wrong port (8080 is UDP relay, 8081 is HTTP metrics)

**Solution**:
```bash
# Correct URL
curl http://RELAY_IP:8081/metrics

# NOT http://RELAY_IP:8080/metrics (this is UDP)
```

---

## Getting Help

- **Documentation**: [docs/](../README.md)
- **Issues**: [GitHub Issues](https://github.com/file-network/relay-server/issues)
- **Discussions**: [GitHub Discussions](https://github.com/file-network/relay-server/discussions)
- **Security**: See [SECURITY.md](../SECURITY.md)

---

## What's Next?

You now have a working FILE Relay Server! Here's what to explore:

1. **Architecture** — [System Overview](architecture/SYSTEM-OVERVIEW.md)
2. **Operations** — [Operations Runbook](deployment/OPERATIONS-RUNBOOK.md)
3. **Performance** — [Performance Tuning Guide](deployment/PERFORMANCE-TUNING.md)
4. **Disaster Recovery** — [DR Guide](deployment/DISASTER-RECOVERY.md)
5. **Compliance** — [GDPR/SOC 2/ISO 27001](../COMPLIANCE.md)

Happy relaying! 🚀
