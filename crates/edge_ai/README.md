# Edge AI — Multi-Tier Routing Agent for AORATA v2

Multi-platform AI acceleration for adaptive MPQUIC path selection.

## Architecture Hybride (3 Tiers)

```
┌─────────────┬──────────────────┬────────────────┬──────────────────────┐
│ Tier        │ Platform         │ Latency        │ Distribution         │
├─────────────┼──────────────────┼────────────────┼──────────────────────┤
│ **Tier 1**  │ iOS 15+          │ 300-800µs      │ ✅ App Store         │
│ Core ML     │ macOS 12+        │ (LoRA update)  │ ✅ TestFlight        │
│ (Baseline)  │                  │ 1-2ms (infer)  │ ✅ Notarization      │
├─────────────┼──────────────────┼────────────────┼──────────────────────┤
│ **Tier 2**  │ macOS 12+ only   │ <100µs         │ ⚠️  Direct download  │
│ ANE Helper  │ (direct download)│ (LoRA update)  │ lorislab.fr/helper   │
│ (Power)     │                  │ <1ms (infer)   │ (non-notarized)      │
├─────────────┼──────────────────┼────────────────┼──────────────────────┤
│ **Tier 3**  │ Android 8.1+     │ 500-1500µs     │ ✅ APK direct        │
│ Android     │ (NNAPI)          │ (NNAPI)        │ ✅ F-Droid           │
│ (TODO)      │ Snapdragon       │ <200µs         │                      │
│             │ (Hexagon DSP)    │ (Hexagon)      │                      │
└─────────────┴──────────────────┴────────────────┴──────────────────────┘
```

## Backend Détection Runtime

L'agent détecte automatiquement le meilleur backend disponible :

1. **ANE Helper** (si installé sur macOS) → performance maximale
2. **Core ML** (iOS/macOS) → toujours disponible
3. **Hexagon DSP** (Snapdragon Android) → haute performance
4. **NNAPI** (Android baseline) → compatible tous devices

## Usage

```rust
use edge_ai::routing_agent::AneRoutingAgent;
use edge_ai::backend::BackendType;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Auto-détection
    let mut agent = AneRoutingAgent::auto().await?;
    println!("Backend: {} (tier {})",
        agent.backend_name(),
        agent.backend_tier()
    );

    // Update routing model depuis network context
    let context = NetworkContext {
        avg_ttl: 64,
        asn: 15169,
        iat_variance: 500,
        cpu_load: 25,
        num_paths: 3,
    };

    let latency = agent.update_from_context(&context).await?;
    println!("LoRA update: {:?}", latency);

    Ok(())
}
```

## Features Cargo

```toml
[dependencies]
edge_ai = { path = "../crates/edge_ai", features = ["tier1-coreml", "tier2-ane-helper"] }
```

### Features Disponibles

- `tier1-coreml` : Core ML backend (App Store)
- `tier2-ane-helper` : ANE Helper client (macOS power users)
- `tier3-android` : Android NNAPI/Hexagon (TODO)
- `default` : Tier 1 + Tier 2

## Status Implémentation

### ✅ Phase 4.1 — Infrastructure Hybride

- [x] Trait `AiBackend` unifié
- [x] Détection runtime multi-platform
- [x] Core ML backend (mock — FFI Swift TODO)
- [x] ANE Helper client (Unix socket IPC)
- [x] Android stubs (NNAPI/Hexagon TODO)
- [x] `AneRoutingAgent` orchestrator
- [x] LoRA adapter (rank 8/16/32/64/128)
- [x] FP16 numerical stability
- [x] Compilation clean (0 errors, warnings TODO)

### ⏳ Phase 4.2 — Production Backends

#### Tier 1 : Core ML FFI

**Requis** : Swift wrapper compilé en dylib

