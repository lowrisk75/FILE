# FILE Relay VPS Setup Guide

Guide complet pour déployer 3 relays FILE géographiquement distribués.

## Architecture cible

```
┌─────────────────────────────────────────────────┐
│  AORATA Client (iPhone/Mac)                     │
│  ├─ Essaie 3 relays en parallèle (Happy Eyeballs)│
│  ├─ Plus rapide gagne (latency RTT/2 critique)  │
│  └─ Failover auto si relay down                 │
└──────────┬──────────┬──────────┬─────────────────┘
           │          │          │
    ┌──────▼───┐ ┌───▼──────┐ ┌─▼──────────┐
    │ 🇪🇺 EU    │ │ 🇺🇸 USA   │ │ 🇯🇵 Asia   │
    │ Hetzner  │ │ Vultr    │ │ Vultr      │
    │ Germany  │ │ New York │ │ Tokyo      │
    │ ~5€/mois │ │ ~6€/mois │ │ ~6€/mois   │
    └──────────┘ └──────────┘ └────────────┘
         ↓            ↓            ↓
    Coordination MPQUIC (pas de data forwarding)
    PUNCH_ME_NOW + OBSERVED_ADDRESS frames
```

**Coût total: ~15.50€/mois** (coordination uniquement, pas de bandwidth data)

---

## Étape 1: Créer les VPS

### VPS 1 — Hetzner (Europe)

1. **Créer compte**: https://accounts.hetzner.com/signUp
2. **Créer VPS**:
   - Cloud → New Project → "FILE Relay"
   - Add Server
   - Location: **Falkenstein** (Germany, FSN1)
   - Image: **Debian 12**
   - Type: **CPX11** (2 vCPU, 2GB RAM, 40GB NVMe)
   - SSH Keys: Ajouter ta clé publique `~/.ssh/id_ed25519.pub`
   - Firewall: Create new "FILE-Relay"
     - Inbound: SSH (22/tcp), QUIC (8080/udp), QUIC-TCP (8080/tcp)
     - Outbound: Allow all
   - Volumes: None
   - Additional features: None
   - **Prix: 4.51€/mois**

3. **Noter l'IP publique**: `XXX.XXX.XXX.XXX` → tu la mettras dans le script

### VPS 2 — Vultr (USA)

1. **Créer compte**: https://my.vultr.com/
2. **Créer VPS**:
   - Deploy → Deploy New Server
   - Choose Server: **Cloud Compute - Regular Performance**
   - Server Location: **New York (EWR)** (USA, New Jersey)
   - Server Image: **Debian 12 x64**
   - Server Size: **1 vCore, 1GB RAM, 25GB SSD** ($6/month)
   - Additional Features:
     - ✅ Enable IPv6
     - ✅ Enable Auto Backups (optionnel, +$1.20/mois)
   - SSH Keys: Add Key → coller `~/.ssh/id_ed25519.pub`
   - Firewall Group: Create new "FILE-Relay"
     - SSH: 22/tcp from anywhere
     - QUIC: 8080/udp from anywhere
     - QUIC-TCP: 8080/tcp from anywhere
   - Server Hostname: `file-relay-us`
   - **Prix: $6/mois (~5.50€)**

3. **Noter l'IP publique**: `YYY.YYY.YYY.YYY`

### VPS 3 — Vultr (Asia)

1. **Même compte Vultr** (tu peux avoir plusieurs VPS)
2. **Créer VPS**:
   - Deploy → Deploy New Server
   - Choose Server: **Cloud Compute - Regular Performance**
   - Server Location: **Tokyo (NRT)** (Japan)
   - Server Image: **Debian 12 x64**
   - Server Size: **1 vCore, 1GB RAM, 25GB SSD** ($6/month)
   - SSH Keys: Same as VPS 2
   - Firewall Group: Réutiliser "FILE-Relay"
   - Server Hostname: `file-relay-asia`
   - **Prix: $6/mois (~5.50€)**

3. **Noter l'IP publique**: `ZZZ.ZZZ.ZZZ.ZZZ`

---

## Étape 2: Vérifier SSH connectivity

Teste que tu peux te connecter aux 3 VPS:

```bash
# Europe
ssh root@XXX.XXX.XXX.XXX
exit

# USA
ssh root@YYY.YYY.YYY.YYY
exit

# Asia
ssh root@ZZZ.ZZZ.ZZZ.ZZZ
exit
```

Si ça ne marche pas:
- Vérifie que ta clé SSH publique est bien ajoutée sur Hetzner/Vultr
- Vérifie firewall permet SSH (port 22)
- Vérifie que les VPS sont bien "Running"

---

## Étape 3: Configurer le script deployment

Édite `deploy-multi-vps.sh` et remplis les IPs:

```bash
# Configuration - UPDATE THESE AFTER VPS CREATION
RELAY_EU_IP="XXX.XXX.XXX.XXX"    # Hetzner Germany
RELAY_US_IP="YYY.YYY.YYY.YYY"    # Vultr New York
RELAY_ASIA_IP="ZZZ.ZZZ.ZZZ.ZZZ"  # Vultr Tokyo
```

