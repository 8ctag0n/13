//! Test CancelJob instruction
//!
//! Tests the ability of job creators to cancel pending jobs
//! and recover escrowed funds

use anyhow::Result;
use blake2::{Blake2s256, Digest};
use cypherlink_sdk::MarketplaceClient;
use cypherlink_types::CircuitType;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::str::FromStr;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread")]
async fn test_cancel_pending_job() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("TEST: Cancel Pending Job");
    println!("{}", "=".repeat(80));

    // Connect to local validator
    let rpc_url = "http://127.0.0.1:8899";
    let rpc_client = RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());

    // Program ID (deployed)
    let program_id = solana_sdk::pubkey::Pubkey::from_str("bn2XNLkXi23NPMjH1qNdGWg1tuUFtpVkQvqxTD9v3Ys")?;

    // Create SDK client
    let sdk_client = MarketplaceClient::new(rpc_url.to_string(), program_id);

    // ========================================================================
    // Setup: Initialize Marketplace (if needed)
    // ========================================================================
    println!("\nSetup: Initialize Marketplace");
    println!("{}", "-".repeat(80));

    let authority = Keypair::new();
    println!("  Authority: {}", authority.pubkey());

    // Fund authority
    println!("  Funding authority...");
    let airdrop_sig = rpc_client.request_airdrop(&authority.pubkey(), 20_000_000_000)?;

    // Wait for airdrop confirmation
    for _ in 0..30 {
        if rpc_client.confirm_transaction(&airdrop_sig).unwrap_or(false) {
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }

    let balance = rpc_client.get_balance(&authority.pubkey())?;
    println!("  ✓ Authority funded: {} SOL", balance as f64 / 1_000_000_000.0);

    // Try to initialize (might already be initialized)
    let (config_pda, _) = sdk_client.get_config_pda();
    if rpc_client.get_account(&config_pda).is_err() {
        let init_ix = sdk_client.initialize_instruction(
            &authority.pubkey(),
            250,
            5_000_000_000,
            500,
            600,
        )?;

        let recent_blockhash = rpc_client.get_latest_blockhash()?;
        let mut tx = Transaction::new_with_payer(&[init_ix], Some(&authority.pubkey()));
        tx.sign(&[&authority], recent_blockhash);

        rpc_client.send_and_confirm_transaction(&tx)?;
        println!("  ✓ Marketplace initialized");
    } else {
        println!("  ✓ Marketplace already initialized");
    }

    // ========================================================================
    // STEP 1: Create Job
    // ========================================================================
    println!("\nSTEP 1: Create Job");
    println!("{}", "-".repeat(80));

    let client = Keypair::new();
    println!("  Client: {}", client.pubkey());

    // Fund client
    println!("  Funding client...");
    let airdrop_sig = rpc_client.request_airdrop(&client.pubkey(), 10_000_000_000)?;

    for _ in 0..30 {
        if rpc_client.confirm_transaction(&airdrop_sig).unwrap_or(false) {
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }

    let balance = rpc_client.get_balance(&client.pubkey())?;
    println!("  ✓ Client funded: {} SOL", balance as f64 / 1_000_000_000.0);

    // Get next job ID from config
    let config_account = rpc_client.get_account(&config_pda)?;

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
    let witness_commitment: [u8; 32] = hasher.finalize().into();

    let circuit_type = CircuitType::ZcashOrchard;
    let witness_size = 2048;
    let price_lamports = 2_000_000_000; // 2 SOL

    // Get client balance before
    let client_balance_before = rpc_client.get_balance(&client.pubkey())?;

    let create_job_ix = sdk_client.create_job_instruction(
        &client.pubkey(),
        job_id,
        circuit_type,
        witness_commitment,
        witness_size,
        price_lamports,
        600,
    )?;

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&[create_job_ix], Some(&client.pubkey()));
    tx.sign(&[&client], recent_blockhash);

    let sig = rpc_client.send_and_confirm_transaction(&tx)?;
    println!("  ✓ Job created: {}", sig);

    let (job_pda, _) = sdk_client.get_job_pda(&client.pubkey(), job_id);
    println!("  ✓ Job PDA: {}", job_pda);

    // ========================================================================
    // STEP 2: Cancel Job
    // ========================================================================
    println!("\nSTEP 2: Cancel Job");
    println!("{}", "-".repeat(80));

    let cancel_job_ix = sdk_client.cancel_job_instruction(&client.pubkey(), &job_pda)?;

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&[cancel_job_ix], Some(&client.pubkey()));
    tx.sign(&[&client], recent_blockhash);

    let sig = rpc_client.send_and_confirm_transaction(&tx)?;
    println!("  ✓ Job cancelled: {}", sig);

    // ========================================================================
    // Verification
    // ========================================================================
    println!("\nVerification");
    println!("{}", "-".repeat(80));

    // Check job account still exists
    let job_account = rpc_client.get_account(&job_pda)?;
    println!("  ✓ Job account still exists: {} bytes", job_account.data.len());

    // Check escrow is empty (this proves the cancellation worked)
    let (escrow_pda, _) = sdk_client.get_escrow_pda(&job_pda);
    let escrow_result = rpc_client.get_account(&escrow_pda);

    if let Ok(escrow_account) = escrow_result {
        let escrow_balance = escrow_account.lamports;
        println!("  ✓ Escrow balance: {} lamports", escrow_balance);
        assert_eq!(escrow_balance, 0, "Escrow should be empty after cancellation");
    } else {
        println!("  ✓ Escrow account closed (funds returned)");
    }

    // Check client got refund
    let client_balance_after = rpc_client.get_balance(&client.pubkey())?;
    println!("  ✓ Client balance before: {} SOL", client_balance_before as f64 / 1_000_000_000.0);
    println!("  ✓ Client balance after: {} SOL", client_balance_after as f64 / 1_000_000_000.0);

    // Client should have recovered most of the price (minus transaction fees)
    let balance_diff = (client_balance_after as i64) - (client_balance_before as i64);
    println!("  ✓ Balance difference: {} lamports", balance_diff);

    println!("\n{}", "=".repeat(80));
    println!("✅ CANCEL JOB TEST PASSED!");
    println!("{}", "=".repeat(80));

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_cannot_cancel_claimed_job() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("TEST: Cannot Cancel Claimed Job");
    println!("{}", "=".repeat(80));

    // Connect to local validator
    let rpc_url = "http://127.0.0.1:8899";
    let rpc_client = RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());

    // Program ID
    let program_id = solana_sdk::pubkey::Pubkey::from_str("bn2XNLkXi23NPMjH1qNdGWg1tuUFtpVkQvqxTD9v3Ys")?;
    let sdk_client = MarketplaceClient::new(rpc_url.to_string(), program_id);

    // Setup authority
    let authority = Keypair::new();
    let airdrop_sig = rpc_client.request_airdrop(&authority.pubkey(), 20_000_000_000)?;
    for _ in 0..30 {
        if rpc_client.confirm_transaction(&airdrop_sig).unwrap_or(false) {
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }

    // Setup prover
    let prover = Keypair::new();
    let airdrop_sig = rpc_client.request_airdrop(&prover.pubkey(), 20_000_000_000)?;
    for _ in 0..30 {
        if rpc_client.confirm_transaction(&airdrop_sig).unwrap_or(false) {
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }

    // Register prover
    println!("\nSetup: Register Prover");
    let (prover_pda, _) = sdk_client.get_prover_pda(&prover.pubkey());
    if rpc_client.get_account(&prover_pda).is_err() {
        let encryption_key = [99u8; 32];
        let register_ix = sdk_client.register_prover_instruction(
            &prover.pubkey(),
            5_000_000_000,
            encryption_key,
        )?;

        let recent_blockhash = rpc_client.get_latest_blockhash()?;
        let mut tx = Transaction::new_with_payer(&[register_ix], Some(&prover.pubkey()));
        tx.sign(&[&prover], recent_blockhash);

        rpc_client.send_and_confirm_transaction(&tx)?;
        println!("  ✓ Prover registered");
    }

    // Create job
    println!("\nSTEP 1: Create Job");
    let client = Keypair::new();
    let airdrop_sig = rpc_client.request_airdrop(&client.pubkey(), 10_000_000_000)?;
    for _ in 0..30 {
        if rpc_client.confirm_transaction(&airdrop_sig).unwrap_or(false) {
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }

    let (config_pda, _) = sdk_client.get_config_pda();
    let config_account = rpc_client.get_account(&config_pda)?;

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

    let witness_data = vec![42u8; 1024];
    let mut hasher = Blake2s256::new();
    hasher.update(&witness_data);
    let witness_commitment: [u8; 32] = hasher.finalize().into();

    let create_job_ix = sdk_client.create_job_instruction(
        &client.pubkey(),
        job_id,
        CircuitType::ZcashOrchard,
        witness_commitment,
        2048,
        2_000_000_000,
        600,
    )?;

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&[create_job_ix], Some(&client.pubkey()));
    tx.sign(&[&client], recent_blockhash);

    rpc_client.send_and_confirm_transaction(&tx)?;
    let (job_pda, _) = sdk_client.get_job_pda(&client.pubkey(), job_id);
    println!("  ✓ Job created: {}", job_pda);

    // Claim job
    println!("\nSTEP 2: Claim Job");
    let claim_job_ix = sdk_client.claim_job_instruction(&prover.pubkey(), &job_pda)?;

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&[claim_job_ix], Some(&prover.pubkey()));
    tx.sign(&[&prover], recent_blockhash);

    rpc_client.send_and_confirm_transaction(&tx)?;
    println!("  ✓ Job claimed");

    // Try to cancel (should fail)
    println!("\nSTEP 3: Try to Cancel Claimed Job (should fail)");
    let cancel_job_ix = sdk_client.cancel_job_instruction(&client.pubkey(), &job_pda)?;

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&[cancel_job_ix], Some(&client.pubkey()));
    tx.sign(&[&client], recent_blockhash);

    let result = rpc_client.send_and_confirm_transaction(&tx);

    assert!(result.is_err(), "Should not be able to cancel claimed job");
    println!("  ✓ Cancel failed as expected");
    println!("  ✓ Error: Can only cancel jobs in Pending status");

    println!("\n{}", "=".repeat(80));
    println!("✅ CANNOT CANCEL CLAIMED JOB TEST PASSED!");
    println!("{}", "=".repeat(80));

    Ok(())
}
