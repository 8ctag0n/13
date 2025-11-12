//! Edge case tests for marketplace instructions
//!
//! Tests error handling for invalid inputs and edge cases

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
async fn test_register_prover_insufficient_stake() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("TEST: Register Prover with Insufficient Stake");
    println!("{}", "=".repeat(80));

    // Connect to local validator
    let rpc_url = "http://127.0.0.1:8899";
    let rpc_client = RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());

    // Program ID
    let program_id = solana_sdk::pubkey::Pubkey::from_str("bn2XNLkXi23NPMjH1qNdGWg1tuUFtpVkQvqxTD9v3Ys")?;
    let sdk_client = MarketplaceClient::new(rpc_url.to_string(), program_id);

    // Get minimum stake from config
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
    let min_stake = config_data.min_stake_amount;

    println!("  Minimum stake required: {} SOL", min_stake as f64 / 1_000_000_000.0);

    // Create prover with insufficient stake
    let prover = Keypair::new();
    let airdrop_sig = rpc_client.request_airdrop(&prover.pubkey(), 20_000_000_000)?;
    for _ in 0..30 {
        if rpc_client.confirm_transaction(&airdrop_sig).unwrap_or(false) {
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }

    let insufficient_stake = min_stake - 1_000_000_000; // 1 SOL less than minimum
    println!("  Attempting to register with: {} SOL", insufficient_stake as f64 / 1_000_000_000.0);

    let encryption_key = [99u8; 32];

    let register_ix = sdk_client.register_prover_instruction(
        &prover.pubkey(),
        insufficient_stake,
        encryption_key,
    )?;

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&[register_ix], Some(&prover.pubkey()));
    tx.sign(&[&prover], recent_blockhash);

    let result = rpc_client.send_and_confirm_transaction(&tx);

    assert!(result.is_err(), "Should not register with insufficient stake");
    println!("  ✓ Registration failed as expected");
    println!("  ✓ Error: Stake amount below minimum");

    println!("\n{}", "=".repeat(80));
    println!("✅ INSUFFICIENT STAKE TEST PASSED!");
    println!("{}", "=".repeat(80));

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_create_job_zero_price() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("TEST: Create Job with Zero Price");
    println!("{}", "=".repeat(80));

    // Connect to local validator
    let rpc_url = "http://127.0.0.1:8899";
    let rpc_client = RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());

    // Program ID
    let program_id = solana_sdk::pubkey::Pubkey::from_str("bn2XNLkXi23NPMjH1qNdGWg1tuUFtpVkQvqxTD9v3Ys")?;
    let sdk_client = MarketplaceClient::new(rpc_url.to_string(), program_id);

    // Create client
    let client = Keypair::new();
    let airdrop_sig = rpc_client.request_airdrop(&client.pubkey(), 10_000_000_000)?;
    for _ in 0..30 {
        if rpc_client.confirm_transaction(&airdrop_sig).unwrap_or(false) {
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }

    // Get next job ID
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

    println!("  Attempting to create job with 0 price");

    let create_job_ix = sdk_client.create_job_instruction(
        &client.pubkey(),
        job_id,
        CircuitType::ZcashOrchard,
        witness_commitment,
        2048,
        0, // Zero price - should fail
        600,
    )?;

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&[create_job_ix], Some(&client.pubkey()));
    tx.sign(&[&client], recent_blockhash);

    let result = rpc_client.send_and_confirm_transaction(&tx);

    assert!(result.is_err(), "Should not create job with zero price");
    println!("  ✓ Job creation failed as expected");
    println!("  ✓ Error: Price must be greater than zero");

    println!("\n{}", "=".repeat(80));
    println!("✅ ZERO PRICE TEST PASSED!");
    println!("{}", "=".repeat(80));

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_claim_job_twice() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("TEST: Cannot Claim Already Claimed Job");
    println!("{}", "=".repeat(80));

    // Connect to local validator
    let rpc_url = "http://127.0.0.1:8899";
    let rpc_client = RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());

    // Program ID
    let program_id = solana_sdk::pubkey::Pubkey::from_str("bn2XNLkXi23NPMjH1qNdGWg1tuUFtpVkQvqxTD9v3Ys")?;
    let sdk_client = MarketplaceClient::new(rpc_url.to_string(), program_id);

    // Register two provers
    println!("\nSetup: Register Provers");
    let prover1 = Keypair::new();
    let prover2 = Keypair::new();

    for prover in &[&prover1, &prover2] {
        let airdrop_sig = rpc_client.request_airdrop(&prover.pubkey(), 20_000_000_000)?;
        for _ in 0..30 {
            if rpc_client.confirm_transaction(&airdrop_sig).unwrap_or(false) {
                break;
            }
            std::thread::sleep(Duration::from_millis(500));
        }

        let (prover_pda, _) = sdk_client.get_prover_pda(&prover.pubkey());
        if rpc_client.get_account(&prover_pda).is_err() {
            let register_ix = sdk_client.register_prover_instruction(
                &prover.pubkey(),
                5_000_000_000,
                [99u8; 32],
            )?;

            let recent_blockhash = rpc_client.get_latest_blockhash()?;
            let mut tx = Transaction::new_with_payer(&[register_ix], Some(&prover.pubkey()));
            tx.sign(&[prover], recent_blockhash);

            rpc_client.send_and_confirm_transaction(&tx)?;
        }
    }
    println!("  ✓ Provers registered");

    // Create job
    println!("\nSetup: Create Job");
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
    println!("  ✓ Job created");

    // Prover 1 claims job
    println!("\nTest: Prover 1 Claims Job");
    let claim_ix = sdk_client.claim_job_instruction(&prover1.pubkey(), &job_pda)?;

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&[claim_ix], Some(&prover1.pubkey()));
    tx.sign(&[&prover1], recent_blockhash);

    rpc_client.send_and_confirm_transaction(&tx)?;
    println!("  ✓ Prover 1 claimed successfully");

    // Prover 2 tries to claim same job
    println!("\nTest: Prover 2 Tries to Claim Same Job");
    let claim_ix = sdk_client.claim_job_instruction(&prover2.pubkey(), &job_pda)?;

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&[claim_ix], Some(&prover2.pubkey()));
    tx.sign(&[&prover2], recent_blockhash);

    let result = rpc_client.send_and_confirm_transaction(&tx);

    assert!(result.is_err(), "Should not claim already claimed job");
    println!("  ✓ Claim failed as expected");
    println!("  ✓ Error: Job already claimed");

    println!("\n{}", "=".repeat(80));
    println!("✅ CLAIM TWICE TEST PASSED!");
    println!("{}", "=".repeat(80));

    Ok(())
}
