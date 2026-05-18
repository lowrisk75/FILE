# FILE Relay Server - Proxmox Testing

**Purpose**: Test FILE Relay Server locally on Proxmox before deploying to production VPS.

---

## Pourquoi Proxmox First?

✅ **Gratuit** — pas de frais VPS avant validation  
✅ **Local** — debug facile, accès direct  
✅ **Rapide** — test/redeploy en secondes  
✅ **Safe** — aucun risque financier  
✅ **Réaliste** — même config que production

---

## Prerequisites

- Proxmox VE: 10.9.8.8 (ton homelab)
- SSH access: `ssh root@10.9.8.8`
- Binary built: `cargo build --release --package relay_server`
- LXC template: `ubuntu-22.04-standard_22.04-1_amd64.tar.zst`

---

## Quick Start

### 1. Build Binary

```bash
cd ~/GitHub/FILE
cargo build --release --package relay_server
```

### 2. Deploy to LXC

```bash
cd deploy/proxmox
./deploy-lxc.sh 199 file-relay-test
```

**Ce que ça fait**:
- Crée LXC 199 (si inexistant): Ubuntu 22.04, 2 vCPU, 2GB RAM
- Upload binary
- Install systemd service
- Start relay
- Vérifie health/challenge endpoints

**Résultat**: LXC avec IP (ex: 10.9.8.199), relay listening sur :8080/:8081

### 3. Test Endpoints

```bash
# Get LXC IP
LXC_IP=$(ssh root@10.9.8.8 "pct exec 199 -- hostname -I | awk '{print \$1}'")

# Health check
curl http://$LXC_IP:8081/health

# Challenge (CAPoW)
curl http://$LXC_IP:8081/challenge | jq

# Metrics (Prometheus)
curl http://$LXC_IP:8081/metrics | grep file_relay
```

### 4. Monitor Logs

```bash
# Via Proxmox
ssh root@10.9.8.8 pct exec 199 -- journalctl -u file-relay -f

# Ou direct dans le LXC
ssh root@10.9.8.8
pct enter 199
journalctl -u file-relay -f
```

---

## Testing Workflow

### Phase 1: Service Validation
```bash
# 1. Deploy
./deploy-lxc.sh 199

# 2. Check service
ssh root@10.9.8.8 pct exec 199 -- systemctl status file-relay

# 3. Check all endpoints
LXC_IP=10.9.8.199  # Replace with actual IP
curl http://$LXC_IP:8081/health
curl http://$LXC_IP:8081/ready
curl http://$LXC_IP:8081/metrics
curl http://$LXC_IP:8081/challenge
```

### Phase 2: Client Testing

**Option A: Mock Client (Python)**
```python
import socket
import json

# Fetch challenge
import requests
challenge = requests.get('http://10.9.8.199:8081/challenge').json()
print(f"Challenge: {challenge['merkle_root'][:16]}...")

# TODO: Generate CAPoW proof, send REGISTER packet
```

**Option B: Rust Test Client**
```bash
# TODO: Create simple test client in examples/
cargo run --example test_client -- 10.9.8.199:8080
```

### Phase 3: Load Testing
```bash
# Simulate 100 peers registering
for i in {1..100}; do
    # Send REGISTER packet (need CAPoW proof)
    echo "Peer $i registering..."
done

# Check metrics
curl http://10.9.8.199:8081/metrics | grep peers_registered
```

---

## Proxmox Network Setup

### Current Network (vmbr0)
```
10.9.8.0/24 - Main homelab network
  10.9.8.8   - Proxmox host
  10.9.8.199 - FILE Relay LXC (example)
```

### Firewall (OPNsense)

**Si le relay doit être accessible depuis Internet**:
1. OPNsense port forward: WAN :8080/UDP → 10.9.8.199:8080
2. Firewall rule: Allow WAN → 10.9.8.199:8080/UDP

**Pour test local seulement**: Pas de port forward nécessaire

---

## Resource Monitoring

### LXC Resources
```bash
# CPU/Memory usage
ssh root@10.9.8.8 pct status 199
ssh root@10.9.8.8 pct exec 199 -- top -bn1 | head -20

# Disk usage
ssh root@10.9.8.8 pct exec 199 -- df -h

# Network stats
ssh root@10.9.8.8 pct exec 199 -- netstat -tuln | grep file-relay
```

