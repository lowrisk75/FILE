# FILE Project - TODO List

**Last Updated**: 2026-05-15
**Status**: Protocol v2 Finalized ✅ — Ready for Implementation
**Architecture**: AORATA Protocol v2 (ZTATF - Zero Trust Asynchronous Transport Fabric)

---

## 🎯 Protocol v2 Improvements (vs v1)

**Source**: Advanced Network Architecture Deep Dive + NotebookLM synthesis

### Critical Vulnerabilities Fixed
- [x] Noise XX → **XXfallback** (0-RTT nonce reuse mitigation)
- [x] QUIC Hash Tables → **SipHash-2-4** (Hash DoS mitigation)
- [x] Full-payload AONT → **Hybrid AONT-Threshold** (99% performance gain)
- [x] Bidirectional mmap → **Unidirectional PROT_READ/WRITE** (TOCTOU mitigation)

### Performance Optimizations
- [x] Argon2 → **Asymmetric CAPoW** (avoid Self-DoS)
- [x] Tokio blocking → **Rayon dedicated pool** (thread starvation fix)
- [x] CoreML recompilation → **ANE LoRA injection** (sub-millisecond updates)
- [x] BFE modulo → **Power-of-2 bitwise AND** (1-cycle hash lookup)

**Full Specification**: `AORATA/Docs/AORATA_PROTOCOL_v2.md`

---

## ✅ Completed

- [x] Create Cargo workspace structure
- [x] Define all workspace dependencies (Cargo.toml)
- [x] Scaffold core_transport (mpquic.rs, nat_punch.rs)
- [x] Scaffold crypto_fabric (noise_auth.rs, aont.rs, puncturable.rs)
- [x] Scaffold vfs_guard (canary.rs, windows.rs, unix.rs)
- [x] Scaffold edge_ai (ane_ffi.rs, routing.rs)
- [x] Scaffold relay_daemon (s3_fifo.rs, capow.rs)
- [x] Write README.md
- [x] Write IMPLEMENTATION_ROADMAP.md
- [x] Create project memory file
- [x] Add NotebookLM notebook as documentation source
- [x] **Create AORATA Xcode project** (iOS + macOS multiplatform)
- [x] **Configure Xcode entitlements + capabilities** (100% automated)
- [x] **Verify builds** (macOS ✓ iOS Simulator ✓)
- [x] **Write AORATA_PROTOCOL_v2.md** (complete architecture spec)

---

## 🔥 Phase 1: Core Transport (PRIORITY) — v2 Updates

### Implementation
- [ ] **[NEW v2]** Implement SipHash-2-4 for QUIC Connection ID routing
  - [ ] Replace default hash table with `siphasher` crate
  - [ ] Configure cryptographically secure CID→Connection mapping
  - [ ] **Mitigation**: Hash DoS attack prevention
- [ ] Implement `MpQuicFabric::new()` with ant-quic
  - [ ] Configure SO_REUSEPORT properly
  - [ ] Initialize Endpoint with NAT traversal enabled
  - [ ] Enable multipath feature
- [ ] Implement NAT hole punching
  - [ ] TTL manipulation (3-4 hops)
  - [ ] DCUtR timing (RTT/2 synchronization)
  - [ ] QUIC frame extensions (ADD_ADDRESS, PUNCH_ME_NOW, etc.)
- [ ] Implement Connection ID (CID) handling with SipHash-2-4
- [ ] Implement 0-RTT handover
- [ ] **[NEW v2]** Implement defensive validation rules (MerCuriuzz fuzzing patterns)
  - [ ] Bounded resource retention
  - [ ] Strict state-machine validation
  - [ ] Adversarial packet rejection

### Tests
- [ ] **[NEW v2]** Test SipHash-2-4 CID routing under DoS simulation
- [ ] Test SO_REUSEPORT on Linux/macOS/Windows
- [ ] Test MPQUIC path switching (Wi-Fi → 5G)
- [ ] Test NAT traversal (2 nodes behind NAT)
- [ ] Test TTL manipulation doesn't trigger IDS
- [ ] Benchmark handover latency (target: < 10ms)
- [ ] **[NEW v2]** Fuzzing tests (MerCuriuzz patterns)

### NotebookLM Queries Needed
- "Complete implementation of mpquic.rs with ant-quic 0.14.192 + SipHash-2-4 for CID routing"
- "NAT hole punching: complete nat_punch.rs with DCUtR timing"
- "QUIC frame extensions: ADD_ADDRESS, OBSERVED_ADDRESS implementation"
- "SipHash-2-4 integration in QUIC connection table for Hash DoS mitigation"

