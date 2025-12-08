//! Integration tests for ZK Generator using solana-program-test

use borsh::BorshDeserialize;
use solana_program::pubkey::Pubkey;
use solana_program_test::{processor, ProgramTest};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    signature::{Keypair, Signer},
    system_program,
    transaction::Transaction,
};
use zk_generator::{process_instruction, ZkGeneratorInstruction, ZkJob, ZK_JOB_SEED};
use zyberlink_types::JobStatus;

/// Escrow seed (must match the one in the program)
const ESCROW_SEED: &[u8] = b"zk_escrow";

/// Create a ProgramTest instance with ZK Generator loaded
fn program_test() -> ProgramTest {
    let mut pt = ProgramTest::new(
        "zk_generator",
        zk_generator_program_id(),
        processor!(process_instruction),
    );
    pt.prefer_bpf(false);
    pt
}

/// Get ZK Generator program ID (deterministic for tests)
fn zk_generator_program_id() -> Pubkey {
    Pubkey::new_from_array([
        0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
        0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e,
        0x2f, 0x30,
    ])
}

/// Helper to derive job PDA
fn derive_job_pda(creator: &Pubkey, job_id: u64, program_id: &Pubkey) -> (Pubkey, u8) {
    let job_id_bytes = job_id.to_le_bytes();
    Pubkey::find_program_address(&[ZK_JOB_SEED, creator.as_ref(), &job_id_bytes], program_id)
}

/// Helper to derive escrow PDA
fn derive_escrow_pda(job_pda: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[ESCROW_SEED, job_pda.as_ref()], program_id)
}

/// Helper to create CreateJob instruction
fn create_job_ix(
    creator: &Pubkey,
    job_pda: &Pubkey,
    escrow_pda: &Pubkey,
    circuit_type: u8,
    witness_hash: [u8; 32],
    witness_size: u32,
    price_lamports: u64,
    timeout_seconds: i64,
) -> Instruction {
    let data = borsh::to_vec(&ZkGeneratorInstruction::CreateJob {
        circuit_type,
        witness_hash,
        witness_size,
        price_lamports,
        timeout_seconds,
    })
    .unwrap();

    Instruction {
        program_id: zk_generator_program_id(),
        accounts: vec![
            AccountMeta::new(*creator, true),
            AccountMeta::new(*job_pda, false),
            AccountMeta::new(*escrow_pda, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data,
    }
}

/// Helper to create ClaimJob instruction
fn claim_job_ix(prover: &Pubkey, job_pda: &Pubkey) -> Instruction {
    let data = borsh::to_vec(&ZkGeneratorInstruction::ClaimJob).unwrap();

    Instruction {
        program_id: zk_generator_program_id(),
        accounts: vec![
            AccountMeta::new(*prover, true),
            AccountMeta::new(*job_pda, false),
        ],
        data,
    }
}

/// Helper to create SubmitProof instruction
fn submit_proof_ix(
    prover: &Pubkey,
    job_pda: &Pubkey,
    escrow_pda: &Pubkey,
    fee_recipient: &Pubkey,
    proof_hash: [u8; 32],
) -> Instruction {
    let data = borsh::to_vec(&ZkGeneratorInstruction::SubmitProof { proof_hash }).unwrap();

    Instruction {
        program_id: zk_generator_program_id(),
        accounts: vec![
            AccountMeta::new(*prover, true),
            AccountMeta::new(*job_pda, false),
            AccountMeta::new(*escrow_pda, false),
            AccountMeta::new(*fee_recipient, false),
        ],
        data,
    }
}

/// Helper to create CancelJob instruction
fn cancel_job_ix(creator: &Pubkey, job_pda: &Pubkey, escrow_pda: &Pubkey) -> Instruction {
    let data = borsh::to_vec(&ZkGeneratorInstruction::CancelJob).unwrap();

    Instruction {
        program_id: zk_generator_program_id(),
        accounts: vec![
            AccountMeta::new(*creator, true),
            AccountMeta::new(*job_pda, false),
            AccountMeta::new(*escrow_pda, false),
        ],
        data,
    }
}

// =============================================================================
// CreateJob Tests
// =============================================================================

#[tokio::test]
async fn test_create_job() {
    let pt = program_test();
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let program_id = zk_generator_program_id();

    // Job parameters
    let circuit_type = 0u8; // CIRCUIT_ZCASH_ORCHARD
    let witness_hash = [1u8; 32];
    let witness_size = 2048u32;
    let price_lamports = 10_000_000u64; // 0.01 SOL
    let timeout_seconds = 300i64; // 5 minutes

    // Get current time to predict job_id
    let clock = banks_client.get_sysvar::<solana_sdk::sysvar::clock::Clock>().await.unwrap();
    let job_id = (clock.unix_timestamp as u64) ^ (payer.pubkey().to_bytes()[0] as u64);

    // Derive PDAs
    let (job_pda, _job_bump) = derive_job_pda(&payer.pubkey(), job_id, &program_id);
    let (escrow_pda, _escrow_bump) = derive_escrow_pda(&job_pda, &program_id);

    // Create instruction
    let ix = create_job_ix(
        &payer.pubkey(),
        &job_pda,
        &escrow_pda,
        circuit_type,
        witness_hash,
        witness_size,
        price_lamports,
        timeout_seconds,
    );

    // Send transaction
    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);

    banks_client.process_transaction(tx).await.unwrap();

    // Verify job was created
    let job_account = banks_client.get_account(job_pda).await.unwrap().unwrap();
    let zk_job: ZkJob = BorshDeserialize::deserialize(&mut &job_account.data[..]).unwrap();

    assert_eq!(zk_job.circuit_type, circuit_type);
    assert_eq!(zk_job.common.creator, payer.pubkey());
    assert_eq!(zk_job.common.status, JobStatus::Pending);
    assert_eq!(zk_job.common.witness_hash, witness_hash);
    assert_eq!(zk_job.common.witness_size, witness_size);
    assert_eq!(zk_job.common.price_lamports, price_lamports);
    assert!(zk_job.common.prover.is_none());

    // Verify escrow has funds
    let escrow_account = banks_client.get_account(escrow_pda).await.unwrap().unwrap();
    assert!(escrow_account.lamports >= price_lamports);
}

#[tokio::test]
async fn test_create_job_invalid_circuit_type() {
    let pt = program_test();
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let program_id = zk_generator_program_id();

    // Invalid circuit type (5 is > CIRCUIT_CREDENTIAL)
    let circuit_type = 5u8;
    let witness_hash = [1u8; 32];
    let witness_size = 2048u32;
    let price_lamports = 10_000_000u64;
    let timeout_seconds = 300i64;

    let clock = banks_client.get_sysvar::<solana_sdk::sysvar::clock::Clock>().await.unwrap();
    let job_id = (clock.unix_timestamp as u64) ^ (payer.pubkey().to_bytes()[0] as u64);

    let (job_pda, _) = derive_job_pda(&payer.pubkey(), job_id, &program_id);
    let (escrow_pda, _) = derive_escrow_pda(&job_pda, &program_id);

    let ix = create_job_ix(
        &payer.pubkey(),
        &job_pda,
        &escrow_pda,
        circuit_type,
        witness_hash,
        witness_size,
        price_lamports,
        timeout_seconds,
    );

    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);

    // Should fail
    let result = banks_client.process_transaction(tx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_job_price_too_low() {
    let pt = program_test();
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let program_id = zk_generator_program_id();

    // Price below minimum (MIN_JOB_PRICE = 1_000_000)
    let price_lamports = 100u64;

    let clock = banks_client.get_sysvar::<solana_sdk::sysvar::clock::Clock>().await.unwrap();
    let job_id = (clock.unix_timestamp as u64) ^ (payer.pubkey().to_bytes()[0] as u64);

    let (job_pda, _) = derive_job_pda(&payer.pubkey(), job_id, &program_id);
    let (escrow_pda, _) = derive_escrow_pda(&job_pda, &program_id);

    let ix = create_job_ix(
        &payer.pubkey(),
        &job_pda,
        &escrow_pda,
        0,
        [1u8; 32],
        2048,
        price_lamports,
        300,
    );

    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);

    let result = banks_client.process_transaction(tx).await;
    assert!(result.is_err());
}

// =============================================================================
// ClaimJob Tests
// =============================================================================

#[tokio::test]
async fn test_claim_job() {
    let pt = program_test();
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let program_id = zk_generator_program_id();
    let prover = Keypair::new();

    // First create a job
    let clock = banks_client.get_sysvar::<solana_sdk::sysvar::clock::Clock>().await.unwrap();
    let job_id = (clock.unix_timestamp as u64) ^ (payer.pubkey().to_bytes()[0] as u64);

    let (job_pda, _) = derive_job_pda(&payer.pubkey(), job_id, &program_id);
    let (escrow_pda, _) = derive_escrow_pda(&job_pda, &program_id);

    let create_ix = create_job_ix(
        &payer.pubkey(),
        &job_pda,
        &escrow_pda,
        0,
        [1u8; 32],
        2048,
        10_000_000,
        300,
    );

    let mut tx = Transaction::new_with_payer(&[create_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Now claim the job
    let claim_ix = claim_job_ix(&prover.pubkey(), &job_pda);

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[claim_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &prover], recent_blockhash);

    banks_client.process_transaction(tx).await.unwrap();

    // Verify job is now claimed
    let job_account = banks_client.get_account(job_pda).await.unwrap().unwrap();
    let zk_job: ZkJob = BorshDeserialize::deserialize(&mut &job_account.data[..]).unwrap();

    assert_eq!(zk_job.common.status, JobStatus::Claimed);
    assert_eq!(zk_job.common.prover, Some(prover.pubkey()));
}

#[tokio::test]
async fn test_claim_job_already_claimed() {
    let pt = program_test();
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let program_id = zk_generator_program_id();
    let prover1 = Keypair::new();
    let prover2 = Keypair::new();

    // Create a job
    let clock = banks_client.get_sysvar::<solana_sdk::sysvar::clock::Clock>().await.unwrap();
    let job_id = (clock.unix_timestamp as u64) ^ (payer.pubkey().to_bytes()[0] as u64);

    let (job_pda, _) = derive_job_pda(&payer.pubkey(), job_id, &program_id);
    let (escrow_pda, _) = derive_escrow_pda(&job_pda, &program_id);

    let create_ix = create_job_ix(
        &payer.pubkey(),
        &job_pda,
        &escrow_pda,
        0,
        [1u8; 32],
        2048,
        10_000_000,
        300,
    );

    let mut tx = Transaction::new_with_payer(&[create_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // First prover claims
    let claim_ix = claim_job_ix(&prover1.pubkey(), &job_pda);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[claim_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &prover1], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Second prover tries to claim - should fail
    let claim_ix2 = claim_job_ix(&prover2.pubkey(), &job_pda);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[claim_ix2], Some(&payer.pubkey()));
    tx.sign(&[&payer, &prover2], recent_blockhash);

    let result = banks_client.process_transaction(tx).await;
    assert!(result.is_err());
}

// =============================================================================
// SubmitProof Tests
// =============================================================================

#[tokio::test]
async fn test_submit_proof() {
    let pt = program_test();
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let program_id = zk_generator_program_id();
    let prover = Keypair::new();
    let fee_recipient = Keypair::new();

    let price_lamports = 10_000_000u64;

    // Create a job
    let clock = banks_client.get_sysvar::<solana_sdk::sysvar::clock::Clock>().await.unwrap();
    let job_id = (clock.unix_timestamp as u64) ^ (payer.pubkey().to_bytes()[0] as u64);

    let (job_pda, _) = derive_job_pda(&payer.pubkey(), job_id, &program_id);
    let (escrow_pda, _) = derive_escrow_pda(&job_pda, &program_id);

    let create_ix = create_job_ix(
        &payer.pubkey(),
        &job_pda,
        &escrow_pda,
        0,
        [1u8; 32],
        2048,
        price_lamports,
        300,
    );

    let mut tx = Transaction::new_with_payer(&[create_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Claim the job
    let claim_ix = claim_job_ix(&prover.pubkey(), &job_pda);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[claim_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &prover], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Get prover balance before
    let prover_balance_before = banks_client
        .get_account(prover.pubkey())
        .await
        .unwrap()
        .map(|a| a.lamports)
        .unwrap_or(0);

    // Fund fee_recipient with enough for rent exemption
    let transfer_ix = solana_sdk::system_instruction::transfer(
        &payer.pubkey(),
        &fee_recipient.pubkey(),
        1_000_000, // 0.001 SOL for rent
    );
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[transfer_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Submit proof
    let proof_hash = [42u8; 32];
    let submit_ix = submit_proof_ix(
        &prover.pubkey(),
        &job_pda,
        &escrow_pda,
        &fee_recipient.pubkey(),
        proof_hash,
    );

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[submit_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &prover], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Verify job is completed
    let job_account = banks_client.get_account(job_pda).await.unwrap().unwrap();
    let zk_job: ZkJob = BorshDeserialize::deserialize(&mut &job_account.data[..]).unwrap();

    assert_eq!(zk_job.common.status, JobStatus::Completed);
    assert_eq!(zk_job.common.proof_hash, Some(proof_hash));

    // Verify prover received payment (minus platform fee of 2.5%)
    let prover_balance_after = banks_client
        .get_account(prover.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    let expected_payout = price_lamports - (price_lamports * 250 / 10_000); // 97.5%
    assert!(
        prover_balance_after >= prover_balance_before + expected_payout,
        "Prover should receive payout. Before: {}, After: {}, Expected payout: {}",
        prover_balance_before,
        prover_balance_after,
        expected_payout
    );

    // Verify fee recipient received fee (initial 1_000_000 + fee)
    let fee_recipient_balance = banks_client
        .get_account(fee_recipient.pubkey())
        .await
        .unwrap()
        .map(|a| a.lamports)
        .unwrap_or(0);

    let expected_fee = price_lamports * 250 / 10_000; // 2.5%
    let expected_total = 1_000_000 + expected_fee; // initial + fee
    assert!(
        fee_recipient_balance >= expected_total,
        "Fee recipient should have at least {} lamports, got {}",
        expected_total,
        fee_recipient_balance
    );
}

#[tokio::test]
async fn test_submit_proof_wrong_prover() {
    let pt = program_test();
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let program_id = zk_generator_program_id();
    let prover1 = Keypair::new();
    let prover2 = Keypair::new();
    let fee_recipient = Keypair::new();

    // Create a job
    let clock = banks_client.get_sysvar::<solana_sdk::sysvar::clock::Clock>().await.unwrap();
    let job_id = (clock.unix_timestamp as u64) ^ (payer.pubkey().to_bytes()[0] as u64);

    let (job_pda, _) = derive_job_pda(&payer.pubkey(), job_id, &program_id);
    let (escrow_pda, _) = derive_escrow_pda(&job_pda, &program_id);

    let create_ix = create_job_ix(
        &payer.pubkey(),
        &job_pda,
        &escrow_pda,
        0,
        [1u8; 32],
        2048,
        10_000_000,
        300,
    );

    let mut tx = Transaction::new_with_payer(&[create_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Prover1 claims
    let claim_ix = claim_job_ix(&prover1.pubkey(), &job_pda);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[claim_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &prover1], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Prover2 tries to submit - should fail
    let submit_ix = submit_proof_ix(
        &prover2.pubkey(),
        &job_pda,
        &escrow_pda,
        &fee_recipient.pubkey(),
        [42u8; 32],
    );

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[submit_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &prover2], recent_blockhash);

    let result = banks_client.process_transaction(tx).await;
    assert!(result.is_err());
}

// =============================================================================
// CancelJob Tests
// =============================================================================

#[tokio::test]
async fn test_cancel_job() {
    let pt = program_test();
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let program_id = zk_generator_program_id();
    let price_lamports = 10_000_000u64;

    // Create a job
    let clock = banks_client.get_sysvar::<solana_sdk::sysvar::clock::Clock>().await.unwrap();
    let job_id = (clock.unix_timestamp as u64) ^ (payer.pubkey().to_bytes()[0] as u64);

    let (job_pda, _) = derive_job_pda(&payer.pubkey(), job_id, &program_id);
    let (escrow_pda, _) = derive_escrow_pda(&job_pda, &program_id);

    // Get creator balance before
    let creator_balance_before = banks_client
        .get_account(payer.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    let create_ix = create_job_ix(
        &payer.pubkey(),
        &job_pda,
        &escrow_pda,
        0,
        [1u8; 32],
        2048,
        price_lamports,
        300,
    );

    let mut tx = Transaction::new_with_payer(&[create_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Get balance after creation (should be less by price + rent + fees)
    let creator_balance_after_create = banks_client
        .get_account(payer.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    assert!(
        creator_balance_after_create < creator_balance_before,
        "Balance should decrease after creating job"
    );

    // Cancel the job
    let cancel_ix = cancel_job_ix(&payer.pubkey(), &job_pda, &escrow_pda);

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[cancel_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Verify job is cancelled
    let job_account = banks_client.get_account(job_pda).await.unwrap().unwrap();
    let zk_job: ZkJob = BorshDeserialize::deserialize(&mut &job_account.data[..]).unwrap();

    assert_eq!(zk_job.common.status, JobStatus::Cancelled);

    // Verify creator got refund (escrow should be empty)
    let escrow_account = banks_client.get_account(escrow_pda).await.unwrap();
    assert!(
        escrow_account.is_none() || escrow_account.unwrap().lamports == 0,
        "Escrow should be empty after cancel"
    );

    // Creator balance should be higher than after create (refund received)
    let creator_balance_after_cancel = banks_client
        .get_account(payer.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    assert!(
        creator_balance_after_cancel > creator_balance_after_create,
        "Creator should receive refund"
    );
}

#[tokio::test]
async fn test_cancel_job_not_creator() {
    let pt = program_test();
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let program_id = zk_generator_program_id();
    let other_user = Keypair::new();

    // Create a job
    let clock = banks_client.get_sysvar::<solana_sdk::sysvar::clock::Clock>().await.unwrap();
    let job_id = (clock.unix_timestamp as u64) ^ (payer.pubkey().to_bytes()[0] as u64);

    let (job_pda, _) = derive_job_pda(&payer.pubkey(), job_id, &program_id);
    let (escrow_pda, _) = derive_escrow_pda(&job_pda, &program_id);

    let create_ix = create_job_ix(
        &payer.pubkey(),
        &job_pda,
        &escrow_pda,
        0,
        [1u8; 32],
        2048,
        10_000_000,
        300,
    );

    let mut tx = Transaction::new_with_payer(&[create_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Other user tries to cancel - should fail
    let cancel_ix = cancel_job_ix(&other_user.pubkey(), &job_pda, &escrow_pda);

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[cancel_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &other_user], recent_blockhash);

    let result = banks_client.process_transaction(tx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_cancel_job_already_claimed() {
    let pt = program_test();
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let program_id = zk_generator_program_id();
    let prover = Keypair::new();

    // Create a job
    let clock = banks_client.get_sysvar::<solana_sdk::sysvar::clock::Clock>().await.unwrap();
    let job_id = (clock.unix_timestamp as u64) ^ (payer.pubkey().to_bytes()[0] as u64);

    let (job_pda, _) = derive_job_pda(&payer.pubkey(), job_id, &program_id);
    let (escrow_pda, _) = derive_escrow_pda(&job_pda, &program_id);

    let create_ix = create_job_ix(
        &payer.pubkey(),
        &job_pda,
        &escrow_pda,
        0,
        [1u8; 32],
        2048,
        10_000_000,
        300,
    );

    let mut tx = Transaction::new_with_payer(&[create_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Claim the job
    let claim_ix = claim_job_ix(&prover.pubkey(), &job_pda);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[claim_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &prover], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Try to cancel - should fail (job is claimed)
    let cancel_ix = cancel_job_ix(&payer.pubkey(), &job_pda, &escrow_pda);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[cancel_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);

    let result = banks_client.process_transaction(tx).await;
    assert!(result.is_err());
}

// =============================================================================
// E2E Flow Test
// =============================================================================

#[tokio::test]
async fn test_full_zk_flow() {
    let pt = program_test();
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let program_id = zk_generator_program_id();
    let prover = Keypair::new();
    let fee_recipient = Keypair::new();

    let price_lamports = 50_000_000u64; // 0.05 SOL

    // === Step 1: Create Job ===
    let clock = banks_client.get_sysvar::<solana_sdk::sysvar::clock::Clock>().await.unwrap();
    let job_id = (clock.unix_timestamp as u64) ^ (payer.pubkey().to_bytes()[0] as u64);

    let (job_pda, _) = derive_job_pda(&payer.pubkey(), job_id, &program_id);
    let (escrow_pda, _) = derive_escrow_pda(&job_pda, &program_id);

    let create_ix = create_job_ix(
        &payer.pubkey(),
        &job_pda,
        &escrow_pda,
        2, // CIRCUIT_ANONYMOUS_VOTE
        [0xAB; 32],
        4096,
        price_lamports,
        600, // 10 minutes
    );

    let mut tx = Transaction::new_with_payer(&[create_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Verify: Pending, no prover
    let job_account = banks_client.get_account(job_pda).await.unwrap().unwrap();
    let zk_job: ZkJob = BorshDeserialize::deserialize(&mut &job_account.data[..]).unwrap();
    assert_eq!(zk_job.common.status, JobStatus::Pending);

    // === Step 2: Claim Job ===
    let claim_ix = claim_job_ix(&prover.pubkey(), &job_pda);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[claim_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &prover], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Verify: Claimed, prover set
    let job_account = banks_client.get_account(job_pda).await.unwrap().unwrap();
    let zk_job: ZkJob = BorshDeserialize::deserialize(&mut &job_account.data[..]).unwrap();
    assert_eq!(zk_job.common.status, JobStatus::Claimed);
    assert_eq!(zk_job.common.prover, Some(prover.pubkey()));

    // === Step 3: Submit Proof ===
    // First fund fee_recipient for rent
    let transfer_ix = solana_sdk::system_instruction::transfer(
        &payer.pubkey(),
        &fee_recipient.pubkey(),
        1_000_000, // 0.001 SOL for rent
    );
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[transfer_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    let proof_hash = [0xCD; 32];
    let submit_ix = submit_proof_ix(
        &prover.pubkey(),
        &job_pda,
        &escrow_pda,
        &fee_recipient.pubkey(),
        proof_hash,
    );

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[submit_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &prover], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Verify: Completed, proof hash set
    let job_account = banks_client.get_account(job_pda).await.unwrap().unwrap();
    let zk_job: ZkJob = BorshDeserialize::deserialize(&mut &job_account.data[..]).unwrap();
    assert_eq!(zk_job.common.status, JobStatus::Completed);
    assert_eq!(zk_job.common.proof_hash, Some(proof_hash));

    // Verify payments
    let expected_fee = price_lamports * 250 / 10_000;

    let fee_balance = banks_client
        .get_account(fee_recipient.pubkey())
        .await
        .unwrap()
        .map(|a| a.lamports)
        .unwrap_or(0);

    assert!(
        fee_balance >= 1_000_000 + expected_fee,
        "Fee recipient should have at least {} lamports, got {}",
        1_000_000 + expected_fee,
        fee_balance
    );
}
