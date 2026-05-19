# FILE Relay Server - Production Deployment Guide

**Target Platform**: Hetzner FSN1 (Falkenstein, Germany)  
**OS**: Ubuntu 24.04 LTS  
**Architecture**: x86_64  

This guide provides step-by-step instructions for deploying a production-ready FILE Relay Server with security hardening, performance optimization, and monitoring.

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Initial Server Setup](#initial-server-setup)
3. [System Hardening](#system-hardening)
4. [Binary Deployment](#binary-deployment)
5. [Systemd Service Configuration](#systemd-service-configuration)
6. [Kernel Tuning](#kernel-tuning)
7. [Firewall Configuration](#firewall-configuration)
8. [Monitoring Setup](#monitoring-setup)
9. [Validation & Testing](#validation--testing)
10. [Troubleshooting](#troubleshooting)

---

## Prerequisites

### Hetzner VPS Specifications

**Recommended minimum**:
- **CPU**: 2 vCores
- **RAM**: 2 GB (relay uses ~15-20 MB baseline, ~64 MB during CAPoW validation spikes)
- **Disk**: 20 GB
- **Network**: 1 Gbps port, 20 TB traffic included
- **Location**: FSN1 (Falkenstein) for EU coverage

**Why Hetzner FSN1?**
- 1 Gbps bandwidth (10x faster than OVH's 100 Mbps) — critical for MASQUE proxy fallback
- 20 TB traffic included — prevents bandwidth overage bills during heavy relay load
- Excellent network quality for UDP/QUIC
- ~€4-6/month pricing

### Required Tools

On your local machine:
```bash
# GitHub CLI (for downloading releases)
brew install gh

# Or curl for direct binary download
which curl
```

---

## Initial Server Setup

### 1. Create Hetzner VPS

1. Go to [Hetzner Cloud Console](https://console.hetzner.cloud/)
2. Create new server:
   - **Location**: Falkenstein (FSN1)
   - **Image**: Ubuntu 24.04
   - **Type**: CPX11 (2 vCPU, 2 GB RAM, €4.51/mo) or higher
   - **SSH Key**: Add your public key

3. Note the server IP address (e.g., `1.2.3.4`)

### 2. Initial SSH Connection

```bash
ssh root@YOUR_SERVER_IP
```

### 3. Update System

```bash
apt update && apt upgrade -y
apt install -y curl ufw
```

---

## System Hardening

### 1. Create Dedicated User

```bash
# Create non-privileged user for relay service
useradd -r -s /usr/sbin/nologin -d /opt/file-relay file-relay

# Create working directory
mkdir -p /opt/file-relay
chown file-relay:file-relay /opt/file-relay
```

### 2. Create Cache Directory

```bash
# Create cache directory for bootstrap_cache.json
mkdir -p /var/cache/file-relay
chown file-relay:file-relay /var/cache/file-relay
chmod 755 /var/cache/file-relay
```

### 3. Create Log Directory

```bash
mkdir -p /var/log/file-relay
chown file-relay:file-relay /var/log/file-relay
```

---

## Binary Deployment

### Option A: Download from GitHub Release

```bash
# Download latest release
cd /opt/file-relay
RELEASE_URL=$(curl -s https://api.github.com/repos/YOUR_ORG/FILE/releases/latest | grep "browser_download_url.*file-relay-linux-x86_64" | cut -d '"' -f 4)
curl -L -o file-relay "$RELEASE_URL"
chmod +x file-relay
chown file-relay:file-relay file-relay

# Verify
./file-relay --version
```

### Option B: Build from Source

```bash
# On your local machine (with Rust installed)
cd ~/GitHub/FILE
cargo build --release --package relay_server

# Binary location: target/release/file-relay

# Copy to server
scp target/release/file-relay root@YOUR_SERVER_IP:/opt/file-relay/file-relay

# On server: set permissions
chmod +x /opt/file-relay/file-relay
chown file-relay:file-relay /opt/file-relay/file-relay
```

---

## Systemd Service Configuration

### 1. Create Service File

```bash
cat > /etc/systemd/system/file-relay.service << 'EOF'
[Unit]
Description=FILE Relay Server - MPQUIC P2P Relay with CAPoW Anti-DDoS
After=network-online.target
Wants=network-online.target
Documentation=https://github.com/yourorg/FILE

[Service]
Type=simple
User=file-relay
Group=file-relay
WorkingDirectory=/opt/file-relay

# Binary location
ExecStart=/opt/file-relay/file-relay \
    --bind 0.0.0.0:8080 \
    --health-port 8081 \
    --max-peers 1000 \
    --capow \
    --capow-timeout 60 \
    --log-level info

# Security hardening (NotebookLM recommendations)
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/log/file-relay /var/cache/file-relay
LimitCORE=0

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096

# Restart policy (best practice)
Restart=on-failure
RestartSec=5
StartLimitIntervalSec=60
StartLimitBurst=5
TimeoutStopSec=30

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=file-relay

[Install]
WantedBy=multi-user.target
EOF
```

### 2. Enable and Start Service

```bash
systemctl daemon-reload
systemctl enable file-relay
systemctl start file-relay

# Verify
systemctl status file-relay
```

**Expected output**:
```
● file-relay.service - FILE Relay Server - MPQUIC P2P Relay with CAPoW Anti-DDoS
     Loaded: loaded (/etc/systemd/system/file-relay.service; enabled)
     Active: active (running) since ...
     Memory: 14.0M
```

---

## Kernel Tuning

### UDP Buffer Optimization (MPQUIC Performance)

```bash
cat > /etc/sysctl.d/99-file-relay.conf << 'EOF'
# FILE Relay UDP optimization for MPQUIC
# Recommended by NotebookLM/Gemini 2.5 analysis

# 25 MB receive buffer (handles QUIC burst traffic)
net.core.rmem_max = 26214400

# 25 MB send buffer
net.core.wmem_max = 26214400

# 5 MB default receive buffer
net.core.rmem_default = 5242880

# 5 MB default send buffer
net.core.wmem_default = 5242880
EOF

# Apply immediately
sysctl -p /etc/sysctl.d/99-file-relay.conf
```

**Why these values?**
- QUIC sends bursts of packets (congestion control windows)
- Small buffers = dropped packets at kernel level before they reach userspace
- 25 MB max allows 1000 peers @ 25 KB/peer burst capacity

---

## Firewall Configuration

### UFW (Uncomplicated Firewall)

```bash
# Default policies
ufw default deny incoming
ufw default allow outgoing

# SSH (replace 22 with your custom port if changed)
ufw allow 22/tcp

# FILE Relay MPQUIC (UDP)
ufw allow 8080/udp comment 'FILE Relay MPQUIC'

# Health/Metrics endpoint (RESTRICT to monitoring server IPs)
# Option 1: Allow from specific monitoring server
# ufw allow from YOUR_MONITORING_IP to any port 8081 proto tcp

# Option 2: Block public access (uncomment if no external monitoring)
# ufw deny 8081/tcp

# Enable firewall
ufw --force enable

# Verify rules
ufw status verbose
```

**⚠️ CRITICAL: Do NOT use Cloudflare Magic Transit or DDoS proxies in front of the relay**

**Why?** Proxies mask source IPs, breaking `OBSERVED_ADDRESS` QUIC frames needed for NAT hole-punching. The relay MUST be directly exposed to the internet.

**DDoS Protection Strategy**:
- CAPoW (MTP-Argon2id) is the primary DDoS defense
- 32x spatial compression: attacker uses 2GB RAM, relay uses 64MB
- Rate limiting: 10 req/s sustained, 50 burst capacity
- fail2ban can blackhole attacking subnets (optional, see Monitoring section)

---

## Monitoring Setup

### 1. Health Check Endpoint

```bash
# From another machine
curl http://YOUR_SERVER_IP:8081/health

# Expected response
{"service":"FILE Relay Server","status":"healthy"}
```

### 2. Prometheus Metrics

```bash
curl http://YOUR_SERVER_IP:8081/metrics
```

**Key metrics to monitor**:

| Metric | Type | Description | Alert Threshold |
|--------|------|-------------|-----------------|
| `file_relay_active_peers` | gauge | Current connected peers | > 900 (near max_peers) |
| `file_relay_capow_rejected_total` | counter | CAPoW rejections | Spike = DDoS |
| `file_relay_nat_punch_success_total` | counter | NAT traversal success | — |
| `file_relay_nat_fallback_masque_total` | counter | MASQUE proxy fallback | Spike = ISP blocking |
| `file_relay_capow_difficulty_level` | gauge | Current CAPoW difficulty | == MAX for >2min = severe DDoS |
| `file_relay_peers_timeout_total` | counter | Peer timeouts | — |
| `file_relay_rate_limited_total` | counter | Rate-limited requests | Spike = attack |

### 3. Prometheus Scrape Configuration

Add to your `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'file-relay'
    static_configs:
      - targets: ['YOUR_SERVER_IP:8081']
    scrape_interval: 15s
```

### 4. Recommended Alerts (Prometheus Alertmanager)

```yaml
groups:
  - name: file_relay
    interval: 30s
    rules:
      # Severe DDoS attack
      - alert: SevereDDoSAttack
        expr: file_relay_capow_difficulty_level == 2097152 for 2m
        labels:
          severity: critical
        annotations:
          summary: "FILE Relay under severe DDoS (CAPoW at MAX for >2min)"
          
      # ISP blocking UDP
      - alert: ISPBlockingDetected
        expr: rate(file_relay_nat_fallback_masque_total[5m]) > 0.1
        labels:
          severity: warning
        annotations:
          summary: "High MASQUE fallback rate - possible ISP UDP blocking"
          
      # Relay near capacity
      - alert: RelayNearCapacity
        expr: file_relay_active_peers > 900
        labels:
          severity: warning
        annotations:
          summary: "Relay near max_peers capacity (>900/1000)"
          
      # Relay down
      - alert: RelayDown
        expr: up{job="file-relay"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "FILE Relay is down"
```

### 5. Journald Log Monitoring

```bash
# View live logs
journalctl -u file-relay -f

# View last 100 lines
journalctl -u file-relay -n 100

# Filter by severity
journalctl -u file-relay -p err

# Export logs to file
journalctl -u file-relay --since "1 hour ago" > /tmp/relay-logs.txt
```

### 6. Optional: fail2ban Integration

Create `/etc/fail2ban/filter.d/file-relay.conf`:

```ini
[Definition]
failregex = ^.*⚠️ CAPoW invalid.*client=<HOST>.*$
            ^.*⚠️ Rate limit exceeded.*from <HOST>.*$
ignoreregex =
```

Create `/etc/fail2ban/jail.d/file-relay.conf`:

```ini
[file-relay]
enabled = true
port = 8080
protocol = udp
filter = file-relay
logpath = /var/log/syslog
maxretry = 10
findtime = 60
bantime = 3600
action = iptables-allports[name=file-relay]
```

---

## Validation & Testing

### 1. Service Health

```bash
# Service running?
systemctl is-active file-relay

# Memory usage (should be ~14-20 MB baseline)
systemctl show file-relay --property=MemoryCurrent

# Restart count (should be 0 after deployment)
systemctl show file-relay --property=NRestarts
```

### 2. Network Connectivity

```bash
# UDP port 8080 listening?
ss -ulnp | grep 8080

# Expected output:
# UNCONN 0 0 0.0.0.0:8080 0.0.0.0:* users:(("file-relay",pid=XXX,fd=Y))

# HTTP health port 8081 listening?
ss -tlnp | grep 8081
```

### 3. Endpoints

```bash
# Health check
curl -i http://localhost:8081/health

# Metrics
curl -s http://localhost:8081/metrics | grep file_relay_active_peers

# Challenge endpoint
curl -s http://localhost:8081/challenge | jq .
```

**Expected challenge response**:
```json
{
  "created_at": 1779177930,
  "expires_at": 1779179830,
  "merkle_root": "919419ed1d595b96234e1f0995fa19be86444684f0aab110b967da6e8134e014"
}
```

### 4. External Connectivity Test

From **another machine** (not the server):

```bash
# Ping should fail (ICMP blocked by firewall)
ping YOUR_SERVER_IP

# But UDP port 8080 should be reachable (no direct test for UDP, use client)

# HTTP endpoints should work if allowed
curl http://YOUR_SERVER_IP:8081/health
```

---

## Troubleshooting

### Service won't start

**Check logs**:
```bash
journalctl -u file-relay -n 50 --no-pager
```

**Common issues**:

1. **Port already in use**
   ```bash
   # Check what's using port 8080
   ss -ulnp | grep 8080
   
   # Kill the process or change relay port
   ```

2. **Permission denied (binding to port)**
   - Ports 8080/8081 are >1024, no special permissions needed
   - Check `User=file-relay` in systemd service

3. **Bootstrap cache error**
   ```
   Configuration error: Failed to open bootstrap cache: Read-only file system
   ```
   
   **Fix**: Verify `ReadWritePaths=/var/cache/file-relay` in systemd service

4. **GLIBC version mismatch**
   ```
   ./file-relay: /lib/x86_64-linux-gnu/libc.so.6: version 'GLIBC_2.38' not found
   ```
   
   **Fix**: Binary was compiled on Ubuntu 24.04 (glibc 2.38). Rebuild on target OS or upgrade to Ubuntu 24.04.

### High memory usage

**Normal**: 14-20 MB baseline, spikes to 64 MB during CAPoW validation

**Abnormal**: >100 MB sustained

**Check**:
```bash
# Memory breakdown
systemctl show file-relay --property=MemoryCurrent --property=MemoryPeak

# Active peers count
curl -s http://localhost:8081/metrics | grep file_relay_active_peers
```

**Possible causes**:
- Too many peers (increase max_peers or add more relay servers)
- Memory leak (report bug with `journalctl -u file-relay` output)

### CAPoW verification failures

**Check difficulty level**:
```bash
curl -s http://localhost:8081/metrics | grep capow_difficulty_level
```

**If difficulty == 2097152 (MAX)**: Relay is under severe DDoS attack

**If capow_rejected_total is high**:
- Legitimate clients may be timing out (increase `--capow-timeout`)
- Or ongoing attack (monitor for patterns in logs)

### Firewall issues

**Verify UFW rules**:
```bash
ufw status verbose
```

**Test from external machine**:
```bash
# UDP test (requires UDP client)
nc -u YOUR_SERVER_IP 8080

# HTTP health check (if allowed)
curl http://YOUR_SERVER_IP:8081/health
```

**Check iptables directly**:
```bash
iptables -L -n -v | grep 8080
```

---

## Performance Tuning

### For High-Load Relays (>500 concurrent peers)

1. **Increase file descriptor limits**:
   ```bash
   # Edit /etc/systemd/system/file-relay.service
   LimitNOFILE=131072  # Increase from 65536
   
   systemctl daemon-reload
   systemctl restart file-relay
   ```

2. **Increase worker threads (if needed)**:
   ```bash
   # relay_daemon uses Rayon with default thread pool = num_cpus
   # For CPU-bound CAPoW verification, scale vertically (more vCPUs)
   ```

3. **Monitor tokio_scheduler_lag** (future metric):
   - High lag = thread pool starvation
   - Indicates CAPoW verification blocking async I/O
   - Solution: Increase CPU or reduce max_peers

### For Low-Latency Requirements

1. **Use CPU pinning**:
   ```bash
   # Edit /etc/systemd/system/file-relay.service
   CPUAffinity=0-1  # Pin to specific cores
   ```

2. **Disable CPU frequency scaling**:
   ```bash
   echo performance | tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor
   ```

---

## Backup & Disaster Recovery

### Stateless Design

**The relay is fully stateless** — no data is persisted to disk beyond:
- `bootstrap_cache.json` (ephemeral, regenerated on restart)
- Logs (via journald)

**Disaster recovery**:
1. Deploy a new server following this guide
2. Update DNS to point to new IP
3. Clients' MPQUIC Happy Eyeballs v2 will automatically failover to other relays

**No backup needed** for relay state.

### Configuration Backup

```bash
# Backup systemd service file
cp /etc/systemd/system/file-relay.service ~/file-relay.service.backup

# Backup sysctl config
cp /etc/sysctl.d/99-file-relay.conf ~/99-file-relay.conf.backup

# Backup UFW rules
ufw status numbered > ~/ufw-rules.backup
```

---

## Upgrading

### Binary Update

```bash
# Stop service
systemctl stop file-relay

# Backup current binary
cp /opt/file-relay/file-relay /opt/file-relay/file-relay.backup

# Download new binary (Option A: from release)
cd /opt/file-relay
RELEASE_URL=$(curl -s https://api.github.com/repos/YOUR_ORG/FILE/releases/latest | grep "browser_download_url.*file-relay-linux-x86_64" | cut -d '"' -f 4)
curl -L -o file-relay "$RELEASE_URL"
chmod +x file-relay
chown file-relay:file-relay file-relay

# Or Option B: scp from local build
# scp target/release/file-relay root@YOUR_SERVER_IP:/opt/file-relay/file-relay

# Start service
systemctl start file-relay

# Verify version
journalctl -u file-relay -n 20 | grep version

# If issues, rollback:
# cp /opt/file-relay/file-relay.backup /opt/file-relay/file-relay
# systemctl restart file-relay
```

### Zero-Downtime Upgrade (Blue-Green Deployment)

1. Deploy second relay server with new binary
2. Add to client relay pool
3. Wait for old relay to drain (no active peers)
4. Decommission old relay

---

## Security Checklist

- [ ] Service runs as non-root user (`file-relay`)
- [ ] `ProtectSystem=strict` enabled
- [ ] `LimitCORE=0` set (prevents core dumps with cryptographic secrets)
- [ ] `NoNewPrivileges=true` set
- [ ] UFW firewall enabled with minimal rules
- [ ] SSH key-based authentication (disable password auth)
- [ ] Automatic security updates enabled (`unattended-upgrades`)
- [ ] Monitoring & alerting configured (Prometheus/Grafana)
- [ ] No Cloudflare/DDoS proxy in front (breaks NAT traversal)
- [ ] fail2ban configured (optional but recommended)

---

## Support & Resources

- **GitHub Issues**: https://github.com/YOUR_ORG/FILE/issues
- **Documentation**: https://github.com/YOUR_ORG/FILE/blob/main/README.md
- **Architecture Deep Dive**: `docs/Advanced Network Architecture Deep Dive.md`

---

## Changelog

| Date | Version | Changes |
|------|---------|---------|
| 2026-05-19 | 1.0 | Initial production deployment guide |

---

**Deployment validated on**:
- Proxmox LXC 199 (Ubuntu 24.04, x86_64) — 24-48h stability test
- Configuration hardened per NotebookLM/Gemini 2.5 security analysis
- Kernel tuning optimized for 1000 concurrent peers @ 1 Gbps
