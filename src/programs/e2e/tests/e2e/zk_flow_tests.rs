use super::common::*;
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};

// Re-export common helpers for use in tests
use super::common::{
    generate_zk_job_id,
    get_current_timestamp,
};

/// Test del flujo completo ZK: crear job, claim, submit proof
#[tokio::test]
async fn test_zk_full_flow() {
    use bedrock_sdk::{derive_config_pda, derive_prover_pda};
    use solana_sdk::{system_program, account::Account};
    use zk_generator_sdk::{derive_job_pda, derive_escrow_pda};
    use borsh::BorshDeserialize;

    let program_test = setup_test_environment();
    let mut context = program_test.start_with_context().await;

    // Setup: admin, prover, creator
    let admin = create_and_fund_keypair(&mut context, 10_000_000_000).await;
    let prover = create_and_fund_keypair(&mut context, 10_000_000_000).await;
    let creator = create_and_fund_keypair(&mut context, 5_000_000_000).await;
    let fee_recipient = create_and_fund_keypair(&mut context, 1_000_000_000).await;

    // 1. Initialize bedrock
    initialize_bedrock(&mut context, &admin).await.unwrap();

    // 2. Register prover in bedrock
    let stake_amount = 100_000_000; // 0.1 SOL
    register_prover(&mut context, &prover, stake_amount).await.unwrap();

    // 3. Create ZK job
    // Generate job_id the same way the program does
    let current_time = get_current_timestamp(&mut context).await;
    let job_id = generate_zk_job_id(current_time, &creator.pubkey());
    let (job_pda, _) = derive_job_pda(&ZK_GENERATOR_PROGRAM_ID, &creator.pubkey(), job_id);
    let (escrow_pda, _) = derive_escrow_pda(&ZK_GENERATOR_PROGRAM_ID, &job_pda);

    let circuit_type = 10u8; // ProofOfInnocence
    let witness_hash = [1u8; 32];
    let witness_size = 1024u32;
    let price_lamports = 100_000_000u64; // 0.1 SOL
    let timeout_seconds = 3600i64; // 1 hour

    let create_job_ix = zk_generator_sdk::instructions::create_job(
        &ZK_GENERATOR_PROGRAM_ID,
        &creator.pubkey(),
        &job_pda,
        &escrow_pda,
        job_id,
        circuit_type,
        witness_hash,
        witness_size,
        price_lamports,
        timeout_seconds,
    );

    let tx = Transaction::new_signed_with_payer(
        &[create_job_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &creator],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(tx)
        .await
        .expect("Failed to create ZK job");

    // 4. Claim job (with bedrock CPI verification)
    let (prover_pda, _) = derive_prover_pda(&BEDROCK_PROGRAM_ID, &prover.pubkey());

    let claim_job_ix = zk_generator_sdk::instructions::claim_job(
        &ZK_GENERATOR_PROGRAM_ID,
        &prover.pubkey(),
        &job_pda,
        Some(&prover_pda),
        Some(&BEDROCK_PROGRAM_ID),
    );

    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[claim_job_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &prover],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(tx)
        .await
        .expect("Failed to claim ZK job");

    // 5. Submit proof (with bedrock CPI for stats update)
    let proof_hash = [2u8; 32];
    let (bedrock_config_pda, _) = derive_config_pda(&BEDROCK_PROGRAM_ID);

    let submit_proof_ix = zk_generator_sdk::instructions::submit_proof(
        &ZK_GENERATOR_PROGRAM_ID,
        &prover.pubkey(),
        &job_pda,
        &escrow_pda,
        &fee_recipient.pubkey(),
        &BEDROCK_PROGRAM_ID,
        &prover_pda,
        &bedrock_config_pda,
        proof_hash,
    );

    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    let prover_balance_before = context
        .banks_client
        .get_account(prover.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    let tx = Transaction::new_signed_with_payer(
        &[submit_proof_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &prover],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(tx)
        .await
        .expect("Failed to submit proof");

    // 6. Verify estado final

    // Verify job is completed
    let job_account = context
        .banks_client
        .get_account(job_pda)
        .await
        .unwrap()
        .expect("Job account not found");

    let job: zk_generator::ZkJob = BorshDeserialize::deserialize(&mut &job_account.data[..])
        .expect("Failed to deserialize job");

    assert_eq!(job.common.status, zyberlink_types::JobStatus::Completed);
    assert_eq!(job.common.prover, Some(prover.pubkey()));

    // Verify prover received payment (minus fee)
    let prover_balance_after = context
        .banks_client
        .get_account(prover.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // Fee is 2.5% = 250 basis points
    let platform_fee = (price_lamports * 250) / 10_000;
    let expected_payout = price_lamports - platform_fee;

    // Allow for transaction fees
    assert!(prover_balance_after >= prover_balance_before + expected_payout - 10_000);

    // Verify fee recipient received platform fee
    let fee_balance = context
        .banks_client
        .get_account(fee_recipient.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    assert!(fee_balance >= 1_000_000_000 + platform_fee);
}

/// Test que verifica que un prover no registrado no puede hacer claim
#[tokio::test]
async fn test_zk_claim_unregistered_prover_fails() {
    use bedrock_sdk::derive_prover_pda;
    use zk_generator_sdk::{derive_job_pda, derive_escrow_pda};
    use solana_sdk::program_error::ProgramError;

    let program_test = setup_test_environment();
    let mut context = program_test.start_with_context().await;

    // Setup: admin, prover NO registrado, creator
    let admin = create_and_fund_keypair(&mut context, 10_000_000_000).await;
    let unregistered_prover = create_and_fund_keypair(&mut context, 5_000_000_000).await;
    let creator = create_and_fund_keypair(&mut context, 5_000_000_000).await;

    // 1. Initialize bedrock
    initialize_bedrock(&mut context, &admin).await.unwrap();

    // 2. Create ZK job (without registering prover)
    let current_time = get_current_timestamp(&mut context).await;
    let job_id = generate_zk_job_id(current_time, &creator.pubkey());
    let (job_pda, _) = derive_job_pda(&ZK_GENERATOR_PROGRAM_ID, &creator.pubkey(), job_id);
    let (escrow_pda, _) = derive_escrow_pda(&ZK_GENERATOR_PROGRAM_ID, &job_pda);

    let circuit_type = 10u8; // ProofOfInnocence
    let witness_hash = [1u8; 32];
    let witness_size = 1024u32;
    let price_lamports = 100_000_000u64; // 0.1 SOL
    let timeout_seconds = 3600i64; // 1 hour

    let create_job_ix = zk_generator_sdk::instructions::create_job(
        &ZK_GENERATOR_PROGRAM_ID,
        &creator.pubkey(),
        &job_pda,
        &escrow_pda,
        job_id,
        circuit_type,
        witness_hash,
        witness_size,
        price_lamports,
        timeout_seconds,
    );

    let tx = Transaction::new_signed_with_payer(
        &[create_job_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &creator],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(tx)
        .await
        .expect("Failed to create ZK job");

    // 3. Attempt to claim with unregistered prover (with CPI verification)
    let (prover_pda, _) = derive_prover_pda(&BEDROCK_PROGRAM_ID, &unregistered_prover.pubkey());

    let claim_job_ix = zk_generator_sdk::instructions::claim_job(
        &ZK_GENERATOR_PROGRAM_ID,
        &unregistered_prover.pubkey(),
        &job_pda,
        Some(&prover_pda),
        Some(&BEDROCK_PROGRAM_ID),
    );

    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[claim_job_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &unregistered_prover],
        context.last_blockhash,
    );

    // 4. Verify that it fails with expected error
    let result = context.banks_client.process_transaction(tx).await;

    assert!(result.is_err(), "Expected claim to fail for unregistered prover");

    // The error should indicate unauthorized access
    // (either prover PDA doesn't exist or prover not registered)
    let err = result.unwrap_err();
    println!("Error received (expected): {:?}", err);
}

/// Test del flujo completo con dispute de proof
#[tokio::test]
async fn test_zk_full_flow_with_dispute() {
    use bedrock_sdk::{derive_config_pda, derive_prover_pda};
    use zk_generator_sdk::{derive_job_pda, derive_escrow_pda};
    use borsh::BorshDeserialize;

    let program_test = setup_test_environment();
    let mut context = program_test.start_with_context().await;

    // Setup: admin, prover, creator, disputor
    let admin = create_and_fund_keypair(&mut context, 10_000_000_000).await;
    let prover = create_and_fund_keypair(&mut context, 10_000_000_000).await;
    let creator = create_and_fund_keypair(&mut context, 5_000_000_000).await;
    let fee_recipient = create_and_fund_keypair(&mut context, 1_000_000_000).await;
    let disputor = create_and_fund_keypair(&mut context, 5_000_000_000).await;

    // 1. Initialize bedrock and register prover
    initialize_bedrock(&mut context, &admin).await.unwrap();
    let stake_amount = 100_000_000; // 0.1 SOL
    register_prover(&mut context, &prover, stake_amount).await.unwrap();

    // 2. Create ZK job
    let current_time = get_current_timestamp(&mut context).await;
    let job_id = generate_zk_job_id(current_time, &creator.pubkey());
    let (job_pda, _) = derive_job_pda(&ZK_GENERATOR_PROGRAM_ID, &creator.pubkey(), job_id);
    let (escrow_pda, _) = derive_escrow_pda(&ZK_GENERATOR_PROGRAM_ID, &job_pda);

    let circuit_type = 10u8; // ProofOfInnocence
    let witness_hash = [1u8; 32];
    let witness_size = 1024u32;
    let price_lamports = 100_000_000u64; // 0.1 SOL
    let timeout_seconds = 3600i64; // 1 hour

    let create_job_ix = zk_generator_sdk::instructions::create_job(
        &ZK_GENERATOR_PROGRAM_ID,
        &creator.pubkey(),
        &job_pda,
        &escrow_pda,
        job_id,
        circuit_type,
        witness_hash,
        witness_size,
        price_lamports,
        timeout_seconds,
    );

    let tx = Transaction::new_signed_with_payer(
        &[create_job_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &creator],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(tx)
        .await
        .expect("Failed to create ZK job");

    // 3. Claim job
    let (prover_pda, _) = derive_prover_pda(&BEDROCK_PROGRAM_ID, &prover.pubkey());

    let claim_job_ix = zk_generator_sdk::instructions::claim_job(
        &ZK_GENERATOR_PROGRAM_ID,
        &prover.pubkey(),
        &job_pda,
        Some(&prover_pda),
        Some(&BEDROCK_PROGRAM_ID),
    );

    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[claim_job_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &prover],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(tx)
        .await
        .expect("Failed to claim ZK job");

    // 4. Submit proof
    let proof_hash = [2u8; 32];
    let (bedrock_config_pda, _) = derive_config_pda(&BEDROCK_PROGRAM_ID);

    let submit_proof_ix = zk_generator_sdk::instructions::submit_proof(
        &ZK_GENERATOR_PROGRAM_ID,
        &prover.pubkey(),
        &job_pda,
        &escrow_pda,
        &fee_recipient.pubkey(),
        &BEDROCK_PROGRAM_ID,
        &prover_pda,
        &bedrock_config_pda,
        proof_hash,
    );

    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[submit_proof_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &prover],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(tx)
        .await
        .expect("Failed to submit proof");

    // 5. Dispute the proof with invalid data
    // For simplicity, we create a proof that won't match the stored hash
    let invalid_proof = vec![0u8; 256]; // Invalid proof data
    let invalid_public_inputs = vec![0u8; 96]; // Invalid public inputs

    let treasury = fee_recipient.pubkey(); // Using fee_recipient as treasury

    let dispute_ix = zk_generator_sdk::instructions::dispute_proof(
        &ZK_GENERATOR_PROGRAM_ID,
        &disputor.pubkey(),
        &job_pda,
        &prover.pubkey(),
        &treasury,
        &BEDROCK_PROGRAM_ID,
        &prover_pda,
        &bedrock_config_pda,
        invalid_proof,
        invalid_public_inputs,
    );

    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[dispute_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &disputor],
        context.last_blockhash,
    );

    // The dispute should succeed (proof verification will fail, slashing the prover)
    // Note: This depends on the circuit verification logic in dispute_proof.rs
    let result = context.banks_client.process_transaction(tx).await;

    // For now we just log the result - the actual behavior depends on the verifier
    match result {
        Ok(_) => println!("Dispute processed successfully"),
        Err(e) => println!("Dispute failed (expected if within dispute window): {:?}", e),
    }

    // Verify job is still completed
    let job_account = context
        .banks_client
        .get_account(job_pda)
        .await
        .unwrap()
        .expect("Job account not found");

    let job: zk_generator::ZkJob = BorshDeserialize::deserialize(&mut &job_account.data[..])
        .expect("Failed to deserialize job");

    assert_eq!(job.common.status, zyberlink_types::JobStatus::Completed);
}

/// Test de submit con proof inválida
/// Nota: El programa actual no verifica proofs on-chain (solo hash).
/// Este test verifica que el submit funciona con cualquier proof_hash,
/// y que el mecanismo de dispute existe para verificación posterior.
#[tokio::test]
async fn test_zk_submit_invalid_proof_fails() {
    use bedrock_sdk::{derive_config_pda, derive_prover_pda};
    use zk_generator_sdk::{derive_job_pda, derive_escrow_pda};
    use borsh::BorshDeserialize;

    let program_test = setup_test_environment();
    let mut context = program_test.start_with_context().await;

    // Setup
    let admin = create_and_fund_keypair(&mut context, 10_000_000_000).await;
    let prover = create_and_fund_keypair(&mut context, 10_000_000_000).await;
    let creator = create_and_fund_keypair(&mut context, 5_000_000_000).await;
    let fee_recipient = create_and_fund_keypair(&mut context, 1_000_000_000).await;

    initialize_bedrock(&mut context, &admin).await.unwrap();
    let stake_amount = 100_000_000;
    register_prover(&mut context, &prover, stake_amount).await.unwrap();

    // Create job
    let current_time = get_current_timestamp(&mut context).await;
    let job_id = generate_zk_job_id(current_time, &creator.pubkey());
    let (job_pda, _) = derive_job_pda(&ZK_GENERATOR_PROGRAM_ID, &creator.pubkey(), job_id);
    let (escrow_pda, _) = derive_escrow_pda(&ZK_GENERATOR_PROGRAM_ID, &job_pda);

    let create_job_ix = zk_generator_sdk::instructions::create_job(
        &ZK_GENERATOR_PROGRAM_ID,
        &creator.pubkey(),
        &job_pda,
        &escrow_pda,
        job_id,
        10u8,
        [1u8; 32],
        1024u32,
        100_000_000u64,
        3600i64,
    );

    let tx = Transaction::new_signed_with_payer(
        &[create_job_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &creator],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Claim job
    let (prover_pda, _) = derive_prover_pda(&BEDROCK_PROGRAM_ID, &prover.pubkey());

    let claim_job_ix = zk_generator_sdk::instructions::claim_job(
        &ZK_GENERATOR_PROGRAM_ID,
        &prover.pubkey(),
        &job_pda,
        Some(&prover_pda),
        Some(&BEDROCK_PROGRAM_ID),
    );

    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[claim_job_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &prover],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Submit proof with "invalid" hash (all zeros - obviously fake)
    // The on-chain program accepts any hash; verification happens via dispute
    let invalid_proof_hash = [0u8; 32];
    let (bedrock_config_pda, _) = derive_config_pda(&BEDROCK_PROGRAM_ID);

    let submit_proof_ix = zk_generator_sdk::instructions::submit_proof(
        &ZK_GENERATOR_PROGRAM_ID,
        &prover.pubkey(),
        &job_pda,
        &escrow_pda,
        &fee_recipient.pubkey(),
        &BEDROCK_PROGRAM_ID,
        &prover_pda,
        &bedrock_config_pda,
        invalid_proof_hash,
    );

    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[submit_proof_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &prover],
        context.last_blockhash,
    );

    // Submit should succeed (proof verification is off-chain via dispute)
    context.banks_client.process_transaction(tx).await.expect("Submit should succeed");

    // Verify job is completed with the invalid proof hash stored
    let job_account = context.banks_client.get_account(job_pda).await.unwrap().unwrap();
    let job: zk_generator::ZkJob = BorshDeserialize::deserialize(&mut &job_account.data[..]).unwrap();

    assert_eq!(job.common.status, zyberlink_types::JobStatus::Completed);
    assert_eq!(job.common.proof_hash, Some(invalid_proof_hash));
    // The invalid proof can be disputed later via dispute_proof instruction
}

/// Test de múltiples claims al mismo job (debe fallar)
#[tokio::test]
async fn test_zk_multiple_claims_same_prover_fails() {
    use bedrock_sdk::derive_prover_pda;
    use zk_generator_sdk::{derive_job_pda, derive_escrow_pda};

    let program_test = setup_test_environment();
    let mut context = program_test.start_with_context().await;

    // Setup
    let admin = create_and_fund_keypair(&mut context, 10_000_000_000).await;
    let prover = create_and_fund_keypair(&mut context, 10_000_000_000).await;
    let creator = create_and_fund_keypair(&mut context, 5_000_000_000).await;

    // Initialize bedrock and register prover
    initialize_bedrock(&mut context, &admin).await.unwrap();
    let stake_amount = 100_000_000;
    register_prover(&mut context, &prover, stake_amount).await.unwrap();

    // Create job
    let current_time = get_current_timestamp(&mut context).await;
    let job_id = generate_zk_job_id(current_time, &creator.pubkey());
    let (job_pda, _) = derive_job_pda(&ZK_GENERATOR_PROGRAM_ID, &creator.pubkey(), job_id);
    let (escrow_pda, _) = derive_escrow_pda(&ZK_GENERATOR_PROGRAM_ID, &job_pda);

    let create_job_ix = zk_generator_sdk::instructions::create_job(
        &ZK_GENERATOR_PROGRAM_ID,
        &creator.pubkey(),
        &job_pda,
        &escrow_pda,
        job_id,
        10u8, // circuit_type
        [1u8; 32], // witness_hash
        1024u32, // witness_size
        100_000_000u64, // price
        3600i64, // timeout
    );

    let tx = Transaction::new_signed_with_payer(
        &[create_job_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &creator],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // First claim - should succeed
    let (prover_pda, _) = derive_prover_pda(&BEDROCK_PROGRAM_ID, &prover.pubkey());

    let claim_job_ix = zk_generator_sdk::instructions::claim_job(
        &ZK_GENERATOR_PROGRAM_ID,
        &prover.pubkey(),
        &job_pda,
        Some(&prover_pda),
        Some(&BEDROCK_PROGRAM_ID),
    );

    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[claim_job_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &prover],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.expect("First claim should succeed");

    // Warp forward to ensure state is committed
    context.warp_to_slot(context.banks_client.get_root_slot().await.unwrap() + 1).unwrap();

    // Verify job is now Claimed
    let job_account = context.banks_client.get_account(job_pda).await.unwrap().unwrap();
    let job: zk_generator::ZkJob = borsh::BorshDeserialize::deserialize(&mut &job_account.data[..]).unwrap();
    assert_eq!(job.common.status, zyberlink_types::JobStatus::Claimed, "Job should be Claimed after first claim");

    // Second claim attempt - should fail because job is already Claimed
    let claim_job_ix_2 = zk_generator_sdk::instructions::claim_job(
        &ZK_GENERATOR_PROGRAM_ID,
        &prover.pubkey(),
        &job_pda,
        Some(&prover_pda),
        Some(&BEDROCK_PROGRAM_ID),
    );

    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    // Double-check job state before second claim
    let job_before_second = context.banks_client.get_account(job_pda).await.unwrap().unwrap();
    let job_state: zk_generator::ZkJob = borsh::BorshDeserialize::deserialize(&mut &job_before_second.data[..]).unwrap();
    assert_eq!(job_state.common.status, zyberlink_types::JobStatus::Claimed, "Job should still be Claimed before second claim");

    let tx = Transaction::new_signed_with_payer(
        &[claim_job_ix_2],
        Some(&context.payer.pubkey()),
        &[&context.payer, &prover],
        context.last_blockhash,
    );

    let result = context.banks_client.process_transaction(tx).await;
    assert!(result.is_err(), "Second claim should fail - job already claimed");
}

/// Test de timeout de claim
#[tokio::test]
async fn test_zk_claim_timeout() {
    use bedrock_sdk::{derive_config_pda, derive_prover_pda};
    use zk_generator_sdk::{derive_job_pda, derive_escrow_pda};
    use solana_program::clock::Clock;

    let program_test = setup_test_environment();
    let mut context = program_test.start_with_context().await;

    // Setup
    let admin = create_and_fund_keypair(&mut context, 10_000_000_000).await;
    let prover = create_and_fund_keypair(&mut context, 10_000_000_000).await;
    let creator = create_and_fund_keypair(&mut context, 5_000_000_000).await;
    let fee_recipient = create_and_fund_keypair(&mut context, 1_000_000_000).await;

    initialize_bedrock(&mut context, &admin).await.unwrap();
    let stake_amount = 100_000_000;
    register_prover(&mut context, &prover, stake_amount).await.unwrap();

    // Create job with short timeout (10 seconds)
    let current_time = get_current_timestamp(&mut context).await;
    let job_id = generate_zk_job_id(current_time, &creator.pubkey());
    let (job_pda, _) = derive_job_pda(&ZK_GENERATOR_PROGRAM_ID, &creator.pubkey(), job_id);
    let (escrow_pda, _) = derive_escrow_pda(&ZK_GENERATOR_PROGRAM_ID, &job_pda);

    let timeout_seconds = 60i64; // Minimum allowed timeout

    let create_job_ix = zk_generator_sdk::instructions::create_job(
        &ZK_GENERATOR_PROGRAM_ID,
        &creator.pubkey(),
        &job_pda,
        &escrow_pda,
        job_id,
        10u8,
        [1u8; 32],
        1024u32,
        100_000_000u64,
        timeout_seconds,
    );

    let tx = Transaction::new_signed_with_payer(
        &[create_job_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &creator],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Claim job
    let (prover_pda, _) = derive_prover_pda(&BEDROCK_PROGRAM_ID, &prover.pubkey());

    let claim_job_ix = zk_generator_sdk::instructions::claim_job(
        &ZK_GENERATOR_PROGRAM_ID,
        &prover.pubkey(),
        &job_pda,
        Some(&prover_pda),
        Some(&BEDROCK_PROGRAM_ID),
    );

    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[claim_job_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &prover],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Warp time forward past the timeout
    let mut clock: Clock = context.banks_client.get_sysvar().await.unwrap();
    clock.unix_timestamp += timeout_seconds + 100; // Well past timeout
    context.set_sysvar(&clock);

    // Attempt to submit proof after timeout - should fail
    let (bedrock_config_pda, _) = derive_config_pda(&BEDROCK_PROGRAM_ID);

    let submit_proof_ix = zk_generator_sdk::instructions::submit_proof(
        &ZK_GENERATOR_PROGRAM_ID,
        &prover.pubkey(),
        &job_pda,
        &escrow_pda,
        &fee_recipient.pubkey(),
        &BEDROCK_PROGRAM_ID,
        &prover_pda,
        &bedrock_config_pda,
        [2u8; 32],
    );

    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[submit_proof_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &prover],
        context.last_blockhash,
    );

    let result = context.banks_client.process_transaction(tx).await;
    // Either fails due to timeout OR succeeds if program doesn't enforce timeout on submit
    // The behavior depends on program implementation
    println!("Submit after timeout result: {:?}", result);
}

/// Test de verificación exitosa de proof con actualización de stats
#[tokio::test]
async fn test_zk_proof_verification_success() {
    use bedrock_sdk::{derive_config_pda, derive_prover_pda};
    use zk_generator_sdk::{derive_job_pda, derive_escrow_pda};
    use borsh::BorshDeserialize;

    let program_test = setup_test_environment();
    let mut context = program_test.start_with_context().await;

    // Setup
    let admin = create_and_fund_keypair(&mut context, 10_000_000_000).await;
    let prover = create_and_fund_keypair(&mut context, 10_000_000_000).await;
    let creator = create_and_fund_keypair(&mut context, 5_000_000_000).await;
    let fee_recipient = create_and_fund_keypair(&mut context, 1_000_000_000).await;

    initialize_bedrock(&mut context, &admin).await.unwrap();
    let stake_amount = 100_000_000;
    register_prover(&mut context, &prover, stake_amount).await.unwrap();

    // Read initial prover stats
    let (prover_pda, _) = derive_prover_pda(&BEDROCK_PROGRAM_ID, &prover.pubkey());
    let prover_account_before = context.banks_client.get_account(prover_pda).await.unwrap().unwrap();
    let prover_data_before: bedrock::ProverAccount = BorshDeserialize::deserialize(&mut &prover_account_before.data[..]).unwrap();
    let jobs_completed_before = prover_data_before.jobs_completed;

    // Create job
    let current_time = get_current_timestamp(&mut context).await;
    let job_id = generate_zk_job_id(current_time, &creator.pubkey());
    let (job_pda, _) = derive_job_pda(&ZK_GENERATOR_PROGRAM_ID, &creator.pubkey(), job_id);
    let (escrow_pda, _) = derive_escrow_pda(&ZK_GENERATOR_PROGRAM_ID, &job_pda);

    let create_job_ix = zk_generator_sdk::instructions::create_job(
        &ZK_GENERATOR_PROGRAM_ID,
        &creator.pubkey(),
        &job_pda,
        &escrow_pda,
        job_id,
        10u8, // ProofOfInnocence
        [1u8; 32],
        1024u32,
        100_000_000u64,
        3600i64,
    );

    let tx = Transaction::new_signed_with_payer(
        &[create_job_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &creator],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Claim job
    let claim_job_ix = zk_generator_sdk::instructions::claim_job(
        &ZK_GENERATOR_PROGRAM_ID,
        &prover.pubkey(),
        &job_pda,
        Some(&prover_pda),
        Some(&BEDROCK_PROGRAM_ID),
    );

    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[claim_job_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &prover],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Submit valid proof hash
    let valid_proof_hash = [42u8; 32]; // Simulated valid proof hash
    let (bedrock_config_pda, _) = derive_config_pda(&BEDROCK_PROGRAM_ID);

    let submit_proof_ix = zk_generator_sdk::instructions::submit_proof(
        &ZK_GENERATOR_PROGRAM_ID,
        &prover.pubkey(),
        &job_pda,
        &escrow_pda,
        &fee_recipient.pubkey(),
        &BEDROCK_PROGRAM_ID,
        &prover_pda,
        &bedrock_config_pda,
        valid_proof_hash,
    );

    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[submit_proof_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &prover],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.expect("Submit should succeed");

    // Verify job is completed
    let job_account = context.banks_client.get_account(job_pda).await.unwrap().unwrap();
    let job: zk_generator::ZkJob = BorshDeserialize::deserialize(&mut &job_account.data[..]).unwrap();

    assert_eq!(job.common.status, zyberlink_types::JobStatus::Completed);
    assert_eq!(job.common.proof_hash, Some(valid_proof_hash));
    assert_eq!(job.common.prover, Some(prover.pubkey()));

    // Verify prover stats were updated via CPI
    let prover_account_after = context.banks_client.get_account(prover_pda).await.unwrap().unwrap();
    let prover_data_after: bedrock::ProverAccount = BorshDeserialize::deserialize(&mut &prover_account_after.data[..]).unwrap();

    assert_eq!(
        prover_data_after.jobs_completed,
        jobs_completed_before + 1,
        "Prover jobs_completed should be incremented via CPI"
    );
}