Rend le script exécutable:

```bash
chmod +x deploy-multi-vps.sh
```

---

## Étape 4: Lancer le deployment

```bash
./deploy-multi-vps.sh
```

Le script va déployer les 3 relays **en parallèle** (durée: ~10-15 minutes).

**Ce qu'il fait sur chaque VPS**:
1. ✅ Teste SSH connectivity
2. 📦 Installe dépendances (build-essential, libssl-dev, etc.)
3. 🦀 Installe Rust toolchain
4. 📤 Upload source FILE
5. 🔨 Build `relay_server` en release (~5-10 min)
6. ⚙️  Installe systemd service
7. 🔒 Configure UFW firewall
8. 🚀 Démarre le relay
9. 🔍 Vérifie UDP port 8080 listening

**Output attendu à la fin**:

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📊 DEPLOYMENT SUMMARY
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

🇪🇺 Europe:  ✅ SUCCESS - XXX.XXX.XXX.XXX:8080
🇺🇸 USA:     ✅ SUCCESS - YYY.YYY.YYY.YYY:8080
🇯🇵 Asia:    ✅ SUCCESS - ZZZ.ZZZ.ZZZ.ZZZ:8080

🎉 All 3 relays deployed successfully!
```

---

## Étape 5: Vérifier que les relays fonctionnent

### Test 1: Vérifier que le process tourne

```bash
# Europe
ssh root@XXX.XXX.XXX.XXX "systemctl status file-relay"

# USA
ssh root@YYY.YYY.YYY.YYY "systemctl status file-relay"

# Asia
ssh root@ZZZ.ZZZ.ZZZ.ZZZ "systemctl status file-relay"
```

Tu dois voir `Active: active (running)`.

### Test 2: Vérifier UDP port 8080 listening

```bash
# Europe
ssh root@XXX.XXX.XXX.XXX "ss -ulnp | grep :8080"

# Output attendu:
# UNCONN 0 0 0.0.0.0:8080 0.0.0.0:* users:(("file-relay",pid=1234,fd=3))
```

### Test 3: Surveiller les logs live

```bash
# Europe
ssh root@XXX.XXX.XXX.XXX "journalctl -fu file-relay"

# Laisse tourner, tu verras les connexions arriver quand AORATA se connecte
```

---

## Étape 6: Intégrer dans AORATA app

Édite `AORATA/AORATATunnelExtension/PacketTunnelProvider.swift`:

```swift
// Multi-relay configuration (Happy Eyeballs v2)
let relayAddresses = [
    "XXX.XXX.XXX.XXX:8080",  // Europe (Hetzner Germany)
    "YYY.YYY.YYY.YYY:8080",  // USA (Vultr New York)
    "ZZZ.ZZZ.ZZZ.ZZZ:8080"   // Asia (Vultr Tokyo)
]

// Try all relays in parallel, fastest wins
for address in relayAddresses {
    Task {
        do {
            let handle = try self.transport?.connect(to: address)
            // First successful connection cancels others
            self.connectionHandle = handle
            self.logger.info("✅ Connected to relay: \(address)")
            break
        } catch {
            self.logger.warning("⚠️ Relay \(address) failed: \(error)")
        }
    }
}
```

---

## Monitoring & Maintenance

### Surveiller les 3 relays

```bash
# Europe health
ssh root@XXX.XXX.XXX.XXX "systemctl status file-relay && ss -ulnp | grep :8080"

# USA health
ssh root@YYY.YYY.YYY.YYY "systemctl status file-relay && ss -ulnp | grep :8080"

# Asia health
ssh root@ZZZ.ZZZ.ZZZ.ZZZ "systemctl status file-relay && ss -ulnp | grep :8080"
```

### Redémarrer un relay

```bash
ssh root@XXX.XXX.XXX.XXX "systemctl restart file-relay"
```

### Mettre à jour le code relay

Si tu changes `relay_server/src/main.rs`:

```bash
# Re-run deployment (il rebuild + redeploy)
./deploy-multi-vps.sh
```

### Surveiller bandwidth usage

```bash
# Hetzner: Dashboard → Servers → file-relay-eu → Traffic
# Vultr: Dashboard → Servers → file-relay-us → Bandwidth

