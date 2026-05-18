# FILE Relay Server - VPS Deployment

**Purpose**: Deploy FILE Relay Server to a VPS (Hetzner, DigitalOcean, etc.) using systemd.

---

## Prerequisites

### Local Machine
- Rust toolchain installed
- SSH access to VPS (root or sudo user)
- Binary built: `cargo build --release --package relay_server`

### VPS Requirements
- **OS**: Ubuntu 22.04+ or Debian 11+
- **CPU**: 2 vCPU minimum
- **RAM**: 2 GB minimum
- **Storage**: 10 GB minimum
- **Network**: Public IPv4 address, UDP port 8080 open

---

## Quick Start

### 1. Build Binary

```bash
cd /path/to/FILE
cargo build --release --package relay_server
```

Binary location: `target/release/file-relay`

### 2. Deploy to VPS

```bash
cd deploy/systemd
./deploy.sh relay.example.com
```

**What this does**:
- Creates `file-relay` system user
- Installs binary to `/opt/file-relay/`
- Installs systemd unit file
- Starts and enables service
- Verifies health endpoints

### 3. Configure Firewall

```bash
ssh root@relay.example.com
ufw allow 8080/udp comment "FILE Relay - P2P traffic"
ufw allow 8081/tcp comment "FILE Relay - Metrics"
ufw enable
```

### 4. Verify Deployment

```bash
# Check service status
ssh root@relay.example.com systemctl status file-relay

# Check logs
ssh root@relay.example.com journalctl -u file-relay -f

# Test health endpoint
curl http://relay.example.com:8081/health

# Get current challenge
curl http://relay.example.com:8081/challenge

# View metrics
curl http://relay.example.com:8081/metrics
```

---

## Update Deployment

To update an existing deployment (binary only):

```bash
cd deploy/systemd
./deploy.sh relay.example.com --update-only
```

This:
- Uploads new binary
- Restarts service
- Skips user/directory creation

---

## Service Management

### Start/Stop/Restart

```bash
ssh root@relay.example.com

# Start service
systemctl start file-relay

# Stop service
systemctl stop file-relay

# Restart service
systemctl restart file-relay

# Check status
systemctl status file-relay
```

### View Logs

```bash
# Follow logs (live)
journalctl -u file-relay -f

# Last 100 lines
journalctl -u file-relay -n 100

# Since boot
journalctl -u file-relay -b

# Last hour
journalctl -u file-relay --since "1 hour ago"
```

### Configuration

Edit systemd unit: `/etc/systemd/system/file-relay.service`

Common changes:
```ini
# Change bind address
ExecStart=/opt/file-relay/file-relay --bind 0.0.0.0:9999 ...

# Change log level
ExecStart=/opt/file-relay/file-relay --log-level debug ...

# Change max peers
ExecStart=/opt/file-relay/file-relay --max-peers 5000 ...
```

After editing:
```bash
systemctl daemon-reload
systemctl restart file-relay
```

---

## Monitoring

### Prometheus Setup

1. Add scrape target to `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'file-relay'
    static_configs:
      - targets: ['relay.example.com:8081']
```

2. Import Grafana dashboard: `../grafana/dashboards/relay_dashboard.json`

### Key Metrics to Monitor

```promql
# Active peers
file_relay_active_peers

# Packet forwarding rate
rate(file_relay_packets_forwarded_total[5m])

# CAPoW verification success rate
rate(file_relay_capow_verified_total[5m]) / 
  (rate(file_relay_capow_verified_total[5m]) + rate(file_relay_capow_rejected_total[5m]))

# Rate limiting (should be low)
rate(file_relay_rate_limited_total[5m])

# Peer timeout rate (zombie peers)
rate(file_relay_peers_timeout_total[5m])
```

### Alerts (Prometheus)

```yaml
groups:
  - name: file_relay
    rules:
      - alert: RelayDown
        expr: up{job="file-relay"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "FILE Relay is down"

      - alert: HighCAPoWRejection
        expr: rate(file_relay_capow_rejected_total[5m]) > 10
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High CAPoW rejection rate"

      - alert: MaxPeersReached
        expr: file_relay_active_peers >= 1000
        for: 1m
        labels:
          severity: warning
        annotations:
          summary: "Relay at max capacity"
```

---

## Troubleshooting

### Service Won't Start

```bash
# Check logs for errors
journalctl -u file-relay -n 50 --no-pager

# Common issues:
# - Port 8080 already in use: netstat -tulpn | grep 8080
# - Permission denied: Check file ownership (should be file-relay:file-relay)
# - Binary not found: ls -la /opt/file-relay/file-relay
```

