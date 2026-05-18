# AORATA Protocol v2 - Deep Search Prompts

Pour approfondir l'implémentation avec des recherches supplémentaires.

---

## 🔍 Phase 1 : Core Transport (En cours)

### Prompt 1 : ant-quic 0.14.192 API exacte

```
Je dois implémenter AORATA Protocol v2 avec ant-quic 0.14.192. 

J'ai besoin de l'API exacte pour:
1. Créer un Endpoint QUIC server avec une socket préconfigurée (SO_REUSEADDR fait)
2. Activer multipath QUIC (plusieurs chemins réseau simultanés)
3. Activer NAT traversal avec les frames IETF (ADD_ADDRESS, OBSERVED_ADDRESS, PUNCH_ME_NOW)
4. Configuration ServerConfig ou EndpointConfig
5. Gestion du runtime Tokio

Code actuel (ne compile pas):
```rust
let endpoint = Endpoint::builder()
    .listen_with_socket(async_udp, server_config)
    .build()?;
```

Donne-moi le code Rust exact avec ant-quic 0.14.192 pour:
- ServerConfig::new() ou équivalent
- Endpoint::new() ou Endpoint::server()
- Configuration multipath
- Configuration NAT traversal (si disponible dans cette version)

Inclus les imports nécessaires et les types exacts.
```

### Prompt 2 : socket2 SO_REUSEPORT sur macOS/Linux

```
Configuration socket2 en Rust pour NAT hole punching (Simultaneous Open).

Plateforme: macOS (Apple Silicon ARM64) + Linux
Crate: socket2 = "0.5.9"

J'ai besoin de:
1. Code exact pour set_reuse_port() sur Socket (méthode n'existe pas dans 0.5.9?)
2. Alternative si la méthode n'existe pas dans cette version
3. Différences entre macOS et Linux pour SO_REUSEPORT
4. Ordre correct des appels: bind, reuse_address, reuse_port, nonblocking

Code actuel:
```rust
socket.set_reuse_address(true)?;
socket.set_reuse_port(true)?; // ← Erreur: méthode introuvable
socket.bind(&bind_addr.into())?;
```

Donne-moi la bonne approche pour socket2 0.5.9.
```

### Prompt 3 : NAT Hole Punching avec QUIC frames

```
Implémentation NAT hole punching pour AORATA Protocol v2.

Contexte:
- QUIC multipath avec ant-quic 0.14.192
- NAT restrictif (Symmetric NAT)
- Besoin de DCUtR (Direct Connection Upgrade through Relay)

Recherche approfondie sur:
1. TTL manipulation (3-4 hops) pour créer l'état firewall sans trigger IDS
2. DCUtR timing exact: RTT/2 synchronization entre initiator et responder
3. QUIC frame extensions:
   - ADD_ADDRESS (0x3d7e90)
   - OBSERVED_ADDRESS (0x9f81a6)
   - PUNCH_ME_NOW (0x3d7e92)
4. Algorithme "Birthday Problem" pour Simultaneous Open

Code skeleton:
```rust
pub async fn execute_simultaneous_open(
    local_addr: SocketAddr,
    target_public_addr: SocketAddr,
    rtt_to_peer: Duration,
    is_initiator: bool,
) -> std::io::Result<()> {
    // TODO
}
```

Donne-moi l'algorithme complet avec timing précis et gestion erreurs.
```

---

## 🔐 Phase 2 : Crypto Fabric (Prochain)

### Prompt 4 : Noise XXfallback implementation avec snow

```
Implémentation Noise Protocol pattern XXfallback pour AORATA Protocol v2.

Crate: snow = "0.10.0"
Pattern: Noise_XXfallback_25519_ChaChaPoly_SHA256

Recherche approfondie:
1. Code complet XXfallback (initiator + responder)
2. Différences vs XX pattern (pourquoi XXfallback est nécessaire)
3. Gestion 0-RTT avec fallback gracieux vers handshake interactif
4. State machine: éviter la réutilisation de nonce en asynchrone multi-path
5. Tests: simuler échec 0-RTT et vérifier fallback

Structure attendue:
```rust
pub struct NoiseHandshake {
    state: snow::HandshakeState,
    pattern: &'static str,
}

