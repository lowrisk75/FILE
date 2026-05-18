# ⚡ AORATA Quick Start

**3 étapes pour être opérationnel.**

---

## ✅ Étape 1 : Setup Automatique (100% COMPLET)

```bash
./setup-no-sudo.sh      # ← Déjà exécuté ✅
./add_capabilities.py   # ← Déjà exécuté ✅
```

**Résultats** :
- ✅ Entitlements configurés dans le projet
- ✅ Team (TDV6D5L785) configuré
- ✅ Capabilities ajoutées (App Groups + Network Extensions)
- ✅ RustCore symlink vérifié
- ✅ Rust compile correctement
- ✅ Xcode build vérifié (macOS + iOS Simulator)

**Build Commands** :
```bash
# macOS
xcodebuild -project AORATA.xcodeproj -scheme AORATA -destination 'platform=macOS' CODE_SIGNING_ALLOWED=NO build

# iOS Simulator
xcodebuild -project AORATA.xcodeproj -scheme AORATA -destination 'platform=iOS Simulator,name=Lumen Test iPhone' CODE_SIGNING_ALLOWED=NO build
```

**Note** : `CODE_SIGNING_ALLOWED=NO` permet de build sans provisioning profile. Parfait pour développement local. Pour déployer sur device physique plus tard, voir section "Apple Developer Portal Setup" ci-dessous.

---

## 🚀 Étape 2 : Développement (START HERE)

### Phase 1 : Core Transport (MPQUIC)

```bash
cd RustCore/crates/core_transport

# 1. Consulter NotebookLM pour specs
# https://notebooklm.google.com/notebook/86bc04f6-444f-4882-b54c-96467cf7cda4

# 2. Question à poser à NotebookLM :
# "Implémentation complète de mpquic.rs avec ant-quic 0.14.192"

# 3. Implémenter dans src/mpquic.rs
# Remplacer les TODO par le code NotebookLM

# 4. Tester
cargo test -p core_transport

# 5. Build
cargo build --release
```

### Créer les premiers Models Swift

Dans Xcode, créer `AORATA/Models/NetworkNode.swift` :

```swift
import SwiftData
import Foundation

@Model
class NetworkNode {
    var id: UUID
    var name: String
    var ipAddress: String
    var lastSeen: Date
    var isActive: Bool
    
    init(name: String, ipAddress: String = "") {
        self.id = UUID()
        self.name = name
        self.ipAddress = ipAddress
        self.lastSeen = Date()
        self.isActive = false
    }
}
```

### Premier Test Swift

Dans `AORATATests/AORATATests.swift` :

```swift
import Testing
import SwiftData
@testable import AORATA

struct NetworkNodeTests {
    @Test func createNode() async throws {
        let node = NetworkNode(name: "Test Node")
        #expect(node.name == "Test Node")
        #expect(node.isActive == false)
    }
}
```

Run : `Cmd+U`

---

## 📚 Documentation

| Quoi | Où |
|------|-----|
| **Specs techniques** | [NotebookLM](https://notebooklm.google.com/notebook/86bc04f6-444f-4882-b54c-96467cf7cda4) |
| **TODO complet** | `../TODO.md` |
| **Roadmap** | `Docs/architecture/IMPLEMENTATION_ROADMAP.md` |
| **Xcode guide** | `Docs/XCODE_SETUP.md` |

---

## 🎯 Workflow Quotidien

```bash
# Matin
1. Choisir une tâche dans ../TODO.md
2. Consulter NotebookLM pour specs
3. Implémenter dans RustCore/
4. Tester : cargo test -p <crate>

# Après-midi
5. Créer FFI bridge Swift si nécessaire
6. Intégrer dans app SwiftUI
7. Tester : Cmd+U dans Xcode

# Soir
8. Commit : git commit -m "[module] feat: description"
9. Update ../TODO.md
```

---

## ⚡ Commandes Rapides

```bash
# Build Rust
cd RustCore && cargo build --release

# Test Rust
cargo test --workspace

# Build Swift (CLI)
xcodebuild -project AORATA.xcodeproj -scheme AORATA -destination 'platform=iOS Simulator,name=iPhone 15'

# Ou dans Xcode
Cmd+B  # Build
Cmd+U  # Test
Cmd+R  # Run
```

---

## 🐛 Troubleshooting

### "No such module 'AORATA'" dans les tests
→ Clean : `Cmd+Shift+K`, puis rebuild `Cmd+B`

### Signing errors
→ Build avec `CODE_SIGNING_ALLOWED=NO` pour développement local
→ Pour device : voir section Apple Developer Portal ci-dessous

### Rust ne compile pas
→ `cd RustCore && cargo clean && cargo build`

---

## 📱 Apple Developer Portal Setup (Optionnel — Pour Device Plus Tard)

Nécessaire uniquement quand tu veux :
- Tester sur iPhone/iPad/Mac physique
- Déployer sur TestFlight
- Publier sur l'App Store

### Étapes

1. **Créer l'App Group**
   - Aller sur https://developer.apple.com/account/resources/identifiers
   - Click `+` → `App Groups`
   - Description : `AORATA Shared Data`
   - Identifier : `group.com.lorislab.aorata`
   - Click `Continue` → `Register`

2. **Activer Capabilities sur le Bundle ID**
   - Identifiers → App IDs → `com.lorislab.aorata` (ou créer si n'existe pas)
   - Click `Edit` (ou configure pendant création)
   - Activer : `☑️ App Groups`, `☑️ Network Extensions`
   - Pour App Groups : sélectionner `group.com.lorislab.aorata`
   - Click `Save`

3. **Regenerer Provisioning Profile**
   - Retourner dans Xcode
   - Signing & Capabilities → Click "Download Manual Profiles" ou rebuild
   - Xcode va auto-générer un nouveau profile avec les capabilities

4. **Build sans `CODE_SIGNING_ALLOWED=NO`**
   ```bash
   xcodebuild -project AORATA.xcodeproj -scheme AORATA -destination 'platform=iOS,name=Malware' build
   ```

---

**Tu es prêt !** 🎯

**Next** : Commence Phase 1 (MPQUIC) — tout compile, tout est configuré !
