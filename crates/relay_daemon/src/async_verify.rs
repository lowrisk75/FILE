/// Intégration Tokio/Rayon pour vérification asynchrone CAPoW
///
/// Pattern: Tokio (I/O) + Rayon (CPU) avec canal oneshot
///
/// Avantages:
/// - Évite thread starvation de Tokio
/// - Latence <1µs du oneshot channel
/// - Isolation parfaite des pools
/// - Target: <100µs de vérification totale
use crate::capow::{verify_gbp_solution, MtpProof};
use rayon::ThreadPoolBuilder;
use std::sync::OnceLock;
use tokio::sync::oneshot;

// ============================================================================
// POOL RAYON GLOBAL
// ============================================================================

/// Pool Rayon global pour calculs CPU-bound
///
/// Initialisé une seule fois au démarrage via init_crypto_workers()
static CRYPTO_POOL: OnceLock<rayon::ThreadPool> = OnceLock::new();

/// Initialise le pool de workers cryptographiques
///
/// Recommandation: Sur un système 16-core, allouer 4 cores à Rayon
/// et laisser 12 cores à Tokio pour I/O
///
/// Exemple:
/// ```no_run
/// # use relay_daemon::async_verify::init_crypto_workers;
/// // Au démarrage de l'application
/// init_crypto_workers(4);
/// ```
pub fn init_crypto_workers(num_cores: usize) -> Result<(), String> {
    let pool = ThreadPoolBuilder::new()
        .num_threads(num_cores)
        .thread_name(|i| format!("crypto-worker-{}", i))
        .build()
        .map_err(|e| format!("Failed to build Rayon pool: {}", e))?;

    CRYPTO_POOL
        .set(pool)
        .map_err(|_| "Crypto pool already initialized".to_string())
}

/// Vérifie une preuve CAPoW de manière asynchrone
///
/// Cette fonction peut être appelée depuis une tâche Tokio.
/// La vérification CPU-bound s'exécute sur le pool Rayon isolé.
///
/// Arguments:
/// - proof: Preuve MTP complète du client
/// - merkle_root: Racine Merkle attendue (32 bytes)
///
/// Retourne: Ok(true) si la preuve est valide, Err sinon
///
/// Performance: Target <100µs (vérification) + <1µs (overhead oneshot)
///
/// Exemple:
/// ```no_run
/// # use relay_daemon::async_verify::async_verify_proof;
/// # use relay_daemon::capow::MtpProof;
/// # async fn example(proof: MtpProof, root: [u8; 32]) {
/// match async_verify_proof(proof, root).await {
///     Ok(()) => println!("Proof valid"),
///     Err(e) => println!("Proof invalid: {}", e),
/// }
/// # }
/// ```
pub async fn async_verify_proof(
    proof: MtpProof,
    merkle_root: [u8; 32],
) -> Result<(), String> {
    // Récupère le pool Rayon (panic si non initialisé)
    let pool = CRYPTO_POOL
        .get()
        .ok_or("Crypto pool not initialized. Call init_crypto_workers() first.")?;

    // Crée le canal oneshot
    let (tx, rx) = oneshot::channel();

    // Délègue au pool Rayon
    pool.spawn(move || {
        // Vérification CPU-bound pure (isolée de Tokio)
        let result = verify_gbp_solution(&proof, &merkle_root);

        // Renvoie le résultat via le canal
        // Si tx.send échoue, la tâche Tokio a été annulée (timeout, etc.)
        let _ = tx.send(result);
    });

    // Attend le résultat de manière non-bloquante
    // La tâche Tokio yield ici et libère le thread
    match rx.await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(format!("Verification failed: {}", e)),
        Err(_) => Err("Crypto worker panicked or was cancelled".to_string()),
    }
}

// ============================================================================
// HELPER: VÉRIFICATION AVEC TIMEOUT
// ============================================================================

/// Vérifie une preuve avec timeout (protection DoS)
///
/// Si la vérification dépasse le timeout, retourne une erreur.
/// Utile pour limiter l'impact des proofs malformées.
///
/// Exemple:
/// ```no_run
/// # use relay_daemon::async_verify::async_verify_proof_with_timeout;
/// # use relay_daemon::capow::MtpProof;
/// # use std::time::Duration;
/// # async fn example(proof: MtpProof, root: [u8; 32]) {
/// let timeout = Duration::from_millis(1); // 1ms max
/// match async_verify_proof_with_timeout(proof, root, timeout).await {
///     Ok(()) => println!("Proof valid within timeout"),
///     Err(e) => println!("Timeout or invalid: {}", e),
/// }
/// # }
/// ```
pub async fn async_verify_proof_with_timeout(
    proof: MtpProof,
    merkle_root: [u8; 32],
    timeout: std::time::Duration,
) -> Result<(), String> {
    match tokio::time::timeout(timeout, async_verify_proof(proof, merkle_root)).await {
        Ok(result) => result,
        Err(_) => Err(format!("Verification timeout after {:?}", timeout)),
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capow::{build_mtp_tree, generate_mtp_proof, Block1024};

    #[tokio::test]
    async fn test_async_verify_initialization() {
        // Initialise le pool (si pas déjà fait)
        let _ = init_crypto_workers(2);

        // Vérifie que le pool est accessible
        assert!(CRYPTO_POOL.get().is_some());
    }

    #[tokio::test]
    async fn test_async_verify_valid_proof() {
        // Initialise le pool
        let _ = init_crypto_workers(2);

        // Crée un DAG où les blocs 0 et 1 sont identiques (XOR = 0)
        let mut dag = Vec::new();
        let mut block = Block1024::zero();
        block.0[0] = 0xDEADBEEF;
        dag.push(block.clone());
        dag.push(block.clone()); // Bloc identique

        for i in 2..16 {
            let mut block = Block1024::zero();
            block.0[0] = i;
            dag.push(block);
        }

        // Construit l'arbre Merkle
        let tree = build_mtp_tree(&dag);
        let root = tree.root().expect("Root should exist");

        // Les blocs 0 et 1 XOR à zéro
        let indices = vec![0u32, 1];
        let proof = generate_mtp_proof(&tree, &indices, &dag);

        // Vérification async devrait passer
        let result = async_verify_proof(proof, root).await;
        assert!(result.is_ok(), "Async verification failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_async_verify_invalid_xor() {
        let _ = init_crypto_workers(2);

        // Crée un DAG de test
        let mut dag = Vec::new();
        for i in 0..16 {
            let mut block = Block1024::zero();
            block.0[0] = i;
            dag.push(block);
        }

        let tree = build_mtp_tree(&dag);
        let root = tree.root().expect("Root should exist");

        // Crée une preuve invalide (XOR != 0)
        let indices = vec![0u32, 1];
        let proof = generate_mtp_proof(&tree, &indices, &dag);

        // Vérification devrait échouer
        let result = async_verify_proof(proof, root).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_async_verify_with_timeout() {
        let _ = init_crypto_workers(2);

        // Crée un DAG avec blocs identiques
        let mut dag = Vec::new();
        let mut block = Block1024::zero();
        block.0[0] = 0xCAFEBABE;
        dag.push(block.clone());
        dag.push(block.clone());

        for i in 2..16 {
            let mut block = Block1024::zero();
            block.0[0] = i;
            dag.push(block);
        }

        let tree = build_mtp_tree(&dag);
        let root = tree.root().expect("Root should exist");

        let indices = vec![0u32, 1];
        let proof = generate_mtp_proof(&tree, &indices, &dag);

        // Timeout généreux (10ms)
        let timeout = std::time::Duration::from_millis(10);
        let result = async_verify_proof_with_timeout(proof, root, timeout).await;

        assert!(result.is_ok(), "Timeout test failed: {:?}", result.err());
    }
}
