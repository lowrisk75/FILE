# Xcode Setup Guide - AORATA

Guide complet pour configurer le projet Xcode AORATA.

---

## ✅ Configuration Initiale (Déjà Fait)

- [x] Projet créé : `AORATA.xcodeproj`
- [x] Product Name : `Aorata`
- [x] Organization Identifier : `com.lorislab`
- [x] Bundle ID : `com.lorislab.aorata`
- [x] Interface : SwiftUI
- [x] Language : Swift
- [x] Tests : Swift Testing with XCTest UI Tests
- [x] Storage : SwiftData
- [x] Entitlements créés : `AORATA.entitlements`

---

## 🔐 Signing & Capabilities

### 1. Signing Configuration

**Target** : AORATA (iOS + macOS)

1. Ouvrir `AORATA.xcodeproj`
2. Sélectionner target **AORATA**
3. Onglet **Signing & Capabilities**

#### iOS
- **Team** : Christine Martin (TDV6D5L785)
- **Provisioning Profile** : Automatic
- **Signing Certificate** : Apple Development

#### macOS
- **Team** : Christine Martin (TDV6D5L785)
- **Provisioning Profile** : Automatic
- **Signing Certificate** : Apple Development

### 2. Capabilities à Ajouter

#### App Groups ✅
1. Click **+ Capability**
2. Chercher **App Groups**
3. Click sur **App Groups**
4. Click **+** sous "App Groups"
5. Entrer : `group.com.lorislab.aorata`
6. Click **OK**

**Usage** : IPC mmap zero-copy entre app principale et Network Extension

#### Network Extensions ✅
1. Click **+ Capability**
2. Chercher **Network Extensions**
3. Click sur **Network Extensions**
4. Cocher : **Packet Tunnel**

**Usage** : NEPacketTunnelProvider pour tunnel IP (Phase 6)

#### Keychain Sharing (Optionnel)
1. Click **+ Capability**
2. Chercher **Keychain Sharing**
3. Ajouter : `com.lorislab.aorata`

**Usage** : Partage de credentials entre app et extension

#### Background Modes (Optionnel - Phase 6)
1. Click **+ Capability**
2. Chercher **Background Modes**
3. Cocher :
   - [ ] Network authentication
   - [ ] Background fetch

---

## 📦 Targets Configuration

### Target: AORATA (App Principale)

**General** :
- Display Name : `Aorata`
- Bundle Identifier : `com.lorislab.aorata`
- Version : `0.1.0`
- Build : `1`
- Deployment Target : iOS 17.0 / macOS 14.0

**Build Settings** :
- Swift Language Version : Swift 5
- Enable Bitcode : No (pour iOS)
- Strip Debug Symbols During Copy : Yes (Release only)

### Target: Network Extension (Phase 6 - À créer plus tard)

**À créer** :
1. File > New > Target
2. iOS / macOS > Network Extension
3. Product Name : `AorataTunnel`
4. Bundle ID : `com.lorislab.aorata.tunnel`

---

## 🔗 Rust Integration (Phase 1+)

### 1. Compiler les Crates Rust

```bash
cd RustCore
cargo build --release --workspace
```

### 2. Créer un Swift Package pour FFI

**Option A : XCFramework (Recommandé)**

```bash
# Build universal binary
cd RustCore
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin

# Créer XCFramework
xcodebuild -create-xcframework \
  -library target/aarch64-apple-darwin/release/libaorata_core.a \
  -library target/x86_64-apple-darwin/release/libaorata_core.a \
  -output AorataCore.xcframework
```

**Option B : Swift Package Manager (SPM)**

Créer `RustCore/Package.swift` :
```swift
// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "AorataCore",
    platforms: [
        .iOS(.v17),
        .macOS(.v14)
    ],
    products: [
        .library(
            name: "AorataCore",
            targets: ["AorataCore"]
        )
    ],
    targets: [
        .binaryTarget(
            name: "AorataCore",
            path: "AorataCore.xcframework"
        )
    ]
)
```

### 3. Ajouter au Projet Xcode

1. File > Add Package Dependencies
2. Add Local... > Sélectionner `RustCore/`
3. Add to Target : **AORATA**

---

## 🧪 Tests Configuration

### AORATATests (Swift Testing)

**Target Configuration** :
- Host Application : AORATA
- Test Target : iOS / macOS

**Exemple Test** :
```swift
import Testing
@testable import AORATA

struct NetworkNodeTests {
    @Test func createNode() async throws {
        let node = NetworkNode(name: "Test")
        #expect(node.name == "Test")
        #expect(node.isActive == false)
    }
}
```

### AORATAUITests (UI Tests)

**Exemple Test** :
```swift
import XCTest

final class AORATAUITests: XCTestCase {
    func testLaunch() {
        let app = XCUIApplication()
        app.launch()
        XCTAssertTrue(app.exists)
    }
}
```

---

## 📱 Build Schemes

### Development Scheme
- Configuration : Debug
- Code Signing : Development
- Optimizations : Off

### Release Scheme
- Configuration : Release
- Code Signing : Distribution
- Optimizations : Aggressive (-O)
- Strip Symbols : Yes

---

## 🔍 Troubleshooting

### Signing Errors

**Problème** : "No matching provisioning profile"
**Solution** :
1. Xcode > Preferences > Accounts
2. Vérifier que Christine Martin (TDV6D5L785) est connectée
3. Manage Certificates > Download All Profiles

### Network Extension Not Working

**Problème** : Extension ne se charge pas
**Solution** :
1. Vérifier que `AORATA.entitlements` contient :
   ```xml
   <key>com.apple.developer.networking.networkextension</key>
   <array>
       <string>packet-tunnel-provider</string>
   </array>
   ```
2. Vérifier App Groups configuré identiquement dans app et extension

### Rust FFI Linking Errors

**Problème** : "Undefined symbols for architecture arm64"
**Solution** :
1. Vérifier que la lib Rust est compilée pour la bonne architecture
2. Build Settings > Other Linker Flags : `-laorata_core`
3. Build Settings > Library Search Paths : `$(PROJECT_DIR)/RustCore/target/release`

---

## 📚 Ressources

- **Apple Documentation** : https://developer.apple.com/documentation/networkextension
- **Swift Package Manager** : https://swift.org/package-manager/
- **Rust FFI** : https://doc.rust-lang.org/nomicon/ffi.html

---

## ✅ Checklist Post-Setup

- [ ] Signing configuré (TDV6D5L785)
- [ ] App Groups ajouté (`group.com.lorislab.aorata`)
- [ ] Network Extensions capability ajoutée
- [ ] Entitlements file référencé dans Build Settings
- [ ] Tests s'exécutent (Cmd+U)
- [ ] App compile et lance (Cmd+R)
- [ ] Rust crates compilent (`cargo build --release`)
- [ ] Documentation lue (`README.md`, `Docs/`)

---

**Prêt pour le développement !** 🚀
