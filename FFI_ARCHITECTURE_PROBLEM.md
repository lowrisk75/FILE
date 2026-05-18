# FFI Architecture Problem — API Mismatch

## Le Problème

Les signatures FFI actuelles ne correspondent PAS à l'architecture MPQUIC:

### FFI actuel (ne marche pas):
```c
// Fonction FFI standalone:
int transport_encrypt(context, plaintext, ciphertext)
```

**Problème:** MPQUIC n'a pas de fonction "encrypt un buffer".

### MPQUIC réel (async, basé connexions):
```rust
// 1. Créer endpoint
let fabric = MpQuicFabric::new(addr).await?;

// 2. Établir connexion P2P
let connection = fabric.endpoint.connect(remote_addr).await?;

// 3. Ouvrir un stream (encryption automatique)
let mut send_stream = connection.open_uni_stream().await?;
send_stream.write(&data).await?;  // ← QUIC encrypts automatiquement

// 4. Recevoir (decryption automatique)
let mut recv_stream = connection.accept_uni_stream().await?;
let decrypted = recv_stream.read_to_end().await?;  // ← QUIC decrypts automatiquement
```

**Réalité:** 
- MPQUIC encrypt/decrypt est INTÉGRÉ dans les streams
- Pas d'API pour "just encrypt this buffer"
- Tout est async
- Nécessite une connexion établie

---

## Solutions Possibles

### Solution 1: Redesign FFI (1-2 jours) ✅ RECOMMANDÉ

Changer les signatures FFI pour matcher l'API MPQUIC:

```c
// Endpoint
int transport_create(addr, &context);
void transport_destroy(context);

// Connexion P2P
int transport_connect(context, remote_addr, &connection_id);
int transport_disconnect(context, connection_id);

// Streams (encryption automatique)
int transport_send_stream(context, connection_id, data, len);
int transport_recv_stream(context, connection_id, buffer, capacity, &out_len);

// Datagrams (pour packets individuels)
int transport_send_datagram(context, connection_id, data, len);
int transport_recv_datagram(context, connection_id, buffer, capacity, &out_len);
```

**Avantages:**
- Correspond à l'architecture MPQUIC réelle
- Encryption/decryption gérée par ant-quic (prouvé sécurisé)
- Pas de bridge manuel async→sync

**Inconvénients:**
- Doit réécrire Swift (AORATACore.swift, PacketTunnelProvider)
- Doit réécrire bridging header
- Signatures FFI changent

**Temps:** 1-2 jours (FFI + Swift)

---

### Solution 2: Couche Crypto Simple (1 jour)

Abandonner ant-quic pour les FFI et créer une couche crypto standalone:

```rust
// Utiliser ChaCha20-Poly1305 directement
pub unsafe extern "C" fn transport_encrypt(...) {
    let key = /* dériver depuis context */;
    let cipher = ChaCha20Poly1305::new(&key);
    let ciphertext = cipher.encrypt(nonce, plaintext)?;
    // ...
}
```

**Avantages:**
- Signatures FFI actuelles fonctionnent
- Pas d'async
- Simple à implémenter

**Inconvénients:**
- N'utilise PAS le backend MPQUIC (phases 1-5 inutilisées)
- Moins sécurisé (pas de Perfect Forward Secrecy, pas de NAT traversal)
- Doit gérer nonces, key derivation manuellement

**Temps:** 1 jour

---

### Solution 3: Hybrid — MPQUIC pour VPN, Crypto simple pour FFI (1.5 jours)

**PacketTunnelProvider** (VPN):
- Utilise MPQUIC complet (async, connexions, streams)
- Implémente startTunnel() avec vraie fabric
- Design FFI custom pour NEPacketTunnelProvider

**AORATACore.swift** (tests):
- Utilise crypto simple (ChaCha20-Poly1305)
- FFI actuels fonctionnent

**Architecture:**
```
PacketTunnelProvider (VPN)
    └─> FFI "advanced" (transport_send_stream)
        └─> MpQuicFabric réel

AORATACore (tests)
    └─> FFI "simple" (transport_encrypt)
        └─> ChaCha20-Poly1305 standalone
```

**Temps:** 1.5 jours

---

## Recommandation

**Solution 1 (Redesign FFI)** est la seule qui utilise vraiment le backend MPQUIC.

**Mais ça nécessite:**
1. Réécrire les 5 fonctions FFI core_transport
2. Réécrire AORATACore.swift (200 lignes)
3. Réécrire le bridging header (160 lignes)
4. Réécrire PacketTunnelProvider.startTunnel() (50 lignes)

**Total:** 410 lignes de réécriture + implémentation FFI

---

## Décision Kevin

Tu as 3 choix:

**A)** Solution 1 — Redesign FFI pour matcher MPQUIC (1-2 jours, utilise vrai backend)

**B)** Solution 2 — Crypto simple standalone (1 jour, n'utilise PAS MPQUIC)

**C)** Solution 3 — Hybrid (1.5 jours, MPQUIC pour VPN seulement)

Quelle lettre : **A, B ou C** ?

(Je suspends l'implémentation en attendant ta décision)
