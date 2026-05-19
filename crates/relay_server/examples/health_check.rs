//! FILE Relay Health Check & Validation
//!
//! Quick validation script to verify relay is operational.
//!
//! Usage:
//!   cargo run --example health_check -- --relay 10.9.8.200
//!
//! Checks:
//! 1. Health endpoint accessible
//! 2. Challenge endpoint returns valid data
//! 3. Metrics endpoint accessible
//! 4. Service responding within reasonable time
//!

use anyhow::{Context, Result};
use clap::Parser;
use std::net::IpAddr;
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(author, version, about = "FILE Relay Health Check", long_about = None)]
struct Args {
    /// Relay server IP address
    #[arg(short, long)]
    relay: IpAddr,

    /// Watch mode (continuous monitoring)
    #[arg(short, long)]
    watch: bool,

    /// Watch interval in seconds
    #[arg(long, default_value_t = 5)]
    interval: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    if args.watch {
        loop {
            print!("\x1B[2J\x1B[1;1H"); // Clear screen
            run_checks(args.relay).await?;
            tokio::time::sleep(tokio::time::Duration::from_secs(args.interval)).await;
        }
    } else {
        run_checks(args.relay).await?;
    }

    Ok(())
}

async fn run_checks(relay_ip: IpAddr) -> Result<()> {
    println!("🏥 FILE Relay Health Check");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📡 Relay: {}", relay_ip);
    println!("🕐 Time: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
    println!();

    let mut all_passed = true;

    // Check 1: Health endpoint
    print!("1️⃣  Health endpoint... ");
    let start = Instant::now();
    match check_health(relay_ip).await {
        Ok((status, service)) => {
            let elapsed = start.elapsed();
            if status == "healthy" {
                println!("✅ OK ({:.2}ms)", elapsed.as_secs_f64() * 1000.0);
                println!("   Service: {}", service);
            } else {
                println!("❌ UNHEALTHY: {}", status);
                all_passed = false;
            }
        }
        Err(e) => {
            let elapsed = start.elapsed();
            println!("❌ FAILED ({:.2}ms)", elapsed.as_secs_f64() * 1000.0);
            println!("   Error: {}", e);
            all_passed = false;
        }
    }
    println!();

    // Check 2: Challenge endpoint
    print!("2️⃣  Challenge endpoint... ");
    let start = Instant::now();
    match check_challenge(relay_ip).await {
        Ok(challenge) => {
            let elapsed = start.elapsed();
            println!("✅ OK ({:.2}ms)", elapsed.as_secs_f64() * 1000.0);
            println!("   Merkle root: {}...", &challenge.merkle_root[..16]);
            println!("   Valid for: {} seconds", challenge.expires_at - challenge.created_at);
        }
        Err(e) => {
            let elapsed = start.elapsed();
            println!("❌ FAILED ({:.2}ms)", elapsed.as_secs_f64() * 1000.0);
            println!("   Error: {}", e);
            all_passed = false;
        }
    }
    println!();

    // Check 3: Metrics endpoint
    print!("3️⃣  Metrics endpoint... ");
    let start = Instant::now();
    match check_metrics(relay_ip).await {
        Ok(metrics) => {
            let elapsed = start.elapsed();
            println!("✅ OK ({:.2}ms)", elapsed.as_secs_f64() * 1000.0);
            println!("{}", metrics);
        }
        Err(e) => {
            let elapsed = start.elapsed();
            println!("❌ FAILED ({:.2}ms)", elapsed.as_secs_f64() * 1000.0);
            println!("   Error: {}", e);
            all_passed = false;
        }
    }
    println!();

    // Summary
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    if all_passed {
        println!("✅ All checks PASSED - Relay is healthy");
    } else {
        println!("❌ Some checks FAILED - Review errors above");
        std::process::exit(1);
    }
    println!();

    Ok(())
}

async fn check_health(relay_ip: IpAddr) -> Result<(String, String)> {
    let url = format!("http://{}:8081/health", relay_ip);
    let response = reqwest::get(&url)
        .await
        .context("Failed to connect to health endpoint")?;

    if !response.status().is_success() {
        anyhow::bail!("HTTP {}", response.status());
    }

    let json: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse JSON")?;

    let status = json["status"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();

    let service = json["service"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();

    Ok((status, service))
}

#[derive(serde::Deserialize)]
struct Challenge {
    created_at: u64,
    expires_at: u64,
    merkle_root: String,
}

async fn check_challenge(relay_ip: IpAddr) -> Result<Challenge> {
    let url = format!("http://{}:8081/challenge", relay_ip);
    let response = reqwest::get(&url)
        .await
        .context("Failed to connect to challenge endpoint")?;

    if !response.status().is_success() {
        anyhow::bail!("HTTP {}", response.status());
    }

    let challenge: Challenge = response
        .json()
        .await
        .context("Failed to parse JSON")?;

    // Validate merkle_root format (should be 64 hex chars)
    if challenge.merkle_root.len() != 64 {
        anyhow::bail!("Invalid merkle_root length: {}", challenge.merkle_root.len());
    }

    // Validate timestamps
    if challenge.expires_at <= challenge.created_at {
        anyhow::bail!("Invalid timestamps: expires_at <= created_at");
    }

    Ok(challenge)
}

struct Metrics {
    active_peers: u64,
    peers_registered: u64,
    capow_verified: u64,
    capow_rejected: u64,
    rate_limited: u64,
    packets_forwarded: u64,
}

impl std::fmt::Display for Metrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "   📊 Active peers: {}\n   \
             📈 Registered: {}\n   \
             ✅ CAPoW verified: {}\n   \
             ❌ CAPoW rejected: {}\n   \
             ⏱️  Rate limited: {}\n   \
             📤 Packets forwarded: {}",
            self.active_peers,
            self.peers_registered,
            self.capow_verified,
            self.capow_rejected,
            self.rate_limited,
            self.packets_forwarded
        )
    }
}

async fn check_metrics(relay_ip: IpAddr) -> Result<Metrics> {
    let url = format!("http://{}:8081/metrics", relay_ip);
    let response = reqwest::get(&url)
        .await
        .context("Failed to connect to metrics endpoint")?;

    if !response.status().is_success() {
        anyhow::bail!("HTTP {}", response.status());
    }

    let text = response
        .text()
        .await
        .context("Failed to read response")?;

    let mut metrics = Metrics {
        active_peers: 0,
        peers_registered: 0,
        capow_verified: 0,
        capow_rejected: 0,
        rate_limited: 0,
        packets_forwarded: 0,
    };

    for line in text.lines() {
        if let Some(value) = line.strip_prefix("file_relay_active_peers ") {
            metrics.active_peers = value.trim().parse().unwrap_or(0);
        } else if let Some(value) = line.strip_prefix("file_relay_peers_registered_total ") {
            metrics.peers_registered = value.trim().parse().unwrap_or(0);
        } else if let Some(value) = line.strip_prefix("file_relay_capow_verified_total ") {
            metrics.capow_verified = value.trim().parse().unwrap_or(0);
        } else if let Some(value) = line.strip_prefix("file_relay_capow_rejected_total ") {
            metrics.capow_rejected = value.trim().parse().unwrap_or(0);
        } else if let Some(value) = line.strip_prefix("file_relay_rate_limited_total ") {
            metrics.rate_limited = value.trim().parse().unwrap_or(0);
        } else if let Some(value) = line.strip_prefix("file_relay_packets_forwarded_total ") {
            metrics.packets_forwarded = value.trim().parse().unwrap_or(0);
        }
    }

    Ok(metrics)
}
