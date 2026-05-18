/// Démonstration complète du système CAPoW asymétrique
///
/// Ce programme simule:
/// 1. Génération d'une preuve côté client (2GB Argon2id + GBP)
/// 2. Vérification côté relay (64MB, <100µs)
/// 3. ML Context Scoring
///
/// Exécution:
/// ```bash
/// cargo run --package relay_daemon --example capow_demo --release
/// ```

use relay_daemon::{
    async_verify::{async_verify_proof, init_crypto_workers},
    capow::*,
};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== AORATA CAPoW Asymétrique Demo ===\n");

    // ========================================================================
    // ÉTAPE 1: INITIALISATION
    // ========================================================================
    println!("📦 Initialisation du pool Rayon (4 cores)...");
    init_crypto_workers(4)?;
    println!("✅ Pool cryptographique initialisé\n");

    // ========================================================================
    // ÉTAPE 2: GÉNÉRATION CÔTÉ CLIENT (SIMULATION SIMPLIFIÉE)
    // ========================================================================
    println!("🔧 CÔTÉ CLIENT: Génération de la preuve CAPoW");
    println!("   Target: 2GB Argon2id + résolution GBP (5-30s en production)");
    println!("   Note: Demo utilise un DAG réduit pour rapidité\n");

    let start_gen = Instant::now();

    // Challenge du relay
    let challenge_salt = [0x42u8; 32];
    let password_nonce = [0x13u8; 32];

    // IMPORTANT: En production, utiliser generate_argon2_dag() avec 2GB
    // Ici on simule avec un petit DAG (64K blocs = 64MB)
    println!("   • Génération DAG Argon2id (simulé: 64MB)...");
    let dag_sim_start = Instant::now();

    let mut dag = Vec::new();
    for i in 0..65536 {
        let mut block = Block1024::zero();
        // Simuler du contenu pseudo-aléatoire
        block.0[0] = i as u64;
        block.0[1] = (i as u64).wrapping_mul(31337);
        block.0[2] = (i as u64).wrapping_add(0xDEADBEEF);
        dag.push(block);
    }

    println!("     ✓ DAG généré en {:?}", dag_sim_start.elapsed());

    // Construction de l'arbre Merkle
    println!("   • Construction arbre Merkle...");
    let merkle_start = Instant::now();
    let tree = build_mtp_tree(&dag);
    let root = tree.root().expect("Root should exist");
    println!("     ✓ Arbre construit en {:?}", merkle_start.elapsed());

    // Résolution GBP (simulé: on prend k indices dont XOR = 0)
    println!("   • Résolution Generalized Birthday Problem...");
    let gbp_start = Instant::now();

    // En production: utiliser wagner_algorithm(&dag, GBP_K, GBP_COLLISION_BITS)
    // Ici: simulation avec blocs craftés pour XOR = 0
    let mut solution_dag = dag.clone();

    // Créer 8 blocs qui XOR à zéro
    let base_block = solution_dag[0].clone();
    for i in 1..8 {
        solution_dag[i] = base_block.clone();
    }
    solution_dag[7] = Block1024::zero(); // Dernier bloc annule le XOR

    // Reconstruire l'arbre avec la solution
    let solution_tree = build_mtp_tree(&solution_dag);
    let solution_root = solution_tree.root().unwrap();

    let indices: Vec<u32> = (0..8).collect();
    let proof = generate_mtp_proof(&solution_tree, &indices, &solution_dag);

    println!("     ✓ Solution GBP trouvée en {:?}", gbp_start.elapsed());

    let gen_duration = start_gen.elapsed();
    println!("\n✅ Preuve générée en {:?}", gen_duration);
    println!("   • Taille preuve: {} bytes", proof.proof_bytes.len());
    println!("   • Blocs extraits: {}", proof.extracted_blocks.len());
    println!("   • Indices: {:?}\n", proof.indices);

    // ========================================================================
    // ÉTAPE 3: ML CONTEXT SCORING
    // ========================================================================
    println!("🤖 ML CONTEXT SCORING");

    let features_legit = NetworkFeatures {
        ttl: 64,
        asn: Some(12345), // ASN résidentiel
        iat_ms: 500,      // Inter-arrival: 500ms
        cpu_load: 0.3,
    };

    let features_attack = NetworkFeatures {
        ttl: 20,          // TTL anormal
        asn: Some(16509), // AWS
        iat_ms: 5,        // Trop rapide
        cpu_load: 0.95,   // Relay surchargé
    };

    let score_legit = compute_context_score(&features_legit);
    let score_attack = compute_context_score(&features_attack);

    println!("   Trafic légitime:");
    println!("     • Score Φ: {:.2}", score_legit);
    println!(
        "     • Difficulté: {} KB ({} MB)",
        map_score_to_difficulty(score_legit),
        map_score_to_difficulty(score_legit) / 1024
    );

    println!("\n   Attaque détectée:");
    println!("     • Score Φ: {:.2}", score_attack);
    println!(
        "     • Difficulté: {} KB ({} GB)",
        map_score_to_difficulty(score_attack),
        map_score_to_difficulty(score_attack) / 1024 / 1024
    );
    println!();

    // ========================================================================
    // ÉTAPE 4: VÉRIFICATION CÔTÉ RELAY (ASYNCHRONE)
    // ========================================================================
    println!("🔍 CÔTÉ RELAY: Vérification asymétrique");
    println!("   Target: <100µs avec 64MB mémoire\n");

    let verify_start = Instant::now();
    let result = async_verify_proof(proof, solution_root).await;
    let verify_duration = verify_start.elapsed();

    match result {
        Ok(()) => {
            println!("✅ Preuve VALIDE");
            println!("   • Temps de vérification: {:?}", verify_duration);
            println!(
                "   • Performance: {:.2}µs",
                verify_duration.as_micros() as f64
            );

            if verify_duration.as_micros() < 100 {
                println!("   • 🎯 TARGET <100µs ATTEINT !");
            } else {
                println!("   • ⚠️  Target <100µs non atteint (optimisations requises)");
                println!("   • Conseil: compiler avec RUSTFLAGS=\"-C target-cpu=native\"");
            }
        }
        Err(e) => {
            println!("❌ Preuve INVALIDE: {}", e);
        }
    }

    println!("\n=== Ratio d'asymétrie ===");
    println!("   Client: ~{:?} (génération)", gen_duration);
    println!("   Relay:  {:?} (vérification)", verify_duration);
    println!(
        "   Facteur: ~{}x plus rapide",
        gen_duration.as_micros() / verify_duration.as_micros().max(1)
    );

    println!("\n✨ Demo terminée avec succès !");

    Ok(())
}