impl NoiseHandshake {
    pub fn new_initiator(static_key: &[u8]) -> Result<Self, Error>;
    pub fn new_responder(static_key: &[u8]) -> Result<Self, Error>;
    pub fn try_0rtt(&mut self, message: &[u8]) -> Result<Vec<u8>, Error>;
    pub fn fallback_to_xx(&mut self) -> Result<(), Error>;
}
```

Donne code production-ready avec tous les edge cases gérés.
```

### Prompt 5 : Hybrid AONT-Threshold avec Reed-Solomon

```
Implémentation Hybrid AONT-Threshold pour AORATA Protocol v2.

Crates:
- reed-solomon-erasure = "6.0.0"
- sharks = "0.5.0" (Shamir Secret Sharing)

Architecture:
1. Header (256 bits = 32 bytes):
   - AONT transform
   - Homomorphic Encryption (additive)
   - Shamir Secret Sharing (3-of-5 threshold)
2. Bulk (reste du payload):
   - Reed-Solomon erasure coding (parallelizable)

Garantie de sécurité:
- Adversaire interceptant <3 chemins MPQUIC = ZERO data leakage
- Bulk chiffré inutile sans les clés du header

Recherche:
1. AONT keyless transform exact (algorithme)
2. Additive Homomorphic Encryption simple (pas besoin full FHE)
3. Shamir 3-of-5 avec sharks crate
4. Reed-Solomon optimal parameters (n data shards, k parity shards)
5. Performance: parallelization du bulk encoding

Code complet avec benchmark attendu: header <1ms, bulk <5ms pour 10MB.
```

---

## 🛡️ Phase 3 : VFS Anti-Ransomware

### Prompt 6 : Shannon Entropy monitoring temps réel

```
Détection ransomware via entropy monitoring pour AORATA Protocol v2.

Algorithme:
- Calcul Shannon entropy en temps réel sur les writes
- Threshold: entropy > 7.8 = suspect ransomware
- Action: Poll::Pending (suspend I/O indéfiniment)

Recherche:
1. Algorithme Shannon entropy optimisé (SIMD possible?)
2. Sliding window pour éviter faux positifs (fichiers légitimement compressés/chiffrés)
3. Whitelist: extensions légitimes haute-entropy (.zip, .jpg, .mp4)
4. Benchmark: overhead CPU acceptable (<2% sur I/O normal)
5. Tests: corpus ransomware (WannaCry, Locky patterns)

Code Rust avec zero-copy sur buffer I/O.
```

---

## 🧠 Phase 4 : Edge AI

### Prompt 7 : ANE LoRA dynamic injection (Apple Silicon)

```
Apple Neural Engine (ANE) avec LoRA dynamic injection pour AORATA Protocol v2.

Objectif: Routing AI qui s'adapte <100µs (vs 500ms recompilation CoreML).

Recherche approfondie:
1. _ANEInMemoryModelDescriptor (private API) - comment charger un MIL graph UNE FOIS
2. _ANEIOSurfaceObject - injection tensor dynamique
3. LoRA (Low-Rank Adaptation) matrices - quantization INT8/FP16
4. Metal framework pour gérer les IOSurface
5. Activation clamping (éviter NaN cascades)

Code:
```rust
pub struct AneRoutingAgent {
    model_descriptor: ANEInMemoryModelDescriptor,
    io_surface: ANEIOSurfaceObject,
    lora_matrices: Vec<Matrix<f16>>,
}

impl AneRoutingAgent {
    pub fn new(model_path: &Path) -> Result<Self, Error>;
    pub fn update_routing_policy(&mut self, policy: &RoutingPolicy) -> Result<(), Error>; // <100µs
    pub fn compute_route(&self, flow: &FlowAttributes) -> Result<PathDecision, Error>; // <1ms
}
```

Donne code FFI Objective-C ↔ Rust + Metal integration.
```

---

## 🔄 Phase 5 : Relay Daemon

### Prompt 8 : Asymmetric CAPoW avec Argon2id

