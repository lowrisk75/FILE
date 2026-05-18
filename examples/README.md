# FILE Relay Server - Examples

This directory contains example configurations for deploying FILE Relay Server in various environments.

## Directory Structure

```
examples/
├── docker/
│   └── docker-compose.production.yaml  # Production Docker Compose setup
├── kubernetes/
│   └── production-values.yaml          # Production Kubernetes values
└── systemd/
    ├── file-relay.service              # Systemd service unit
    └── INSTALL.md                      # Systemd installation guide
```

## Quick Start by Environment

### Docker Compose

**Use case**: Single-server production deployment with integrated monitoring.

```bash
cd examples/docker
docker-compose -f docker-compose.production.yaml up -d
```

Features:
- Resource limits (2 CPU, 2GB RAM)
- Health checks
- Integrated Prometheus + Grafana
- Log rotation
- Security hardening

See: [docker-compose.production.yaml](docker/docker-compose.production.yaml)

### Kubernetes

**Use case**: Cloud-native production deployment with autoscaling and HA.

```bash
helm install file-relay-server . -f examples/kubernetes/production-values.yaml
```

Features:
- 3 replicas with anti-affinity
- Horizontal autoscaling (3-10 pods)
- Pod disruption budget (min 2 available)
- Prometheus integration
- LoadBalancer for UDP traffic

See: [production-values.yaml](kubernetes/production-values.yaml)

### Systemd (Bare Metal/VPS)

**Use case**: Traditional Linux server deployment.

```bash
sudo cp examples/systemd/file-relay.service /etc/systemd/system/
sudo systemctl enable --now file-relay
```

Features:
- Security hardening (sandboxing, capabilities)
- Resource limits (CPU, memory)
- Automatic restarts
- Journal logging

See: [systemd/INSTALL.md](systemd/INSTALL.md)

## Configuration Matrix

| Feature | Docker Compose | Kubernetes | Systemd |
|---------|---------------|------------|---------|
| High Availability | ❌ (single instance) | ✅ (3+ replicas) | ❌ (single instance) |
| Autoscaling | ❌ | ✅ (HPA) | ❌ |
| Monitoring | ✅ (Prometheus + Grafana) | ✅ (ServiceMonitor) | ⚠️ (manual setup) |
| Resource Limits | ✅ | ✅ | ✅ |
| Security Hardening | ✅ | ✅ | ✅ |
| Health Checks | ✅ | ✅ | ❌ (systemd monitors process only) |
| Log Management | ✅ (json-file driver) | ✅ (pod logs) | ✅ (journald) |
| Deployment Speed | Fast | Medium | Fast |
| Infrastructure Complexity | Low | High | Low |
| Best For | Small-medium deployments | Large-scale production | Single server |

## Environment Variables

All deployment methods support these environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | `info` | Log level (debug, info, warn, error) |
| `MAX_PEERS` | `1000` | Maximum concurrent peers |
| `METRICS_ADDR` | `0.0.0.0:8081` | Metrics endpoint address |

## Port Configuration

| Port | Protocol | Purpose | Exposure |
|------|----------|---------|----------|
| 8080 | UDP | Relay traffic (QUIC) | Public |
| 8081 | TCP | Metrics + health checks | Internal only |

**Security Note**: Port 8081 should only be accessible from:
- Localhost (for health checks)
- Monitoring systems (Prometheus)
- Trusted internal networks

Never expose port 8081 to the public internet.

## Resource Requirements

### Minimum Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| CPU | 2 cores | 4 cores |
| Memory | 512 MB | 2 GB |
| Storage | 100 MB | 1 GB (with logs) |
| Network | 100 Mbps | 1 Gbps |

### Scaling Guidelines

**Per 1,000 peers**:
- CPU: ~500m (0.5 cores)
- Memory: ~512 MB
- Network: Depends on traffic (peer-to-peer forwarding)

**Example**: For 3,000 concurrent peers:
- CPU: 3 × 500m = 1.5 cores (allocate 2 cores with headroom)
- Memory: 3 × 512 MB = 1.5 GB (allocate 2 GB with headroom)

