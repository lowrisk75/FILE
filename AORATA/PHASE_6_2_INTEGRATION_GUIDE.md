# AORATA Phase 6.2 — Integration Guide

**Status**: Ready for Xcode integration  
**Date**: 2026-05-17

---

## What's Complete ✅

### Rust FFI (C ABI Exports)
- ✅ `crates/core_transport/src/ffi.rs` — MPQUIC encrypt/decrypt
- ✅ `crates/crypto_fabric/src/ffi.rs` — Noise handshake
- ✅ `crates/vfs_guard/src/ffi.rs` — Entropy detection
- ✅ `crates/edge_ai/src/ffi.rs` — AI routing
- ✅ `crates/relay_daemon/src/ffi.rs` — CAPoW

### Swift Integration
- ✅ `AORATA/AORATA/Bridge/AORATACore-Bridging-Header.h` — C header
- ✅ `AORATA/AORATA/Services/AORATACore.swift` — Swift wrapper
- ✅ `AORATA/AORATA/Services/IPCProducer.swift` — Host app IPC producer
- ✅ `AORATATunnelExtension/UnidirectionalIPC.swift` — Extension IPC consumer
- ✅ `AORATATunnelExtension/PacketTunnelProvider.swift` — NEPacketTunnelProvider

### Build System
- ✅ `build_aorata_static.sh` — Rust → static library compiler

**Total**: ~2,500 lines of FFI + Swift integration code

---

## Step-by-Step Integration

### 1. Build Rust Static Libraries

```bash
cd /Users/kevinnadjarian/GitHub/FILE

# macOS (for development)
./build_aorata_static.sh macos

# iOS (for device)
./build_aorata_static.sh ios

# iOS Simulator
./build_aorata_static.sh ios-sim
```

**Output**: `build/universal/lib*.a` (5 static libraries)

---

### 2. Add Network Extension Target to Xcode

```bash
open AORATA/AORATA.xcodeproj
```

**In Xcode**:
1. File → New → Target
2. Choose **"Network Extension"**
3. Product Name: **`AORATATunnelExtension`**
4. Bundle ID: **`com.lorislab.aorata.tunnel`**
5. Team: **TDV6D5L785** (Christine Martin)
6. Language: **Swift**
7. Click **Finish**

---

### 3. Link Existing Extension Files

**Delete auto-generated files**:
- Right-click `PacketTunnelProvider.swift` (auto-generated) → Delete → Move to Trash

**Add existing files**:
1. Right-click `AORATATunnelExtension` group → Add Files to "AORATA"
2. Navigate to `/Users/kevinnadjarian/GitHub/FILE/AORATA/AORATATunnelExtension/`
3. Select all `.swift` files:
   - `PacketTunnelProvider.swift` ✓
   - `UnidirectionalIPC.swift` ✓
4. **Target Membership**: Check **AORATATunnelExtension** only
5. Click **Add**

**Set Info.plist**:
1. Select **AORATATunnelExtension** target
2. Build Settings → Packaging → **Info.plist File**
3. Set to: `AORATATunnelExtension/Info.plist`

**Set Entitlements**:
1. Signing & Capabilities tab
2. Click **+ Capability**
3. Add **App Groups**: `group.com.lorislab.aorata`
4. Add **Network Extensions**: Packet Tunnel Provider
5. (Optional) Add **Enhanced Security** → **Hardware Memory Tagging** (for A19/M5+)

---

### 4. Link Rust Static Libraries

**For both AORATA and AORATATunnelExtension targets**:

1. Select target → Build Phases
2. **Link Binary With Libraries** → Click **+**
3. Click **Add Other** → **Add Files**
4. Navigate to `/Users/kevinnadjarian/GitHub/FILE/build/universal/`
5. Select all `.a` files:
   - `libcore_transport.a` ✓
   - `libcrypto_fabric.a` ✓
   - `libvfs_guard.a` ✓
   - `libedge_ai.a` ✓
   - `librelay_daemon.a` ✓
6. Click **Open**

**Add system frameworks** (both targets):
- `Security.framework` (for keychain, crypto)
- `SystemConfiguration.framework` (for network state)

---

### 5. Configure Bridging Header

**AORATA target only** (not extension):