---

## 🔐 Phase 2: Crypto Fabric — v2 Updates

### Implementation
- [ ] **[CRITICAL v2]** Implement Noise XXfallback pattern (not XX!)
  - [ ] Pattern: `Noise_XXfallback_25519_ChaChaPoly_SHA256`
  - [ ] Initiator mode with fallback support
  - [ ] Responder mode with fallback support
  - [ ] 0-RTT attempt → fallback to interactive XX if fails
  - [ ] **Mitigation**: Nonce reuse in asynchronous 0-RTT
- [ ] **[CRITICAL v2]** Implement Hybrid AONT-Threshold
  - [ ] **Header (256 bits)**: AONT + Homomorphic Encryption + Shamir (3-of-5)
  - [ ] **Bulk data**: Reed-Solomon Erasure Coding (parallelizable)
  - [ ] Integration with `reed-solomon-erasure` + `sharks` crates
  - [ ] **Performance gain**: 99% overhead reduction vs full-payload AONT
- [ ] **[NEW v2]** Implement power-of-2 BFE optimization
  - [ ] Bitset size = 2^N (enable bitwise AND)
  - [ ] Replace modulo with `hash & (SIZE - 1)`
  - [ ] **Performance gain**: 1 CPU cycle per lookup
  - [ ] Puncture operation (O(1) revocation)
  - [ ] False Positive Rate configuration (0.01% target)
- [ ] **[NEW v2]** Post-Quantum integration
  - [ ] ML-KEM (FIPS-203) for key encapsulation
  - [ ] ML-DSA (FIPS-204) for signatures (optional)

### Tests
- [ ] **[NEW v2]** Test Noise XXfallback: 0-RTT success path
- [ ] **[NEW v2]** Test Noise XXfallback: 0-RTT failure → graceful degradation
- [ ] **[NEW v2]** Test Hybrid AONT: header reconstruction requires 3/5 shares
- [ ] **[NEW v2]** Test Hybrid AONT: bulk data parallelization performance
- [ ] Test BFE revocation (instant, no redistribution)
- [ ] **[NEW v2]** Test BFE FPR < 0.01% with 10^6 elements
- [ ] Benchmark handshake latency (target: < 1 RTT)
- [ ] **[NEW v2]** Benchmark AONT header vs bulk (header <1ms, bulk <5ms for 10MB)

### NotebookLM Queries Needed
- "Noise XXfallback pattern: complete implementation with snow 0.10.0 for 0-RTT safety"
- "Hybrid AONT-Threshold: header (AONT+HE+Shamir) + bulk (Reed-Solomon) implementation"
- "BFE power-of-2 optimization: bitwise AND instead of modulo for sub-microsecond puncturing"
- "ML-KEM (FIPS-203) integration with Noise XXfallback for post-quantum security"

---

## 🛡️ Phase 3: VFS Anti-Ransomware

### Implementation
- [ ] Implement canary detection logic
  - [ ] Generate canary paths
  - [ ] Detect Write() attempts
  - [ ] Trigger stasis
- [ ] Windows VFS (WinFSP)
  - [ ] STATUS_PENDING implementation
  - [ ] IRPLOCK_CANCELABLE handling
  - [ ] Avoid dispatcher pool starvation
- [ ] Unix VFS (FUSE + Tokio)
  - [ ] Poll::Pending implementation
  - [ ] Never wake Waker
  - [ ] Avoid executor blocking

### Tests
- [ ] Test canary detection (Write on lure file)
- [ ] Test stasis (ransomware thread frozen)
- [ ] Test no BSOD (Windows)
- [ ] Test no Kernel Panic (Linux)
- [ ] Test IoCancelIrp handling (Windows)
- [ ] Test normal operations unaffected
- [ ] Load test: 1000 files + 1 canary

### NotebookLM Queries Needed
- "VFS anti-ransomware: WinFSP STATUS_PENDING complete implementation"
- "FUSE + Tokio: Poll::Pending stasis without blocking executor"
- "Canary file patterns and behavioral detection algorithms"

---

## 🧠 Phase 4: Edge AI Routing — v2 Updates

### Implementation
- [ ] **[CRITICAL v2]** ANE Dynamic LoRA Injection (NOT static compilation)
  - [ ] Load MIL graph ONCE via `_ANEInMemoryModelDescriptor`
  - [ ] Create `_ANEIOSurfaceObject` tensors
  - [ ] Inject quantized LoRA matrices (INT8/FP16) dynamically
  - [ ] **Performance**: <100 microseconds update (vs 100-500ms recompilation)
  - [ ] **Mitigation**: Real-time route adaptation without latency spikes
