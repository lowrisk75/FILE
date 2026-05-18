# FILE Project - Documentation Index

Index central de toute la documentation du projet.

---

## 🎯 Par Rôle

### Je débute sur le projet
1. 📖 **[README.md](../README.md)** - Vue d'ensemble
2. 🚀 **[GETTING_STARTED.md](../GETTING_STARTED.md)** - Guide démarrage
3. 📚 **[NOTEBOOKLM_ACCESS.md](NOTEBOOKLM_ACCESS.md)** - Accès specs
4. ✅ **[TODO.md](../TODO.md)** - Tâches à faire

### J'implémente un module
1. 📚 **[NOTEBOOKLM_ACCESS.md](NOTEBOOKLM_ACCESS.md)** - Specs techniques
2. 🗺️ **[IMPLEMENTATION_ROADMAP.md](architecture/IMPLEMENTATION_ROADMAP.md)** - Roadmap détaillée
3. ✅ **[TODO.md](../TODO.md)** - Checklist par phase
4. 💾 **Code inline** - Voir TODOs dans les fichiers `.rs`

### Je cherche une info précise
1. 📚 **NotebookLM** - Source de vérité (voir NOTEBOOKLM_ACCESS.md)
2. 💭 **[PROJECT_MEMORY.md](PROJECT_MEMORY.md)** - Contexte complet
3. 🗺️ **[IMPLEMENTATION_ROADMAP.md](architecture/IMPLEMENTATION_ROADMAP.md)** - Architecture

---

## 📑 Par Type de Document

### Documentation Principale

| Fichier | Description | Audience |
|---------|-------------|----------|
| [README.md](../README.md) | Vue d'ensemble, vision, stack | Tous |
| [GETTING_STARTED.md](../GETTING_STARTED.md) | Guide de démarrage rapide | Nouveaux dev |
| [TODO.md](../TODO.md) | Liste tâches par phase | Tous |

### Documentation Technique

| Fichier | Description | Audience |
|---------|-------------|----------|
| [NOTEBOOKLM_ACCESS.md](NOTEBOOKLM_ACCESS.md) | Accès specs NotebookLM | Tous |
| [architecture/IMPLEMENTATION_ROADMAP.md](architecture/IMPLEMENTATION_ROADMAP.md) | Roadmap 7 phases détaillée | Dev |
| [PROJECT_MEMORY.md](PROJECT_MEMORY.md) | Contexte & décisions | Tous |

### Code Source

| Crate | Description | Phase |
|-------|-------------|-------|
| `crates/core_transport/` | MPQUIC + NAT traversal | **1** |
| `crates/crypto_fabric/` | Noise, AONT, BFE | **2** |
| `crates/vfs_guard/` | Anti-ransomware VFS | **3** |
| `crates/edge_ai/` | ANE routing | **4** |
| `crates/relay_daemon/` | S3-FIFO, CAPoW | **5** |
| `apple_platform/` | iOS/macOS integration | **6** |

---

## 🔍 Par Sujet

### Architecture Zero Trust
- [README.md](../README.md) - Vision & piliers
- [IMPLEMENTATION_ROADMAP.md](architecture/IMPLEMENTATION_ROADMAP.md) - Architecture détaillée
- NotebookLM - Spécifications complètes

### MPQUIC & Transport
- `crates/core_transport/src/mpquic.rs` - Code + TODOs
- `crates/core_transport/src/nat_punch.rs` - NAT traversal
- NotebookLM → "Implémentation MPQUIC avec ant-quic"
- [TODO.md](../TODO.md) → Phase 1

### Cryptographie (Noise, AONT, BFE)
- `crates/crypto_fabric/src/` - Tous les modules
- NotebookLM → "Noise / AONT / BFE implémentation"
- [TODO.md](../TODO.md) → Phase 2

### Anti-Ransomware VFS
- `crates/vfs_guard/src/` - Windows + Unix + Canary
- NotebookLM → "VFS anti-ransomware WinFSP / FUSE"
- [TODO.md](../TODO.md) → Phase 3

### Apple Neural Engine (ANE)
- `crates/edge_ai/src/` - FFI + Routing
- NotebookLM → "Apple Neural Engine FFI"
- [TODO.md](../TODO.md) → Phase 4

