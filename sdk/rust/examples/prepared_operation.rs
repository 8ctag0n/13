//! Example: PreparedOperation - Inspect and simulate before executing
//!
//! This example shows how to use the Layer 2 API to:
//! 1. Prepare an operation without executing it
//! 2. Inspect costs and PDAs
//! 3. Simulate to verify success
//! 4. Execute when ready

use anyhow::Result;
use zyberlink_sdk::Zyber;

#[tokio::main]
async fn main() -> Result<()> {
    // Connect to network
    let mut zyber = Zyber::builder()
        .network("localnet")
        .timeout(std::time::Duration::from_secs(60))
        .provers(2)
        .price_sol(0.01)
        .build()
        .await?;

    println!("Connected to network: {}", zyber.network());
    println!("Wallet: {}", zyber.pubkey());

    // Sample values to sum
    let values = vec![10u8, 20, 30, 40, 50];

    // =========================================================================
    // Layer 2: Prepare operation
    // =========================================================================
    println!("\n--- Preparing sum operation ---");
    let mut prepared = zyber.prepare_sum(&values).await?;

    // Inspect the operation
    println!("Operation: {:?}", prepared.operation());
    println!("Job ID: {}", prepared.job_id());
    println!("Witness size: {} bytes", prepared.witness_size());
    println!("Required provers: {}", prepared.required_provers());
    println!("Timeout: {:?}", prepared.timeout());

    // Get PDAs
    let pdas = prepared.pdas();
    println!("\n--- PDAs ---");
    println!("Job PDA: {}", pdas.job);
    println!("Escrow PDA: {}", pdas.escrow);
    println!("FHE Consensus PDA: {}", pdas.fhe_consensus);
    println!("Config PDA: {}", pdas.config);

    // Estimate costs
    let cost = prepared.estimate_cost();
    println!("\n--- Cost Estimate ---");
    println!("Job creation: {} lamports", cost.job_creation_lamports);
    println!("Prover payment: {} lamports", cost.prover_payment_lamports);
    println!("TX fee: {} lamports", cost.tx_fee_lamports);
    println!("Total: {} lamports ({:.6} SOL)", cost.total_lamports, cost.total_sol());

    // =========================================================================
    // Simulation (dry-run)
    // =========================================================================
    println!("\n--- Simulating transaction ---");
    match prepared.simulate().await {
        Ok(sim) => {
            if sim.success {
                println!("Simulation SUCCESS");
                println!("Compute units: {}", sim.compute_units);
            } else {
                println!("Simulation FAILED: {:?}", sim.error);
            }
            if !sim.logs.is_empty() {
                println!("Logs:");
                for log in sim.logs.iter().take(5) {
                    println!("  {}", log);
                }
            }
        }
        Err(e) => {
            println!("Simulation error: {}", e);
            println!("(This is expected if the network is not running)");
        }
    }

    // =========================================================================
    // Get instruction for composition
    // =========================================================================
    println!("\n--- Instruction for composition ---");
    let ix = prepared.instruction()?;
    println!("Program ID: {}", ix.program_id);
    println!("Accounts: {} total", ix.accounts.len());
    println!("Data size: {} bytes", ix.data.len());

    // =========================================================================
    // Execute (optional - commented out for safety)
    // =========================================================================
    // Uncomment to actually execute:
    //
    // println!("\n--- Executing ---");
    // let result = prepared.execute().await?;
    // println!("Result: {}", result);

    println!("\n(Skipping execution - uncomment to run on live network)");

    Ok(())
}
