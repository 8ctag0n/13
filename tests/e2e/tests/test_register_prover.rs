use anyhow::Result;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
    commitment_config::CommitmentConfig,
};
use solana_client::rpc_client::RpcClient;
use std::str::FromStr;

#[tokio::test(flavor = "multi_thread")]
async fn test_register_prover() -> Result<()> {
    println!("\n=== Testing Register Prover ===\n");

    let rpc_url = "http://127.0.0.1:8899";
    let program_id = solana_sdk::pubkey::Pubkey::from_str("bn2XNLkXi23NPMjH1qNdGWg1tuUFtpVkQvqxTD9v3Ys")?;

    // Create RPC client
    let rpc_client = RpcClient::new_with_commitment(
        rpc_url.to_string(),
        CommitmentConfig::confirmed(),
    );

    // === STEP 1: Initialize Marketplace (if needed) ===
    println!("Step 1: Checking marketplace state...");

    let authority = Keypair::new();
    println!("Authority: {}", authority.pubkey());

    // Airdrop SOL to authority
    println!("Requesting airdrop for authority...");
    let _airdrop_sig = rpc_client.request_airdrop(&authority.pubkey(), 10_000_000_000)?;

    // Wait for airdrop to be finalized
    println!("Waiting for airdrop confirmation...");
    for _ in 0..30 {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let balance = rpc_client.get_balance(&authority.pubkey())?;
        if balance >= 10_000_000_000 {
            println!("Airdrop confirmed. Balance: {} SOL", balance as f64 / 1_000_000_000.0);
            break;
        }
    }

    // Double check balance
    let balance = rpc_client.get_balance(&authority.pubkey())?;
    if balance == 0 {
        anyhow::bail!("Airdrop failed - balance is still 0");
    }

    // Create SDK client
    let sdk_client = zyberlink_sdk::MarketplaceClient::new(
        rpc_url.to_string(),
        program_id,
    );

    // Check if marketplace is already initialized
    let (config_pda, _) = sdk_client.get_config_pda();
    let is_initialized = rpc_client.get_account(&config_pda).is_ok();

    if is_initialized {
        println!("Marketplace already initialized, skipping initialization");
    } else {
        // Build Initialize instruction
        println!("Creating Initialize instruction...");
        let initialize_ix = sdk_client.initialize_instruction(
            &authority.pubkey(),
            1000,              // 10% platform fee
            1_000_000_000,     // 1 SOL minimum stake
            500,               // 500/1000 minimum reputation
            600,               // 10 minutes default timeout
        )?;

        // Get recent blockhash
        let recent_blockhash = rpc_client.get_latest_blockhash()?;

        // Create and sign transaction
        let mut transaction = Transaction::new_with_payer(
            &[initialize_ix],
            Some(&authority.pubkey()),
        );
        transaction.sign(&[&authority], recent_blockhash);

        // Send transaction
        println!("Sending Initialize transaction...");
        let signature = rpc_client.send_and_confirm_transaction(&transaction)?;
        println!("Initialize succeeded: {}", signature);
    }

    // === STEP 2: Register Prover ===
    println!("\nStep 2: Registering prover...");

    // Generate prover keypair
    let prover_authority = Keypair::new();
    println!("Prover Authority: {}", prover_authority.pubkey());

    // Airdrop SOL to prover for stake + transaction fees
    println!("Requesting airdrop for prover...");
    let _airdrop_sig = rpc_client.request_airdrop(&prover_authority.pubkey(), 15_000_000_000)?;

    // Wait for prover airdrop
    println!("Waiting for prover airdrop confirmation...");
    for _ in 0..30 {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let balance = rpc_client.get_balance(&prover_authority.pubkey())?;
        if balance >= 15_000_000_000 {
            println!("Prover airdrop confirmed. Balance: {} SOL", balance as f64 / 1_000_000_000.0);
            break;
        }
    }

    // Double check prover balance
    let prover_balance = rpc_client.get_balance(&prover_authority.pubkey())?;
    if prover_balance == 0 {
        anyhow::bail!("Prover airdrop failed - balance is still 0");
    }

    // Generate encryption public key (normally from X25519 keypair)
    let encryption_pubkey = [42u8; 32]; // Mock encryption pubkey

    // Build RegisterProver instruction
    println!("Creating RegisterProver instruction...");
    let register_ix = sdk_client.register_prover_instruction(
        &prover_authority.pubkey(),
        5_000_000_000,  // 5 SOL stake
        encryption_pubkey,
    )?;

    // Get recent blockhash
    let recent_blockhash = rpc_client.get_latest_blockhash()?;

    // Create and sign transaction
    let mut transaction = Transaction::new_with_payer(
        &[register_ix],
        Some(&prover_authority.pubkey()),
    );
    transaction.sign(&[&prover_authority], recent_blockhash);

    // Send transaction
    println!("Sending RegisterProver transaction...");
    let signature = rpc_client.send_and_confirm_transaction(&transaction)?;

    println!("\n✅ RegisterProver succeeded!");
    println!("   Transaction: {}", signature);

    // === STEP 3: Verify Prover Account ===
    println!("\nStep 3: Verifying prover account...");

    let (prover_pda, _) = sdk_client.get_prover_pda(&prover_authority.pubkey());
    let prover_account = rpc_client.get_account(&prover_pda)?;

    println!("   Prover PDA: {}", prover_pda);
    println!("   Prover data length: {} bytes", prover_account.data.len());

    assert!(prover_account.data.len() > 0, "Prover account should have data");

    // Expected length from ProverAccount::LEN (114 bytes)
    assert_eq!(prover_account.data.len(), 114, "Prover account should be 114 bytes");

    // Deserialize and verify prover data
    use borsh::BorshDeserialize;

    #[derive(Debug, BorshDeserialize)]
    #[allow(dead_code)]
    struct ProverAccount {
        authority: solana_sdk::pubkey::Pubkey,
        stake_amount: u64,
        reputation_score: u32,
        total_jobs_completed: u64,
        total_jobs_failed: u64,
        avg_completion_time_secs: u32,
        is_active: bool,
        registration_timestamp: i64,
        total_earnings_lamports: u64,
        encryption_pubkey: [u8; 32],
        bump: u8,
    }

    let prover_data = ProverAccount::try_from_slice(&prover_account.data)?;

    println!("   Authority: {}", prover_data.authority);
    println!("   Stake Amount: {} lamports ({} SOL)", prover_data.stake_amount, prover_data.stake_amount as f64 / 1_000_000_000.0);
    println!("   Reputation Score: {}/1000", prover_data.reputation_score);
    println!("   Is Active: {}", prover_data.is_active);
    println!("   Encryption Pubkey: {:?}", &prover_data.encryption_pubkey[..8]);

    // Verify prover data
    assert_eq!(prover_data.authority, prover_authority.pubkey(), "Authority should match");
    assert_eq!(prover_data.stake_amount, 5_000_000_000, "Stake should be 5 SOL");
    assert_eq!(prover_data.reputation_score, 1000, "Should start with perfect reputation");
    assert_eq!(prover_data.total_jobs_completed, 0, "Should have 0 completed jobs");
    assert_eq!(prover_data.total_jobs_failed, 0, "Should have 0 failed jobs");
    assert_eq!(prover_data.is_active, true, "Should be active");
    assert_eq!(prover_data.encryption_pubkey, encryption_pubkey, "Encryption pubkey should match");

    println!("\n✅ All prover account verifications passed!");

    Ok(())
}
