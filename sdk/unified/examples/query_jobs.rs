//! Example: Query pending jobs using the Unified SDK
//!
//! This example shows how to use JobQuery to find pending jobs
//! from both ZK-Generator and FHE-Generator programs.
//!
//! Run with:
//! ```bash
//! cargo run --example query_jobs
//! ```

use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use zyberlink_unified_sdk::query::JobQuery;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    // Program IDs from environment or defaults
    let zk_program_id = std::env::var("ZK_GENERATOR_PROGRAM_ID")
        .map(|s| Pubkey::from_str(&s).expect("Invalid ZK program ID"))
        .unwrap_or_else(|_| {
            // Default ZK Generator program ID
            Pubkey::from_str("Dzvy1pzCBgtMw5Fte2GybpeN2PsLPW8t7zDvLfKpxnSS").unwrap()
        });

    let fhe_program_id = std::env::var("FHE_GENERATOR_PROGRAM_ID")
        .map(|s| Pubkey::from_str(&s).expect("Invalid FHE program ID"))
        .unwrap_or_else(|_| {
            // Default FHE Generator program ID
            Pubkey::from_str("C8PpHFCKZ4F2Szbir2EMS4S4H3mwQqHNWUXK1N21nfAB").unwrap()
        });

    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "http://localhost:8899".to_string());

    println!("=== ZyberLink Unified SDK - Job Query Example ===");
    println!("RPC URL: {}", rpc_url);
    println!("ZK Generator: {}", zk_program_id);
    println!("FHE Generator: {}", fhe_program_id);
    println!();

    // Create JobQuery
    let query = JobQuery::from_url(&rpc_url, zk_program_id, fhe_program_id);

    // Query pending jobs from all programs
    println!("Querying pending jobs...");
    let pending_jobs = query.get_pending_jobs().await?;

    if pending_jobs.is_empty() {
        println!("No pending jobs found.");
    } else {
        println!("Found {} pending jobs:", pending_jobs.len());
        println!();

        for job in &pending_jobs {
            let job_type = if job.is_zk() { "ZK" } else { "FHE" };
            println!(
                "  [{:^3}] Job #{} - {} lamports - circuit {}",
                job_type,
                job.id(),
                job.price_lamports(),
                job.circuit_type()
            );
            println!("         Address: {}", job.address());
            println!("         Creator: {}", job.creator());
            println!();
        }
    }

    // Also query FHE jobs that need provers (for consensus)
    println!("Querying FHE jobs needing provers...");
    let prover = Pubkey::new_unique(); // Dummy prover for testing
    let fhe_jobs = query.get_fhe_jobs_needing_provers(&prover).await?;

    if fhe_jobs.is_empty() {
        println!("No FHE jobs need additional provers.");
    } else {
        println!("Found {} FHE jobs needing provers:", fhe_jobs.len());
        for job in &fhe_jobs {
            println!(
                "  Job #{} - {} lamports - circuit {}",
                job.id(),
                job.price_lamports(),
                job.circuit_type()
            );
        }
    }

    println!();
    println!("Done!");

    Ok(())
}
