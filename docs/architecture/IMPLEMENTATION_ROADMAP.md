# FILE Project - Implementation Roadmap

**Source de vérité**: NotebookLM "FILE Project" contient toutes les spécifications techniques détaillées.

## Vue d'ensemble

Architecture Zero Trust P2P décentralisée et asynchrone pour remplacer VPN et SMB/NFS hérités.

**Stack**: Rust (sécurité mémoire) + MPQUIC (mobilité 0-RTT) + Noise (auth pré-connexion) + VFS (anti-ransomware)

---

## Phase 1: Fondation Transport (core_transport) ✓ SCAFFOLDED

### Objectifs
- Socket UDP partagée (SO_REUSEADDR + SO_REUSEPORT)
- Multipath QUIC opérationnel (bascule Wi-Fi/5G/Ethernet)
- NAT traversal natif (draft-seemann)

### Modules
- ✓ `mpquic.rs` - Structure de base créée
- ✓ `nat_punch.rs` - Structure de base créée

### Tests critiques
- [ ] Test socket SO_REUSEPORT sur Linux/macOS/Windows
- [ ] Test mobilité 0-RTT (simulation bascule interface)
- [ ] Test hole punching symétrique (2 nœuds derrière NAT)
- [ ] Test TTL manipulation pour firewall restrictif
- [ ] Test DCUtR timing (synchronisation RTT/2)

### Point de validation
- [ ] 2 nœuds derrière NAT établissent connexion QUIC directe
- [ ] Bascule Wi-Fi → 5G sans perte de connexion
- [ ] Latence handover < 10ms (0-RTT confirmé)

### Référence NotebookLM
Interroger: "Implémentation complète de core_transport/mpquic.rs avec ant-quic"

---

## Phase 2: Cryptographie Zero Trust (crypto_fabric) ✓ SCAFFOLDED

### Objectifs
- Authentification Noise (pré-connexion)
- AONT (fragmentation aveugle pour relais)
- BFE (révocation instantanée sans redistribution)

### Modules
- ✓ `noise_auth.rs` - Structure de base créée
- ✓ `aont.rs` - Structure de base créée
- ✓ `puncturable.rs` - Structure de base créée

### Tests critiques
- [ ] Test handshake Noise (IK + XX patterns)
- [ ] Test AONT: reconstruction échoue si 1 part manque
- [ ] Test BFE: révocation en O(1) sans redistribution
- [ ] Test intégration cosmian_findex + BFE

### Point de validation
- [ ] Handshake Noise complété < 1 RTT
- [ ] AONT fragmente + reconstruit sans perte
- [ ] Révocation BFE instantanée (pas de propagation de clé)

### Référence NotebookLM
Interroger: "Spécifications cryptographiques complètes: Noise, AONT, BFE avec code Rust"

---

## Phase 3: Fusible Anti-Ransomware (vfs_guard) ✓ SCAFFOLDED

### Objectifs
- VFS Windows (WinFSP + STATUS_PENDING)
- VFS Linux/macOS (FUSE + Poll::Pending)
- Détection canari + stase I/O sans BSOD/Kernel Panic

### Modules
- ✓ `canary.rs` - Structure de base créée
- ✓ `windows.rs` - Structure de base créée
- ✓ `unix.rs` - Structure de base créée

### Tests critiques
- [ ] Test détection Write() sur canari
- [ ] Test stase I/O (thread piégé indéfiniment)
- [ ] Test annulation propre (IoCancelIrp / Ctrl+C)
- [ ] Test EDR notification (eBPF/ETW mock)
- [ ] Test charge: 1000 fichiers légitimes + 1 canari

### Point de validation
- [ ] Ransomware simulé piégé sans BSOD
- [ ] VFS répond aux opérations légitimes normalement
- [ ] Aucun deadlock pool WinFSP / Tokio executor

### Référence NotebookLM
Interroger: "VFS anti-ransomware: implémentation Windows WinFSP + Linux FUSE avec stase I/O"

---

## Phase 4: Routage IA Edge (edge_ai) — À IMPLÉMENTER

### Objectifs
- FFI vers Apple Neural Engine (_ANEClient, _ANECompiler)
- Optimisation Pareto (sécurité vs latence)
- Décisions de routage en temps réel (38 TOPS ANE)

### Modules à créer
- `ane_ffi.rs` - Bridge Objective-C → Rust
- `routing.rs` - Algorithme Pareto + décision chemin

### Tests critiques
- [ ] Test chargement modèle ANE (mdaiter/ane)
- [ ] Test inférence < 10ms pour décision routage
- [ ] Test fallback CPU si ANE indisponible
- [ ] Test Pareto front (3 chemins: 5G, Wi-Fi, Ethernet)

### Point de validation
- [ ] Inférence ANE fonctionnelle sur iPhone/Mac
- [ ] Routage adaptatif selon contraintes (latence/sécurité)

### Référence NotebookLM
Interroger: "Intégration Apple Neural Engine: FFI, API privées, routage IA edge"

---

