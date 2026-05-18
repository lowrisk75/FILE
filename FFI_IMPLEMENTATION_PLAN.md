# FFI Implementation Plan — Connecter le Backend Rust au Swift

## Problème

Les phases 1-5 ont implémenté le backend Rust complet (MPQUIC, Noise, VFS Guard, Edge AI, CAPoW).
La phase 6.2 a créé les **signatures FFI** mais elles sont des **stubs** — elles ne connectent pas au vrai code Rust.

## Solution: 3 Options

### Option 1: Implémentation FFI Complète (2-3 jours)

Connecter chaque fonction FFI au backend Rust:

**core_transport/src/ffi.rs:**
```rust
#[no_mangle]
pub unsafe extern "C" fn transport_create(
    local_addr: *const c_char,
    out_context: *mut *mut TransportContext,
) -> c_int {
    let addr_str = CStr::from_ptr(local_addr).to_str().unwrap();
    let socket_addr: SocketAddr = addr_str.parse().unwrap();
    
    // VRAI CODE: Créer un runtime Tokio pour async
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let fabric = runtime.block_on(async {
        MpQuicFabric::new(socket_addr, 10_000).await
    }).unwrap();
    
    // Emballer dans Box pour FFI
    let boxed = Box::new(TransportContextReal {
        fabric,
        runtime,
    });
    *out_context = Box::into_raw(boxed) as *mut TransportContext;
    TransportError::Success as c_int
}

#[no_mangle]
pub unsafe extern "C" fn transport_encrypt(
    context: *mut TransportContext,
    plaintext: *const u8,
    plaintext_len: usize,
    ciphertext: *mut u8,
    ciphertext_capacity: usize,
    out_len: *mut usize,
) -> c_int {
    let ctx = &*(context as *const TransportContextReal);
    let plain_slice = slice::from_raw_parts(plaintext, plaintext_len);
    
    // VRAI CODE: Appeler ant-quic encryption
    let encrypted = ctx.runtime.block_on(async {
        ctx.fabric.encrypt_packet(plain_slice).await
    }).unwrap();
    
    // Copier résultat
    let cipher_slice = slice::from_raw_parts_mut(ciphertext, ciphertext_capacity);
    cipher_slice[..encrypted.len()].copy_from_slice(&encrypted);
    *out_len = encrypted.len();
    
    TransportError::Success as c_int
}
```

**Défis:**
1. **Async Rust → Sync FFI**: ant-quic est async, FFI est sync → besoin de `block_on()` (bloque thread)
2. **Lifetime management**: Box/Arc pour passer ownership à travers FFI
3. **Error propagation**: Rust Result → C int error codes
4. **Memory safety**: Vérifier tous les raw pointers

**Temps estimé:** 2-3 jours pour les 5 modules (27 fonctions FFI)

---

### Option 2: FFI Minimal + Tests Swift (1 jour)

Implémenter **uniquement** les fonctions testées par Swift:

**Priorité 1** (testées dans AORATAApp.swift):
- `transport_encrypt` / `transport_decrypt`
- `vfs_calculate_entropy`
- `ai_get_backend_name` / `ai_get_tier`

**Priorité 2** (utilisées par NEPacketTunnelProvider):
- `transport_create` / `transport_destroy`
- `noise_create_initiator` / `noise_handshake_write`

Laisser le reste en stub pour le moment.

**Temps estimé:** 1 jour

---

### Option 3: Mode Démo — Garder les Placeholders (0 jours)

**Pour la démo/prototype:**
- Les FFI retournent `Success` et copient des données
- Swift compile et tourne
- Pas de vraie crypto mais prouve l'intégration

**Avantages:**
- Xcode integration fonctionne immédiatement
- Montre le concept end-to-end
- Teste la chaîne Swift → C FFI → Rust

**Désavantages:**
- Pas de vraie sécurité
- Ne peut pas être déployé en production

**Pour passer en prod:**
- Implémenter Option 1 ou 2 après validation du concept

---

## Recommandation

**Court terme (aujourd'hui):**
→ **Option 3** — Valider l'intégration Xcode avec placeholders (40 min)

**Moyen terme (cette semaine):**
→ **Option 2** — Implémenter FFI minimal pour fonctions testées (1 jour)

**Long terme (2-3 semaines):**
→ **Option 1** — FFI complet avec tous les backends connectés (2-3 jours)

---

## Checklist Option 1 (FFI Complet)

### core_transport (8h)
- [ ] `transport_create` → `MpQuicFabric::new()`
- [ ] `transport_connect` → Connection handshake
- [ ] `transport_encrypt` → QUIC packet encryption
- [ ] `transport_decrypt` → QUIC packet decryption
- [ ] `transport_destroy` → Drop fabric + runtime
- [ ] Tests FFI → Rust roundtrip

### crypto_fabric (6h)
- [ ] `noise_create_initiator` → `NoiseHandshake::initiator()`
- [ ] `noise_create_responder` → `NoiseHandshake::responder()`
- [ ] `noise_handshake_write` → Write handshake message
- [ ] `noise_handshake_read` → Process handshake response
- [ ] `noise_encrypt` → `Transport::encrypt()`
- [ ] `noise_decrypt` → `Transport::decrypt()`
- [ ] `noise_destroy` → Drop handshake state
- [ ] Tests FFI → Rust roundtrip

### vfs_guard (4h)
- [ ] `vfs_calculate_entropy` → `shannon_entropy()`
- [ ] `vfs_check_write` → Full validation pipeline
- [ ] `vfs_is_high_entropy` → Threshold check
- [ ] Tests FFI → Rust roundtrip

### edge_ai (6h)
- [ ] `ai_create` → `RoutingAgent::new()`
- [ ] `ai_infer` → `score_paths()` avec CoreML
- [ ] `ai_get_backend_name` → Backend enum → string
- [ ] `ai_get_tier` → Tier detection
- [ ] `ai_destroy` → Drop agent
- [ ] Tests FFI → Rust roundtrip

### relay_daemon (4h)
- [ ] `capow_generate_challenge` → `Challenge::new()`
- [ ] `capow_compute_proof` → `Prover::solve()`
- [ ] `capow_verify_proof` → `Verifier::check()`
- [ ] Tests FFI → Rust roundtrip

**Total:** 28h = 3.5 jours

---

## Décision

**Kevin, choisis:**

1. **Aujourd'hui** → Option 3 (40 min Xcode, placeholders OK pour démo)
2. **Cette semaine** → Option 2 (1 jour, fonctions critiques seulement)
3. **Production** → Option 1 (3.5 jours, FFI complet)

Quelle option veux-tu ?