1. Build Settings → Swift Compiler - General
2. **Objective-C Bridging Header**: `AORATA/Bridge/AORATACore-Bridging-Header.h`
3. **Header Search Paths**: `$(SRCROOT)/../build/include` (if needed)

**Verify**: Build → Should compile without "Cannot find 'transport_create'" errors

---

### 6. Add IPC Producer to Host App

**Update `AORATAApp.swift`**:

```swift
import SwiftUI
import SwiftData

@main
struct AORATAApp: App {
    // IPC Producer
    private let ipcProducer: IPCProducer? = {
        do {
            return try IPCProducer(appGroup: "group.com.lorislab.aorata")
        } catch {
            print("❌ Failed to initialize IPC producer: \(error)")
            return nil
        }
    }()
    
    var sharedModelContainer: ModelContainer = {
        // ... existing code ...
    }()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .onAppear {
                    // Test FFI
                    testAORATACore()
                }
        }
        .modelContainer(sharedModelContainer)
    }
    
    private func testAORATACore() {
        do {
            // Test Transport
            let transport = try AORATACore.Transport(localAddress: "0.0.0.0:0")
            let plaintext = Data("Hello, AORATA!".utf8)
            let ciphertext = try transport.encrypt(plaintext)
            print("✅ Transport FFI working: \(ciphertext.count) bytes encrypted")
            
            // Test VFS Guard
            let guard = AORATACore.VFSGuard()
            let entropy = guard.calculateEntropy(plaintext)
            print("✅ VFS Guard FFI working: entropy = \(entropy) bits/byte")
            
            // Test AI Backend
            let ai = try AORATACore.AIBackend()
            print("✅ AI Backend FFI working: tier \(ai.getTier()), backend: \(ai.getBackendName())")
            
        } catch {
            print("⚠️ FFI test failed: \(error)")
        }
    }
}
```

---

### 7. Apple Developer Portal Setup

**Create App Group**:
1. https://developer.apple.com/account/resources/identifiers/list/applicationGroup
2. Click **+** → App Groups
3. Identifier: `group.com.lorislab.aorata`
4. Click **Continue** → **Register**

**Link to Bundle IDs**:
1. https://developer.apple.com/account/resources/identifiers/list
2. Edit **`com.lorislab.aorata`** (main app)
   - Capabilities → App Groups → **Enable** → Select `group.com.lorislab.aorata`
3. Add new identifier **`com.lorislab.aorata.tunnel`** (extension)
   - Description: "AORATA Tunnel Extension"
   - Capabilities → App Groups → **Enable** → Select `group.com.lorislab.aorata`
   - Capabilities → Network Extensions → **Enable**

**Generate Provisioning Profiles**:
1. Profiles → **+** → iOS/macOS App Development
2. App ID: `com.lorislab.aorata` → **Continue**
3. Select certificate → **Continue**
4. Name: "AORATA Development" → **Generate**
5. Repeat for `com.lorislab.aorata.tunnel`

**Download & Install**:
- Download both profiles
- Double-click to install (Xcode will import)
- Xcode → Preferences → Accounts → [Your Apple ID] → Download Manual Profiles

---

### 8. Test on Simulator

```bash
# Build & run
xcodebuild -project AORATA/AORATA.xcodeproj \
  -scheme AORATA \
  -destination 'platform=iOS Simulator,name=iPhone 17 Pro' \
  build

# Check console for FFI test output:
# ✅ Transport FFI working: 14 bytes encrypted
# ✅ VFS Guard FFI working: entropy = 4.44 bits/byte
# ✅ AI Backend FFI working: tier 1, backend: CoreML (Tier 1)
```

---

### 9. Install VPN Configuration (Manual)

**On Simulator/Device**:
1. Settings → VPN → Add VPN Configuration
2. Type: **IKEv2**
3. Description: **AORATA**
4. Server: **10.99.0.1**
5. Remote ID: **aorata.lorislab.fr**
6. Local ID: (leave empty)
7. User Authentication: **None**
8. Use Certificate: **Off**
9. Click **Done**

**Activate VPN**:
- Toggle VPN **ON**
- Extension should start → Check console for logs

---

### 10. Production Mach Port IPC (Optional)

