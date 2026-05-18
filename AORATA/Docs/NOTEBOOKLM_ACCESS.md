# NotebookLM - Source de Vérité du Projet FILE

**IMPORTANT**: Toutes les spécifications techniques détaillées sont dans NotebookLM. Ce fichier explique comment y accéder.

---

## 📚 Accès NotebookLM

**Notebook ID**: `file-project`  
**URL**: https://notebooklm.google.com/notebook/86bc04f6-444f-4882-b54c-96467cf7cda4

### Via Claude Code (MCP)

```javascript
// Interroger NotebookLM depuis Claude Code
mcp__notebooklm__ask_question({
  notebook_id: "file-project",
  question: "Votre question ici",
  source_format: "footnotes"  // ou "json", "inline", "none"
})
```

### Via Web

1. Ouvrir https://notebooklm.google.com/notebook/86bc04f6-444f-4882-b54c-96467cf7cda4
2. Poser une question dans le chat
3. Copier la réponse avec les citations

---

## 🎯 Contenu du Notebook

Le notebook contient **toute** la documentation technique du projet FILE:

### Architecture & Design
- Spécifications MPQUIC complètes
- Architecture NAT traversal (draft-seemann)
- Noise protocol (patterns IK/XX)
- AONT (All-Or-Nothing Transform)
- BFE (Bloom-Filter Encryption)
- VFS anti-ransomware (Windows + Unix)
- Edge AI routing (Apple Neural Engine)
- S3-FIFO cache algorithm
- CAPoW anti-DDoS

### Code de Référence
- Implémentations Rust complètes
- Exemples ant-quic / noq
- WinFSP STATUS_PENDING
- FUSE Poll::Pending
- Objective-C FFI pour ANE
- Tests critiques

### Intégration Apple
- NEPacketTunnelProvider (iOS/macOS)
- mmap IPC (App Groups, Zero-Copy)
- Wi-Fi Aware (DMA compliance)
- ANE API privées (_ANEClient, _ANECompiler)
- Contraintes Jetsam (15 MB RAM max)

### Benchmarks & Validation
- Performance targets
- Tests critiques par phase
- Points de validation
- Trade-offs architecture

---

## 💡 Workflow Recommandé

### Avant d'implémenter un module

1. **Consulter NotebookLM** pour les specs exactes
   ```
   Question: "Implémentation complète de [module].rs avec [crate]"
   Exemple: "Implémentation complète de mpquic.rs avec ant-quic 0.14.192"
   ```

2. **Lire la réponse complète** (peut être longue)
   - Algorithmes détaillés
   - Code de référence
   - Edge cases
   - Contraintes système

3. **Implémenter** selon les specs

4. **Revenir à NotebookLM** pour questions/edge cases
   ```
   Question: "Comment gérer [edge case spécifique] dans [module]?"
   Exemple: "Comment gérer Jetsam iOS dans mmap IPC ring buffer?"
   ```

### Questions Types à Poser

#### Phase 1 (core_transport)
- "Implémentation complète de mpquic.rs avec ant-quic 0.14.192"
- "NAT hole punching: DCUtR timing et TTL manipulation"
- "QUIC frame extensions: ADD_ADDRESS, PUNCH_ME_NOW implémentation"
- "Comment tester la mobilité 0-RTT en simulation?"

#### Phase 2 (crypto_fabric)
- "Noise protocol: handshake IK complet avec snow 0.10.0"
- "AONT: algorithme de fragmentation avec preuve de sécurité"
- "BFE avec cosmian_findex: puncture operation en O(1)"

#### Phase 3 (vfs_guard)
- "VFS Windows: STATUS_PENDING sans deadlock WinFSP"
- "VFS Unix: Poll::Pending sans bloquer Tokio executor"
- "Canary detection: patterns de fichiers leurres"

#### Phase 4 (edge_ai)
- "Apple Neural Engine: FFI bridge complet depuis Rust"
- "Pareto routing: algorithme multi-objectifs pour MPQUIC"
- "mdaiter/ane: intégration _ANEClient et _ANECompiler"

#### Phase 5 (relay_daemon)
- "S3-FIFO: implémentation complète avec 3 queues"
- "CAPoW: Argon2 difficulté adaptative selon saturation disque"

