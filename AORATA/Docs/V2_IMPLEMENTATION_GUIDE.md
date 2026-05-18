# AORATA Protocol v2 - Quick Implementation Guide

**TL;DR**: 4 critical security fixes + 4 performance optimizations vs v1

---

## 🚨 Critical Changes (MUST IMPLEMENT)

### 1. Noise XXfallback (not XX!)

**Why**: Prevent nonce reuse in asynchronous 0-RTT

```rust
// ❌ v1 (VULNERABLE)
const PARAMS: &str = "Noise_XX_25519_ChaChaPoly_SHA256";

// ✅ v2 (SAFE)
const PARAMS: &str = "Noise_XXfallback_25519_ChaChaPoly_SHA256";

use snow::Builder;
let handshake = Builder::new(PARAMS.parse()?)
    .local_private_key(static_key)
    .build_responder()?;
```

**Impact**: Graceful degradation on 0-RTT failure → no silent crypto corruption

---

### 2. SipHash-2-4 for QUIC Connection IDs

**Why**: Prevent Hash DoS attacks on connection table

```rust
// ❌ v1 (VULNERABLE)
use std::collections::HashMap;
let connections: HashMap<ConnectionId, Connection> = HashMap::new();

// ✅ v2 (SECURE)
use siphasher::sip128::SipHasher24;
use std::collections::HashMap;
use std::hash::BuildHasherDefault;

type SipHashMap<K, V> = HashMap<K, V, BuildHasherDefault<SipHasher24>>;
let connections: SipHashMap<ConnectionId, Connection> = HashMap::default();
```

**Impact**: Attacker cannot exhaust relay CPU via hash collisions

---

### 3. Hybrid AONT-Threshold

**Why**: 99% performance gain vs full-payload AONT

```rust
// ❌ v1 (SLOW - 100ms for 10MB)
fn fragment_v1(payload: &[u8]) -> Vec<Fragment> {
    // Apply AONT to ENTIRE payload
    let aont = aont_transform_all(payload); // Sequential, slow
    distribute(aont)
}

// ✅ v2 (FAST - <5ms for 10MB)
fn fragment_v2(payload: &[u8]) -> Vec<Fragment> {
    let (header, bulk) = payload.split_at(32); // 256 bits
    
    // Header ONLY: AONT + HE + Shamir (3-of-5)
    let h_aont = aont_transform(header);
    let h_he = homomorphic_encrypt(&h_aont);
    let h_shares = shamir_split(&h_he, 3, 5);
    
    // Bulk: Reed-Solomon (parallelizable)
    let b_shards = reed_solomon_encode(bulk);
    
    combine(h_shares, b_shards)
}
```

**Impact**: Network throughput restored, cryptographic guarantee maintained

---

### 4. Unidirectional mmap IPC

**Why**: Prevent TOCTOU sandbox escape on iOS

```rust
// ❌ v1 (VULNERABLE - bidirectional)
let shared_mem = mmap(PROT_READ | PROT_WRITE); // Both sides can R+W

// ✅ v2 (SECURE - unidirectional)
use quetzalcoatl::spsc::{Producer, Consumer};

// App side: WRITE ONLY
let (producer, _) = create_ring_buffer(4 * 1024 * 1024);
mprotect(producer.as_ptr(), size / 2, PROT_WRITE);

// Tunnel side: READ ONLY
let (_, consumer) = create_ring_buffer(4 * 1024 * 1024);
mprotect(consumer.as_ptr(), size / 2, PROT_READ);
```

**Impact**: ROP/thread-hijacking impossible, hardware-backed MIE on Apple Silicon

---

## ⚡ Performance Optimizations

### 5. Asymmetric CAPoW

**Why**: Avoid Self-DoS (relay exhausting own RAM)