- [ ] ANE FFI bridge (Objective-C)
  - [ ] Load _ANEClient framework
  - [ ] Bypass CoreML static compilation
  - [ ] Execute inference
  - [ ] Fallback to CPU if unavailable
- [ ] Pareto router with dynamic policy injection
  - [ ] Define input features (RTT, BW, security, congestion)
  - [ ] Train/load base model ONCE
  - [ ] Update routing policy via LoRA matrices (no retrain)
  - [ ] Select optimal path
  - [ ] Benchmark inference latency (target: <1ms)

### Tests
- [ ] **[NEW v2]** Test ANE model loading ONCE at startup
- [ ] **[NEW v2]** Test LoRA injection latency (<100 µs)
- [ ] **[NEW v2]** Test dynamic policy update (10 Hz rate)
- [ ] Test inference < 1ms (38 TOPS on A17 Pro)
- [ ] Test fallback to CPU
- [ ] Test Pareto selection (5 concurrent paths)
- [ ] Validate decision quality under DDoS simulation

### NotebookLM Queries Needed
- "Apple Neural Engine LoRA dynamic injection: _ANEInMemoryModelDescriptor + _ANEIOSurfaceObject"
- "Bypass CoreML static compilation: direct ANE MIL graph programming"
- "INT8/FP16 quantized LoRA matrices for sub-millisecond model updates"
- "Pareto routing with real-time policy adaptation via tensor injection"

---

## 🔄 Phase 5: Relay Daemon — v2 Updates

### Implementation
- [ ] S3-FIFO cache (keep as-is, best-in-class)
  - [ ] Three queues (Small, Main, Ghost)
  - [ ] O(1) insert/evict
  - [ ] Hit rate tracking (+38% vs LRU)
- [ ] **[CRITICAL v2]** Asymmetric CAPoW (NOT 2 GiB symmetric)
  - [ ] **Client/Attacker**: Argon2id(memory=512MB-2GB, time=16, para=4)
    - Force GPU VRAM thrashing
    - 5-30 seconds per proof
  - [ ] **Relay Verification**: Argon2id(memory=64MB, time=1, para=1)
    - Minimal CPU + RAM
    - <100 microseconds verify
  - [ ] **Mitigation**: Avoid Self-DoS (relay exhausting own RAM)
  - [ ] Context-aware difficulty scaling (ML scoring)
- [ ] **[CRITICAL v2]** Rayon Thread Pool Isolation
  - [ ] Create dedicated Rayon pool for Argon2 (CPU-bound)
  - [ ] Keep Tokio pure I/O (never block async executor)
  - [ ] **Mitigation**: Thread pool starvation fix
  - [ ] Work-stealing scheduler for crypto tasks
- [ ] Context Scoring ML
  - [ ] TTL, port profile, source IP reputation
  - [ ] Inter-arrival timing
  - [ ] Relay CPU load + disk saturation
  - [ ] Map to Argon2 difficulty dynamically

### Tests
- [ ] Benchmark S3-FIFO vs LRU (hit rate +38%)
- [ ] **[NEW v2]** Test Asymmetric CAPoW: client 512MB, relay 64MB
- [ ] **[NEW v2]** Test Rayon isolation: Tokio never blocked
- [ ] **[NEW v2]** Test GPU botnet simulation (RTX 5090 equivalent)
- [ ] Test Context Scoring under DDoS
- [ ] Load test: 10 GB cache, 1 Gbps throughput, 10k concurrent connections

### NotebookLM Queries Needed
- "S3-FIFO cache: complete implementation with benchmark data"
- "Asymmetric CAPoW: Argon2id client=2GB verification=64MB birthday problem approach"
- "Rayon + Tokio isolation: CPU-bound crypto never blocking async I/O executor"
- "Context-aware ML scoring: threat multiplier algorithm for dynamic difficulty"

---

## 🍎 Phase 6: Apple Platform Integration — v2 Updates

### iOS
- [ ] Create NEPacketTunnelProvider target
  - [ ] Tunnel IP packets
  - [ ] Route through MPQUIC
  - [ ] Stay under 15 MB RAM (Jetsam)
