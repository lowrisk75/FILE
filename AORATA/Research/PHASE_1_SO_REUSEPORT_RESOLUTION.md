# Phase 1 - SO_REUSEPORT Resolution

**Date**: 2026-05-16  
**Status**: ✅ RÉSOLU  
**Research**: #2 "Ant-quic SO_REUSEPORT NAT Hole Punching.md"

## Problème Initial

Le code initial dans `mpquic.rs` configurait `SO_REUSEPORT` via socket2 0.5.9:

```rust
#[cfg(not(windows))]
socket.set_reuse_port(true)?;
```

**Problème**: Ce socket était immédiatement abandonné car `P2pEndpoint::new(p2p_config)` crée son propre socket UDP en interne. Il n'existe AUCUNE API pour injecter un socket pré-configuré dans ant-quic.

## Deep Research Findings

La recherche approfondie (25,000+ tokens via Gemini Deep Research / Perplexity Pro) a révélé:

### ant-quic "Zero Configuration" Philosophy

- ant-quic 0.14.192 gère le socket UDP en interne
- Pure PQC (ML-KEM-768, ML-DSA-65) requiert MTU strict (1252+ bytes)
- Aucune API pour socket injection: `P2pEndpoint::new()` prend UNIQUEMENT `P2pConfig`
- `P2pConfig::default()` ne propose AUCUNE option de customization socket

### SO_REUSEPORT Packet Distribution Problems

#### Linux (hash-based distribution)
```c
// Kernel Linux: hash sur (src_ip, src_port, dst_ip, dst_port)
sock = listening_sockets[hash(tuple) % num_socks];
```
- Collision adversariales possibles (attaques Hash DoS)
- Fragmentation QUIC si les paquets d'un même flux sont dispersés

#### macOS (pseudo-random distribution)
```c
// Kernel macOS: distribution pseudo-aléatoire
sock = listening_sockets[arc4random_uniform(num_socks)];
```
- Pire que Linux: dispersion totalement aléatoire
- QUIC state machine cassée (paquets out-of-order massif)

### QUIC-Native NAT Traversal (Recommandé)

ant-quic implémente nativement:

1. **PUNCH_ME_NOW frames** (DCUtR synchronization)
   - Timing RTT/2 pour Simultaneous Open
   - Birthday Problem mitigation (coordinated hole punching)

2. **ADD_ADDRESS / OBSERVED_ADDRESS frames**
   - Multi-path NAT discovery
   - Support Symmetric NAT

3. **SharedTransport multiplexing**
   - N connexions logiques sur 1 socket physique
   - Évite le besoin de SO_REUSEPORT

## Décision Architecturale

### Option 1 (ADOPTÉE): QUIC-Native NAT Traversal

✅ **Avantages**:
- Compatible avec ant-quic "Zero Configuration"
- Pure PQC (ML-KEM-768) sans fragmentation MTU
- Fonctionne sur Symmetric NAT (Birthday Problem mitigation)
- Pas de collision hash attacks

❌ **Inconvénients**:
- Requiert coordination RTT/2 (DCUtR)
- Dépend de relay pour découverte initiale

### Option 2 (REJETÉE): Fork ant-quic source

❌ **Raisons du rejet**:
- Maintenance long-terme prohibitive
- Pure PQC updates (ML-KEM-768 → ML-KEM-1024) nécessiteraient re-fork
- Perd les optimisations upstream (BBRv2, GSO/GRO)

### Option 3 (REJETÉE): LD_PRELOAD hooking

❌ **Raisons du rejet**:
- Fragile (casse si ant-quic change l'ordre d'appels socket)
- Non portable (macOS ≠ Linux)
- Interdit sur iOS/macOS hardened runtime

## Changements Implémentés

### 1. Cargo.toml
```diff
-# Low-level Socket Configuration
-socket2 = { version = "0.5.9", features = ["all"] }
+# Note: socket2 removed — ant-quic gère le socket UDP en interne avec:
+#   - MTU path discovery (Pure PQC 1252+ bytes)
+#   - GSO/GRO offload
+#   - QUIC-native NAT traversal (PUNCH_ME_NOW, ADD_ADDRESS, OBSERVED_ADDRESS)
```