```
Context-Aware Proof of Work asymétrique pour AORATA Protocol v2.

Problème: Argon2id 2GB bloque le relay (Self-DoS) si utilisé pour verification.

Solution: Asymmetric design basé sur "Birthday Problem":
- Client/Attacker: Argon2id(memory=2GB, time=16, para=4) → 5-30 sec
- Relay verification: Argon2id(memory=64MB, time=1, para=1) → <100µs

Recherche:
1. Algorithme "Birthday Problem" appliqué à Argon2
2. Pourquoi la verification peut être 32× plus légère que la génération
3. ML Context Scoring:
   - TTL, port profile, source IP reputation
   - Inter-arrival timing
   - Relay CPU load + disk saturation
   - Mapping → Argon2 difficulty (64MB à 2GB dynamique)
4. Rayon thread pool: isolation CPU-bound (ne jamais bloquer Tokio)

Code production avec adaptive difficulty + ML scoring complet.
```

---

## 📱 Phase 6 : iOS/macOS Integration

### Prompt 9 : NEPacketTunnelProvider avec unidirectional IPC

```
Network Extension iOS/macOS pour AORATA Protocol v2.

Sécurité critique: IPC unidirectionnel pour éviter TOCTOU sandbox escape.

Recherche:
1. NEPacketTunnelProvider lifecycle complet
2. App Groups mmap configuration:
   - MAP_ANON | MAP_SHARED
   - PROT_READ (consumer) / PROT_WRITE (producer) strict separation
3. quetzalcoatl SPSC ring buffer (lock-free)
4. Apple Silicon MIE (Memory Integrity Enforcement) - comment vérifier activation
5. Jetsam memory limit: rester sous 15 MB sur iOS

Code:
```swift
class AORATATunnelProvider: NEPacketTunnelProvider {
    var ipc: UnidirectionalIPC?
    
    override func startTunnel(options: [String : NSObject]?, completionHandler: @escaping (Error?) -> Void)
    override func stopTunnel(with reason: NEProviderStopReason, completionHandler: @escaping () -> Void)
    override func handleAppMessage(_ messageData: Data, completionHandler: ((Data?) -> Void)?)
}
```

Donne implémentation complète Swift + Rust FFI bridge.
```

---

## 🎯 Utilisation des Prompts

### Avec Gemini Deep Research
1. Copier le prompt
2. Coller dans https://gemini.google.com/ (mode Deep Research)
3. Attendre 2-5 minutes (recherche exhaustive)
4. Récupérer le markdown généré
5. Me le passer ou l'intégrer directement dans le code

### Avec NotebookLM
1. Ajouter le prompt comme question dans NotebookLM
2. Session ID: file-project
3. Récupérer la réponse avec citations
4. Intégrer dans le code avec références

### Avec Perplexity Pro
1. Mode "Pro Search" activé
2. Coller le prompt
3. Sources académiques + GitHub + documentation officielle
4. Export markdown

---

## 📊 Priorité des Recherches

| # | Prompt | Phase | Priorité | Bloquant? |
|---|--------|-------|----------|-----------|
| 1 | ant-quic API | 1 | 🔴 URGENT | ✅ Oui |
| 2 | socket2 REUSEPORT | 1 | 🔴 URGENT | ✅ Oui |
| 3 | NAT Hole Punching | 1 | 🟡 Haute | ⏸️ Non |
| 4 | Noise XXfallback | 2 | 🟡 Haute | ⏸️ Non |
| 5 | Hybrid AONT | 2 | 🟡 Haute | ⏸️ Non |
| 6 | Shannon Entropy | 3 | 🟢 Moyenne | ❌ Non |
| 7 | ANE LoRA | 4 | 🟢 Moyenne | ❌ Non |
| 8 | CAPoW Asymmetric | 5 | 🟢 Moyenne | ❌ Non |
| 9 | NEPacketTunnel | 6 | 🔵 Basse | ❌ Non |

---

**Focus immédiat** : Prompts 1 & 2 (ant-quic + socket2) pour débloquer Phase 1 ! 🚀