## Deployment Checklist

Before deploying to production:

- [ ] Choose deployment method (Docker/Kubernetes/Systemd)
- [ ] Review and adjust resource limits
- [ ] Configure environment variables
- [ ] Set up firewall rules (allow 8080/udp, restrict 8081/tcp)
- [ ] Configure monitoring (Prometheus scraping)
- [ ] Set up alerts (see [Prometheus alerts](../deploy/prometheus/alerts.yml))
- [ ] Test health check endpoint (`curl http://localhost:8081/health`)
- [ ] Test metrics endpoint (`curl http://localhost:8081/metrics`)
- [ ] Configure log rotation/retention
- [ ] Document rollback procedure
- [ ] Set up backup/disaster recovery (if stateful)

## Migration Between Environments

### Docker Compose → Kubernetes

1. Export environment variables from docker-compose.yaml
2. Convert to Kubernetes ConfigMap/Secret
3. Use production-values.yaml as base
4. Adjust resource limits based on actual usage
5. Deploy to staging first, validate metrics
6. Gradual rollout to production

### Systemd → Docker

1. Stop systemd service: `sudo systemctl stop file-relay`
2. Note current environment variables from service file
3. Create docker-compose.yaml with same env vars
4. Start Docker container: `docker-compose up -d`
5. Verify health: `curl http://localhost:8081/health`
6. Disable systemd service: `sudo systemctl disable file-relay`

### Docker → Kubernetes

1. Use existing Docker image (same image works in both)
2. Create Kubernetes manifests from docker-compose.yaml
3. Or use production-values.yaml if using Helm
4. Deploy to test namespace first
5. Validate with smoke tests
6. Switch DNS/load balancer to Kubernetes service

## Monitoring Examples

### Prometheus Scrape Config

```yaml
scrape_configs:
  - job_name: 'file-relay'
    static_configs:
      - targets: ['relay-server:8081']
    scrape_interval: 30s
```

### Grafana Dashboard

Pre-built dashboard available at: [../deploy/grafana/provisioning/dashboards/file-relay.json](../deploy/grafana/provisioning/dashboards/file-relay.json)

Import into Grafana:
1. Dashboard → Import
2. Upload JSON file
3. Select Prometheus datasource

### Alert Rules

See: [../deploy/prometheus/alerts.yml](../deploy/prometheus/alerts.yml)

Critical alerts:
- Server down (no metrics for 2 minutes)
- Capacity reached (>1000 peers)
- DoS attack (>50 CAPoW rejections/s)

## Troubleshooting

### Container Won't Start

```bash
# Docker
docker logs file-relay-production

# Kubernetes
kubectl logs -l app=file-relay-server

# Systemd
sudo journalctl -u file-relay -n 50
```

### High Memory Usage

Check metrics:
```bash
curl http://localhost:8081/metrics | grep file_relay_active_peers
```

Expected memory: ~512 MB + (active_peers × 0.5 MB)

If higher than expected:
- Check for memory leak (uncommon in Rust)
- Review replay cache size (5-minute TTL, auto-cleanup)
- Consider horizontal scaling

### Port Already in Use

```bash
# Find process using port 8080
sudo lsof -i :8080

# Kill if safe
sudo kill <PID>
```

### Firewall Issues

Test UDP connectivity:
```bash
nc -vzu <relay-ip> 8080
```

If fails:
- Check cloud provider security groups
- Check host firewall (ufw, firewalld, iptables)
- Verify LoadBalancer configuration (Kubernetes)

## Additional Resources

- [Deployment Guide](../docs/deployment/DEPLOYMENT-GUIDE.md) - Comprehensive deployment procedures
- [Operations Runbook](../docs/deployment/OPERATIONS-RUNBOOK.md) - Incident response playbooks
- [FAQ](../docs/FAQ.md) - Frequently asked questions
- [SECURITY.md](../SECURITY.md) - Security best practices

## Contributing

Found an issue with these examples or have a new deployment scenario?

1. Open an issue: https://github.com/file-network/relay-server/issues
2. Submit a PR with your example configuration
3. See [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines
