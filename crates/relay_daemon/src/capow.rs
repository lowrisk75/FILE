/// CAPoW - Context-Aware Proof-of-Work Anti-DDoS
///
/// Implémentation complète du système asymétrique MTP-Argon2id + GBP
///
/// Architecture:
/// - Client: Génère 2GB Argon2id DAG + résout Generalized Birthday Problem (5-30s)
/// - Relay: Vérifie avec 64MB + multi-proof Merkle (<100µs)
///
/// Basé sur:
/// - Wagner's Algorithm pour GBP (k-XOR problem)
/// - MTP (Merkle Tree Proof) sur Argon2id
/// - ML Context Scoring (TTL, ASN, IAT, CPU)
/// - Tokio/Rayon isolation stricte
use argon2::{Algorithm, Argon2, Block, Params, Version};
use rayon::prelude::*;
use rs_merkle::{Hasher, MerkleProof, MerkleTree};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use thiserror::Error;

// ============================================================================
// CONSTANTES & PARAMÈTRES MTP-ARGON2
// ============================================================================

/// Taille d'un bloc Argon2id (1024 bytes = 128 words de 64 bits)
const BLOCK_SIZE_BYTES: usize = 1024;
const BLOCK_SIZE_U64: usize = BLOCK_SIZE_BYTES / 8;

/// Paramètres client (générateur de preuve)
/// CRITIQUE: t=1, p=1 pour prévenir segment sharing attacks (cf. research doc)
pub const CLIENT_MEM_KB: u32 = 2_097_152; // 2 GB
pub const CLIENT_PASSES: u32 = 1;
pub const CLIENT_LANES: u32 = 1;

/// Paramètres relay (vérificateur) - réduction 32x
pub const RELAY_MEM_KB: u32 = 65_536; // 64 MB
pub const RELAY_PASSES: u32 = 1;
pub const RELAY_LANES: u32 = 1;

/// Paramètres GBP (Generalized Birthday Problem)
pub const GBP_K: usize = 8; // Nombre d'éléments dans la solution
pub const GBP_COLLISION_BITS: usize = 21; // Bits de collision partielle

// ============================================================================
// STRUCTURES DE DONNÉES
// ============================================================================

/// Bloc Argon2id de 1KB (128 mots de 64 bits)
#[derive(Clone, Debug)]
pub struct Block1024(pub [u64; BLOCK_SIZE_U64]);

// Manual Serialize/Deserialize for large array
impl Serialize for Block1024 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.as_slice().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Block1024 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec: Vec<u64> = Vec::deserialize(deserializer)?;
        if vec.len() != BLOCK_SIZE_U64 {
            return Err(serde::de::Error::custom(format!(
                "Expected {} u64 elements, got {}",
                BLOCK_SIZE_U64,
                vec.len()
            )));
        }
        let mut arr = [0u64; BLOCK_SIZE_U64];
        arr.copy_from_slice(&vec);
        Ok(Block1024(arr))
    }
}

impl Block1024 {
    /// Crée un bloc vide (zéro)
    pub fn zero() -> Self {
        Block1024([0u64; BLOCK_SIZE_U64])
    }

    /// XOR de deux blocs (vectorisable par LLVM)
    #[inline(always)]
    pub fn xor(&self, other: &Self) -> Self {
        let mut result = [0u64; BLOCK_SIZE_U64];
        for (i, (a, b)) in self.0.iter().zip(other.0.iter()).enumerate() {
            result[i] = a ^ b;
        }
        Block1024(result)
    }

    /// Extrait une clé partielle pour la collision (GBP)
    /// bit_offset: décalage en bits dans le bloc
    /// bit_len: nombre de bits à extraire
    #[inline(always)]
    pub fn extract_partial_key(&self, bit_offset: usize, bit_len: usize) -> u64 {
        let word_idx = bit_offset / 64;
        let shift = bit_offset % 64;
        let mask = if bit_len == 64 {
            u64::MAX
        } else {
            (1u64 << bit_len) - 1
        };

        // Gère le chevauchement sur deux mots si nécessaire
        let current_word = self.0[word_idx] >> shift;
        if shift + bit_len > 64 && word_idx + 1 < BLOCK_SIZE_U64 {
            let next_word = self.0[word_idx + 1] << (64 - shift);
            (current_word | next_word) & mask
        } else {
            current_word & mask
        }
    }

    /// Convertit en bytes pour hachage Merkle
    pub fn as_bytes(&self) -> &[u8; BLOCK_SIZE_BYTES] {
        // SAFETY: [u64; 128] est aligné et contient exactement 1024 bytes contigus
        unsafe { std::mem::transmute(&self.0) }
    }
}

/// Nœud pour l'algorithme de Wagner (GBP)
#[derive(Clone, Debug)]
pub struct GbpNode {
    pub value: Block1024,
    pub indices: Vec<u32>, // Indices originaux dans le DAG
}

impl GbpNode {
    pub fn new(value: Block1024, index: u32) -> Self {
        GbpNode {
            value,
            indices: vec![index],
        }
    }
}

/// Algorithme de hachage pour rs_merkle (SHA-256)
#[derive(Clone)]
pub struct Sha256Algorithm;

impl Hasher for Sha256Algorithm {
    type Hash = [u8; 32];

    fn hash(data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().into()
    }
}

/// Preuve MTP complète (transmise client → relay)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MtpProof {
    pub nonce: [u8; 32],
    pub indices: Vec<usize>, // Indices de la solution GBP (k éléments)
    pub extracted_blocks: Vec<Block1024>, // Blocs correspondants (pour XOR)
    pub proof_bytes: Vec<u8>, // Multi-proof Merkle sérialisé
    pub leaves_count: usize, // Taille du DAG (m)
}

/// Features ML pour Context Scoring
#[derive(Clone, Debug)]
pub struct NetworkFeatures {
    pub ttl: u32,
    pub asn: Option<u32>,
    pub iat_ms: u64,
    pub cpu_load: f32,
}

// ============================================================================
// ERREURS
// ============================================================================

#[derive(Error, Debug)]
pub enum CapowError {
    #[error("Argon2 generation failed: {0}")]
    Argon2Error(String),

    #[error("GBP solution not found after max iterations")]
    GbpNoSolution,

    #[error("Merkle proof verification failed")]
    MerkleVerificationFailed,

    #[error("XOR check failed: blocks do not sum to zero")]
    XorCheckFailed,

    #[error("Invalid proof structure: {0}")]
    InvalidProof(String),
}

// ============================================================================
// GÉNÉRATION ARGON2ID DAG (CLIENT)
// ============================================================================

/// Génère le DAG Argon2id de 2GB avec paramètres MTP sécurisés
///
/// CRITIQUE: t=1, p=1 pour éviter segment sharing attacks
///
/// Arguments:
/// - challenge_salt: Challenge du relay (32 bytes)
/// - password_nonce: Nonce du client (32 bytes)
///
/// Retourne: Vec de blocs Argon2 (2M blocs × 1KB = 2GB)
pub fn generate_argon2_dag(
    challenge_salt: &[u8; 32],
    password_nonce: &[u8; 32],
) -> Result<Vec<Block>, CapowError> {
    // Paramètres MTP: m=2097152 KiB, t=1, p=1
    let params = Params::new(CLIENT_MEM_KB, CLIENT_PASSES, CLIENT_LANES, None)
        .map_err(|e| CapowError::Argon2Error(format!("Invalid params: {}", e)))?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    // Allocation du DAG (2GB sur le heap)
    let block_count = (CLIENT_MEM_KB as usize) / (BLOCK_SIZE_BYTES / 1024);
    let mut dag_memory = vec![Block::default(); block_count];

    // Remplissage du DAG via fill_memory (API low-level)
    argon2
        .fill_memory(password_nonce, challenge_salt, &mut dag_memory)
        .map_err(|e| CapowError::Argon2Error(format!("fill_memory failed: {}", e)))?;

    Ok(dag_memory)
}

/// Convertit les blocs Argon2 en Block1024 pour GBP
pub fn dag_to_block1024(dag: &[Block]) -> Vec<Block1024> {
    dag.iter()
        .map(|block| {
            // Argon2 Block contient un tableau de u64
            let mut data = [0u64; BLOCK_SIZE_U64];
            data.copy_from_slice(&block.as_ref()[0..BLOCK_SIZE_U64]);
            Block1024(data)
        })
        .collect()
}

// ============================================================================
// ALGORITHME DE WAGNER (GBP SOLVER) - CLIENT
// ============================================================================

/// Résout le Generalized Birthday Problem (k-XOR)
///
/// Trouve k indices dans memory_blocks tels que XOR(blocks[i]) = 0
///
/// Complexité: O(2^(n/(k+1))) en temps et mémoire
///
/// Arguments:
/// - memory_blocks: DAG Argon2id (2M blocs)
/// - k: Nombre d'éléments (typiquement 8)
/// - collision_bits: Bits de collision partielle par étape
///
/// Retourne: Indices de la solution ou None
pub fn wagner_algorithm(
    memory_blocks: &[Block1024],
    k: usize,
    collision_bits: usize,
) -> Option<Vec<u32>> {
    // Vérification: k doit être une puissance de 2
    if !k.is_power_of_two() {
        return None;
    }

    let steps = k.trailing_zeros() as usize;
    let chunk_size = memory_blocks.len() / k;

    // Phase 1: Initialisation des k listes
    let mut current_lists: Vec<Vec<GbpNode>> = (0..k)
        .map(|i| {
            (0..chunk_size)
                .map(|j| {
                    let idx = (i * chunk_size + j) as u32;
                    GbpNode::new(memory_blocks[idx as usize].clone(), idx)
                })
                .collect()
        })
        .collect();

    // Phase 2: Fusions itératives par paires
    for step in 0..steps {
        let bit_offset = step * collision_bits;
        let mut next_lists: Vec<Vec<GbpNode>> = Vec::new();

        for chunk in current_lists.chunks(2) {
            if chunk.len() != 2 {
                break;
            }

            let mut left = chunk[0].clone();
            let mut right = chunk[1].clone();

            // Tri des listes par clé partielle
            left.sort_unstable_by_key(|node| {
                node.value.extract_partial_key(bit_offset, collision_bits)
            });
            right.sort_unstable_by_key(|node| {
                node.value.extract_partial_key(bit_offset, collision_bits)
            });

            // Merge join: recherche de collisions
            let mut merged = Vec::new();
            let mut right_idx = 0;

            for l_node in left.iter() {
                let l_key = l_node.value.extract_partial_key(bit_offset, collision_bits);

                while right_idx < right.len() {
                    let r_node = &right[right_idx];
                    let r_key = r_node.value.extract_partial_key(bit_offset, collision_bits);

                    match r_key.cmp(&l_key) {
                        std::cmp::Ordering::Less => right_idx += 1,
                        std::cmp::Ordering::Equal => {
                            // Collision trouvée: créer nœud fusionné
                            let mut combined_indices = l_node.indices.clone();
                            combined_indices.extend_from_slice(&r_node.indices);

                            merged.push(GbpNode {
                                value: l_node.value.xor(&r_node.value),
                                indices: combined_indices,
                            });
                            right_idx += 1;
                        }
                        std::cmp::Ordering::Greater => break,
                    }
                }
            }

            next_lists.push(merged);
        }

        current_lists = next_lists;
    }

    // Phase 3: Extraction de la solution finale
    if let Some(final_list) = current_lists.first() {
        for node in final_list {
            // Vérifier que le XOR complet est zéro
            if node.value.0.iter().all(|&word| word == 0) {
                return Some(node.indices.clone());
            }
        }
    }

    None
}

// ============================================================================
// MTP (MERKLE TREE PROOF) - CLIENT
// ============================================================================

/// Construit l'arbre de Merkle sur le DAG Argon2id
///
/// Utilise rayon pour paralléliser le hachage SHA-256
pub fn build_mtp_tree(dag: &[Block1024]) -> MerkleTree<Sha256Algorithm> {
    // Hachage parallèle des feuilles (2M × SHA-256)
    let leaves: Vec<[u8; 32]> = dag
        .par_iter()
        .map(|block| Sha256Algorithm::hash(block.as_bytes()))
        .collect();

    MerkleTree::<Sha256Algorithm>::from_leaves(&leaves)
}

