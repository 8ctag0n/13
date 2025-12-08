//! Integration tests for FHE Generator using solana-program-test

use borsh::BorshDeserialize;
use solana_program::pubkey::Pubkey;
use solana_program_test::{processor, ProgramTest};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    signature::{Keypair, Signer},
    system_program,
    transaction::Transaction,
};
use fhe_generator::{
    process_instruction, FheGeneratorInstruction, FheJob, FheConsensusData,
    FHE_JOB_SEED, FHE_CONSENSUS_SEED, CIRCUIT_FHE_ADD,
};
use zyberlink_types::JobStatus;

/// Escrow seed (must match the one in the program)
const ESCROW_SEED: &[u8] = b"fhe_escrow";

/// Create a ProgramTest instance with FHE Generator loaded
fn program_test() -> ProgramTest {
    let mut pt = ProgramTest::new(
        "fhe_generator",
        fhe_generator_program_id(),
        processor!(process_instruction),
    );
    pt.prefer_bpf(false);
    pt
}

/// Get FHE Generator program ID (deterministic for tests)
fn fhe_generator_program_id() -> Pubkey {
    Pubkey::new_from_array([
        0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f,
        0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e,
        0x3f, 0x40,
    ])
}

/// Helper to derive job PDA
fn derive_job_pda(creator: &Pubkey, job_id: u64, program_id: &Pubkey) -> (Pubkey, u8) {
    let job_id_bytes = job_id.to_le_bytes();
    Pubkey::find_program_address(&[FHE_JOB_SEED, creator.as_ref(), &job_id_bytes], program_id)
}

/// Helper to derive consensus PDA
fn derive_consensus_pda(job_id: u64, program_id: &Pubkey) -> (Pubkey, u8) {
    let job_id_bytes = job_id.to_le_bytes();
    Pubkey::find_program_address(&[FHE_CONSENSUS_SEED, &job_id_bytes], program_id)
}

/// Helper to derive escrow PDA
fn derive_escrow_pda(job_pda: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[ESCROW_SEED, job_pda.as_ref()], program_id)
}

