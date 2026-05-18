/// Benchmark pour vérification CAPoW asymétrique
///
/// Target: <100µs pour verify_gbp_solution()
///
/// Exécution:
/// ```bash
/// cargo bench --package relay_daemon
/// ```
///
/// Avec optimisations natives:
/// ```bash
/// RUSTFLAGS="-C target-cpu=native" cargo bench --package relay_daemon
/// ```
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use relay_daemon::capow::{build_mtp_tree, generate_mtp_proof, verify_gbp_solution, Block1024};
use std::time::Duration;

/// Crée un DAG de test avec taille variable
fn create_test_dag(size: usize) -> Vec<Block1024> {
    (0..size)
        .map(|i| {
            let mut block = Block1024::zero();
            block.0[0] = i as u64;
            block.0[1] = (i as u64).wrapping_mul(31337);
            block
        })
        .collect()
}

/// Crée une preuve valide (XOR = 0) pour le benchmark
fn create_valid_proof(dag_size: usize, k: usize) -> (relay_daemon::capow::MtpProof, [u8; 32]) {
    let dag = create_test_dag(dag_size);
    let tree = build_mtp_tree(&dag);
    let root = tree.root().expect("Root should exist");

    // Sélectionne k indices
    let indices: Vec<u32> = (0..k).map(|i| i as u32).collect();
    let mut proof = generate_mtp_proof(&tree, &indices, &dag);

    // Force XOR = 0 en créant k blocs identiques
    // (simplifie le benchmark, mais valide le path de vérification)
    let zero_block = Block1024::zero();
    proof.extracted_blocks = vec![zero_block; k];

    (proof, root)
}

fn bench_verify_small_dag(c: &mut Criterion) {
    let mut group = c.benchmark_group("CAPoW_Verify_SmallDAG");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(1000);

    // Benchmark avec DAG de 1K blocs (1MB)
    let (proof, root) = create_valid_proof(1024, 8);

    group.bench_function("verify_1KB_blocks_k8", |b| {
        b.iter(|| {
            let result = verify_gbp_solution(black_box(&proof), black_box(&root));
            black_box(result)
        });
    });

    group.finish();
}

fn bench_verify_medium_dag(c: &mut Criterion) {
    let mut group = c.benchmark_group("CAPoW_Verify_MediumDAG");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(500);

    // Benchmark avec DAG de 64K blocs (64MB - taille relay)
    let (proof, root) = create_valid_proof(65536, 8);

    group.bench_function("verify_64MB_DAG_k8", |b| {
        b.iter(|| {
            let result = verify_gbp_solution(black_box(&proof), black_box(&root));
            black_box(result)
        });
    });

    group.finish();
}

fn bench_verify_variable_k(c: &mut Criterion) {
    let mut group = c.benchmark_group("CAPoW_Verify_VariableK");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(500);

    let dag_size = 4096; // 4MB DAG

    for k in [4, 8, 16].iter() {
        let (proof, root) = create_valid_proof(dag_size, *k);

        group.bench_with_input(BenchmarkId::from_parameter(k), k, |b, _| {
            b.iter(|| {
                let result = verify_gbp_solution(black_box(&proof), black_box(&root));
                black_box(result)
            });
        });
    }

    group.finish();
}

fn bench_merkle_tree_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("Merkle_Tree_Build");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(100);

    // Benchmark construction d'arbre (opération client-side)
    for size in [1024, 8192, 65536].iter() {
        let dag = create_test_dag(*size);

        group.bench_with_input(BenchmarkId::new("build_tree", size), size, |b, _| {
            b.iter(|| {
                let tree = build_mtp_tree(black_box(&dag));
                black_box(tree)
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_verify_small_dag,
    bench_verify_medium_dag,
    bench_verify_variable_k,
    bench_merkle_tree_construction,
);
criterion_main!(benches);