### 2. mpquic.rs - Header Documentation
```rust
//! ## NAT Traversal Architecture
//!
//! AORATA abandonne SO_REUSEPORT au niveau OS pour les raisons suivantes
//! (cf. Research #2 "Ant-quic SO_REUSEPORT NAT Hole Punching.md"):
//!
//! 1. **ant-quic "Zero Configuration" philosophy**: P2pEndpoint crée son propre
//!    socket en interne; aucune API n'existe pour injecter un socket pré-configuré.
//!
//! 2. **Packet distribution problems**: SO_REUSEPORT distribue les paquets UDP
//!    via hash (Linux) ou pseudo-random (macOS), ce qui casse la sémantique QUIC
//!    "1 flux logique = 1 socket physique".
//!
//! 3. **QUIC-native traversal is superior**: ant-quic implémente nativement:
//!    - `PUNCH_ME_NOW` frames pour synchroniser RTT/2 timing (DCUtR)
//!    - `ADD_ADDRESS` / `OBSERVED_ADDRESS` pour multi-path NAT discovery
//!    - Birthday Problem mitigation via coordinated simultaneous open
//!
//! 4. **SharedTransport multiplexing**: AORATA encapsule plusieurs connexions
//!    logiques sur un même endpoint physique, évitant le besoin de SO_REUSEPORT.
```

### 3. mpquic.rs - MpQuicFabric::new()
```diff
-// 1. Configuration de la socket bas niveau via socket2
-let socket = Socket::new(domain, Type::DGRAM, Some(Protocol::UDP))?;
-socket.set_reuse_address(true)?;
-#[cfg(not(windows))]
-socket.set_reuse_port(true)?;
-socket.bind(&bind_addr.into())?;
-socket.set_nonblocking(true)?;
-
-// 2. Note: P2pEndpoint::new() crée son propre socket en interne
-// TODO: Vérifier si P2pConfig expose des options pour socket customization

+// 1. Configuration ant-quic P2P (AORATA v2 Specs)
+// P2pConfig::default() active:
+// - NAT Traversal frames (PUNCH_ME_NOW, ADD_ADDRESS, OBSERVED_ADDRESS)
+// - Multipath QUIC (Wi-Fi + 5G simultané)
+// - Pure PQC (ML-KEM-768, ML-DSA-65)
+// - BBRv2 congestion control
+//
+// Note: P2pEndpoint::new() crée son propre socket UDP en interne.
+// Il n'y a PAS d'API pour injecter un socket pré-configuré (cf. Research #2).
 let p2p_config = P2pConfig::default();
```

### 4. Tests mis à jour
```diff
-let fabric = MpQuicFabric::new(addr, 1000).unwrap();
+let fabric = MpQuicFabric::new(addr, 1000).await.unwrap();

+use std::hash::Hasher; // Pour hasher.write() et hasher.finish()
```

## Résultat Final

✅ **Compilation**: 0 erreurs, 0 warnings  
✅ **Tests**: 7 passed (defensive validation, SipHash-2-4, CID from_slice)  
✅ **Documentation**: Header complet expliquant l'architecture NAT traversal  
✅ **Phase 1 URGENT blockers**: RÉSOLUS (Endpoint init + NAT strategy)

## Prochaines Étapes

### Phase 1 Suite
- [ ] Implémenter `nat_punch.rs` avec Research #3 (DCUtR timing, TTL manipulation, Birthday Problem)
- [ ] Tester PUNCH_ME_NOW synchronization RTT/2
- [ ] Valider sur Symmetric NAT

### Phase 2: Crypto Fabric
- [ ] Noise XXfallback (Research #4)
- [ ] Hybrid AONT-Threshold (Research #5)

### Phase 3-6
- [ ] VFS Anti-Ransomware (Research #6)
- [ ] Edge AI (Research #7)
- [ ] Relay Daemon (Research #8)
- [ ] iOS/macOS Integration (Research #9)

## Références

- **Research #2**: "Ant-quic SO_REUSEPORT NAT Hole Punching.md" (25,000+ tokens)
- **ant-quic docs**: https://docs.rs/ant-quic/0.14.192
- **RFC 9000**: QUIC Transport Protocol (Amplification Defense, MTU)
- **DCUtR**: Direct Connection Upgrade through Relay (RTT/2 timing)
- **Birthday Problem**: Simultaneous Open collision probability