### Relais Asynchrone
- `crates/relay_daemon/src/` - S3-FIFO + CAPoW
- NotebookLM → "S3-FIFO cache / CAPoW anti-DDoS"
- [TODO.md](../TODO.md) → Phase 5

### iOS/macOS Integration
- `apple_platform/` - NEPacketTunnelProvider + ANE Runtime
- NotebookLM → "iOS NEPacketTunnelProvider / mmap IPC"
- [TODO.md](../TODO.md) → Phase 6

---

## 🎓 Ressources Externes

### NotebookLM (Source de Vérité)
🔗 **https://notebooklm.google.com/notebook/86bc04f6-444f-4882-b54c-96467cf7cda4**

**Contient**:
- Toutes les spécifications techniques
- Code de référence Rust
- Algorithmes complets
- Benchmarks & validation
- Edge cases

**Accès**: Voir [NOTEBOOKLM_ACCESS.md](NOTEBOOKLM_ACCESS.md)

### RFCs & Standards
- QUIC (RFC 9000): https://www.rfc-editor.org/rfc/rfc9000.html
- Noise Protocol: https://noiseprotocol.org/
- draft-seemann NAT traversal: https://datatracker.ietf.org/doc/draft-seemann-quic-nat-traversal/

### Crates Documentation
- ant-quic: https://docs.rs/ant-quic/
- snow: https://docs.rs/snow/
- tokio: https://docs.rs/tokio/
- socket2: https://docs.rs/socket2/

### Platform Docs
- WinFSP: https://winfsp.dev/doc/
- FUSE: https://www.kernel.org/doc/html/latest/filesystems/fuse.html
- Apple Developer: https://developer.apple.com/documentation/

---

## 🗂️ Structure Documentation

```
FILE/
├── README.md                       # Vue d'ensemble
├── GETTING_STARTED.md              # Guide démarrage
├── TODO.md                         # Tâches par phase
├── .gitignore                      # Git ignore rules
│
└── docs/
    ├── INDEX.md                    # Ce fichier
    ├── NOTEBOOKLM_ACCESS.md        # Accès NotebookLM
    ├── PROJECT_MEMORY.md           # Mémoire complète
    │
    └── architecture/
        └── IMPLEMENTATION_ROADMAP.md   # Roadmap 7 phases
```

---

## 💡 FAQ Documentation

### Où trouver les spécifications d'un module?
→ **NotebookLM** (voir [NOTEBOOKLM_ACCESS.md](NOTEBOOKLM_ACCESS.md))

### Comment démarrer le développement?
→ **[GETTING_STARTED.md](../GETTING_STARTED.md)**

### Quelle est la prochaine tâche?
→ **[TODO.md](../TODO.md)** section Phase actuelle

### Comment est structuré le projet?
→ **[README.md](../README.md)** + **[IMPLEMENTATION_ROADMAP.md](architecture/IMPLEMENTATION_ROADMAP.md)**

### Pourquoi telle décision architecture?
→ **[PROJECT_MEMORY.md](PROJECT_MEMORY.md)** + **NotebookLM**

### Comment tester un module?
→ **Code inline** dans `crates/*/tests/` + **[TODO.md](../TODO.md)** tests critiques

---

## 🔄 Maintenir cette Documentation

### Ajouter un nouveau document

1. Créer le fichier dans `docs/` (ou sous-dossier approprié)
2. Ajouter une entrée dans ce INDEX.md
3. Référencer depuis README.md ou GETTING_STARTED.md si pertinent
4. Commit avec message clair

### Modifier un document existant

1. Mettre à jour le contenu
2. Vérifier les liens dans INDEX.md sont toujours valides
3. Mettre à jour la date "Last Updated" si applicable
4. Commit avec message descriptif

### Supprimer un document

1. Supprimer le fichier
2. Retirer toutes les références dans INDEX.md
3. Vérifier aucun autre doc ne le référence
4. Commit avec raison de la suppression

---

**Navigation rapide**:
- 🏠 [Retour README](../README.md)
- 🚀 [Getting Started](../GETTING_STARTED.md)
- ✅ [TODO List](../TODO.md)
- 📚 [NotebookLM Access](NOTEBOOKLM_ACCESS.md)