- [ ] **[CRITICAL v2]** Unidirectional mmap IPC (App Groups)
  - [ ] **NOT** bidirectional shared memory
  - [ ] SPSC ring buffers (quetzalcoatl crate)
  - [ ] **App → Tunnel**: `PROT_WRITE` only (Producer)
  - [ ] **Tunnel → App**: `PROT_READ` only (Consumer)
  - [ ] `MAP_ANON | MAP_SHARED` (no file descriptor)
  - [ ] **Mitigation**: TOCTOU + sandbox escape prevention
  - [ ] Apple Silicon MIE (Memory Integrity Enforcement)
  - [ ] ARM64 atomic ordering (Release/Acquire)
  - [ ] 4 MB buffer size (zero-copy, lock-free)
- [ ] Wi-Fi Aware (DMA compliance)
  - [ ] P2P discovery
  - [ ] Replace AWDL

### macOS
- [ ] System Extension (replace kext)
- [ ] ANE runtime integration
- [ ] Same unidirectional IPC as iOS

### Tests
- [ ] Test NEPacketTunnelProvider tunnel
- [ ] **[NEW v2]** Test unidirectional IPC: App cannot read Tunnel memory
- [ ] **[NEW v2]** Test unidirectional IPC: Tunnel cannot write App memory
- [ ] **[NEW v2]** Test MIE enforcement on Apple Silicon
- [ ] Test mmap IPC latency (< 1 µs)
- [ ] Test RAM usage (< 15 MB)
- [ ] Test Wi-Fi Aware discovery
- [ ] Test ANE FFI from Swift

### NotebookLM Queries Needed
- "iOS NEPacketTunnelProvider: complete implementation with strict memory isolation"
- "Unidirectional mmap IPC: quetzalcoatl SPSC with PROT_READ/WRITE separation"
- "TOCTOU mitigation: lock-free zero-copy IPC with hardware-backed MIE"
- "Wi-Fi Aware: P2P discovery replacing AWDL"

---

## 🧪 Phase 7: Integration & Deployment

### End-to-End Tests
- [ ] Scenario: 2 NAT restrictive firewalls
- [ ] Scenario: Mobile 5G → Desktop Ethernet
- [ ] Scenario: Ransomware detection + stasis
- [ ] Scenario: Relay failure → fallback

### Performance Benchmarks
- [ ] Handover latency < 10ms
- [ ] Throughput > 80% physical link
- [ ] iOS RAM < 15 MB
- [ ] CPU idle < 5%

### Deployment
- [ ] Package crates for crates.io
- [ ] Build XCFramework (iOS/macOS)
- [ ] Write user documentation
- [ ] GDPR compliance review
- [ ] DMA compliance review

---

## 📚 Documentation TODO

- [ ] Architecture decision records (ADR)
  - [ ] Why ant-quic vs noq?
  - [ ] Why Noise IK vs XX?
  - [ ] Why S3-FIFO vs other caches?
- [ ] API documentation (rustdoc)
- [ ] User guides
- [ ] Deployment guides
- [ ] Security whitepaper

---

## 🔍 Research Questions (Ask NotebookLM)

- [ ] "What are the exact QUIC frame type IDs for NAT traversal?"
- [ ] "How does S3-FIFO handle concurrent access?"
- [ ] "What is the ANE model input/output format for routing?"
- [ ] "How to handle Jetsam memory limit on iOS Network Extension?"
- [ ] "What are the Wi-Fi Aware DMA requirements for EU?"

---

## 💡 Future Enhancements

- [ ] Android support (Network VPN Service)
- [ ] Windows System Extension support
- [ ] Post-quantum crypto (pqcrypto-kyber)
- [ ] Merkle-CRDT for IoT sync
- [ ] Negentropy set reconciliation
- [ ] GUI management app

---

## 🚨 Known Risks & Blockers

1. **Apple API Private** — ANE requires reverse engineering (mdaiter/ane)
2. **iOS RAM Limit** — Jetsam kills > 15 MB (need profiling)
3. **Windows BSOD** — WinFSP stasis must be perfect (no sleep)
4. **NAT Traversal** — Some corporate firewalls block all UDP
5. **Wi-Fi Aware DMA** — EU regulation compliance unclear

---

## 📞 Help & Resources

- **NotebookLM**: https://notebooklm.google.com/notebook/86bc04f6-444f-4882-b54c-96467cf7cda4
- **MCP Command**: `mcp__notebooklm__ask_question` with `notebook_id: "file-project"`
- **Roadmap**: `~/GitHub/FILE/docs/architecture/IMPLEMENTATION_ROADMAP.md`
- **Memory**: `~/.claude/projects/-Users-kevinnadjarian/memory/file-project.md`

---

**Next Action**: Start Phase 1 (core_transport) by querying NotebookLM for complete ant-quic implementation.
