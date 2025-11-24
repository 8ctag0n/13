/// Example: Wallet Integration Mode
///
/// This example demonstrates how to use the SDK with wallet integration.
/// Instructions are built WITHOUT signing, allowing wallets (Phantom, Solflare, etc.)
/// to handle the signing process.
use cypherlink_sdk::MarketplaceClient;
use cypherlink_types::{CircuitType, FheOperation};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

fn main() -> anyhow::Result<()> {
    println!("=== CypherLink SDK - Wallet Mode Example ===\n");

    // Initialize client
    let rpc_url = "https://api.devnet.solana.com".to_string();
    let program_id = Pubkey::from_str("bn2XNLkXi23NPMjH1qNdGWg1tuUFtpVkQvqxTD9v3Ys")?;
    let client = MarketplaceClient::new(rpc_url, program_id);

    // Simulated wallet pubkey (in real app, this comes from wallet-adapter)
    let wallet_pubkey = Pubkey::new_unique();

    println!("📋 Example 1: Create ZK Job (Wallet Mode)");
    println!("{}", "=".repeat(60));

    // Build instruction WITHOUT signing
    let witness_commitment = [42u8; 32];
    let create_job_ix = client.create_zk_job_ix(
        wallet_pubkey,
        CircuitType::ZcashOrchard,
        witness_commitment,
        1024,
        1_000_000_000, // 0.001 SOL
    )?;

    println!("✅ Instruction built (unsigned):");
    println!("   Program ID: {}", create_job_ix.program_id);
    println!("   Accounts: {} accounts", create_job_ix.accounts.len());
    println!("   Data: {} bytes", create_job_ix.data.len());
    println!("   ⚠️  Wallet will sign this transaction\n");

    // Build complete transaction (still unsigned)
    let transaction = client.build_transaction(vec![create_job_ix], wallet_pubkey)?;

    println!("✅ Transaction ready for wallet:");
    println!("   Fee payer: {}", transaction.message.account_keys[0]);
    println!(
        "   Instructions: {}",
        transaction.message.instructions.len()
    );
    println!("   Signers required: {}", transaction.signatures.len());
    println!("   Status: UNSIGNED - ready for wallet\n");

    println!("💡 In a real wallet integration:");
    println!("   wallet.sendTransaction(transaction)");
    println!("   ↓");
    println!("   Wallet shows preview → User approves → Transaction signed → Sent\n");

    println!("📋 Example 2: Create FHE Job with Multi-Prover Consensus");
    println!("{}", "=".repeat(60));

    // FHE job with 3 provers, 2-of-3 consensus
    let encrypted_input_commitment = [99u8; 32];
    let fhe_job_ix = client.create_fhe_job_ix(
        wallet_pubkey,
        FheOperation::Add(10),
        encrypted_input_commitment,
        2048,
        3, // 3 provers
        2, // 2-of-3 consensus
    )?;

    println!("✅ FHE Job instruction built:");
    println!("   Operation: Add(10)");
    println!("   Required provers: 3");
    println!("   Consensus threshold: 2");
    println!("   Auto-calculated price: 0.003 SOL (3 provers × 0.001)\n");

    println!("📋 Example 3: Transaction Simulation (Before Signing)");
    println!("{}", "=".repeat(60));

    // Simulate to estimate compute units and check for errors
    match client.simulate_transaction(vec![fhe_job_ix.clone()], wallet_pubkey) {
        Ok((compute_units, logs, error)) => {
            println!("✅ Simulation successful:");
            if let Some(units) = compute_units {
                println!("   Compute units: {}", units);
            }
            if let Some(err) = error {
                println!("   ⚠️  Would fail: {}", err);
            } else {
                println!("   Status: Transaction would succeed ✓");
            }
            if let Some(logs) = logs {
                println!("   Logs: {} lines", logs.len());
            }
        }
        Err(e) => {
            println!("❌ Simulation failed: {}", e);
        }
    }

    println!("\n📋 Example 4: Using Low-Level Instruction Builder");
    println!("{}", "=".repeat(60));

    // Direct access to instruction builder for advanced use cases
    let builder = client.instructions();

    // Get next job ID manually
    let job_id = client.fetch_next_job_id()?;
    println!("✅ Next job ID from chain: {}", job_id);

    // Build instruction with full control
    let custom_ix = builder.create_job(
        wallet_pubkey,
        job_id,
        CircuitType::ZcashOrchard,
        [1u8; 32],
        512,
        500_000_000,
        300, // 5min timeout
        None,
    )?;

    println!("✅ Custom instruction built with full control:");
    println!("   Job ID: {}", job_id);
    println!("   Timeout: 300s (5min)");
    println!("   Price: 0.0005 SOL\n");

    println!("📋 Example 5: Register Prover (Wallet Mode)");
    println!("{}", "=".repeat(60));

    let register_ix = client.register_prover_ix(
        wallet_pubkey,
        5_000_000_000, // 5 SOL stake
        [77u8; 32],    // Encryption key
    )?;

    println!("✅ Register prover instruction:");
    println!("   Stake: 5 SOL");
    println!("   Encryption key: [77, 77, ...]");
    println!("   Wallet will sign and pay rent + stake\n");

    println!("🎯 Summary: Wallet Mode Benefits");
    println!("{}", "=".repeat(60));
    println!("✅ No keypair management in frontend");
    println!("✅ Wallet handles all signing");
    println!("✅ User sees tx preview before approval");
    println!("✅ Works with all Solana wallets (Phantom, Solflare, Backpack)");
    println!("✅ Simulation before signing prevents errors");
    println!("✅ Full control over transaction composition\n");

    Ok(())
}
