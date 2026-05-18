# AORATA Tunnel Extension (Phase 6)

**NEPacketTunnelProvider** implementation for AORATA Protocol v2 iOS/macOS integration.

---

## Architecture

### Unidirectional IPC (TOCTOU Prevention)

```
┌─────────────────────┐              ┌──────────────────────┐
│   Host App          │              │  Tunnel Extension    │
│   (Producer)        │              │  (Consumer)          │
│                     │              │                      │
│  PROT_WRITE         │   mmap       │  PROT_READ only      │
│  └─ Quetzalcoatl ───┼──────────────┼─► SPSC Ring Buffer  │
│     SPSC Writer     │  App Groups  │     Reader           │
└─────────────────────┘              └──────────────────────┘
     │                                        │
     │ Mach port transfer                    │
     └────────────────────────────────────────┘
         (via XPC/handleAppMessage)
```

### Memory Safety

- **TOCTOU**: Consumer has PROT_READ only → MMU prevents write (hardware-enforced)
- **Single Fetch**: Data copied once from shared memory to local stack
- **MIE (A19/M5+)**: 4-bit tags per 16-byte granule prevent buffer overflows

### Jetsam Mitigation (<15MB)

- Pre-allocated 2MB mmap pool
- `@autoreleasepool` around packet loops
- Zero-copy via `packetFlow.readPackets`
- SPSC avoids heap fragmentation

---

## Files

| File | Purpose |
|---|---|
| `PacketTunnelProvider.swift` | Main NEPacketTunnelProvider subclass |
| `UnidirectionalIPC.swift` | Swift wrapper for mmap IPC |
| `Info.plist` | Extension metadata |
| `AORATATunnelExtension.entitlements` | App Groups + Network Extension |

---

## Setup Instructions

### 1. Add Network Extension Target to Xcode

```bash
# Open project
open AORATA.xcodeproj

# In Xcode:
# 1. File → New → Target
# 2. Choose "Network Extension"
# 3. Product Name: "AORATATunnelExtension"
# 4. Bundle ID: "com.lorislab.aorata.tunnel"
# 5. Team: TDV6D5L785 (Christine Martin)
# 6. Language: Swift
```

### 2. Link Existing Source Files