### Relay Metrics
```bash
LXC_IP=10.9.8.199

# Active peers
curl -s http://$LXC_IP:8081/metrics | grep "^file_relay_active_peers"

# Packets forwarded
curl -s http://$LXC_IP:8081/metrics | grep packets_forwarded_total

# CAPoW stats
curl -s http://$LXC_IP:8081/metrics | grep capow_
```

---

## Update & Redeploy

### Binary Only (Fast)
```bash
# Rebuild
cargo build --release --package relay_server

# Redeploy (reuses existing LXC)
./deploy-lxc.sh 199
```

### Full Rebuild
```bash
# Destroy LXC
ssh root@10.9.8.8 pct destroy 199

# Redeploy from scratch
./deploy-lxc.sh 199 file-relay-test
```

---

## Troubleshooting

### Service Won't Start
```bash
# Check logs
ssh root@10.9.8.8 pct exec 199 -- journalctl -u file-relay -n 50 --no-pager

# Check binary
ssh root@10.9.8.8 pct exec 199 -- ls -la /opt/file-relay/file-relay
ssh root@10.9.8.8 pct exec 199 -- ldd /opt/file-relay/file-relay

# Test manual start
ssh root@10.9.8.8 pct exec 199 -- /opt/file-relay/file-relay --help
```

### Port Already in Use
```bash
# Check what's using port 8080
ssh root@10.9.8.8 pct exec 199 -- netstat -tulpn | grep 8080

# Kill process if needed
ssh root@10.9.8.8 pct exec 199 -- systemctl stop file-relay
```

### Network Issues
```bash
# Test Internet connectivity from LXC
ssh root@10.9.8.8 pct exec 199 -- ping -c 3 1.1.1.1

# Check LXC network config
ssh root@10.9.8.8 pct config 199 | grep net0

# Restart LXC networking
ssh root@10.9.8.8 pct reboot 199
```

---

## Migration to Production (Hetzner)

Une fois validé sur Proxmox:

### Hetzner Strasbourg (FSN1)
```bash
# 1. Provision VPS
# Location: Falkenstein (FSN1) - closest to Nantes
# Type: CPX21 (4 vCPU, 4GB RAM, €10/month)

# 2. Deploy
cd ../systemd
./deploy.sh relay-fsn1.hetzner.net

# 3. Verify
./verify-deployment.sh relay-fsn1.hetzner.net

# 4. Update DNS
# relay.yourorg.com → relay-fsn1.hetzner.net
```

**Latency Comparison**:
- Nantes → Strasbourg (FSN1): ~5-10ms
- Nantes → Frankfurt (Germany): ~15-20ms
- Nantes → Amsterdam: ~20-25ms

**Recommendation**: FSN1 (Strasbourg) pour latency optimale

---

## Performance Targets (Local Test)

### Expected Performance (LXC 2 vCPU, 2GB RAM)
- **Max peers**: 500-800 (before CPU bottleneck)
- **Packet rate**: 5k-8k packets/sec
- **CAPoW verification**: 100ms per proof
- **Memory**: ~200MB baseline + ~1KB per peer
- **Latency**: <5ms local network

### Validation Criteria
- ✅ Health endpoints respond < 50ms
- ✅ Challenge rotates every 1 hour
- ✅ Service auto-restarts on crash
- ✅ No memory leaks over 24h
- ✅ CPU < 50% under normal load

---

## Cleanup

### Stop Service
```bash
ssh root@10.9.8.8 pct exec 199 -- systemctl stop file-relay
```

### Destroy LXC
```bash
ssh root@10.9.8.8 pct stop 199
ssh root@10.9.8.8 pct destroy 199
```

---

## Next Steps

**Phase 1: Local Testing (Proxmox)** ← **YOU ARE HERE**
- [x] Deploy to LXC
- [ ] Test endpoints
- [ ] Test with mock client
- [ ] 24h stability test
- [ ] Validate metrics

**Phase 2: Production Deployment (Hetzner FSN1)**
- [ ] Provision VPS Strasbourg
- [ ] Deploy via `../systemd/deploy.sh`
- [ ] Configure monitoring (Prometheus + Grafana)
- [ ] Beta test (7 jours, 100 users)

**Phase 3: Multi-Region (Si nécessaire)**
- [ ] Deploy US-East, AP-Southeast
- [ ] GeoDNS routing
- [ ] Load testing

---

**Status**: Ready for Proxmox testing  
**Recommendation**: Test 24-48h sur Proxmox avant Hetzner  
**Cost**: €0 (Proxmox) → €10/mois (Hetzner CPX21 après validation)