## Phase 5: Relais Asynchrone (relay_daemon) — À IMPLÉMENTER

### Objectifs
- Cache S3-FIFO (éviction optimale)
- CAPoW (Anti-DDoS via Argon2)
- Store-and-forward asynchrone

### Modules à créer
- `s3_fifo.rs` - Algorithme cache S3-FIFO
- `capow.rs` - Context-Aware Proof-of-Work

### Tests critiques
- [ ] Test S3-FIFO éviction (hit rate > LRU)
- [ ] Test CAPoW difficulté adaptative (saturation disque)
- [ ] Test store-and-forward (paquets hors-ligne)
- [ ] Test profiling sysinfo (DiskUsage)

### Point de validation
- [ ] Relais stocke 10 Go de paquets aveugles
- [ ] CAPoW ajuste difficulté selon charge disque
- [ ] Débit relais > 1 Gbps

### Référence NotebookLM
Interroger: "Relay daemon: S3-FIFO cache, CAPoW anti-DDoS, store-and-forward"

---

## Phase 6: Intégration Apple (apple_platform/) — À IMPLÉMENTER

### Objectifs iOS
- NEPacketTunnelProvider (Network Extension)
- mmap Zero-Copy IPC (App Groups)
- Wi-Fi Aware (DMA EU compliance)

### Objectifs macOS
- System Extension (remplacement kext)
- ANE runtime intégré

### Fichiers à créer
- `IOS_NetworkExtension/PacketTunnel.swift`
- `IOS_NetworkExtension/IpcRingBuffer.c`
- `ANE_Runtime/ane_bridge.m` (Objective-C)

### Tests critiques
- [ ] Test NEPacketTunnelProvider tunnel IP
- [ ] Test mmap IPC < 15 Mo RAM (limite Jetsam)
- [ ] Test Wi-Fi Aware P2P (sans AWDL)
- [ ] Test ANE FFI depuis Swift

### Point de validation
- [ ] Tunnel IP fonctionnel sur iPhone/Mac
- [ ] IPC Zero-Copy < 1 µs latence
- [ ] Wi-Fi Aware découvre pairs locaux

### Référence NotebookLM
Interroger: "Intégration iOS: NEPacketTunnelProvider, mmap IPC, Wi-Fi Aware"

---

## Phase 7: Tests d'Intégration & Déploiement

### Tests end-to-end
- [ ] Scénario 1: Hôpital A → Hôpital B (2 NAT restrictifs)
- [ ] Scénario 2: Mobile 5G → Desktop Ethernet
- [ ] Scénario 3: Ransomware simulé (détection + stase)
- [ ] Scénario 4: Panne relais (fallback automatique)

### Benchmarks performance
- [ ] Latence handover Wi-Fi/5G < 10ms
- [ ] Débit MPQUIC > 80% du lien physique
- [ ] RAM iOS < 15 Mo (contrainte Jetsam)
- [ ] CPU idle < 5% (batterie mobile)

### Déploiement
- [ ] Packages Cargo (crates.io)
- [ ] Frameworks Apple (XCFramework)
- [ ] Documentation utilisateur
- [ ] Licences & conformité (GDPR, DMA)

---

## Dépendances entre Phases

```
Phase 1 (Transport)
    ↓
Phase 2 (Crypto) ← dépend de Phase 1 pour handshake
    ↓
Phase 5 (Relay) ← utilise Phase 1 + 2
    ↓
Phase 3 (VFS) ← indépendant mais testé avec Phase 1
    ↓
Phase 4 (Edge AI) ← optimise routage de Phase 1
    ↓
Phase 6 (Apple) ← intègre toutes les phases
    ↓
Phase 7 (Tests E2E + Deploy)
```

---

## Prochaines Actions Immédiates

1. **Implémenter Phase 1** (core_transport)
   - Consulter NotebookLM: "Code complet mpquic.rs avec ant-quic 0.14.192"
   - Écrire tests NAT traversal
   - Valider mobilité 0-RTT

2. **Paralléliser Phase 2** (crypto_fabric)
   - Consulter NotebookLM: "Implémentation Noise IK handshake avec snow 0.10.0"
   - Implémenter AONT selon spec
   - Tester révocation BFE

3. **Documenter décisions architecture**
   - Pourquoi ant-quic vs noq?
   - Pourquoi Noise IK vs XX?
   - Trade-offs S3-FIFO vs autres caches

---

## Ressources

- **NotebookLM Notebook**: https://notebooklm.google.com/notebook/86bc04f6-444f-4882-b54c-96467cf7cda4
- **Commande MCP**: `mcp__notebooklm__ask_question` avec session_id pour contexte
- **Crates critiques**: ant-quic, snow, winfsp-rs, async-fusex, cosmian_findex
- **API privées Apple**: mdaiter/ane (reverse engineering ANE)

---

**Règle d'or**: Avant d'implémenter un module, interroger NotebookLM pour les spécifications exactes et le code de référence. Toutes les décisions techniques critiques sont documentées dans les sources NotebookLM.