### High CPU Usage

```bash
# Check peer count
curl http://localhost:8081/metrics | grep file_relay_active_peers

# Check CAPoW verification rate
curl http://localhost:8081/metrics | grep capow_verified

# If CPU > 80%, consider:
# - Increasing max peers (spread load)
# - Adding more relay instances (horizontal scale)
# - Reducing CAPoW timeout (faster rejection)
```

### Memory Leak

```bash
# Monitor memory usage
watch -n 5 'ps aux | grep file-relay'

# Check DashMap size (peer registry)
curl http://localhost:8081/metrics | grep active_peers

# Zombie peer cleanup should run every 60s
# Check logs for "Zombie peer removed" messages
```

### Network Connectivity

```bash
# Test UDP port 8080
nc -vzu relay.example.com 8080

# Test TCP port 8081
curl -v http://relay.example.com:8081/health

# Check firewall
ufw status

# Check listening ports
netstat -tulpn | grep file-relay
```

---

## Security Hardening

### 1. Firewall (UFW)

```bash
# Default deny incoming
ufw default deny incoming
ufw default allow outgoing

# Allow only necessary ports
ufw allow 22/tcp comment "SSH"
ufw allow 8080/udp comment "FILE Relay P2P"
ufw allow 8081/tcp comment "FILE Relay Metrics" from 10.0.0.0/8  # Prometheus only

ufw enable
```

### 2. Fail2ban (SSH Protection)

```bash
apt install fail2ban
systemctl enable fail2ban
```

### 3. Automatic Security Updates

```bash
apt install unattended-upgrades
dpkg-reconfigure -plow unattended-upgrades
```

### 4. Rate Limiting (Application Level)

Already enabled by default:
- 10 req/s sustained per IP
- 50 request burst capacity

### 5. DDoS Mitigation

CAPoW (Computational Amortized Proof of Work) provides:
- CPU-bound proof (100ms per registration)
- Challenge rotation (1 hour interval)
- Anti-replay cache (5 minute TTL)

---

## Performance Tuning

### Kernel Parameters

```bash
# Edit /etc/sysctl.conf
cat >> /etc/sysctl.conf <<EOF
# FILE Relay Server optimizations

# Increase UDP buffer sizes (for high throughput)
net.core.rmem_max = 134217728
net.core.wmem_max = 134217728
net.ipv4.udp_mem = 8388608 12582912 16777216

# Increase connection tracking
net.netfilter.nf_conntrack_max = 524288

# Enable TCP BBR (for metrics endpoint)
net.core.default_qdisc = fq
net.ipv4.tcp_congestion_control = bbr
EOF

sysctl -p
```

### Systemd Resource Limits

```ini
[Service]
# Increase file descriptor limit
LimitNOFILE=65536

# Increase process limit
LimitNPROC=4096

# Limit memory usage (prevent OOM)
MemoryMax=4G
```

---

## Backup & Recovery

### Backup Strategy

FILE Relay Server is **stateless** - no data to backup!

- Peer registry is in-memory only
- Challenge rotates automatically
- Metrics are scraped by Prometheus (external)

### Recovery Procedure

1. Redeploy binary: `./deploy.sh relay.example.com`
2. Service auto-starts
3. Peers will re-register (CAPoW verification required)

**Recovery time**: < 1 minute

---

## Multi-Region Deployment

For global low-latency:

1. Deploy to 3 regions:
   ```bash
   ./deploy.sh relay-us.example.com
   ./deploy.sh relay-eu.example.com
   ./deploy.sh relay-ap.example.com
   ```

2. Configure GeoDNS (Cloudflare, Route 53):
   ```
   relay.example.com → CNAME → geo-routed:
     - US clients → relay-us.example.com
     - EU clients → relay-eu.example.com
     - AP clients → relay-ap.example.com
   ```

3. Monitor all instances via Prometheus federation

---

## References

- [QUICK-START.md](../../docs/QUICK-START.md) - Local development
- [DEPLOYMENT-GUIDE.md](../../docs/DEPLOYMENT-GUIDE.md) - Full deployment guide
- [OPERATIONS-RUNBOOK.md](../../docs/OPERATIONS-RUNBOOK.md) - Day-2 operations
- [PROTOCOL-SPECIFICATION.md](../../docs/architecture/PROTOCOL-SPECIFICATION.md) - Protocol details

---

**Status**: Production-ready  
**Last Updated**: 2026-05-18