/// Helper to create CreateJob instruction
#[allow(clippy::too_many_arguments)]
fn create_job_ix(
    creator: &Pubkey,
    job_pda: &Pubkey,
    consensus_pda: &Pubkey,
    escrow_pda: &Pubkey,
    circuit_type: u8,
    witness_hash: [u8; 32],
    witness_size: u32,
    price_lamports: u64,
    timeout_seconds: i64,
    required_provers: u8,
    consensus_threshold: u8,
) -> Instruction {
    let data = borsh::to_vec(&FheGeneratorInstruction::CreateJob {
        circuit_type,
        witness_hash,
        witness_size,
        price_lamports,
        timeout_seconds,
        required_provers,
        consensus_threshold,
        operation_param1: 0,
        operation_param2: 0,
        operation_param3: 0,
    })
    .unwrap();

    Instruction {
        program_id: fhe_generator_program_id(),
        accounts: vec![
            AccountMeta::new(*creator, true),
            AccountMeta::new(*job_pda, false),
            AccountMeta::new(*consensus_pda, false),
            AccountMeta::new(*escrow_pda, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data,
    }
}

/// Helper to create ClaimJob instruction
fn claim_job_ix(prover: &Pubkey, job_pda: &Pubkey, consensus_pda: &Pubkey) -> Instruction {
    let data = borsh::to_vec(&FheGeneratorInstruction::ClaimJob).unwrap();

    Instruction {
        program_id: fhe_generator_program_id(),
        accounts: vec![
            AccountMeta::new(*prover, true),
            AccountMeta::new(*job_pda, false),
            AccountMeta::new(*consensus_pda, false),
        ],
        data,
    }
}

/// Helper to create SubmitResult instruction
fn submit_result_ix(
    prover: &Pubkey,
    job_pda: &Pubkey,
    consensus_pda: &Pubkey,
    result_hash: [u8; 32],
) -> Instruction {
    let data = borsh::to_vec(&FheGeneratorInstruction::SubmitResult { result_hash }).unwrap();

    Instruction {
        program_id: fhe_generator_program_id(),
        accounts: vec![
            AccountMeta::new(*prover, true),
            AccountMeta::new_readonly(*job_pda, false),
            AccountMeta::new(*consensus_pda, false),
        ],
        data,
    }
}

/// Helper to create FinalizeJob instruction
fn finalize_job_ix(
    finalizer: &Pubkey,
    job_pda: &Pubkey,
    consensus_pda: &Pubkey,
    escrow_pda: &Pubkey,
    creator: &Pubkey,
    fee_recipient: &Pubkey,
    provers: &[Pubkey],
) -> Instruction {
    let data = borsh::to_vec(&FheGeneratorInstruction::FinalizeJob).unwrap();

    let mut accounts = vec![
        AccountMeta::new(*finalizer, true),
        AccountMeta::new(*job_pda, false),
        AccountMeta::new(*consensus_pda, false),
        AccountMeta::new(*escrow_pda, false),
        AccountMeta::new(*creator, false),
        AccountMeta::new(*fee_recipient, false),
    ];

    for prover in provers {
        accounts.push(AccountMeta::new(*prover, false));
    }

    Instruction {
        program_id: fhe_generator_program_id(),
        accounts,
        data,
    }
}

/// Helper to create CancelJob instruction
fn cancel_job_ix(
    creator: &Pubkey,
    job_pda: &Pubkey,
    consensus_pda: &Pubkey,
    escrow_pda: &Pubkey,
) -> Instruction {
    let data = borsh::to_vec(&FheGeneratorInstruction::CancelJob).unwrap();

    Instruction {
        program_id: fhe_generator_program_id(),
        accounts: vec![
            AccountMeta::new(*creator, true),
            AccountMeta::new(*job_pda, false),
            AccountMeta::new(*consensus_pda, false),
            AccountMeta::new(*escrow_pda, false),
        ],
        data,
    }
}

// =============================================================================
// CreateJob Tests
// =============================================================================

#[tokio::test]
async fn test_create_fhe_job() {
    let pt = program_test();
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let program_id = fhe_generator_program_id();

    // Job parameters
    let circuit_type = CIRCUIT_FHE_ADD;
    let witness_hash = [1u8; 32];
    let witness_size = 2048u32;
    let price_lamports = 10_000_000u64;
    let timeout_seconds = 300i64;
    let required_provers = 2u8;
    let consensus_threshold = 2u8;

    // Get current time to predict job_id
    let clock = banks_client.get_sysvar::<solana_sdk::sysvar::clock::Clock>().await.unwrap();
    let job_id = (clock.unix_timestamp as u64) ^ (payer.pubkey().to_bytes()[0] as u64);

    // Derive PDAs
    let (job_pda, _) = derive_job_pda(&payer.pubkey(), job_id, &program_id);
    let (consensus_pda, _) = derive_consensus_pda(job_id, &program_id);
    let (escrow_pda, _) = derive_escrow_pda(&job_pda, &program_id);

    // Create instruction
    let ix = create_job_ix(
        &payer.pubkey(),
        &job_pda,
        &consensus_pda,
        &escrow_pda,
        circuit_type,
        witness_hash,
        witness_size,
        price_lamports,
        timeout_seconds,
        required_provers,
        consensus_threshold,
    );

    // Send transaction
    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);

    banks_client.process_transaction(tx).await.unwrap();

    // Verify job was created
    let job_account = banks_client.get_account(job_pda).await.unwrap().unwrap();
    let fhe_job: FheJob = BorshDeserialize::deserialize(&mut &job_account.data[..]).unwrap();

    assert_eq!(fhe_job.circuit_type, circuit_type);
    assert_eq!(fhe_job.common.creator, payer.pubkey());
    assert_eq!(fhe_job.common.status, JobStatus::Pending);
    assert_eq!(fhe_job.common.price_lamports, price_lamports);

    // Verify consensus was created
    let consensus_account = banks_client.get_account(consensus_pda).await.unwrap().unwrap();
    let consensus: FheConsensusData =
        BorshDeserialize::deserialize(&mut &consensus_account.data[..]).unwrap();

    assert_eq!(consensus.required_provers, required_provers);
    assert_eq!(consensus.consensus_threshold, consensus_threshold);
    assert_eq!(consensus.claimed_count, 0);

    // Verify escrow has funds
    let escrow_account = banks_client.get_account(escrow_pda).await.unwrap().unwrap();
    assert!(escrow_account.lamports >= price_lamports);
}

#[tokio::test]
async fn test_create_fhe_job_invalid_circuit() {
    let pt = program_test();
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let program_id = fhe_generator_program_id();

    // Invalid circuit type (3 is not FHE, should be 4-11)
    let circuit_type = 3u8;

    let clock = banks_client.get_sysvar::<solana_sdk::sysvar::clock::Clock>().await.unwrap();
    let job_id = (clock.unix_timestamp as u64) ^ (payer.pubkey().to_bytes()[0] as u64);

    let (job_pda, _) = derive_job_pda(&payer.pubkey(), job_id, &program_id);
    let (consensus_pda, _) = derive_consensus_pda(job_id, &program_id);
    let (escrow_pda, _) = derive_escrow_pda(&job_pda, &program_id);

    let ix = create_job_ix(
        &payer.pubkey(),
        &job_pda,
        &consensus_pda,
        &escrow_pda,
        circuit_type,
        [1u8; 32],
        2048,
        10_000_000,
        300,
        2,
        2,
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
async fn test_claim_fhe_job_multi_prover() {
    let pt = program_test();
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let program_id = fhe_generator_program_id();
    let prover1 = Keypair::new();
    let prover2 = Keypair::new();

    let required_provers = 2u8;

    // Create job
    let clock = banks_client.get_sysvar::<solana_sdk::sysvar::clock::Clock>().await.unwrap();
    let job_id = (clock.unix_timestamp as u64) ^ (payer.pubkey().to_bytes()[0] as u64);

    let (job_pda, _) = derive_job_pda(&payer.pubkey(), job_id, &program_id);
    let (consensus_pda, _) = derive_consensus_pda(job_id, &program_id);
    let (escrow_pda, _) = derive_escrow_pda(&job_pda, &program_id);

    let create_ix = create_job_ix(
        &payer.pubkey(),
        &job_pda,
        &consensus_pda,
        &escrow_pda,
        CIRCUIT_FHE_ADD,
        [1u8; 32],
        2048,
        10_000_000,
        300,
        required_provers,
        2,
    );

    let mut tx = Transaction::new_with_payer(&[create_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // First prover claims
    let claim_ix1 = claim_job_ix(&prover1.pubkey(), &job_pda, &consensus_pda);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[claim_ix1], Some(&payer.pubkey()));
    tx.sign(&[&payer, &prover1], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Verify partial claim
    let consensus_account = banks_client.get_account(consensus_pda).await.unwrap().unwrap();
    let consensus: FheConsensusData =
        BorshDeserialize::deserialize(&mut &consensus_account.data[..]).unwrap();
    assert_eq!(consensus.claimed_count, 1);

    let job_account = banks_client.get_account(job_pda).await.unwrap().unwrap();
    let fhe_job: FheJob = BorshDeserialize::deserialize(&mut &job_account.data[..]).unwrap();
    assert_eq!(fhe_job.common.status, JobStatus::Pending); // Still pending

    // Second prover claims
    let claim_ix2 = claim_job_ix(&prover2.pubkey(), &job_pda, &consensus_pda);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[claim_ix2], Some(&payer.pubkey()));
    tx.sign(&[&payer, &prover2], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Verify fully claimed
    let consensus_account = banks_client.get_account(consensus_pda).await.unwrap().unwrap();
    let consensus: FheConsensusData =
        BorshDeserialize::deserialize(&mut &consensus_account.data[..]).unwrap();
    assert_eq!(consensus.claimed_count, 2);
    assert!(consensus.is_fully_claimed());

    let job_account = banks_client.get_account(job_pda).await.unwrap().unwrap();
    let fhe_job: FheJob = BorshDeserialize::deserialize(&mut &job_account.data[..]).unwrap();
    assert_eq!(fhe_job.common.status, JobStatus::Claimed); // Now claimed
}

#[tokio::test]
async fn test_claim_fhe_job_prover_already_claimed() {
    let pt = program_test();
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let program_id = fhe_generator_program_id();
    let prover = Keypair::new();

    // Create job
    let clock = banks_client.get_sysvar::<solana_sdk::sysvar::clock::Clock>().await.unwrap();
    let job_id = (clock.unix_timestamp as u64) ^ (payer.pubkey().to_bytes()[0] as u64);

    let (job_pda, _) = derive_job_pda(&payer.pubkey(), job_id, &program_id);
    let (consensus_pda, _) = derive_consensus_pda(job_id, &program_id);
    let (escrow_pda, _) = derive_escrow_pda(&job_pda, &program_id);

    let create_ix = create_job_ix(
        &payer.pubkey(),
        &job_pda,
        &consensus_pda,
        &escrow_pda,
        CIRCUIT_FHE_ADD,
        [1u8; 32],
        2048,
        10_000_000,
        300,
        2,
        2,
    );

    let mut tx = Transaction::new_with_payer(&[create_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // First claim
    let claim_ix = claim_job_ix(&prover.pubkey(), &job_pda, &consensus_pda);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[claim_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &prover], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Try to claim again - should fail
    let claim_ix2 = claim_job_ix(&prover.pubkey(), &job_pda, &consensus_pda);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[claim_ix2], Some(&payer.pubkey()));
    tx.sign(&[&payer, &prover], recent_blockhash);

    let result = banks_client.process_transaction(tx).await;
    assert!(result.is_err());
}

// =============================================================================
// SubmitResult Tests
// =============================================================================

#[tokio::test]
async fn test_submit_result() {
    let pt = program_test();
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let program_id = fhe_generator_program_id();
    let prover1 = Keypair::new();
    let prover2 = Keypair::new();

    // Create job with 2 provers
    let clock = banks_client.get_sysvar::<solana_sdk::sysvar::clock::Clock>().await.unwrap();
    let job_id = (clock.unix_timestamp as u64) ^ (payer.pubkey().to_bytes()[0] as u64);

    let (job_pda, _) = derive_job_pda(&payer.pubkey(), job_id, &program_id);
    let (consensus_pda, _) = derive_consensus_pda(job_id, &program_id);
    let (escrow_pda, _) = derive_escrow_pda(&job_pda, &program_id);

    let create_ix = create_job_ix(
        &payer.pubkey(),
        &job_pda,
        &consensus_pda,
        &escrow_pda,
        CIRCUIT_FHE_ADD,
        [1u8; 32],
        2048,
        10_000_000,
        300,
        2,
        2,
    );

    let mut tx = Transaction::new_with_payer(&[create_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Both provers claim
    let claim_ix1 = claim_job_ix(&prover1.pubkey(), &job_pda, &consensus_pda);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[claim_ix1], Some(&payer.pubkey()));
    tx.sign(&[&payer, &prover1], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    let claim_ix2 = claim_job_ix(&prover2.pubkey(), &job_pda, &consensus_pda);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[claim_ix2], Some(&payer.pubkey()));
    tx.sign(&[&payer, &prover2], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Prover1 submits result
    let result_hash = [0xAB; 32];
    let submit_ix = submit_result_ix(&prover1.pubkey(), &job_pda, &consensus_pda, result_hash);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[submit_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &prover1], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Verify result submitted
    let consensus_account = banks_client.get_account(consensus_pda).await.unwrap().unwrap();
    let consensus: FheConsensusData =
        BorshDeserialize::deserialize(&mut &consensus_account.data[..]).unwrap();

    assert_eq!(consensus.results_count, 1);
    assert!(consensus.result_submitted[0]);
    assert_eq!(consensus.result_hashes[0], result_hash);
}

// =============================================================================
// FinalizeJob Tests
// =============================================================================

#[tokio::test]
async fn test_finalize_job_with_consensus() {
    let pt = program_test();
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let program_id = fhe_generator_program_id();
    let prover1 = Keypair::new();
    let prover2 = Keypair::new();
    let fee_recipient = Keypair::new();

    let price_lamports = 10_000_000u64;

    // Create job
    let clock = banks_client.get_sysvar::<solana_sdk::sysvar::clock::Clock>().await.unwrap();
    let job_id = (clock.unix_timestamp as u64) ^ (payer.pubkey().to_bytes()[0] as u64);

    let (job_pda, _) = derive_job_pda(&payer.pubkey(), job_id, &program_id);
    let (consensus_pda, _) = derive_consensus_pda(job_id, &program_id);
    let (escrow_pda, _) = derive_escrow_pda(&job_pda, &program_id);

    let create_ix = create_job_ix(
        &payer.pubkey(),
        &job_pda,
        &consensus_pda,
        &escrow_pda,
        CIRCUIT_FHE_ADD,
        [1u8; 32],
        2048,
        price_lamports,
        300,
        2,
        2,
    );

    let mut tx = Transaction::new_with_payer(&[create_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Both provers claim
    let claim_ix1 = claim_job_ix(&prover1.pubkey(), &job_pda, &consensus_pda);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[claim_ix1], Some(&payer.pubkey()));
    tx.sign(&[&payer, &prover1], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    let claim_ix2 = claim_job_ix(&prover2.pubkey(), &job_pda, &consensus_pda);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[claim_ix2], Some(&payer.pubkey()));
    tx.sign(&[&payer, &prover2], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Both provers submit SAME result (consensus!)
    let result_hash = [0xAB; 32];

    let submit_ix1 = submit_result_ix(&prover1.pubkey(), &job_pda, &consensus_pda, result_hash);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[submit_ix1], Some(&payer.pubkey()));
    tx.sign(&[&payer, &prover1], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    let submit_ix2 = submit_result_ix(&prover2.pubkey(), &job_pda, &consensus_pda, result_hash);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[submit_ix2], Some(&payer.pubkey()));
    tx.sign(&[&payer, &prover2], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Fund fee_recipient and provers for rent
    let transfer_ix = solana_sdk::system_instruction::transfer(
        &payer.pubkey(),
        &fee_recipient.pubkey(),
        1_000_000,
    );
    let transfer_ix2 = solana_sdk::system_instruction::transfer(
        &payer.pubkey(),
        &prover1.pubkey(),
        1_000_000,
    );
    let transfer_ix3 = solana_sdk::system_instruction::transfer(
        &payer.pubkey(),
        &prover2.pubkey(),
        1_000_000,
    );
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(
        &[transfer_ix, transfer_ix2, transfer_ix3],
        Some(&payer.pubkey()),
    );
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Finalize
    let finalize_ix = finalize_job_ix(
        &payer.pubkey(),
        &job_pda,
        &consensus_pda,
        &escrow_pda,
        &payer.pubkey(),
        &fee_recipient.pubkey(),
        &[prover1.pubkey(), prover2.pubkey()],
    );

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[finalize_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Verify job completed
    let job_account = banks_client.get_account(job_pda).await.unwrap().unwrap();
    let fhe_job: FheJob = BorshDeserialize::deserialize(&mut &job_account.data[..]).unwrap();
    assert_eq!(fhe_job.common.status, JobStatus::Completed);
    assert_eq!(fhe_job.common.proof_hash, Some(result_hash));

    // Verify consensus achieved
    let consensus_account = banks_client.get_account(consensus_pda).await.unwrap().unwrap();
    let consensus: FheConsensusData =
        BorshDeserialize::deserialize(&mut &consensus_account.data[..]).unwrap();
    assert!(consensus.has_consensus());
    assert_eq!(consensus.consensus_hash, Some(result_hash));
}

#[tokio::test]
async fn test_finalize_job_no_consensus() {
    let pt = program_test();
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let program_id = fhe_generator_program_id();
    let prover1 = Keypair::new();
    let prover2 = Keypair::new();
    let fee_recipient = Keypair::new();

    let price_lamports = 10_000_000u64;

    // Create job
    let clock = banks_client.get_sysvar::<solana_sdk::sysvar::clock::Clock>().await.unwrap();
    let job_id = (clock.unix_timestamp as u64) ^ (payer.pubkey().to_bytes()[0] as u64);

    let (job_pda, _) = derive_job_pda(&payer.pubkey(), job_id, &program_id);
    let (consensus_pda, _) = derive_consensus_pda(job_id, &program_id);
    let (escrow_pda, _) = derive_escrow_pda(&job_pda, &program_id);

    let create_ix = create_job_ix(
        &payer.pubkey(),
        &job_pda,
        &consensus_pda,
        &escrow_pda,
        CIRCUIT_FHE_ADD,
        [1u8; 32],
        2048,
        price_lamports,
        300,
        2,
        2, // Need both to agree
    );

    let mut tx = Transaction::new_with_payer(&[create_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Get creator balance before
    let creator_balance_before = banks_client
        .get_account(payer.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // Both provers claim
    let claim_ix1 = claim_job_ix(&prover1.pubkey(), &job_pda, &consensus_pda);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[claim_ix1], Some(&payer.pubkey()));
    tx.sign(&[&payer, &prover1], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    let claim_ix2 = claim_job_ix(&prover2.pubkey(), &job_pda, &consensus_pda);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[claim_ix2], Some(&payer.pubkey()));
    tx.sign(&[&payer, &prover2], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Provers submit DIFFERENT results (no consensus!)
    let result_hash1 = [0xAB; 32];
    let result_hash2 = [0xCD; 32]; // Different!

    let submit_ix1 = submit_result_ix(&prover1.pubkey(), &job_pda, &consensus_pda, result_hash1);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[submit_ix1], Some(&payer.pubkey()));
    tx.sign(&[&payer, &prover1], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    let submit_ix2 = submit_result_ix(&prover2.pubkey(), &job_pda, &consensus_pda, result_hash2);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[submit_ix2], Some(&payer.pubkey()));
    tx.sign(&[&payer, &prover2], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Fund fee_recipient and provers for rent
    let transfer_ix = solana_sdk::system_instruction::transfer(
        &payer.pubkey(),
        &fee_recipient.pubkey(),
        1_000_000,
    );
    let transfer_ix2 = solana_sdk::system_instruction::transfer(
        &payer.pubkey(),
        &prover1.pubkey(),
        1_000_000,
    );
    let transfer_ix3 = solana_sdk::system_instruction::transfer(
        &payer.pubkey(),
        &prover2.pubkey(),
        1_000_000,
    );
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(
        &[transfer_ix, transfer_ix2, transfer_ix3],
        Some(&payer.pubkey()),
    );
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    let creator_balance_mid = banks_client
        .get_account(payer.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // Finalize
    let finalize_ix = finalize_job_ix(
        &payer.pubkey(),
        &job_pda,
        &consensus_pda,
        &escrow_pda,
        &payer.pubkey(),
        &fee_recipient.pubkey(),
        &[prover1.pubkey(), prover2.pubkey()],
    );

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[finalize_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Verify job failed
    let job_account = banks_client.get_account(job_pda).await.unwrap().unwrap();
    let fhe_job: FheJob = BorshDeserialize::deserialize(&mut &job_account.data[..]).unwrap();
    assert_eq!(fhe_job.common.status, JobStatus::Failed);

    // Verify creator got refund
    let creator_balance_after = banks_client
        .get_account(payer.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    assert!(
        creator_balance_after > creator_balance_mid,
        "Creator should receive refund"
    );
}

// =============================================================================
// CancelJob Tests
// =============================================================================

#[tokio::test]
async fn test_cancel_fhe_job() {
    let pt = program_test();
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let program_id = fhe_generator_program_id();

    // Create job
    let clock = banks_client.get_sysvar::<solana_sdk::sysvar::clock::Clock>().await.unwrap();
    let job_id = (clock.unix_timestamp as u64) ^ (payer.pubkey().to_bytes()[0] as u64);

    let (job_pda, _) = derive_job_pda(&payer.pubkey(), job_id, &program_id);
    let (consensus_pda, _) = derive_consensus_pda(job_id, &program_id);
    let (escrow_pda, _) = derive_escrow_pda(&job_pda, &program_id);

    let price_lamports = 10_000_000u64;

    let create_ix = create_job_ix(
        &payer.pubkey(),
        &job_pda,
        &consensus_pda,
        &escrow_pda,
        CIRCUIT_FHE_ADD,
        [1u8; 32],
        2048,
        price_lamports,
        300,
        2,
        2,
    );

    let mut tx = Transaction::new_with_payer(&[create_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Cancel
    let cancel_ix = cancel_job_ix(&payer.pubkey(), &job_pda, &consensus_pda, &escrow_pda);

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[cancel_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Verify cancelled
    let job_account = banks_client.get_account(job_pda).await.unwrap().unwrap();
    let fhe_job: FheJob = BorshDeserialize::deserialize(&mut &job_account.data[..]).unwrap();
    assert_eq!(fhe_job.common.status, JobStatus::Cancelled);
}

#[tokio::test]
async fn test_cancel_fhe_job_after_results_submitted() {
    let pt = program_test();
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let program_id = fhe_generator_program_id();
    let prover = Keypair::new();

    // Create job
    let clock = banks_client.get_sysvar::<solana_sdk::sysvar::clock::Clock>().await.unwrap();
    let job_id = (clock.unix_timestamp as u64) ^ (payer.pubkey().to_bytes()[0] as u64);

    let (job_pda, _) = derive_job_pda(&payer.pubkey(), job_id, &program_id);
    let (consensus_pda, _) = derive_consensus_pda(job_id, &program_id);
    let (escrow_pda, _) = derive_escrow_pda(&job_pda, &program_id);

    let create_ix = create_job_ix(
        &payer.pubkey(),
        &job_pda,
        &consensus_pda,
        &escrow_pda,
        CIRCUIT_FHE_ADD,
        [1u8; 32],
        2048,
        10_000_000,
        300,
        2,
        2,
    );

    let mut tx = Transaction::new_with_payer(&[create_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Prover claims
    let claim_ix = claim_job_ix(&prover.pubkey(), &job_pda, &consensus_pda);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[claim_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &prover], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Prover submits result
    let submit_ix = submit_result_ix(&prover.pubkey(), &job_pda, &consensus_pda, [0xAB; 32]);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[submit_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &prover], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Try to cancel - should fail (results submitted)
    let cancel_ix = cancel_job_ix(&payer.pubkey(), &job_pda, &consensus_pda, &escrow_pda);
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
async fn test_full_fhe_flow_with_consensus() {
    let pt = program_test();
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let program_id = fhe_generator_program_id();
    let prover1 = Keypair::new();
    let prover2 = Keypair::new();
    let prover3 = Keypair::new();
    let fee_recipient = Keypair::new();

    let price_lamports = 30_000_000u64; // 0.03 SOL

    // === Step 1: Create Job ===
    let clock = banks_client.get_sysvar::<solana_sdk::sysvar::clock::Clock>().await.unwrap();
    let job_id = (clock.unix_timestamp as u64) ^ (payer.pubkey().to_bytes()[0] as u64);

    let (job_pda, _) = derive_job_pda(&payer.pubkey(), job_id, &program_id);
    let (consensus_pda, _) = derive_consensus_pda(job_id, &program_id);
    let (escrow_pda, _) = derive_escrow_pda(&job_pda, &program_id);

    let create_ix = create_job_ix(
        &payer.pubkey(),
        &job_pda,
        &consensus_pda,
        &escrow_pda,
        CIRCUIT_FHE_ADD,
        [0xAB; 32],
        4096,
        price_lamports,
        600,
        3, // 3 provers required
        2, // 2 must agree
    );

    let mut tx = Transaction::new_with_payer(&[create_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // === Step 2: All 3 provers claim ===
    for prover in [&prover1, &prover2, &prover3] {
        let claim_ix = claim_job_ix(&prover.pubkey(), &job_pda, &consensus_pda);
        let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
        let mut tx = Transaction::new_with_payer(&[claim_ix], Some(&payer.pubkey()));
        tx.sign(&[&payer, prover], recent_blockhash);
        banks_client.process_transaction(tx).await.unwrap();
    }

    // === Step 3: Submit results (2 agree, 1 different) ===
    let correct_result = [0xCD; 32];
    let wrong_result = [0xEF; 32];

    // Prover1 and Prover2 submit same result
    let submit_ix1 = submit_result_ix(&prover1.pubkey(), &job_pda, &consensus_pda, correct_result);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[submit_ix1], Some(&payer.pubkey()));
    tx.sign(&[&payer, &prover1], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    let submit_ix2 = submit_result_ix(&prover2.pubkey(), &job_pda, &consensus_pda, correct_result);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[submit_ix2], Some(&payer.pubkey()));
    tx.sign(&[&payer, &prover2], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Prover3 submits different result
    let submit_ix3 = submit_result_ix(&prover3.pubkey(), &job_pda, &consensus_pda, wrong_result);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[submit_ix3], Some(&payer.pubkey()));
    tx.sign(&[&payer, &prover3], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // === Step 4: Fund accounts and Finalize ===
    let transfer_ix = solana_sdk::system_instruction::transfer(
        &payer.pubkey(),
        &fee_recipient.pubkey(),
        1_000_000,
    );
    let transfer_ix2 = solana_sdk::system_instruction::transfer(
        &payer.pubkey(),
        &prover1.pubkey(),
        1_000_000,
    );
    let transfer_ix3 = solana_sdk::system_instruction::transfer(
        &payer.pubkey(),
        &prover2.pubkey(),
        1_000_000,
    );
    let transfer_ix4 = solana_sdk::system_instruction::transfer(
        &payer.pubkey(),
        &prover3.pubkey(),
        1_000_000,
    );
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(
        &[transfer_ix, transfer_ix2, transfer_ix3, transfer_ix4],
        Some(&payer.pubkey()),
    );
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    let prover1_balance_before = banks_client
        .get_account(prover1.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let prover3_balance_before = banks_client
        .get_account(prover3.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    let finalize_ix = finalize_job_ix(
        &payer.pubkey(),
        &job_pda,
        &consensus_pda,
        &escrow_pda,
        &payer.pubkey(),
        &fee_recipient.pubkey(),
        &[prover1.pubkey(), prover2.pubkey(), prover3.pubkey()],
    );

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[finalize_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // === Verify ===
    let job_account = banks_client.get_account(job_pda).await.unwrap().unwrap();
    let fhe_job: FheJob = BorshDeserialize::deserialize(&mut &job_account.data[..]).unwrap();
    assert_eq!(fhe_job.common.status, JobStatus::Completed);
    assert_eq!(fhe_job.common.proof_hash, Some(correct_result));

    // Prover1 and Prover2 should have received payout
    let prover1_balance_after = banks_client
        .get_account(prover1.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    assert!(
        prover1_balance_after > prover1_balance_before,
        "Prover1 (matching) should receive payment"
    );

    // Prover3 should NOT have received payout (mismatching)
    let prover3_balance_after = banks_client
        .get_account(prover3.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    assert_eq!(
        prover3_balance_after, prover3_balance_before,
        "Prover3 (mismatching) should NOT receive payment"
    );
}