```swift
// CoreMLWrapper.swift
import CoreML

@_cdecl("coreml_load_model")
func loadModel(path: UnsafePointer<CChar>) -> OpaquePointer? {
    guard let url = URL(string: String(cString: path)) else { return nil }
    let model = try? MLModel(contentsOf: url)
    return Unmanaged.passRetained(model as AnyObject).toOpaque()
}

@_cdecl("coreml_update_lora")
func updateLora(
    model: OpaquePointer,
    a_ptr: UnsafePointer<Float16>,
    a_len: Int,
    b_ptr: UnsafePointer<Float16>,
    b_len: Int
) -> UInt64 {
    let start = DispatchTime.now()
    let model = Unmanaged<MLModel>.fromOpaque(model).takeUnretainedValue()
    let state = try! MLState(forModel: model)

    let a = Array(UnsafeBufferPointer(start: a_ptr, count: a_len))
    let b = Array(UnsafeBufferPointer(start: b_ptr, count: b_len))

    try! state.setValue(MLMultiArray(a), for: "lora_a")
    try! state.setValue(MLMultiArray(b), for: "lora_b")

    let end = DispatchTime.now()
    return UInt64(end.uptimeNanoseconds - start.uptimeNanoseconds) / 1000
}
```

**Build** :

```bash
cd crates/edge_ai/swift-wrapper
swiftc -emit-library -o libCoreMLWrapper.dylib CoreMLWrapper.swift
install_name_tool -id @rpath/libCoreMLWrapper.dylib libCoreMLWrapper.dylib
```

#### Tier 2 : ANE Helper Daemon

**Requis** : Daemon standalone avec APIs privées ANE

Le client IPC est déjà implémenté (`ane_helper_client.rs`). Le daemon sera un package macOS séparé :

```
/Applications/AORATA Helper.app/
├── Contents/
│   ├── MacOS/
│   │   └── aorata-helper          # Daemon avec dlopen PrivateFrameworks
│   ├── Frameworks/
│   │   └── ANEPrivate.framework   # Wrapper APIs privées
│   └── Info.plist
```

Distribution : lorislab.fr/aorata-helper.dmg (non-notarisé)

#### Tier 3 : Android

**TODO** : JNI bindings + Kotlin wrapper

```kotlin
// AndroidBackend.kt
import org.tensorflow.lite.Interpreter
import com.qualcomm.qti.snpe.SNPE

class AndroidBackend(modelPath: String) {
    private val nnapi: Interpreter
    private val hexagon: NeuralNetwork?

    @JvmName("updateLora")
    external fun updateLora(a: FloatArray, b: FloatArray): Long
}
```

## Tests

```bash
# Unit tests (mock backends)
cargo test -p edge_ai

# Avec ANE Helper (si installé)
cargo test -p edge_ai --features tier2-ane-helper

# Android (cross-compile)
cargo test -p edge_ai --target aarch64-linux-android --features tier3-android
```

## Benchmarks

```bash
cargo bench -p edge_ai
```

Expected results (macOS M3):

- Core ML: 300-800µs
- ANE Helper: <100µs
- Android NNAPI: 500-1500µs
- Android Hexagon: <200µs

## Documentation

```bash
cargo doc -p edge_ai --open
```

## Déploiement

### iOS App Store

```toml
[dependencies]
edge_ai = { features = ["tier1-coreml"] }  # Core ML only
```

### macOS App Store

```toml
[dependencies]
edge_ai = { features = ["tier1-coreml"] }
```

App affiche un bouton "Download ANE Helper for 3x boost" qui ouvre Safari → lorislab.fr/aorata-helper.dmg

### macOS Direct Download

```toml
[dependencies]
edge_ai = { features = ["tier1-coreml", "tier2-ane-helper"] }
```

Bundle ANE Helper avec l'app (pas d'App Store).

### Android APK

```toml
[dependencies]
edge_ai = { features = ["tier3-android"] }
```

Distribution F-Droid / APK direct.

## License

Proprietary — AORATA Protocol v2
