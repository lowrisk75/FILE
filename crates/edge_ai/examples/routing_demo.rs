//! # AORATA Edge AI Routing Demo
//!
//! Demonstrates multi-tier backend detection and LoRA-based adaptive routing.

use edge_ai::backend::{backend_available, detect_backend};
use edge_ai::routing_agent::{AneRoutingAgent, NetworkContext};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== AORATA Edge AI — Multi-Tier Routing Demo ===\n");

    // Step 1: Backend detection
    let backend_type = detect_backend();
    println!("🔍 Auto-detected backend: {:?}", backend_type);
    println!("   • Name: {}", backend_type.name());
    println!("   • Tier: {}", backend_type.tier());

    let (min, max) = backend_type.latency_range();
    println!("   • Expected latency: {}-{}µs\n", min, max);

    // Step 2: Initialize agent
    println!("📦 Initializing routing agent...");
    let mut agent = AneRoutingAgent::auto().await?;
    println!(
        "✅ Agent ready: {} (tier {})\n",
        agent.backend_name(),
        agent.backend_tier()
    );

    // Step 3: Simulate network contexts
    let scenarios = vec![
        (
            "Low latency, clean network",
            NetworkContext {
                avg_ttl: 64,
                asn: 15169, // Google AS
                iat_variance: 100,
                cpu_load: 10,
                num_paths: 3,
            },
        ),
        (
            "High latency, congested",
            NetworkContext {
                avg_ttl: 48, // More hops
                asn: 3356,   // Level3/Lumen (historically congested)
                iat_variance: 5000,
                cpu_load: 75,
                num_paths: 5,
            },
        ),
        (
            "CGNAT deep",
            NetworkContext {
                avg_ttl: 32, // Many NAT layers
                asn: 12345,
                iat_variance: 2000,
                cpu_load: 40,
                num_paths: 2,
            },
        ),
    ];

    println!("🧪 TESTING ADAPTIVE ROUTING\n");

    for (desc, context) in scenarios {
        println!("Scenario: {}", desc);
        println!("   • TTL: {}", context.avg_ttl);
        println!("   • ASN: {}", context.asn);
        println!("   • IAT variance: {}µs", context.iat_variance);
        println!("   • CPU load: {}%", context.cpu_load);
        println!("   • Paths: {}", context.num_paths);

        // Update LoRA weights based on context
        let latency = agent.update_from_context(&context).await?;

        println!("   ✓ LoRA update: {:?}", latency);

        // Validate latency is within expected range
        if latency.as_micros() >= min as u128 && latency.as_micros() <= (max * 2) as u128 {
            println!("   ✅ Latency within expected range\n");
        } else {
            println!(
                "   ⚠️  Latency outside expected range ({}µs)\n",
                latency.as_micros()
            );
        }
    }

    // Step 4: Backend availability summary
    println!("=== Backend Availability Summary ===\n");

    for backend in [
        edge_ai::backend::BackendType::CoreML,
        edge_ai::backend::BackendType::AneHelper,
        edge_ai::backend::BackendType::Nnapi,
        edge_ai::backend::BackendType::HexagonDsp,
    ] {
        let available = backend_available(backend);
        let status = if available {
            "✅ Available"
        } else {
            "❌ Not available"
        };
        println!(
            "{:20} {} (tier {}) — {}",
            backend.name(),
            status,
            backend.tier(),
            if available {
                format!(
                    "{}-{}µs",
                    backend.latency_range().0,
                    backend.latency_range().1
                )
            } else {
                "N/A".to_string()
            }
        );
    }

    println!("\n✨ Demo complete!");

    Ok(())
}