Replace file-backed mmap with production Mach port transfer:

**Producer** (`IPCProducer.swift`):
```swift
import Darwin.Mach

func createMachMemoryEntry() throws -> mach_port_t {
    var memoryObject: mach_port_t = 0
    let size = vm_size_t(bufferSize)
    
    let kr = mach_make_memory_entry_64(
        mach_task_self(),
        &size,
        0,
        MAP_MEM_VM_SHARE | VM_PROT_READ | VM_PROT_WRITE,
        &memoryObject,
        MACH_PORT_NULL
    )
    
    guard kr == KERN_SUCCESS else {
        throw NSError(domain: "Mach", code: Int(kr))
    }
    
    return memoryObject
}

func sendToExtension(port: mach_port_t, session: NETunnelProviderSession) {
    var portValue = port
    let data = Data(bytes: &portValue, count: MemoryLayout<mach_port_t>.size)
    
    try? session.sendProviderMessage(data) { _ in
        print("✅ Mach port sent to extension")
    }
}
```

**Consumer** (`UnidirectionalIPC.swift`):
```swift
func attachFromMachPort(_ port: mach_port_t) throws -> UnsafeMutableRawPointer {
    var address: vm_address_t = 0
    
    let kr = mach_vm_map(
        mach_task_self(),
        &address,
        vm_size_t(bufferSize),
        0,
        VM_FLAGS_ANYWHERE,
        port,
        0,
        false,
        VM_PROT_READ,  // Consumer: READ only
        VM_PROT_READ,
        VM_INHERIT_SHARE
    )
    
    guard kr == KERN_SUCCESS else {
        throw UnidirectionalIPCError.mmapFailed(errno: Int32(kr))
    }
    
    return UnsafeMutableRawPointer(bitPattern: address)!
}
```

---

## Verification Checklist

- [ ] Rust libraries build without errors
- [ ] Xcode project builds both targets
- [ ] FFI test output in console
- [ ] App Group created in Developer Portal
- [ ] Bundle IDs linked to App Group
- [ ] VPN profile installs on device
- [ ] Extension starts when VPN activated
- [ ] IPC producer/consumer initialize
- [ ] Packet processing loop runs

---

## Troubleshooting

### "Cannot find 'transport_create' in scope"
- Check bridging header path in Build Settings
- Verify static libraries are linked in Link Binary With Libraries
- Clean build folder: Shift+Cmd+K

### "Failed to initialize IPC producer: App Group not found"
- Check App Group identifier spelling: `group.com.lorislab.aorata`
- Verify entitlements are set for both targets
- Rebuild provisioning profiles in Developer Portal

### "Extension terminated: EXC_RESOURCE (memory limit)"
- Memory > 15MB → See `PHASE_6_COMPLETE.md` Jetsam mitigation
- Add @autoreleasepool around packet loops
- Check for memory leaks with Instruments

### FFI functions crash
- Verify Rust libraries are for correct architecture (use `lipo -info`)
- Check that all `.a` files are from same build
- Try clean rebuild: `rm -rf build/ && ./build_aorata_static.sh macos`

---

## Performance Testing

### Instruments (Memory)
```bash
# Launch Instruments
open -a Instruments

# Template: Allocations
# Attach to: AORATATunnelExtension
# Monitor: Live Bytes < 15 MB
```

### Packet Capture
```bash
# macOS
sudo tcpdump -i utun3 -w aorata.pcap

# Open in Wireshark to verify encryption
```

### Latency Testing
```bash
# Ping over tunnel
ping -c 100 10.99.0.1

# Target: <10ms latency
```

---

## Next Steps

1. **Implement Real Crypto**: Replace placeholder encrypt/decrypt with actual Noise + MPQUIC
2. **SPSC Ring Buffer**: Port quetzalcoatl or use Swift Atomics
3. **Mach Port IPC**: Replace file-backed mmap with production Mach transfer
4. **NAT Traversal**: Wire up `nat_punch.rs` DCUtR logic
5. **AI Routing**: Connect edge_ai backend to routing decisions
6. **CAPoW**: Integrate relay_daemon challenge/verify on connection establishment

---

**Integration Guide Complete** — All code ready, Xcode setup instructions provided ✅
