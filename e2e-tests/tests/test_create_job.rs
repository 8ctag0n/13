use anyhow::Result;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
    commitment_config::CommitmentConfig,
};
use solana_client::rpc_client::RpcClient;
use std::str::FromStr;
use cypherlink_types::CircuitType;

#[tokio::test(flavor = "multi_thread")]
async fn test_create_job() -> Result<()> {
    println!("\n=== Testing Create Job ===\n");

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
    let sdk_client = cypherlink_sdk::MarketplaceClient::new(
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

    // === STEP 2: Create Job ===
    println!("\nStep 2: Creating job...");

    // Generate job creator keypair
    let job_creator = Keypair::new();
    println!("Job Creator: {}", job_creator.pubkey());

    // Airdrop SOL to job creator for job payment + transaction fees
    println!("Requesting airdrop for job creator...");
    let _airdrop_sig = rpc_client.request_airdrop(&job_creator.pubkey(), 20_000_000_000)?;

    // Wait for job creator airdrop
    println!("Waiting for job creator airdrop confirmation...");
    for _ in 0..30 {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let balance = rpc_client.get_balance(&job_creator.pubkey())?;
        if balance >= 20_000_000_000 {
            println!("Job creator airdrop confirmed. Balance: {} SOL", balance as f64 / 1_000_000_000.0);
            break;
        }
    }

    // Double check job creator balance
    let creator_balance = rpc_client.get_balance(&job_creator.pubkey())?;
    if creator_balance == 0 {
        anyhow::bail!("Job creator airdrop failed - balance is still 0");
    }

    // Job parameters
    // Read the config to get the current next_job_id
    let (config_pda, _) = sdk_client.get_config_pda();
    let config_account = rpc_client.get_account(&config_pda)?;

    // Deserialize config to get next_job_id
    use borsh::BorshDeserialize;

    #[derive(Debug, BorshDeserialize)]
    #[allow(dead_code)]
    struct MarketplaceConfig {
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

    let config_data = MarketplaceConfig::try_from_slice(&config_account.data)?;
    let job_id = config_data.next_job_id;

    println!("   Using job ID: {} (from config.next_job_id)", job_id);
    let circuit_type = CircuitType::ZcashOrchard;
    let witness_commitment = [123u8; 32]; // Mock witness commitment
    let witness_size = 2048;
    let price_lamports = 2_000_000_000; // 2 SOL
    let timeout_seconds = 600; // 10 minutes

    // Build CreateJob instruction
    println!("Creating CreateJob instruction...");
    println!("   Job ID: {}", job_id);
    println!("   Circuit Type: {:?}", circuit_type);
    println!("   Price: {} SOL", price_lamports as f64 / 1_000_000_000.0);
    println!("   Timeout: {} seconds", timeout_seconds);

    let create_job_ix = sdk_client.create_job_instruction(
        &job_creator.pubkey(),
        job_id,
        circuit_type,
        witness_commitment,
        witness_size,
        price_lamports,
        timeout_seconds,
    )?;

    // Get recent blockhash
    let recent_blockhash = rpc_client.get_latest_blockhash()?;

    // Create and sign transaction
    let mut transaction = Transaction::new_with_payer(
        &[create_job_ix],
        Some(&job_creator.pubkey()),
    );
    transaction.sign(&[&job_creator], recent_blockhash);

    // Send transaction
    println!("Sending CreateJob transaction...");
    let signature = rpc_client.send_and_confirm_transaction(&transaction)?;

    println!("\n✅ CreateJob succeeded!");
    println!("   Transaction: {}", signature);

    // === STEP 3: Verify Job Account ===
    println!("\nStep 3: Verifying job account...");

    let (job_pda, _) = sdk_client.get_job_pda(&job_creator.pubkey(), job_id);
    let job_account = rpc_client.get_account(&job_pda)?;

    println!("   Job PDA: {}", job_pda);
    println!("   Job data length: {} bytes", job_account.data.len());

    assert!(job_account.data.len() > 0, "Job account should have data");

    // Verify the job account has reasonable size
    // JobAccount::LEN is a conservative estimate, actual size depends on CircuitType variant
    assert!(job_account.data.len() >= 200, "Job account should have at least 200 bytes");
    assert!(job_account.data.len() <= 400, "Job account should not exceed 400 bytes");

    println!("   Job account data verified: {} bytes", job_account.data.len());

    // Verify escrow account was created
    let (escrow_pda, _) = sdk_client.get_escrow_pda(&job_pda);
    println!("\n   Escrow PDA: {}", escrow_pda);

    let escrow_account = rpc_client.get_account(&escrow_pda)?;
    let escrow_balance = escrow_account.lamports;

    println!("   Escrow Balance: {} lamports ({} SOL)", escrow_balance, escrow_balance as f64 / 1_000_000_000.0);

    // Escrow should hold the job price + rent
    // Rent is typically ~890880 lamports for rent-exempt account
    assert!(escrow_balance >= price_lamports, "Escrow should hold at least job price");
    assert!(escrow_balance < price_lamports + 10_000_000, "Escrow should not have excessive balance");

    println!("\n✅ All job account verifications passed!");

    Ok(())
}
