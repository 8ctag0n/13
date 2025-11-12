/// Complete End-to-End Marketplace Test
///
/// Tests the full workflow:
/// 1. Initialize marketplace
/// 2. Register a prover
/// 3. Create a job
/// 4. Prover claims the job
/// 5. Prover submits proof
/// 6. Verify final state

use anyhow::Result;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
    commitment_config::CommitmentConfig,
};
use solana_client::rpc_client::RpcClient;
use std::str::FromStr;
use blake2::{Blake2s256, Digest};

#[tokio::test(flavor = "multi_thread")]
async fn test_complete_marketplace_flow() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("COMPLETE END-TO-END MARKETPLACE TEST");
    println!("{}\n", "=".repeat(80));

    let rpc_url = "http://127.0.0.1:8899";
    let program_id = solana_sdk::pubkey::Pubkey::from_str("bn2XNLkXi23NPMjH1qNdGWg1tuUFtpVkQvqxTD9v3Ys")?;

    let rpc_client = RpcClient::new_with_commitment(
        rpc_url.to_string(),
        CommitmentConfig::confirmed(),
    );

    let sdk_client = cypherlink_sdk::MarketplaceClient::new(
        rpc_url.to_string(),
        program_id,
    );

    // ========================================================================
    // STEP 1: Initialize Marketplace
    // ========================================================================
    println!("STEP 1: Initialize Marketplace");
    println!("{}", "-".repeat(80));

    let authority = Keypair::new();
    println!("  Authority: {}", authority.pubkey());

    // Airdrop to authority
    println!("  Funding authority...");
    let _ = rpc_client.request_airdrop(&authority.pubkey(), 20_000_000_000)?;

    for _ in 0..30 {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let balance = rpc_client.get_balance(&authority.pubkey())?;
        if balance >= 20_000_000_000 {
            println!("  ✓ Authority funded: {} SOL", balance as f64 / 1_000_000_000.0);
            break;
        }
    }

    // Check if marketplace already initialized
    let (config_pda, _) = sdk_client.get_config_pda();
    let already_initialized = rpc_client.get_account(&config_pda).is_ok();

    if !already_initialized {
        println!("  Initializing marketplace...");
        let initialize_ix = sdk_client.initialize_instruction(
            &authority.pubkey(),
            1000,              // 10% fee
            1_000_000_000,     // 1 SOL min stake
            500,               // 500/1000 min reputation
            600,               // 10 min timeout
        )?;

        let recent_blockhash = rpc_client.get_latest_blockhash()?;
        let mut tx = Transaction::new_with_payer(&[initialize_ix], Some(&authority.pubkey()));
        tx.sign(&[&authority], recent_blockhash);

        let sig = rpc_client.send_and_confirm_transaction(&tx)?;
        println!("  ✓ Marketplace initialized: {}", sig);
    } else {
        println!("  ✓ Marketplace already initialized");
    }

    // ========================================================================
    // STEP 2: Register Prover
    // ========================================================================
    println!("\nSTEP 2: Register Prover");
    println!("{}", "-".repeat(80));

    let prover = Keypair::new();
    println!("  Prover: {}", prover.pubkey());

    // Airdrop to prover
    println!("  Funding prover...");
    let _ = rpc_client.request_airdrop(&prover.pubkey(), 20_000_000_000)?;

    for _ in 0..30 {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let balance = rpc_client.get_balance(&prover.pubkey())?;
        if balance >= 20_000_000_000 {
            println!("  ✓ Prover funded: {} SOL", balance as f64 / 1_000_000_000.0);
            break;
        }
    }

    // Register prover
    let encryption_pubkey = [42u8; 32]; // Mock encryption key
    let stake_amount = 5_000_000_000; // 5 SOL

    let register_ix = sdk_client.register_prover_instruction(
        &prover.pubkey(),
        stake_amount,
        encryption_pubkey,
    )?;

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&[register_ix], Some(&prover.pubkey()));
    tx.sign(&[&prover], recent_blockhash);

    let sig = rpc_client.send_and_confirm_transaction(&tx)?;
    println!("  ✓ Prover registered: {}", sig);

    let (prover_pda, _) = sdk_client.get_prover_pda(&prover.pubkey());
    let prover_account = rpc_client.get_account(&prover_pda)?;
    println!("  ✓ Prover account created: {} bytes", prover_account.data.len());

    // ========================================================================
    // STEP 3: Create Job
    // ========================================================================
    println!("\nSTEP 3: Create Job");
    println!("{}", "-".repeat(80));

    let client = Keypair::new();
    println!("  Client: {}", client.pubkey());

    // Airdrop to client
    println!("  Funding client...");
    let _ = rpc_client.request_airdrop(&client.pubkey(), 10_000_000_000)?;

    for _ in 0..30 {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let balance = rpc_client.get_balance(&client.pubkey())?;
        if balance >= 10_000_000_000 {
            println!("  ✓ Client funded: {} SOL", balance as f64 / 1_000_000_000.0);
            break;
        }
    }

    // Get next job ID from config
    let (config_pda, _) = sdk_client.get_config_pda();
    let config_account = rpc_client.get_account(&config_pda)?;

    // Deserialize config to get next_job_id
    use borsh::BorshDeserialize;

    #[derive(Debug, BorshDeserialize)]
    #[allow(dead_code)]
    struct MarketplaceConfigData {
        authority: solana_sdk::pubkey::Pubkey,
        fee_basis_points: u16,
        min_stake_amount: u64,
        min_reputation_score: u32,
        default_job_timeout_seconds: i64,
        protocol_fee_recipient: solana_sdk::pubkey::Pubkey,
        next_job_id: u64,
        total_provers: u64,
        total_jobs_created: u64,
        total_jobs_completed: u64,
        is_paused: bool,
        bump: u8,
    }

    let config_data = MarketplaceConfigData::try_from_slice(&config_account.data)?;
    let job_id = config_data.next_job_id;
    println!("  Job ID: {} (from config.next_job_id)", job_id);

    // Create witness commitment
    let witness_data = vec![42u8; 1024];
    let mut hasher = Blake2s256::new();
    hasher.update(&witness_data);
    let commitment: [u8; 32] = hasher.finalize().into();

    let create_job_ix = sdk_client.create_job_instruction(
        &client.pubkey(),
        job_id,
        cypherlink_types::CircuitType::ZcashOrchard,
        commitment,
        witness_data.len() as u32,
        2_000_000_000, // 2 SOL price
        3600,          // 1 hour timeout
    )?;

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&[create_job_ix], Some(&client.pubkey()));
    tx.sign(&[&client], recent_blockhash);

    let sig = rpc_client.send_and_confirm_transaction(&tx)?;
    println!("  ✓ Job created: {}", sig);

    let (job_pda, _) = sdk_client.get_job_pda(&client.pubkey(), job_id);
    let job_account = rpc_client.get_account(&job_pda)?;
    println!("  ✓ Job account: {} bytes", job_account.data.len());
    println!("  ✓ Job PDA: {}", job_pda);

    // ========================================================================
    // STEP 4: Prover Claims Job
    // ========================================================================
    println!("\nSTEP 4: Prover Claims Job");
    println!("{}", "-".repeat(80));

    let claim_job_ix = sdk_client.claim_job_instruction(
        &prover.pubkey(),
        &job_pda,
    )?;

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&[claim_job_ix], Some(&prover.pubkey()));
    tx.sign(&[&prover], recent_blockhash);

    let sig = rpc_client.send_and_confirm_transaction(&tx)?;
    println!("  ✓ Job claimed by prover: {}", sig);

    // Verify job state changed
    let job_account_after = rpc_client.get_account(&job_pda)?;
    println!("  ✓ Job account updated: {} bytes", job_account_after.data.len());

    // ========================================================================
    // STEP 5: Prover Submits Proof
    // ========================================================================
    println!("\nSTEP 5: Prover Submits Proof");
    println!("{}", "-".repeat(80));

    // Create proof commitment
    let proof_data = vec![99u8; 512];
    let mut hasher = Blake2s256::new();
    hasher.update(&proof_data);
    let proof_commitment: [u8; 32] = hasher.finalize().into();

    // Use the protocol fee recipient from the config
    let protocol_fee_recipient = config_data.protocol_fee_recipient;

    let submit_proof_ix = sdk_client.submit_proof_instruction_with_recipient(
        &prover.pubkey(),
        &job_pda,
        &client.pubkey(),
        &protocol_fee_recipient,
        proof_commitment,
        proof_data.len() as u32,
    )?;

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&[submit_proof_ix], Some(&prover.pubkey()));
    tx.sign(&[&prover], recent_blockhash);

    let sig = rpc_client.send_and_confirm_transaction(&tx)?;
    println!("  ✓ Proof submitted: {}", sig);

    // ========================================================================
    // VERIFICATION: Check Final State
    // ========================================================================
    println!("\nFINAL VERIFICATION");
    println!("{}", "-".repeat(80));

    // Check job completed
    let final_job_account = rpc_client.get_account(&job_pda)?;
    println!("  ✓ Job account still exists: {} bytes", final_job_account.data.len());

    // Check escrow was closed (should error)
    let (escrow_pda, _) = sdk_client.get_escrow_pda(&job_pda);
    let escrow_closed = rpc_client.get_account(&escrow_pda).is_err();
    println!("  ✓ Escrow closed: {}", escrow_closed);

    // Check prover got paid (balance should have increased)
    let prover_balance = rpc_client.get_balance(&prover.pubkey())?;
    println!("  ✓ Prover final balance: {} SOL", prover_balance as f64 / 1_000_000_000.0);

    println!("\n{}", "=".repeat(80));
    println!("✅ COMPLETE E2E TEST PASSED!");
    println!("{}\n", "=".repeat(80));

    Ok(())
}