#### Phase 6 (apple_platform)
- "NEPacketTunnelProvider: tunnel IP complet iOS/macOS"
- "mmap IPC: ring buffer zero-copy avec App Groups"
- "Wi-Fi Aware: P2P discovery conforme DMA"

---

## 🔍 Sessions NotebookLM

NotebookLM garde le contexte des conversations dans des **sessions**.

### Créer une nouvelle session
Chaque nouveau `ask_question` sans `session_id` crée une nouvelle session.

### Continuer une session
```javascript
// Première question
const response1 = ask_question({
  notebook_id: "file-project",
  question: "Explain MPQUIC architecture"
});

// Question de suivi (même session)
const response2 = ask_question({
  notebook_id: "file-project",
  session_id: response1.session_id,  // ← Réutiliser le session_id
  question: "How does 0-RTT handover work in that architecture?"
});
```

### Lister les sessions actives
```javascript
mcp__notebooklm__list_sessions()
```

### Réinitialiser une session
```javascript
mcp__notebooklm__reset_session({ session_id: "..." })
```

---

## 📊 Format des Réponses

### source_format: "footnotes" (recommandé)
Réponse + sources en bas avec `[1]`, `[2]`, etc.

### source_format: "inline"
Citations intégrées dans le texte avec `[N] (Source: "excerpt...")`

### source_format: "json"
Réponse + array `sources` structuré (pour parsing programmatique)

### source_format: "none"
Réponse brute sans citations (plus rapide)

---

## ⚠️ Limitations NotebookLM

### Quotas (Free Tier)
- **100 notebooks** max
- **50 sources** par notebook
- **50 queries/jour**

### Réponses longues
Si la réponse dépasse le token limit:
1. Elle est sauvegardée dans un fichier temporaire
2. Lire le fichier par chunks
3. Ou utiliser un subagent pour l'analyser

### Timeout
Si NotebookLM timeout (rare):
- Réessayer avec une question plus précise
- Ou diviser en plusieurs questions

---

## 🎓 Tips

1. **Questions spécifiques** > questions générales
   - ❌ "Comment marche le projet?"
   - ✅ "Implémentation complète de mpquic.rs avec ant-quic"

2. **Demander le code** explicitement
   - ❌ "Explique AONT"
   - ✅ "Code Rust complet de aont.rs avec tests"

3. **Citer les crates** exactes
   - ✅ "avec ant-quic 0.14.192"
   - ✅ "avec snow 0.10.0 features ring-accelerated"

4. **Edge cases** = questions séparées
   - Après implémentation de base
   - "Comment gérer [cas spécifique]?"

5. **Benchmarks** disponibles
   - "Quels sont les benchmarks S3-FIFO vs LRU?"
   - "Latence target pour ANE inference?"

---

## 📝 Exemples de Requêtes Complètes

### Démarrer Phase 1
```javascript
ask_question({
  notebook_id: "file-project",
  question: `
    Donne-moi l'implémentation complète de core_transport/mpquic.rs avec ant-quic 0.14.192.
    
    Inclus:
    1. Configuration SO_REUSEPORT
    2. Initialisation Endpoint avec NAT traversal
    3. Enable multipath feature
    4. Gestion Connection ID (CID)
    5. Tests unitaires critiques
    
    Format: Code Rust complet + explications inline.
  `,
  source_format: "footnotes"
})
```

### Debugging Edge Case
```javascript
ask_question({
  notebook_id: "file-project",
  session_id: "...",  // Session précédente
  question: `
    Dans l'implémentation MPQUIC précédente, comment gérer le cas où:
    - Le Wi-Fi tombe pendant un handover 5G
    - Le pare-feu bloque UDP temporairement
    - Le relais devient injoignable
    
    Quelle est la stratégie de fallback recommandée?
  `,
  source_format: "footnotes"
})
```

---

## 🔗 Références

- Notebook URL: https://notebooklm.google.com/notebook/86bc04f6-444f-4882-b54c-96467cf7cda4
- Projet local: `~/GitHub/FILE/`
- Mémoire: `docs/PROJECT_MEMORY.md`
- Roadmap: `docs/architecture/IMPLEMENTATION_ROADMAP.md`
- TODO: `TODO.md`
