# Systemd Installation Guide

This guide covers installing FILE Relay Server as a systemd service on Linux.

## Prerequisites

- Linux system with systemd (most modern distributions)
- Root or sudo access
- FILE Relay Server binary compiled for your platform

## Installation Steps

### 1. Create Service User

Create a dedicated user for running the relay server:

```bash
sudo useradd --system --no-create-home --shell /usr/sbin/nologin file-relay
```

### 2. Install Binary

Copy the compiled binary to `/usr/local/bin`:

```bash
sudo cp target/release/file-relay /usr/local/bin/
sudo chown root:root /usr/local/bin/file-relay
sudo chmod 755 /usr/local/bin/file-relay
```

Verify installation:

```bash
/usr/local/bin/file-relay --version
```

### 3. Create Working Directory

Create a working directory for runtime state:

```bash
sudo mkdir -p /var/lib/file-relay
sudo chown file-relay:file-relay /var/lib/file-relay
sudo chmod 755 /var/lib/file-relay
```

### 4. Install Systemd Service

Copy the service file:

```bash
sudo cp examples/systemd/file-relay.service /etc/systemd/system/
sudo chown root:root /etc/systemd/system/file-relay.service
sudo chmod 644 /etc/systemd/system/file-relay.service
```

Edit the service file if needed:

```bash
sudo nano /etc/systemd/system/file-relay.service
```

Adjust environment variables:
- `MAX_PEERS` - Maximum concurrent peers (default: 1000)
- `METRICS_ADDR` - Metrics endpoint address (default: 0.0.0.0:8081)
- `RUST_LOG` - Log level (debug, info, warn, error)

### 5. Reload Systemd and Enable Service

```bash
sudo systemctl daemon-reload
sudo systemctl enable file-relay
```

### 6. Start Service

```bash
sudo systemctl start file-relay
```

### 7. Verify Service Status

```bash
sudo systemctl status file-relay
```

Expected output:
```
● file-relay.service - FILE Relay Server
     Loaded: loaded (/etc/systemd/system/file-relay.service; enabled; vendor preset: enabled)
     Active: active (running) since ...
```

## Management Commands

### View Logs

```bash
# Real-time logs
sudo journalctl -u file-relay -f

# Last 100 lines
sudo journalctl -u file-relay -n 100

# Logs since boot
sudo journalctl -u file-relay -b
```

### Restart Service

```bash
sudo systemctl restart file-relay
```

### Stop Service

```bash
sudo systemctl stop file-relay
```

### Disable Service

```bash
sudo systemctl disable file-relay
```

## Monitoring

### Health Check

```bash
curl http://localhost:8081/health
```

Expected response:
```json
{"status":"healthy","service":"FILE Relay Server"}
```

### Metrics

```bash
curl http://localhost:8081/metrics
```

### Resource Usage

```bash
systemctl status file-relay
```

Shows:
- Memory usage
- CPU usage
- Active tasks

## Firewall Configuration

Allow UDP port 8080 for relay traffic:

```bash
# UFW (Ubuntu/Debian)
sudo ufw allow 8080/udp

# firewalld (RHEL/CentOS/Fedora)
sudo firewall-cmd --permanent --add-port=8080/udp
sudo firewall-cmd --reload

# iptables
sudo iptables -A INPUT -p udp --dport 8080 -j ACCEPT
sudo iptables-save > /etc/iptables/rules.v4
```

Metrics port 8081 should only be accessible from localhost or trusted IPs:

```bash
# UFW - allow from specific IP
sudo ufw allow from 10.0.0.0/8 to any port 8081 proto tcp

# firewalld - allow from specific zone
sudo firewall-cmd --permanent --zone=internal --add-port=8081/tcp
```

## Updating

1. Stop the service:
```bash
sudo systemctl stop file-relay
```

2. Replace the binary:
```bash
sudo cp target/release/file-relay /usr/local/bin/
```

3. Start the service:
```bash
sudo systemctl start file-relay
```

4. Verify:
```bash
sudo systemctl status file-relay
/usr/local/bin/file-relay --version
```

## Troubleshooting

### Service Fails to Start

Check logs:
```bash
sudo journalctl -u file-relay -n 50
```

Common issues:
- Port 8080 already in use: `sudo lsof -i :8080`
- Permission denied: Check file ownership and permissions
- Binary not found: Verify path in `ExecStart`

### High Memory Usage

Check current usage:
```bash
systemctl status file-relay
```

Adjust memory limits in service file:
```bash
sudo nano /etc/systemd/system/file-relay.service
```

Change `MemoryMax` and `MemoryHigh`, then:
```bash
sudo systemctl daemon-reload
sudo systemctl restart file-relay
```

### Service Crashes Repeatedly

Check restart count:
```bash
systemctl show file-relay | grep Restart
```

View crash logs:
```bash
sudo journalctl -u file-relay --since "10 minutes ago"
```

Increase restart limits in service file if needed:
```
StartLimitBurst=10
StartLimitInterval=300s
```

## Uninstallation

```bash
# Stop and disable service
sudo systemctl stop file-relay
sudo systemctl disable file-relay

# Remove service file
sudo rm /etc/systemd/system/file-relay.service
sudo systemctl daemon-reload

# Remove binary
sudo rm /usr/local/bin/file-relay

# Remove working directory
sudo rm -rf /var/lib/file-relay

# Remove service user
sudo userdel file-relay
```

## Security Recommendations

1. **Firewall**: Only expose port 8080/udp publicly, keep 8081/tcp internal
2. **Updates**: Regularly update to latest version for security patches
3. **Monitoring**: Set up alerts for service failures and high resource usage
4. **Logs**: Rotate logs to prevent disk space exhaustion:

```bash
# /etc/systemd/journald.conf
SystemMaxUse=1G
SystemMaxFileSize=100M
```

5. **Resource limits**: Keep CPU/memory limits appropriate for your instance size

## Advanced Configuration

### Running Multiple Instances

To run multiple relay instances on different ports:

1. Copy service file:
```bash
sudo cp /etc/systemd/system/file-relay.service /etc/systemd/system/file-relay@.service
```

2. Edit template to use instance name:
```
ExecStart=/usr/local/bin/file-relay --port 808%i --metrics-port 808%i1
```

3. Start instances:
```bash
sudo systemctl start file-relay@0  # Port 8080, metrics 8081
sudo systemctl start file-relay@1  # Port 8081, metrics 8091
```

### Integration with Prometheus

Add job to Prometheus configuration:

```yaml
scrape_configs:
  - job_name: 'file-relay'
    static_configs:
      - targets: ['localhost:8081']
```

### Log Rotation

Create `/etc/logrotate.d/file-relay`:

```
/var/log/journal/file-relay/*.journal {
    weekly
    rotate 4
    compress
    delaycompress
    notifempty
    missingok
}
```