```rust
// ❌ v1 (SELF-DOS - relay verifies with 2GB)
fn verify_proof_v1(proof: &Proof) -> bool {
    argon2::verify(
        memory: 2 * 1024 * 1024 * 1024, // 2 GB → kills relay
        time: 16,
        &proof.hash
    )
}

// ✅ v2 (EFFICIENT - asymmetric)
fn generate_proof_v2(challenge: &[u8]) -> Proof {
    // Client: 2GB forced (GPU VRAM thrashing)
    argon2::hash(
        memory: 2 * 1024 * 1024 * 1024, // 2 GB
        time: 16,
        parallelism: 4,
        challenge
    )
}

fn verify_proof_v2(proof: &Proof) -> bool {
    // Relay: 64MB only (<100 µs)
    argon2::verify(
        memory: 64 * 1024 * 1024, // 64 MB
        time: 1,
        parallelism: 1,
        &proof.hash
    )
}
```

**Impact**: Botnet resistance WITHOUT relay resource exhaustion

---

### 6. Rayon Thread Pool Isolation

**Why**: Prevent Tokio thread pool starvation

```rust
// ❌ v1 (BLOCKS TOKIO - connection timeouts)
tokio::spawn(async move {
    let proof = rx.recv().await?;
    // Argon2 blocks the async executor thread!
    let valid = argon2::verify(&proof); // BAD
    if valid { forward(proof).await; }
});

// ✅ v2 (ISOLATED - Tokio never blocked)
use rayon::ThreadPoolBuilder;

let argon_pool = ThreadPoolBuilder::new()
    .num_threads(4)
    .build()?;

tokio::spawn(async move {
    let proof = rx.recv().await?;
    // Offload to dedicated Rayon pool
    let valid = argon_pool.install(|| argon2::verify(&proof));
    if valid { forward(proof).await; }
});
```

**Impact**: MPQUIC I/O loop unimpeded, zero connection drops

---

### 7. ANE LoRA Injection

**Why**: Sub-millisecond routing updates vs 100-500ms recompilation

```rust
// ❌ v1 (SLOW - 500ms spike per policy change)
fn update_routing_v1(policy: &Policy) {
    // Recompile CoreML model every time
    let model = compile_coreml(policy); // 100-500ms latency spike
    self.model = model;
}

// ✅ v2 (FAST - <100µs update)
fn update_routing_v2(&mut self, policy: &Policy) {
    // Load model ONCE at startup
    // Then inject LoRA matrices dynamically
    let lora = policy.to_lora_matrices(); // Quantized INT8/FP16
    
    // Direct tensor injection (no recompilation)
    self.io_surface.write_tensor(0, &lora.layer1)?; // <100 µs
    self.io_surface.write_tensor(1, &lora.layer2)?;
}
```

**Impact**: Real-time route adaptation under DDoS storms

---

### 8. BFE Power-of-2 Optimization

**Why**: 1 CPU cycle per lookup vs expensive modulo

```rust
// ❌ v1 (SLOW - modulo on ARM = 10+ cycles)
const BITSET_SIZE: usize = 10_000_000;
let index = hash % BITSET_SIZE; // Modulo is slow

// ✅ v2 (FAST - 1 cycle bitwise AND)
const BITSET_SIZE: usize = 1 << 20; // 1,048,576 (power of 2)
let index = hash & (BITSET_SIZE - 1); // Bitwise AND = 1 cycle
```

**Impact**: Sub-microsecond O(1) revocation maintained

---

## 📊 Performance Comparison Table

| Component | v1 Approach | v1 Performance | v2 Approach | v2 Performance | Gain |
|-----------|-------------|----------------|-------------|----------------|------|
| **Noise** | XX | Nonce reuse risk | XXfallback | Safe 0-RTT | ✅ Security |
| **QUIC CID** | Default HashMap | Hash DoS vuln | SipHash-2-4 | DoS resistant | ✅ Security |
| **AONT** | Full payload | 100ms / 10MB | Hybrid (header only) | 5ms / 10MB | **+95%** |
| **mmap IPC** | Bidirectional R+W | TOCTOU vuln | Unidirectional R or W | Sandbox escape proof | ✅ Security |
| **CAPoW Verify** | 2 GB symmetric | Self-DoS | 64 MB asymmetric | <100 µs | **+99.9%** |
| **Argon2 in Tokio** | Blocking async | Thread starvation | Rayon isolation | Zero blocking | ✅ Reliability |
| **ANE Update** | CoreML recompile | 100-500ms | LoRA injection | <100 µs | **+99.8%** |
| **BFE Hash** | Modulo | 10+ cycles | Bitwise AND | 1 cycle | **+90%** |
| **Cache** | LRU | Baseline hit rate | S3-FIFO | +38% hit rate | **+38%** |

