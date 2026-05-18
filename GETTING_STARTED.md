# Getting Started - Projet FILE

Guide rapide pour démarrer le développement du projet FILE (Zero Trust P2P Network).

---

## 🚀 Démarrage Rapide

### 1. Vérifier l'environnement

```bash
# Rust toolchain (requis)
rustc --version  # 1.94.1 minimum
cargo --version

# Optionnel (pour Phase 6 - Apple)
xcode-select --version
```

### 2. Compiler le workspace

```bash
cd ~/GitHub/FILE
cargo check --workspace  # Vérification rapide
cargo build --workspace  # Build complet
cargo test --workspace   # Tests (peu de tests pour l'instant)
```

✅ Si tout compile, c'est bon !

---

## 📚 Étape par Étape

### Phase 1: Core Transport (COMMENCER ICI)

**Objectif**: Implémenter MPQUIC + NAT traversal

#### 1. Consulter NotebookLM

Voir `docs/NOTEBOOKLM_ACCESS.md` pour les instructions complètes.

Via Claude Code:
```javascript
ask_question({
  notebook_id: "file-project",
  question: "Implémentation complète de mpquic.rs avec ant-quic 0.14.192",
  source_format: "footnotes"
})
```

Ou directement sur https://notebooklm.google.com/notebook/86bc04f6-444f-4882-b54c-96467cf7cda4

#### 2. Implémenter `crates/core_transport/src/mpquic.rs`

Le fichier existe déjà avec:
- Structure `MpQuicFabric`
- Fonction `new()` avec TODOs
- Commentaires détaillés

Remplacer les `// TODO:` par le code obtenu de NotebookLM.

#### 3. Implémenter `crates/core_transport/src/nat_punch.rs`

Fonction `execute_simultaneous_open()`:
- TTL manipulation (3-4 hops)
- DCUtR timing (RTT/2)
- QUIC frame extensions

#### 4. Écrire les tests

```bash
# Créer tests/mpquic_tests.rs
cd ~/GitHub/FILE/crates/core_transport
mkdir -p tests
```

Tests critiques (voir TODO.md):
- Test SO_REUSEPORT
- Test mobilité 0-RTT
- Test NAT traversal (2 nœuds)

#### 5. Valider

```bash
cargo test -p core_transport
cargo clippy -p core_transport
```

**Point de validation Phase 1**:
- [ ] 2 nœuds derrière NAT établissent connexion QUIC directe
- [ ] Bascule Wi-Fi → 5G sans perte de connexion
- [ ] Latence handover < 10ms

---

## 📖 Documentation Essentielle

### Dans ce repo

| Fichier | Description |
|---------|-------------|
| `README.md` | Vue d'ensemble du projet |
| `TODO.md` | Liste complète des tâches par phase |
| `GETTING_STARTED.md` | Ce fichier - guide de démarrage |
| `docs/NOTEBOOKLM_ACCESS.md` | Comment accéder aux specs NotebookLM |
| `docs/PROJECT_MEMORY.md` | Mémoire complète du projet |
| `docs/architecture/IMPLEMENTATION_ROADMAP.md` | Roadmap détaillée 7 phases |

### NotebookLM

**Source de vérité** pour toutes les spécifications techniques:
- Algorithmes complets
- Code de référence
- Benchmarks
- Edge cases

🔗 https://notebooklm.google.com/notebook/86bc04f6-444f-4882-b54c-96467cf7cda4

---

## 🛠️ Structure du Projet

```
FILE/
├── Cargo.toml                  # Workspace avec 5 crates
├── README.md                   # Vue d'ensemble
├── TODO.md                     # Tâches par phase
├── GETTING_STARTED.md          # Ce fichier
│
├── docs/
│   ├── NOTEBOOKLM_ACCESS.md           # Accès specs détaillées
│   ├── PROJECT_MEMORY.md              # Mémoire complète
│   └── architecture/
│       └── IMPLEMENTATION_ROADMAP.md  # Roadmap 7 phases
│
├── crates/
│   ├── core_transport/         # Phase 1 ← COMMENCER ICI
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── mpquic.rs       # MPQUIC + ant-quic
│   │   │   └── nat_punch.rs    # NAT hole punching
│   │   └── Cargo.toml
│   │
│   ├── crypto_fabric/          # Phase 2
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── noise_auth.rs   # Noise protocol
│   │   │   ├── aont.rs         # All-Or-Nothing Transform
│   │   │   └── puncturable.rs  # BFE révocation
│   │   └── Cargo.toml
│   │
│   ├── vfs_guard/              # Phase 3
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── canary.rs       # Détection ransomware
│   │   │   ├── windows.rs      # WinFSP STATUS_PENDING
│   │   │   └── unix.rs         # FUSE Poll::Pending
│   │   └── Cargo.toml
│   │
│   ├── edge_ai/                # Phase 4 (TODO)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── ane_ffi.rs      # Apple Neural Engine FFI
│   │   │   └── routing.rs      # Pareto router
│   │   └── Cargo.toml
│   │
│   └── relay_daemon/           # Phase 5 (TODO)
│       ├── src/
│       │   ├── lib.rs
│       │   ├── s3_fifo.rs      # Cache S3-FIFO
│       │   └── capow.rs        # Anti-DDoS CAPoW
│       └── Cargo.toml
│
└── apple_platform/             # Phase 6 (TODO - Xcode)
    ├── IOS_NetworkExtension/   # NEPacketTunnelProvider
    └── ANE_Runtime/            # Objective-C FFI
```

