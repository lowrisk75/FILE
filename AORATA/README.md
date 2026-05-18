# AORATA

**Zero Trust P2P Network - Invisible, Sovereign, Resilient**

> *"Security beyond the visible spectrum"*

---

## 🎯 Vision

AORATA (du grec ancien *ἀόρατος* - "invisible") est une architecture réseau Zero Trust P2P décentralisée et asynchrone qui remplace les VPN traditionnels et les protocoles SMB/NFS vulnérables.

**Promesse** : Un réseau invisible (aucun port ouvert), souverain (pas de cloud SaaS), et résilient (anti-ransomware intégré).

---

## 🏗️ Architecture

### Stack Technique

**Backend (Rust)** : `../crates/` (symlink → `RustCore/`)
- `core_transport` - MPQUIC + NAT traversal
- `crypto_fabric` - Noise, AONT, BFE
- `vfs_guard` - Anti-ransomware VFS
- `edge_ai` - Apple Neural Engine routing
- `relay_daemon` - S3-FIFO cache, CAPoW

**Frontend (Swift)** : `AORATA/`
- SwiftUI multiplatform (iOS + macOS)
- SwiftData pour métadonnées locales
- FFI bridge vers Rust core

**Network Extension** : (Phase 6)
- NEPacketTunnelProvider
- mmap IPC (App Groups)

---

## 🚀 Getting Started

### Prérequis

- Xcode 15.3+
- Rust toolchain 1.94.1+
- macOS 14+ / iOS 17+

### ⚡ Setup Automatique (100% COMPLET)

```bash
# Setup déjà exécuté automatiquement ✅
./setup-no-sudo.sh      # Configure entitlements & team
./add_capabilities.py   # Ajoute capabilities au projet
```

**Tout est configuré !** Prêt pour Phase 1 (MPQUIC).

### Build

```bash
# 1. Compiler le Rust core
cd RustCore
cargo build --release --workspace

# 2. Build Swift (sans signing pour développement)
xcodebuild -project AORATA.xcodeproj -scheme AORATA \
  -destination 'platform=macOS' CODE_SIGNING_ALLOWED=NO build

# Ou dans Xcode
open AORATA.xcodeproj
# Cmd+B pour build, Cmd+R pour run
```

**Note** : `CODE_SIGNING_ALLOWED=NO` permet de développer sans provisioning profile. Pour déployer sur device physique plus tard, voir `QUICK_START.md` section "Apple Developer Portal Setup".

---

## 📦 Structure du Projet

```
AORATA/
├── AORATA/                     # Swift app (iOS + macOS)
│   ├── AorataApp.swift        # @main
│   ├── Models/                # SwiftData models
│   ├── Views/                 # SwiftUI views
│   └── Services/              # FFI bridge, Network
│
├── RustCore/                   # Symlink → ../crates/
│   ├── core_transport/        # MPQUIC
│   ├── crypto_fabric/         # Noise, AONT
│   └── ...
│
├── Docs/                       # Documentation complète
│   ├── NOTEBOOKLM_ACCESS.md   # Specs techniques
│   ├── PROJECT_MEMORY.md       # Contexte projet
│   └── architecture/
│       └── IMPLEMENTATION_ROADMAP.md
│
├── AORATATests/               # Swift Testing
├── AORATAUITests/             # UI Tests
└── Resources/                  # Assets, configs
```

---

## 🔐 Capabilities & Entitlements

### App Groups
- **Identifier** : `group.com.lorislab.aorata`
- **Usage** : IPC mmap zero-copy entre app et Network Extension

### Network Extensions
- **Type** : Packet Tunnel Provider
- **Usage** : Tunnel IP pour MPQUIC P2P

### Background Modes (optionnel)
- Network authentication
- Background fetch

---

## 🧪 Tests

```bash
# Swift Testing
# Xcode: Cmd+U

# Rust tests
cd RustCore
cargo test --workspace
```

---

## 📚 Documentation

### Documentation Locale
- **Getting Started** : `Docs/GETTING_STARTED.md`
- **Implementation Roadmap** : `Docs/architecture/IMPLEMENTATION_ROADMAP.md`
- **NotebookLM Access** : `Docs/NOTEBOOKLM_ACCESS.md`
- **TODO** : `../TODO.md`

### NotebookLM (Source de Vérité)
🔗 https://notebooklm.google.com/notebook/86bc04f6-444f-4882-b54c-96467cf7cda4

Toutes les spécifications techniques détaillées sont dans NotebookLM.

---

## 🎯 Roadmap

| Phase | Module | Status |
|-------|--------|--------|
| **1** | core_transport (MPQUIC + NAT) | 🔨 In Progress |
| **2** | crypto_fabric (Noise, AONT, BFE) | ⏳ TODO |
| **3** | vfs_guard (Anti-ransomware) | ⏳ TODO |
| **4** | edge_ai (ANE routing) | ⏳ TODO |
| **5** | relay_daemon (S3-FIFO, CAPoW) | ⏳ TODO |
| **6** | Swift App + Network Extension | ⏳ TODO |
| **7** | Integration & Deploy | ⏳ TODO |

Voir `../TODO.md` pour la liste complète des tâches.

---

## 🔧 Configuration

### Bundle IDs
- **App** : `com.lorislab.aorata`
- **Network Extension** : `com.lorislab.aorata.tunnel`
- **App Group** : `group.com.lorislab.aorata`

### Signing
- **Team** : TDV6D5L785 (Christine Martin)
- **Provisioning** : Automatic

---

## 🌐 Identité de Marque

### Nom
**AORATA** (ἀόρατος) - "Invisible" en grec ancien

### Tagline
*"Security beyond the visible spectrum"*

### Couleurs
- **Primary** : Noir mat (#1A1A1A)
- **Secondary** : Violet électrique (#7C3AED)
- **Accent** : Gris ardoise (#64748B)

### Logo Concept
Hexagone avec contours centraux invisibles, suggérés par ombres périphériques. Symbolise l'invisibilité réseau (aucun port ouvert).

---

## 📖 Liens Rapides

- **Projet parent** : `../` (FILE project)
- **Rust crates** : `RustCore/` (symlink)
- **Documentation** : `Docs/`
- **NotebookLM** : https://notebooklm.google.com/notebook/86bc04f6-444f-4882-b54c-96467cf7cda4
- **Mémoire Claude** : `~/.claude/projects/-Users-kevinnadjarian/memory/file-project.md`

---

## 🤝 Contributing

Ce projet est en développement actif. Voir `../TODO.md` pour contribuer.

**Workflow** :
1. Consulter NotebookLM pour specs
2. Implémenter le module Rust
3. Tester avec `cargo test`
4. Créer FFI bridge Swift
5. Intégrer dans l'app SwiftUI

---

## 📄 License

Propriétaire - LorisLabs

**Developer** : Kevin Nadjarian  
**Publisher** : Christine Martin (TDV6D5L785)  
**Contact** : support@lorislab.fr

---

**Status** : 🔨 Phase 1 - Core Transport (MPQUIC) en cours