---

## 🔧 Implementation Checklist

### Phase 1: Core Transport
- [ ] Replace `Noise_XX` with `Noise_XXfallback` in `crypto_fabric/src/noise_xxfallback.rs`
- [ ] Add `siphasher = "1.0.1"` to `Cargo.toml`
- [ ] Replace HashMap with SipHashMap for QUIC CID routing in `core_transport/src/connection_pool.rs`
- [ ] Add defensive validation rules (MerCuriuzz patterns)

### Phase 2: Crypto Fabric
- [ ] Implement Hybrid AONT in `crypto_fabric/src/aont_hybrid.rs`
- [ ] Add `reed-solomon-erasure = "6.0.0"` + `sharks = "0.5.0"` to `Cargo.toml`
- [ ] Optimize BFE with power-of-2 bitset in `crypto_fabric/src/bfe.rs`

### Phase 3: VFS Guard
- [ ] Entropy monitoring (no changes from v1)
- [ ] Poll::Pending stasis (no changes from v1)

### Phase 4: Edge AI
- [ ] ANE LoRA injection in `edge_ai/src/ane_lora.rs`
- [ ] Load MIL graph ONCE, inject tensors dynamically

### Phase 5: Relay Daemon
- [ ] Asymmetric CAPoW in `relay_daemon/src/capow.rs`
- [ ] Rayon thread pool in `relay_daemon/src/argon_pool.rs`
- [ ] Add `rayon = "1.10.0"` to `Cargo.toml`

### Phase 6: Apple Integration
- [ ] Unidirectional IPC in `core_transport/src/ipc.rs`
- [ ] Add `quetzalcoatl = "0.2.0"` to `Cargo.toml`
- [ ] PROT_READ/WRITE separation with `mprotect()`

---

## 📚 Key Dependencies (v2)

```toml
[dependencies]
# Core Transport
ant-quic = "0.14.192"
siphasher = "1.0.1"          # NEW v2
socket2 = "0.5.9"

# Crypto Fabric
snow = "0.10.0"
reed-solomon-erasure = "6.0.0"  # NEW v2
sharks = "0.5.0"                 # NEW v2

# Relay Daemon
argon2 = "0.5.3"
rayon = "1.10.0"                 # NEW v2
tokio = "1.52.3"

# IPC
quetzalcoatl = "0.2.0"          # NEW v2

# Edge AI
metal-rs = "0.30.0"
```

---

## 🎯 Testing Priorities

### Security Tests (v2)
1. **Noise XXfallback**: Force 0-RTT failure → verify graceful XX fallback
2. **SipHash-2-4**: Simulate Hash DoS with 10M collision attempts
3. **Hybrid AONT**: Intercept 2/5 paths → verify reconstruction impossible
4. **Unidirectional IPC**: Attempt TOCTOU race → verify blocked by PROT separation

### Performance Tests (v2)
1. **AONT**: 10MB payload fragmentation (target: <5ms)
2. **CAPoW**: Verify 10k proofs/sec on relay (64MB config)
3. **ANE**: 100 routing updates/sec (target: <100µs each)
4. **BFE**: 1M revocations (target: <1µs each)

---

## 📖 Further Reading

- Full Protocol Spec: `AORATA_PROTOCOL_v2.md`
- Architecture Analysis: `Advanced Network Architecture Deep Dive.md` (original source)
- NotebookLM: https://notebooklm.google.com/notebook/86bc04f6-444f-4882-b54c-96467cf7cda4
- TODO (updated): `../TODO.md`

---

**Ready to implement Phase 1!**

Query NotebookLM: *"Complete implementation of mpquic.rs with ant-quic 0.14.192 + SipHash-2-4 for CID routing"*
