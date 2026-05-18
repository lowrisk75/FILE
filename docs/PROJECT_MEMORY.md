---
name: FILE - Zero Trust P2P Network
description: Architecture souveraine Zero Trust P2P pour remplacer VPN/SMB (anti-ransomware, MPQUIC, Noise, VFS)
metadata:
  type: project
  originSessionId: 79b334ab-e480-44a7-aee5-51595476a9d5
---

# FILE Project — Zero Trust P2P Transport Fabric

**Repo**: `~/GitHub/FILE/`

**Vision**: Tissu de transport réseau P2P décentralisé et asynchrone pour remplacer VPN et SMB/NFS hérités. Protège hôpitaux et gouvernements contre ransomwares.

**Why**: 
- VPN traditionnels = single point of failure
- SMB/NFS vulnérables aux ransomwares
- Mobilité réseau casse les connexions (Wi-Fi → 5G)
- Besoin souveraineté (pas de cloud SaaS style Tailscale)

**How to apply**:
- Consulter NotebookLM (ID: file-project) pour TOUTES les spécifications techniques détaillées
- Commande MCP: `mcp__notebooklm__ask_question` avec `notebook_id: "file-project"`
- Roadmap complète: `~/GitHub/FILE/docs/architecture/IMPLEMENTATION_ROADMAP.md`

---

## Stack Technique

- **Langage**: Rust (sécurité mémoire, performance déterministe)
- **Transport**: Multipath QUIC (ant-quic 0.14.192 ou noq 1.0.0-rc.0)
- **Crypto**: Noise 0.10.0, ChaCha20Poly1305, SHA3, BFE (révocation)
- **VFS**: WinFSP (Windows), async-fusex (Linux/macOS)
- **Apple**: Objective-C FFI, ANE (38 TOPS), NEPacketTunnelProvider

---

## Architecture (Cargo Workspace)

```
FILE/
├── crates/
│   ├── core_transport/    # MPQUIC + NAT hole punching
│   ├── crypto_fabric/     # Noise, AONT, BFE
│   ├── vfs_guard/         # Anti-ransomware VFS
│   ├── edge_ai/           # ANE routing (TODO)
│   └── relay_daemon/      # S3-FIFO cache, CAPoW (TODO)
│
├── apple_platform/
│   ├── IOS_NetworkExtension/  # NEPacketTunnelProvider (TODO)
│   └── ANE_Runtime/           # _ANEClient FFI (TODO)
│
└── docs/architecture/
    └── IMPLEMENTATION_ROADMAP.md
```

---

## Piliers Techniques

1. **Invisibilité Réseau** — Auth Noise pré-connexion, 0 port ouvert
2. **Mobilité 0-RTT** — MPQUIC (Wi-Fi/5G/Ethernet sans coupure)
3. **Fragmentation Asynchrone** — AONT (relais aveugles)
4. **Routage IA Edge** — ANE 38 TOPS (Pareto sécurité/latence)
5. **Fusible Anti-Ransomware** — VFS + stase I/O (piège sans BSOD)
6. **P2P Natif** — Wi-Fi Aware (DMA EU), pas de STUN/TURN

---

## État Actuel (2026-05-15)

- ✅ Structure Cargo workspace créée
- ✅ Phase 1 (core_transport) scaffoldée: `mpquic.rs`, `nat_punch.rs`
- ✅ Phase 2 (crypto_fabric) scaffoldée: `noise_auth.rs`, `aont.rs`, `puncturable.rs`
- ✅ Phase 3 (vfs_guard) scaffoldée: `canary.rs`, `windows.rs`, `unix.rs`
- ⏳ Phase 4 (edge_ai) — TODO
- ⏳ Phase 5 (relay_daemon) — TODO
- ⏳ Phase 6 (apple_platform) — TODO
- ⏳ Phase 7 (tests E2E) — TODO

---

## NotebookLM — Source de Vérité

**Notebook ID**: `file-project`
**URL**: https://notebooklm.google.com/notebook/86bc04f6-444f-4882-b54c-96467cf7cda4

**Contenu**:
- Spécifications techniques complètes de chaque module
- Code Rust de référence (ant-quic, snow, WinFSP, FUSE)
- Algorithmes cryptographiques (Noise, AONT, BFE)
- Intégration Apple (ANE, NEPacketTunnelProvider, mmap IPC)
- Architecture relais (S3-FIFO, CAPoW)
- Tests critiques et validation

**Workflow**:
1. Avant d'implémenter un module: interroger NotebookLM pour specs exactes
2. Exemple: "Implémentation complète de core_transport/mpquic.rs avec ant-quic"
3. Retour vers NotebookLM quand besoin de détails (algorithmes, edge cases, benchmarks)

---

## Commandes Rapides

```bash
# Build workspace complet
cd ~/GitHub/FILE && cargo build --workspace

# Tests d'un module
cargo test -p core_transport

# Interroger NotebookLM pour specs
# (via MCP dans Claude Code)
ask_question(
  notebook_id="file-project",
  question="Code complet de nat_punch.rs avec DCUtR timing"
)
```

---

## Identifiants Apple (Proposition)

- **Bundle ID**: `com.lorislab.zerotrust.network`
- **Network Extension**: `com.lorislab.zerotrust.network.tunnel`
- **App Groups**: `group.com.lorislab.zerotrust`

---

## Contraintes Critiques

### iOS Jetsam
- RAM max Network Extension: **15 Mo** (killed sinon)
- mmap IPC buffer: 2-4 Mo recommandé
- Profiler avec Instruments (Allocations)

### Windows BSOD
- Jamais `std::thread::sleep` dans VFS (deadlock WinFSP)
- Toujours `STATUS_PENDING` asynchrone
- Gérer `IoCancelIrp` (IRPLOCK_CANCELABLE)

### Linux Kernel Panic
- Jamais de blocage synchrone dans FUSE handler
- Utiliser `async-fusex` + Tokio (pas `fuse-rs` bloquant)
- Retourner `Poll::Pending` sans réveiller Waker

---

## Prochaines Actions

1. **Phase 1 (core_transport)** — Implémenter MPQUIC complet
   - Consulter NotebookLM: "ant-quic endpoint configuration"
   - Écrire tests NAT traversal
   - Valider mobilité 0-RTT

2. **Phase 2 (crypto_fabric)** — Noise handshake
   - Consulter NotebookLM: "Noise IK pattern avec snow"
   - AONT fragmentation/reconstruction
   - BFE révocation tests

3. **Documentation décisions** — Pourquoi ant-quic vs noq? IK vs XX?

---

## Références Liées

- [[moi-school-access-idea]] — Moi utilise NFC/PACE (différent mais même philosophie Zero Trust)
- [[rampart-project]] — Rampart monitore OPNsense (overlap réseau/sécurité)
- [[homelab-ai-gpu-state]] — Arc A310 + P2000 disponibles (test ANE offline avec Ollama)