# Expected: très faible (~100MB/mois) car coordination-only
```

### Cost tracking

- **Hetzner**: €4.51/mois → facturé mensuellement
- **Vultr USA**: $6/mois → facturé horaire ($0.009/hour)
- **Vultr Asia**: $6/mois → facturé horaire ($0.009/hour)

**Total: ~€15.50/mois**

---

## Troubleshooting

### Problème: SSH connection refused

```bash
ssh root@XXX.XXX.XXX.XXX
# ssh: connect to host XXX.XXX.XXX.XXX port 22: Connection refused
```

**Solution**:
1. Vérifie VPS is running (Hetzner/Vultr dashboard)
2. Vérifie firewall permet SSH (Hetzner: Firewall rules, Vultr: Firewall Group)
3. Vérifie SSH key bien ajoutée (Hetzner: SSH Keys, Vultr: SSH Keys)

### Problème: Build fails avec "error: linker cc not found"

**Solution**: C'est normal si `build-essential` pas installé. Le script l'installe automatiquement.

### Problème: Port 8080 already in use

```bash
# Output: Error: Address already in use (os error 48)
```

**Solution**: Un autre process utilise le port 8080.

```bash
ssh root@XXX.XXX.XXX.XXX "ss -ulnp | grep :8080"
# Kill le process existant
ssh root@XXX.XXX.XXX.XXX "systemctl restart file-relay"
```

### Problème: AORATA ne se connecte à aucun relay

**Checklist**:
1. ✅ Les 3 relays tournent (`systemctl status file-relay`)
2. ✅ UDP 8080 listening (`ss -ulnp | grep :8080`)
3. ✅ Firewall VPS permet UDP 8080 (Hetzner/Vultr firewall rules)
4. ✅ AORATA a les bonnes IPs dans `relayAddresses[]`
5. ✅ iPhone/Mac ont accès internet (teste avec Safari)

### Problème: "Permission denied (publickey)"

Tu n'as pas ajouté ta clé SSH sur le VPS.

**Solution**:
```bash
# Copie ta clé publique
cat ~/.ssh/id_ed25519.pub | pbcopy

# Hetzner: Security → SSH Keys → Add SSH Key
# Vultr: Account → SSH Keys → Add SSH Key

# Puis re-crée le VPS avec cette clé
```

---

## Sécurité hardening (optionnel mais recommandé)

### Fail2ban pour SSH

Le script installe déjà `fail2ban` qui:
- Ban automatiquement après 5 tentatives SSH ratées
- Ban pendant 10 minutes
- Protège contre brute-force

Vérifier:
```bash
ssh root@XXX.XXX.XXX.XXX "fail2ban-client status sshd"
```

### Limiter CAPoW complexity (anti-DDoS)

Si les relays sont hammered par des bots:

```bash
# Édite /opt/file-relay/config.toml (TODO: pas encore implémenté)
[capow]
max_difficulty = 22  # Default 20, augmente = plus dur pour attaquant
max_peers = 1000     # Limite connexions simultanées
```

### Monitoring avec Uptime Kuma

Tu peux ajouter les 3 relays dans ton Uptime Kuma (LXC 152):

- URL: `http://XXX.XXX.XXX.XXX:8080`
- Type: HTTP(s)
- Heartbeat Interval: 60s
- Retries: 3

---

## Next steps après deployment

1. ✅ Déployer 3 VPS (ce guide)
2. ⏭️ Tester P2P iPhone ↔ Mac via relay
3. ⏭️ Implémenter MASQUE fallback (si P2P échoue)
4. ⏭️ Ajouter store-and-forward (v2.0)
5. ⏭️ Multi-relay load balancing (Round-robin + health checks)

---

## Alternatives considered (pourquoi pas ?)

### Pourquoi pas Cloudflare Tunnel ?

- ❌ Ajoute Cloudflare comme dependency (pas souverain)
- ❌ Latency overhead (hop supplémentaire)
- ❌ Rate limiting Cloudflare peut casser NAT traversal timing

### Pourquoi pas relay local homelab ?

- ❌ Pas d'IP publique → OBSERVED_ADDRESS frames ne fonctionnent pas
- ❌ Flat network 10.9.8.x → pas de NAT à traverser (faux test)
- ❌ Latency critique pour PUNCH_ME_NOW (RTT/2) nécessite geographic distribution

### Pourquoi pas Tailscale DERP servers ?

- ❌ "aveu de faiblesse" si FILE compete avec Tailscale
- ❌ Metadata leakage à Tailscale Inc.
- ❌ Pas de contrôle sur logs/retention

---

## Coût breakdown

| Item | Provider | Prix/mois | Rôle |
|------|----------|-----------|------|
| VPS Europe | Hetzner CPX11 | €4.51 | Primary relay (latency EU clients) |
| VPS USA | Vultr Regular | $6 (~€5.50) | Americas failover + latency |
| VPS Asia | Vultr Regular | $6 (~€5.50) | APAC failover + latency |
| **TOTAL** | | **~€15.50** | **99.999% SLA coordination** |

**Bandwidth attendu**: ~100-500 MB/mois par relay (coordination frames uniquement, pas de data forwarding).

---

## Support

Si problème pendant deployment:
1. Check logs: `ssh root@XXX.XXX.XXX.XXX "journalctl -fu file-relay"`
2. Check systemd: `systemctl status file-relay`
3. Check firewall: `ufw status verbose`
4. Check port: `ss -ulnp | grep :8080`

Si tout échoue: ouvre issue sur `~/GitHub/FILE/` avec logs complets.