In Xcode Project Navigator:
1. Select **AORATATunnelExtension** target
2. Delete the auto-generated `PacketTunnelProvider.swift`
3. Drag these files from `AORATATunnelExtension/` folder:
   - `PacketTunnelProvider.swift` ✓
   - `UnidirectionalIPC.swift` ✓
   - `Info.plist` (set as extension's Info.plist)
   - `AORATATunnelExtension.entitlements` (set in Signing & Capabilities)

### 3. Configure Target Settings

**Build Settings** → **AORATATunnelExtension** target:

| Setting | Value |
|---|---|
| **Product Name** | `AORATATunnelExtension` |
| **Product Bundle Identifier** | `com.lorislab.aorata.tunnel` |
| **Development Team** | `TDV6D5L785` (Christine Martin) |
| **iOS Deployment Target** | `17.0` |
| **macOS Deployment Target** | `14.0` |
| **Supports Mac Catalyst** | Yes |

**Signing & Capabilities**:
- ✅ **App Groups**: `group.com.lorislab.aorata`
- ✅ **Network Extensions**: Packet Tunnel Provider
- ⏸️ **Hardware Memory Tagging**: Enable after testing on A19/M5+ hardware

### 4. Host App Configuration

Update **AORATA** (main app) target:

**Capabilities**:
- ✅ **App Groups**: `group.com.lorislab.aorata`
- ✅ **Network Extensions**: (already enabled)

**Embed & Sign**: Add `AORATATunnelExtension.appex`
- Target Membership: AORATA
- Embed: Yes
- Code Sign On Copy: Yes

---

## Production Mach Port IPC (TODO)

Current implementation uses **file-backed mmap** (placeholder). Production requires:

### Producer (Host App)

```swift
import Darwin.Mach

// 1. Allocate memory
var memoryObject: mach_port_t = 0
let size = vm_size_t(2 * 1024 * 1024)
mach_make_memory_entry_64(
    mach_task_self(),
    &size,
    0,
    MAP_MEM_VM_SHARE | VM_PROT_READ | VM_PROT_WRITE,
    &memoryObject,
    MACH_PORT_NULL
)

// 2. mmap with PROT_WRITE
let ptr = mmap(nil, Int(size), PROT_WRITE, MAP_SHARED, Int32(memoryObject), 0)

// 3. Send Mach port to extension via XPC/handleAppMessage
let message = Data(bytes: &memoryObject, count: MemoryLayout<mach_port_t>.size)
session.sendProviderMessage(message) { _ in }
```

### Consumer (Extension)

```swift
// 1. Receive Mach port from handleAppMessage
let machPort = messageData.withUnsafeBytes { $0.load(as: mach_port_t.self) }

// 2. Attach with PROT_READ only
var address: vm_address_t = 0
mach_vm_map(
    mach_task_self(),
    &address,
    vm_size_t(bufferSize),
    0,
    VM_FLAGS_ANYWHERE,
    machPort,
    0,
    false,
    VM_PROT_READ,  // Consumer: READ only
    VM_PROT_READ,
    VM_INHERIT_SHARE
)
```

---

## Testing

### 1. VPN Configuration Profile

```bash
# Install VPN profile on simulator/device
# Settings → VPN → Add VPN Configuration
# Type: IKEv2
# Server: 10.99.0.1
# Remote ID: aorata.lorislab.fr
```

### 2. Packet Capture

```bash
# On macOS
sudo tcpdump -i utun3 -w aorata_packets.pcap

# On iOS
# Xcode → Devices & Simulators → [Device] → Start Packet Capture
```

### 3. Memory Profiling (Jetsam)

```bash
# Instruments → Allocations
# Monitor "Live Bytes" < 15 MB during burst traffic
```

---

## MIE (Memory Integrity Enforcement)

### Enable Hardware Tagging

**Requirements**:
- iPhone 17+ (A19) or Mac M5+
- Xcode 26+
- iOS 26+ or macOS 16+

**Steps**:
1. Xcode → Target Settings → Signing & Capabilities
2. Add Capability → **Enhanced Security**
3. Enable **Hardware Memory Tagging** ✓
4. Build & Archive

**Validation**:

```swift
// Check MIE availability
import Darwin

var mteAvailable: Int32 = 0
var size = MemoryLayout<Int32>.size
sysctlbyname("hw.optional.arm.FEAT_MTE4", &mteAvailable, &size, nil, 0)

if mteAvailable == 1 {
    logger.info("✅ MIE (FEAT_MTE4) available")
} else {
    logger.warning("⚠️ MIE not available (older hardware)")
}
```

---

## Performance Targets

| Metric | Target | Current |
|---|---|---|
| **Memory (RSS)** | <15 MB (Jetsam) | TBD |
| **Latency (packet)** | <10 ms | TBD |
| **Throughput** | >100 Mbps | TBD |
| **CPU (avg)** | <5% | TBD |

---

## Rust FFI Integration (Phase 6.2)

### C ABI Exports

```rust
// crates/core_transport/src/ffi.rs
#[no_mangle]
pub extern "C" fn aorata_transport_encrypt(
    plaintext: *const u8,
    len: usize,
    ciphertext: *mut u8,
    out_len: *mut usize,
) -> i32 {
    // Noise + AONT + MPQUIC
}
```

### Swift Import

```swift
// PacketTunnelProvider.swift
import AORATACore  // FFI bridge module

func encryptPacket(_ data: Data) -> Data? {
    var outLen: Int = 0
    var cipher = Data(count: data.count + 32)  // padding
    
    data.withUnsafeBytes { plainPtr in
        cipher.withUnsafeMutableBytes { cipherPtr in
            aorata_transport_encrypt(
                plainPtr.baseAddress!.assumingMemoryBound(to: UInt8.self),
                data.count,
                cipherPtr.baseAddress!.assumingMemoryBound(to: UInt8.self),
                &outLen
            )
        }
    }
    
    return cipher.prefix(outLen)
}
```

---

## References

- [Research: AORATA Protocol v2 Network Extension.md](../Research/AORATA%20Protocol%20v2%20Network%20Extension.md)
- [Apple NetworkExtension Docs](https://developer.apple.com/documentation/networkextension)
- [WireGuard iOS Implementation](https://github.com/WireGuard/wireguard-apple)
- [Apple MIE Deep Dive](https://security.apple.com/blog/memory-integrity-enforcement/)

---

**Status**: 🔨 Phase 6.1 Complete (NEPacketTunnelProvider scaffold)  
**Next**: Phase 6.2 — Rust FFI + MPQUIC integration