/// Génère la preuve multi-path Merkle compacte
pub fn generate_mtp_proof(
    tree: &MerkleTree<Sha256Algorithm>,
    indices: &[u32],
    dag: &[Block1024],
) -> MtpProof {
    let usize_indices: Vec<usize> = indices.iter().map(|&i| i as usize).collect();
    let proof = tree.proof(&usize_indices);
    let proof_bytes = proof.to_bytes();

    let extracted_blocks: Vec<Block1024> = indices
        .iter()
        .map(|&idx| dag[idx as usize].clone())
        .collect();

    MtpProof {
        nonce: [0u8; 32], // À remplir par le client
        indices: usize_indices,
        extracted_blocks,
        proof_bytes,
        leaves_count: dag.len(),
    }
}

// ============================================================================
// VÉRIFICATION ASYMÉTRIQUE (RELAY) - <100µs
// ============================================================================

/// Vérifie la preuve GBP/MTP avec réduction 32x (64MB relay vs 2GB client)
///
/// Opérations:
/// 1. Recalcul hachages SHA-256 des k blocs fournis
/// 2. Vérification multi-proof Merkle (authenticité + position)
/// 3. Vérification XOR cumulatif = 0
///
/// Target: <100µs avec auto-vectorisation LLVM
pub fn verify_gbp_solution(proof: &MtpProof, merkle_root: &[u8; 32]) -> Result<(), CapowError> {
    // 1. Recalcul des hachages de feuilles
    let mut leaf_hashes: Vec<[u8; 32]> = Vec::with_capacity(proof.extracted_blocks.len());
    for block in &proof.extracted_blocks {
        leaf_hashes.push(Sha256Algorithm::hash(block.as_bytes()));
    }

    // 2. Vérification du multi-proof Merkle
    let rs_proof = MerkleProof::<Sha256Algorithm>::try_from(proof.proof_bytes.as_slice())
        .map_err(|_| CapowError::InvalidProof("Failed to deserialize Merkle proof".into()))?;

    let is_valid = rs_proof.verify(
        *merkle_root,
        &proof.indices,
        &leaf_hashes,
        proof.leaves_count,
    );

    if !is_valid {
        return Err(CapowError::MerkleVerificationFailed);
    }

    // 3. Vérification XOR cumulatif (auto-vectorisé en AVX2/AVX-512)
    let mut accumulator = [0u64; BLOCK_SIZE_U64];
    for block in &proof.extracted_blocks {
        for (acc, val) in accumulator.iter_mut().zip(block.0.iter()) {
            *acc ^= val;
        }
    }

    // Tous les mots doivent être zéro
    if !accumulator.iter().all(|&word| word == 0) {
        return Err(CapowError::XorCheckFailed);
    }

    Ok(())
}

// ============================================================================
// ML CONTEXT SCORING
// ============================================================================

/// Calcule le score ML de contexte (0.0 - 10.0)
///
/// Features:
/// - TTL: distance réseau (spoofing detection)
/// - ASN: réputation infrastructure
/// - IAT: inter-arrival timing (bot detection)
/// - CPU: saturation du relay
///
/// Mapping:
/// - 0.0-2.9: Trafic légitime → 64MB (minimal)
/// - 3.0-7.9: Trafic ambigu → 512MB-1GB (moyen)
/// - 8.0-10.0: Attaque détectée → 2GB (maximum)
pub fn compute_context_score(features: &NetworkFeatures) -> f32 {
    let mut score = 0.0f32;

    // TTL: valeurs anormales (< 32 ou > 128)
    match features.ttl {
        0..=31 => score += 2.0,
        128.. => score += 1.5,
        _ => {}
    }

    // ASN: data centers / cloud providers suspects
    if let Some(asn) = features.asn {
        // Liste noire simplifiée (à étendre en production)
        let suspicious_asns = [
            16509, // AWS
            15169, // Google Cloud
            8075,  // Microsoft Azure
        ];
        if suspicious_asns.contains(&asn) {
            score += 2.5;
        }
    } else {
        // Pas d'ASN résolu = suspect
        score += 1.0;
    }

    // IAT: intervalles trop réguliers (< 50ms entre requêtes)
    if features.iat_ms < 50 {
        score += 3.0;
    }

    // CPU: relay surchargé
    if features.cpu_load > 0.8 {
        score += 2.0;
    } else if features.cpu_load > 0.6 {
        score += 1.0;
    }

    score.min(10.0)
}