---

## 🎯 Phases d'Implémentation

| Phase | Module | Priorité | État |
|-------|--------|----------|------|
| **1** | core_transport | 🔥 **START HERE** | Scaffolded |
| **2** | crypto_fabric | High | Scaffolded |
| **3** | vfs_guard | High | Scaffolded |
| **4** | edge_ai | Medium | Scaffolded |
| **5** | relay_daemon | Medium | Scaffolded |
| **6** | apple_platform | Low (Xcode) | TODO |
| **7** | Integration & Deploy | Low | TODO |

**Recommandation**: Faire les phases dans l'ordre (1 → 2 → 3 → ...).

---

## 💡 Workflow Quotidien

### Matin

1. **Choisir une tâche** depuis `TODO.md`
2. **Consulter NotebookLM** pour les specs
3. **Implémenter** le module
4. **Tester** avec `cargo test`

### Fin de journée

1. **Commit** les changements
   ```bash
   git add crates/core_transport/src/mpquic.rs
   git commit -m "[core_transport] feat: implement MPQUIC endpoint with ant-quic"
   ```

2. **Mettre à jour TODO.md**
   ```markdown
   - [x] Implement MpQuicFabric::new()
   - [x] Configure SO_REUSEPORT
   - [ ] Implement 0-RTT handover
   ```

3. **Documenter blockers** si nécessaire
   ```markdown
   ## Blockers
   - ant-quic NAT traversal API unclear → ask NotebookLM
   ```

---

## 🐛 Debugging

### Le workspace ne compile pas

```bash
# Nettoyer et rebuild
cargo clean
cargo build --workspace

# Vérifier une crate spécifique
cargo check -p core_transport --verbose
```

### Tests échouent

```bash
# Run avec output complet
cargo test -p core_transport -- --nocapture

# Run un test spécifique
cargo test -p core_transport test_socket_creation
```

### NotebookLM timeout

1. Réessayer avec question plus précise
2. Diviser en plusieurs questions
3. Ou accéder via web: https://notebooklm.google.com/notebook/86bc04f6-444f-4882-b54c-96467cf7cda4

---

## 📞 Aide

### Questions Architecture
→ Consulter `docs/architecture/IMPLEMENTATION_ROADMAP.md`

### Questions Spécifications Techniques
→ Interroger NotebookLM (voir `docs/NOTEBOOKLM_ACCESS.md`)

### Questions Implémentation
→ Voir les TODOs inline dans les fichiers `.rs`

### Blockers
→ Documenter dans `TODO.md` section Blockers

---

## 🎓 Ressources

### Crates Principales
- **ant-quic** 0.14.192 - QUIC + NAT traversal
- **snow** 0.10.0 - Noise protocol
- **tokio** 1.52.3 - Async runtime
- **socket2** 0.6.3 - Raw sockets
- **argon2** 0.5.3 - Anti-DDoS PoW

### Documentation Externe
- QUIC RFC 9000: https://www.rfc-editor.org/rfc/rfc9000.html
- Noise Protocol: https://noiseprotocol.org/
- draft-seemann NAT traversal: https://datatracker.ietf.org/doc/draft-seemann-quic-nat-traversal/
- WinFSP: https://winfsp.dev/doc/
- FUSE: https://www.kernel.org/doc/html/latest/filesystems/fuse.html

---

## ✅ Checklist Premier Jour

- [ ] Clone/navigate to `~/GitHub/FILE`
- [ ] Run `cargo check --workspace` (doit compiler)
- [ ] Lire `README.md`
- [ ] Lire `TODO.md`
- [ ] Ouvrir https://notebooklm.google.com/notebook/86bc04f6-444f-4882-b54c-96467cf7cda4
- [ ] Poser première question à NotebookLM: "Implémentation complète de mpquic.rs"
- [ ] Commencer à coder `crates/core_transport/src/mpquic.rs`

---

**Bon développement !** 🚀

Pour toute question, consulter NotebookLM en premier — c'est la source de vérité du projet.
