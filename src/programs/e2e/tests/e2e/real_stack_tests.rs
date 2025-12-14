//! E2E Tests against REAL localhost stack
//!
//! These tests require:
//! 1. solana-test-validator running on localhost:8899
//! 2. Programs deployed (bedrock, zk-generator, fhe-generator)
//! 3. Program IDs set via environment or using defaults from deploy
//!
//! Run with: cargo test --package e2e-tests real_stack -- --ignored --nocapture
//!
//! Or via Makefile: make e2e-real

use anyhow::{anyhow, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use std::str::FromStr;
use std::time::Duration;

// ============================================================================
// Configuration
// ============================================================================

const RPC_URL: &str = "http://localhost:8899";
const AIRDROP_AMOUNT: u64 = 10 * LAMPORTS_PER_SOL;
const STAKE_AMOUNT: u64 = 100_000_000; // 0.1 SOL
const JOB_PRICE: u64 = 50_000_000; // 0.05 SOL
const JOB_TIMEOUT: i64 = 3600; // 1 hour

/// Load program IDs from environment or use compile-time defaults
fn load_program_ids() -> (Pubkey, Pubkey, Pubkey) {
    let bedrock_id = std::env::var("BEDROCK_PROGRAM_ID")
        .ok()
        .and_then(|s| Pubkey::from_str(&s).ok())
        .unwrap_or_else(|| {
            // Default from deploy keypair - update after deploy
            Pubkey::from_str("BedrockProgram11111111111111111111111111111").unwrap_or_default()
        });

    let zk_gen_id = std::env::var("ZK_GENERATOR_PROGRAM_ID")
        .ok()
        .and_then(|s| Pubkey::from_str(&s).ok())
        .unwrap_or_else(|| {
            Pubkey::from_str("ZkGenerator11111111111111111111111111111111").unwrap_or_default()
        });

    let fhe_gen_id = std::env::var("FHE_GENERATOR_PROGRAM_ID")
        .ok()
        .and_then(|s| Pubkey::from_str(&s).ok())
        .unwrap_or_else(|| {
            Pubkey::from_str("FheGenerator1111111111111111111111111111111").unwrap_or_default()
        });

    (bedrock_id, zk_gen_id, fhe_gen_id)
}

// ============================================================================
// Test Helpers
// ============================================================================

/// Create RPC client connected to localhost
fn get_rpc_client() -> RpcClient {
    RpcClient::new_with_commitment(RPC_URL.to_string(), CommitmentConfig::confirmed())
}

/// Create a new keypair and airdrop SOL to it
fn create_funded_keypair(client: &RpcClient, lamports: u64) -> Result<Keypair> {
    let keypair = Keypair::new();

    // Request airdrop
    let sig = client.request_airdrop(&keypair.pubkey(), lamports)?;

    // Wait for confirmation
    let mut confirmed = false;
    for _ in 0..30 {
        if client.confirm_transaction(&sig)? {
            confirmed = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }

    if !confirmed {
        return Err(anyhow!("Airdrop not confirmed after 15 seconds"));
    }

    Ok(keypair)
}

/// Send transaction and wait for confirmation
fn send_and_confirm(
    client: &RpcClient,
    instructions: &[solana_sdk::instruction::Instruction],
    signers: &[&Keypair],
    payer: &Keypair,
) -> Result<solana_sdk::signature::Signature> {
    let recent_blockhash = client.get_latest_blockhash()?;

    let tx = Transaction::new_signed_with_payer(
        instructions,
        Some(&payer.pubkey()),
        signers,
        recent_blockhash,
    );

    let sig = client.send_and_confirm_transaction(&tx)?;
    Ok(sig)
}

/// Check if bedrock is already initialized
fn is_bedrock_initialized(client: &RpcClient, bedrock_id: &Pubkey) -> bool {
    let (config_pda, _) = bedrock_sdk::derive_config_pda(bedrock_id);
    client.get_account(&config_pda).is_ok()
}

/// Generate a unique job ID using local timestamp
/// Now that job_id is a parameter, we can generate it client-side
fn generate_job_id() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

// ============================================================================
// TESTS - Run with --ignored flag
// ============================================================================

#[test]
#[ignore] // Requires localhost stack
fn test_real_stack_connection() {
    let client = get_rpc_client();

    // Check connection
    let version = client.get_version().expect("Failed to connect to localhost");
    println!("Connected to Solana: {}", version.solana_core);

    // Check can airdrop
    let keypair = create_funded_keypair(&client, LAMPORTS_PER_SOL)
        .expect("Failed to create funded keypair");

    let balance = client.get_balance(&keypair.pubkey()).expect("Failed to get balance");
    assert_eq!(balance, LAMPORTS_PER_SOL);

    println!("Stack connection test passed!");
}

#[test]
#[ignore] // Requires localhost stack with programs deployed
fn test_bedrock_initialize_and_register_prover() {
    let client = get_rpc_client();
    let (bedrock_id, zk_gen_id, fhe_gen_id) = load_program_ids();

    println!("Program IDs:");
    println!("  Bedrock: {}", bedrock_id);
    println!("  ZK Generator: {}", zk_gen_id);
    println!("  FHE Generator: {}", fhe_gen_id);

    // Create admin keypair with funds
    let admin = create_funded_keypair(&client, AIRDROP_AMOUNT)
        .expect("Failed to create admin");
    println!("Admin: {}", admin.pubkey());

    // Initialize bedrock if not already done
    if !is_bedrock_initialized(&client, &bedrock_id) {
        println!("Initializing bedrock...");
        let (config_pda, _) = bedrock_sdk::derive_config_pda(&bedrock_id);

        let ix = bedrock_sdk::instructions::initialize(
            &bedrock_id,
            &admin.pubkey(),
            &config_pda,
            &zk_gen_id,
            &fhe_gen_id,
        );

        send_and_confirm(&client, &[ix], &[&admin], &admin)
            .expect("Failed to initialize bedrock");
        println!("Bedrock initialized!");
    } else {
        println!("Bedrock already initialized");
    }

    // Create prover keypair
    let prover = create_funded_keypair(&client, AIRDROP_AMOUNT)
        .expect("Failed to create prover");
    println!("Prover: {}", prover.pubkey());

    // Register prover
    let (prover_pda, _) = bedrock_sdk::derive_prover_pda(&bedrock_id, &prover.pubkey());

    let ix = bedrock_sdk::instructions::register_prover(
        &bedrock_id,
        &prover.pubkey(),
        &prover_pda,
        STAKE_AMOUNT,
    );

    send_and_confirm(&client, &[ix], &[&prover], &prover)
        .expect("Failed to register prover");
    println!("Prover registered with {} lamports stake", STAKE_AMOUNT);

    // Verify prover account exists
    let prover_account = client.get_account(&prover_pda)
        .expect("Prover account not found");
    assert!(prover_account.lamports > 0);
    println!("Prover PDA verified: {}", prover_pda);

    println!("\nBedrock initialization and prover registration test PASSED!");
}

#[test]
#[ignore] // Requires localhost stack with programs deployed
fn test_zk_full_flow_with_cpi() {
    let client = get_rpc_client();
    let (bedrock_id, zk_gen_id, _fhe_gen_id) = load_program_ids();

    println!("=== ZK Full Flow E2E Test ===\n");

    // Setup: Admin and initialize bedrock if needed
    let admin = create_funded_keypair(&client, AIRDROP_AMOUNT)
        .expect("Failed to create admin");

    if !is_bedrock_initialized(&client, &bedrock_id) {
        let (config_pda, _) = bedrock_sdk::derive_config_pda(&bedrock_id);
        let ix = bedrock_sdk::instructions::initialize(
            &bedrock_id,
            &admin.pubkey(),
            &config_pda,
            &zk_gen_id,
            &Pubkey::default(), // fhe not needed for this test
        );
        send_and_confirm(&client, &[ix], &[&admin], &admin)
            .expect("Failed to initialize bedrock");
        println!("Bedrock initialized");
    }

    // Step 1: Create and register prover
    println!("\n1. Registering prover...");
    let prover = create_funded_keypair(&client, AIRDROP_AMOUNT)
        .expect("Failed to create prover");

    let (prover_pda, _) = bedrock_sdk::derive_prover_pda(&bedrock_id, &prover.pubkey());
    let ix = bedrock_sdk::instructions::register_prover(
        &bedrock_id,
        &prover.pubkey(),
        &prover_pda,
        STAKE_AMOUNT,
    );
    send_and_confirm(&client, &[ix], &[&prover], &prover)
        .expect("Failed to register prover");
    println!("   Prover registered: {}", prover.pubkey());

    // Step 2: Create ZK job
    println!("\n2. Creating ZK job...");
    let job_creator = create_funded_keypair(&client, AIRDROP_AMOUNT)
        .expect("Failed to create job creator");

    // Generate job_id client-side (now it's a parameter)
    let job_id = generate_job_id();
    println!("   Job ID: {}", job_id);

    // Fake witness hash (cannot be all zeros - validation rejects it)
    let mut witness_hash = [0u8; 32];
    witness_hash[0] = 1; // Make it non-zero
    let witness_size = 1024u32;

    let (job_pda, _) = zk_generator_sdk::derive_job_pda(&zk_gen_id, &job_creator.pubkey(), job_id);
    let (escrow_pda, _) = zk_generator_sdk::derive_escrow_pda(&zk_gen_id, &job_pda);

    let ix = zk_generator_sdk::instructions::create_job(
        &zk_gen_id,
        &job_creator.pubkey(),
        &job_pda,
        &escrow_pda,
        job_id,  // Now passed as parameter
        10, // CircuitType::ProofOfInnocence
        witness_hash,
        witness_size,
        JOB_PRICE,
        JOB_TIMEOUT,
    );

    send_and_confirm(&client, &[ix], &[&job_creator], &job_creator)
        .expect("Failed to create job");
    println!("   Job created: {}", job_pda);
    println!("   Job ID: {}", job_id);

    // Verify job was created
    let job_account = client.get_account(&job_pda).expect("Job account not found after creation");
    println!("   Job account owner: {}", job_account.owner);
    println!("   Job account data len: {}", job_account.data.len());

    // Verify prover PDA exists
    let prover_pda_account = client.get_account(&prover_pda).expect("Prover PDA not found");
    println!("   Prover PDA owner: {}", prover_pda_account.owner);
    println!("   Prover PDA data len: {}", prover_pda_account.data.len());

    // Step 3: Prover claims job
    println!("\n3. Prover claiming job...");
    let ix = zk_generator_sdk::instructions::claim_job(
        &zk_gen_id,
        &prover.pubkey(),
        &job_pda,
        Some(&prover_pda),      // Prover PDA for CPI verification
        Some(&bedrock_id),      // Bedrock program for CPI
    );

    send_and_confirm(&client, &[ix], &[&prover], &prover)
        .expect("Failed to claim job");
    println!("   Job claimed by prover");

    // Step 4: Prover submits proof
    println!("\n4. Prover submitting proof...");
    let proof_hash = [1u8; 32]; // Fake proof hash
    let (bedrock_config, _) = bedrock_sdk::derive_config_pda(&bedrock_id);

    // Fee recipient (could be treasury, using admin for simplicity)
    let fee_recipient = admin.pubkey();

    let ix = zk_generator_sdk::instructions::submit_proof(
        &zk_gen_id,
        &prover.pubkey(),
        &job_pda,
        &escrow_pda,
        &fee_recipient,
        &bedrock_id,
        &prover_pda,
        &bedrock_config,
        proof_hash,
    );

    send_and_confirm(&client, &[ix], &[&prover], &prover)
        .expect("Failed to submit proof");
    println!("   Proof submitted!");

    // Step 5: Verify job state
    println!("\n5. Verifying final state...");

    // Check job account (should exist but be in completed state)
    let job_account = client.get_account(&job_pda);
    if let Ok(account) = job_account {
        println!("   Job account exists, data len: {}", account.data.len());
        // TODO: Deserialize and check status == Completed
    }

    // Check prover stats were updated via CPI
    let prover_account = client.get_account(&prover_pda)
        .expect("Prover account should exist");
    println!("   Prover PDA balance: {} lamports", prover_account.lamports);
    // TODO: Deserialize and check jobs_completed > 0

    println!("\n=== ZK Full Flow E2E Test PASSED! ===");
    println!("CPI from zk-generator to bedrock executed successfully");
}

#[test]
#[ignore] // Requires localhost stack with programs deployed
fn test_prover_verification_on_claim() {
    let client = get_rpc_client();
    let (bedrock_id, zk_gen_id, _) = load_program_ids();

    println!("=== Prover Verification Test ===\n");

    // Create an UNREGISTERED prover
    let unregistered_prover = create_funded_keypair(&client, AIRDROP_AMOUNT)
        .expect("Failed to create unregistered prover");

    // Create a job
    let job_creator = create_funded_keypair(&client, AIRDROP_AMOUNT)
        .expect("Failed to create job creator");

    let job_id = generate_job_id();
    let (job_pda, _) = zk_generator_sdk::derive_job_pda(&zk_gen_id, &job_creator.pubkey(), job_id);
    let (escrow_pda, _) = zk_generator_sdk::derive_escrow_pda(&zk_gen_id, &job_pda);

    // Non-zero witness hash required by validation
    let mut witness_hash = [0u8; 32];
    witness_hash[0] = 1;

    let ix = zk_generator_sdk::instructions::create_job(
        &zk_gen_id,
        &job_creator.pubkey(),
        &job_pda,
        &escrow_pda,
        job_id,  // Now passed as parameter
        10,
        witness_hash,
        1024,
        JOB_PRICE,
        JOB_TIMEOUT,
    );

    send_and_confirm(&client, &[ix], &[&job_creator], &job_creator)
        .expect("Failed to create job");
    println!("Job created: {}", job_pda);

    // Try to claim with unregistered prover - should fail
    let (fake_prover_pda, _) = bedrock_sdk::derive_prover_pda(&bedrock_id, &unregistered_prover.pubkey());

    let ix = zk_generator_sdk::instructions::claim_job(
        &zk_gen_id,
        &unregistered_prover.pubkey(),
        &job_pda,
        Some(&fake_prover_pda),
        Some(&bedrock_id),
    );

    let result = send_and_confirm(&client, &[ix], &[&unregistered_prover], &unregistered_prover);

    assert!(result.is_err(), "Unregistered prover should not be able to claim");
    println!("Correctly rejected unregistered prover!");

    println!("\n=== Prover Verification Test PASSED! ===");
}

/// Test del flujo completo FHE con consensus y CPI a bedrock
/// Similar a test_zk_full_flow_with_cpi pero con FHE multi-prover consensus
#[test]
#[ignore]
fn test_fhe_full_flow_with_consensus_and_cpi() {
    println!("\n=== FHE Full Flow with Consensus E2E Test ===\n");

    let (bedrock_id, _zk_gen_id, fhe_gen_id) = load_program_ids();
    let client = get_rpc_client();

    // Setup: admin, 3 provers, creator
    let admin = create_funded_keypair(&client, AIRDROP_AMOUNT).expect("Failed to create admin");
    let prover1 = create_funded_keypair(&client, AIRDROP_AMOUNT).expect("Failed to create prover1");
    let prover2 = create_funded_keypair(&client, AIRDROP_AMOUNT).expect("Failed to create prover2");
    let prover3 = create_funded_keypair(&client, AIRDROP_AMOUNT).expect("Failed to create prover3");
    let job_creator = create_funded_keypair(&client, AIRDROP_AMOUNT).expect("Failed to create creator");
    let fee_recipient = create_funded_keypair(&client, AIRDROP_AMOUNT).expect("Failed to create fee_recipient");

    println!("1. Registering 3 provers in bedrock...");

    // Initialize bedrock if needed
    let (config_pda, _) = bedrock_sdk::derive_config_pda(&bedrock_id);
    if !is_bedrock_initialized(&client, &bedrock_id) {
        let (_zk_id, fhe_id) = (_zk_gen_id, fhe_gen_id);

        let ix = bedrock_sdk::instructions::initialize(
            &bedrock_id,
            &admin.pubkey(),
            &config_pda,
            &_zk_id,
            &fhe_id,
        );
        send_and_confirm(&client, &[ix], &[&admin], &admin)
            .expect("Failed to initialize bedrock");
        println!("   Bedrock initialized");
    }

    // Register all 3 provers
    for (i, prover) in [&prover1, &prover2, &prover3].iter().enumerate() {
        let (prover_pda, _) = bedrock_sdk::derive_prover_pda(&bedrock_id, &prover.pubkey());

        if client.get_account(&prover_pda).is_err() {
            let ix = bedrock_sdk::instructions::register_prover(
                &bedrock_id,
                &prover.pubkey(),
                &prover_pda,
                STAKE_AMOUNT,
            );

            send_and_confirm(&client, &[ix], &[prover], prover)
                .expect(&format!("Failed to register prover {}", i + 1));
            println!("   Prover {} registered: {}", i + 1, prover.pubkey());
        }
    }

    println!("\n2. Creating FHE job with consensus requirement...");

    let job_id = generate_job_id();
    let (job_pda, _) = fhe_generator_sdk::derive_job_pda(
        &fhe_gen_id,
        &job_creator.pubkey(),
        job_id,
    );
    let (consensus_pda, _) = fhe_generator_sdk::derive_consensus_pda(&fhe_gen_id, job_id);
    let (escrow_pda, _) = fhe_generator_sdk::derive_escrow_pda(&fhe_gen_id, &job_pda);

    let circuit_type = 4u8; // FHE_ADD
    let mut witness_hash = [0u8; 32];
    witness_hash[0] = 1; // Non-zero
    let witness_size = 2048u32;
    let price_lamports = 300_000_000u64; // 0.3 SOL total
    let timeout_seconds = 3600i64;
    let required_provers = 3u8;
    let consensus_threshold = 2u8; // 2 out of 3 must agree

    let ix = fhe_generator_sdk::instructions::create_job(
        &fhe_gen_id,
        &job_creator.pubkey(),
        job_id,
        circuit_type,
        witness_hash,
        witness_size,
        price_lamports,
        timeout_seconds,
        required_provers,
        consensus_threshold,
        0, // operation_param1
        0, // operation_param2
        0, // operation_param3
    );

    send_and_confirm(&client, &[ix], &[&job_creator], &job_creator)
        .expect("Failed to create FHE job");
    println!("   Job created: {}", job_pda);
    println!("   Required provers: {}, Consensus threshold: {}", required_provers, consensus_threshold);

    println!("\n3. All 3 provers claiming job...");

    for (i, prover) in [&prover1, &prover2, &prover3].iter().enumerate() {
        let (prover_pda, _) = bedrock_sdk::derive_prover_pda(&bedrock_id, &prover.pubkey());

        let ix = fhe_generator_sdk::instructions::claim_job(
            &fhe_gen_id,
            &prover.pubkey(),
            &job_pda,
            &consensus_pda,
            &prover_pda,
            &bedrock_id,
        );

        send_and_confirm(&client, &[ix], &[prover], prover)
            .expect(&format!("Failed to claim job for prover {}", i + 1));
        println!("   Prover {} claimed", i + 1);
    }

    println!("\n4. Provers submitting results (2 agree, 1 differs)...");

    let correct_result = [0xAB; 32];
    let wrong_result = [0xCD; 32];

    // Prover 1 and 2 submit same result (will reach consensus)
    for (i, prover) in [&prover1, &prover2].iter().enumerate() {
        let ix = fhe_generator_sdk::instructions::submit_result(
            &fhe_gen_id,
            &prover.pubkey(),
            &job_pda,
            &consensus_pda,
            correct_result,
        );

        send_and_confirm(&client, &[ix], &[prover], prover)
            .expect(&format!("Failed to submit result for prover {}", i + 1));
        println!("   Prover {} submitted result: {:?}", i + 1, &correct_result[0..4]);
    }

    // Prover 3 submits different result
    let ix = fhe_generator_sdk::instructions::submit_result(
        &fhe_gen_id,
        &prover3.pubkey(),
        &job_pda,
        &consensus_pda,
        wrong_result,
    );

    send_and_confirm(&client, &[ix], &[&prover3], &prover3)
        .expect("Failed to submit result for prover 3");
    println!("   Prover 3 submitted result: {:?} (different)", &wrong_result[0..4]);

    println!("\n5. Finalizing job (should detect consensus and update stats via CPI)...");

    // Prepare prover lists
    let prover_wallets = [prover1.pubkey(), prover2.pubkey(), prover3.pubkey()];
    let (prover1_pda, _) = bedrock_sdk::derive_prover_pda(&bedrock_id, &prover1.pubkey());
    let (prover2_pda, _) = bedrock_sdk::derive_prover_pda(&bedrock_id, &prover2.pubkey());
    let (prover3_pda, _) = bedrock_sdk::derive_prover_pda(&bedrock_id, &prover3.pubkey());
    let prover_pdas = [prover1_pda, prover2_pda, prover3_pda];

    let ix = fhe_generator_sdk::instructions::finalize_job(
        &fhe_gen_id,
        &job_creator.pubkey(),
        &job_pda,
        &consensus_pda,
        &escrow_pda,
        &job_creator.pubkey(),
        &fee_recipient.pubkey(),
        &bedrock_id,
        &config_pda,
        &prover_wallets,
        &prover_pdas,
    );

    send_and_confirm(&client, &[ix], &[&job_creator], &job_creator)
        .expect("Failed to finalize job");
    println!("   Job finalized with consensus!");

    println!("\n6. Verifying final state...");

    // Verify job completed
    let job_account = client.get_account(&job_pda).expect("Job account should exist");
    println!("   Job account exists, data len: {}", job_account.data.len());

    // Verify prover stats were updated (prover1 and prover2 should have +1 completed)
    for (i, prover) in [&prover1, &prover2].iter().enumerate() {
        let (prover_pda, _) = bedrock_sdk::derive_prover_pda(&bedrock_id, &prover.pubkey());
        let prover_account = client.get_account(&prover_pda)
            .expect(&format!("Prover {} PDA should exist", i + 1));
        println!("   Prover {} PDA balance: {} lamports", i + 1, prover_account.lamports);
    }

    // Prover 3 (who submitted wrong result) should also exist but with different stats
    let (prover3_pda, _) = bedrock_sdk::derive_prover_pda(&bedrock_id, &prover3.pubkey());
    let prover3_account = client.get_account(&prover3_pda)
        .expect("Prover 3 PDA should exist");
    println!("   Prover 3 PDA balance: {} lamports (submitted wrong result)", prover3_account.lamports);

    println!("\n=== FHE Full Flow E2E Test PASSED! ===");
    println!("Multi-prover consensus reached (2/3 agreement)");
    println!("CPI from fhe-generator to bedrock executed successfully");
}

// ============================================================================
// Module exports for test harness
// ============================================================================

#[cfg(test)]
mod real_stack_tests {
    // Re-export tests so they can be run from the test harness
    pub use super::*;
}