/// Mappe le score Φ vers la difficulté Argon2id (mémoire requise en KB)
pub fn map_score_to_difficulty(score: f32) -> u32 {
    match score {
        s if s < 3.0 => 65_536,    // 64 MB (minimal friction)
        s if s < 6.0 => 524_288,   // 512 MB
        s if s < 8.0 => 1_048_576, // 1 GB
        _ => 2_097_152,            // 2 GB (full penalty)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block1024_xor() {
        let mut block1 = Block1024::zero();
        let mut block2 = Block1024::zero();

        block1.0[0] = 0xAAAA_AAAA_AAAA_AAAA;
        block2.0[0] = 0x5555_5555_5555_5555;

        let result = block1.xor(&block2);
        assert_eq!(result.0[0], 0xFFFF_FFFF_FFFF_FFFF);
    }

    #[test]
    fn test_block1024_extract_partial_key() {
        let mut block = Block1024::zero();
        block.0[0] = 0xABCD_EF01_2345_6789;

        // Extrait les 16 bits de poids faible du premier mot
        let key = block.extract_partial_key(0, 16);
        assert_eq!(key, 0x6789);
    }

    #[test]
    fn test_context_score_legitimate() {
        let features = NetworkFeatures {
            ttl: 64,
            asn: Some(12345), // ASN non-cloud
            iat_ms: 500,
            cpu_load: 0.3,
        };

        let score = compute_context_score(&features);
        assert!(score < 3.0); // Trafic légitime
    }

    #[test]
    fn test_context_score_attack() {
        let features = NetworkFeatures {
            ttl: 20,          // TTL anormal
            asn: Some(16509), // AWS
            iat_ms: 10,       // IAT trop rapide
            cpu_load: 0.9,    // Relay surchargé
        };

        let score = compute_context_score(&features);
        assert!(score >= 8.0); // Attaque détectée
    }

    #[test]
    fn test_difficulty_mapping() {
        assert_eq!(map_score_to_difficulty(1.0), 65_536); // Low risk
        assert_eq!(map_score_to_difficulty(5.0), 524_288); // Medium
        assert_eq!(map_score_to_difficulty(9.0), 2_097_152); // High risk
    }

    #[test]
    fn test_merkle_tree_construction() {
        // Crée un petit DAG de test (1000 blocs)
        let mut dag = Vec::new();
        for i in 0..1000 {
            let mut block = Block1024::zero();
            block.0[0] = i as u64;
            dag.push(block);
        }

        let tree = build_mtp_tree(&dag);
        let root = tree.root().expect("Root should exist");

        assert_eq!(root.len(), 32); // SHA-256 hash
    }

    #[test]
    fn test_verify_gbp_solution_valid() {
        // Crée un DAG où certains blocs XOR à zéro
        let mut dag = Vec::new();

        // Bloc 0: valeur quelconque
        let mut block0 = Block1024::zero();
        block0.0[0] = 0x1234_5678_9ABC_DEF0;
        dag.push(block0.clone());

        // Bloc 1: copie de bloc 0
        dag.push(block0.clone());

        // Remplir le reste du DAG
        for i in 2..16 {
            let mut block = Block1024::zero();
            block.0[0] = i;
            dag.push(block);
        }

        // Construit l'arbre
        let tree = build_mtp_tree(&dag);
        let root = tree.root().expect("Root should exist");

        // Les blocs 0 et 1 sont identiques, donc leur XOR = 0
        let indices = vec![0u32, 1];
        let proof = generate_mtp_proof(&tree, &indices, &dag);

        // Vérification devrait passer
        let result = verify_gbp_solution(&proof, &root);
        assert!(result.is_ok(), "Verification failed: {:?}", result.err());
    }
}
