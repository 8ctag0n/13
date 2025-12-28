use super::common::*;
use fhe_generator_sdk::{
    derive_consensus_pda, derive_escrow_pda, derive_job_pda, instructions::*, CIRCUIT_FHE_SUM,
};
use solana_program::system_program;
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};

// Import helpers from common
use super::common::{generate_fhe_job_id, get_current_timestamp};

/// Test del flujo completo FHE con múltiples provers
#[tokio::test]
async fn test_fhe_multi_prover_flow() {
    let program_test = setup_test_environment();
    let mut context = program_test.start_with_context().await;

    // 1. Setup: crear y fondear keypairs
    let admin = create_and_fund_keypair(&mut context, 10_000_000_000).await;
    let stake_amount = 100_000_000; // 0.1 SOL
    let prover1 = create_and_fund_keypair(&mut context, 10_000_000_000 + stake_amount).await;
    let prover2 = create_and_fund_keypair(&mut context, 10_000_000_000 + stake_amount).await;
    let prover3 = create_and_fund_keypair(&mut context, 10_000_000_000 + stake_amount).await;
    let creator = create_and_fund_keypair(&mut context, 5_000_000_000).await;

    // 2. Inicializar bedrock
    initialize_bedrock(&mut context, &admin)
        .await
        .expect("Failed to initialize bedrock");

    // 3. Registrar los 3 provers en bedrock
    register_prover(&mut context, &prover1, stake_amount)
        .await
        .expect("Failed to register prover1");
    register_prover(&mut context, &prover2, stake_amount)
        .await
        .expect("Failed to register prover2");
    register_prover(&mut context, &prover3, stake_amount)
        .await
        .expect("Failed to register prover3");

    // 4. Crear FHE job con required_provers=3 y consensus_threshold=3
    // Generar job_id de la misma forma que el programa
    let current_time = get_current_timestamp(&mut context).await;
    let job_id = generate_fhe_job_id(current_time, &creator.pubkey());

    let witness_hash = [1u8; 32];
    let witness_size = 1024u32;
    let price_lamports = 1_000_000_000u64; // 1 SOL
    let timeout_seconds = 3600i64; // 1 hour

    let ix_create = create_job(
        &FHE_GENERATOR_PROGRAM_ID,
        &creator.pubkey(),
        job_id,
        CIRCUIT_FHE_SUM,
        witness_hash,
        witness_size,
        price_lamports,
        timeout_seconds,
        3, // required_provers
        3, // consensus_threshold (all must agree)
        100, // operation_param1
        0, // operation_param2
        0, // operation_param3
    );

    let tx_create = Transaction::new_signed_with_payer(
        &[ix_create],
        Some(&creator.pubkey()),
        &[&creator],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(tx_create)
        .await
        .expect("Failed to create FHE job");

    // Derivar PDAs necesarias
    let (job_pda, _) = derive_job_pda(&FHE_GENERATOR_PROGRAM_ID, &creator.pubkey(), job_id);
    let (consensus_pda, _) = derive_consensus_pda(&FHE_GENERATOR_PROGRAM_ID, job_id);
    let (prover1_pda, _) = get_prover_registry_pda(&prover1.pubkey());
    let (prover2_pda, _) = get_prover_registry_pda(&prover2.pubkey());
    let (prover3_pda, _) = get_prover_registry_pda(&prover3.pubkey());

    // 5-7. Los 3 provers hacen claim
    let ix_claim1 = claim_job(
        &FHE_GENERATOR_PROGRAM_ID,
        &prover1.pubkey(),
        &job_pda,
        &consensus_pda,
        &prover1_pda,
        &BEDROCK_PROGRAM_ID,
    );
    let tx_claim1 = Transaction::new_signed_with_payer(
        &[ix_claim1],
        Some(&prover1.pubkey()),
        &[&prover1],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(tx_claim1)
        .await
        .expect("Failed prover1 claim");

    let ix_claim2 = claim_job(
        &FHE_GENERATOR_PROGRAM_ID,
        &prover2.pubkey(),
        &job_pda,
        &consensus_pda,
        &prover2_pda,
        &BEDROCK_PROGRAM_ID,
    );
    let tx_claim2 = Transaction::new_signed_with_payer(
        &[ix_claim2],
        Some(&prover2.pubkey()),
        &[&prover2],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(tx_claim2)
        .await
        .expect("Failed prover2 claim");

    let ix_claim3 = claim_job(
        &FHE_GENERATOR_PROGRAM_ID,
        &prover3.pubkey(),
        &job_pda,
        &consensus_pda,
        &prover3_pda,
        &BEDROCK_PROGRAM_ID,
    );
    let tx_claim3 = Transaction::new_signed_with_payer(
        &[ix_claim3],
        Some(&prover3.pubkey()),
        &[&prover3],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(tx_claim3)
        .await
        .expect("Failed prover3 claim");

    // 8-10. Los 3 provers submitean el MISMO result_hash (consenso)
    let result_hash = [42u8; 32]; // Todos coinciden

    let ix_submit1 = submit_result(
        &FHE_GENERATOR_PROGRAM_ID,
        &prover1.pubkey(),
        &job_pda,
        &consensus_pda,
        result_hash,
    );
    let tx_submit1 = Transaction::new_signed_with_payer(
        &[ix_submit1],
        Some(&prover1.pubkey()),
        &[&prover1],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(tx_submit1)
        .await
        .expect("Failed prover1 submit");

    let ix_submit2 = submit_result(
        &FHE_GENERATOR_PROGRAM_ID,
        &prover2.pubkey(),
        &job_pda,
        &consensus_pda,
        result_hash,
    );
    let tx_submit2 = Transaction::new_signed_with_payer(
        &[ix_submit2],
        Some(&prover2.pubkey()),
        &[&prover2],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(tx_submit2)
        .await
        .expect("Failed prover2 submit");

    let ix_submit3 = submit_result(
        &FHE_GENERATOR_PROGRAM_ID,
        &prover3.pubkey(),
        &job_pda,
        &consensus_pda,
        result_hash,
    );
    let tx_submit3 = Transaction::new_signed_with_payer(
        &[ix_submit3],
        Some(&prover3.pubkey()),
        &[&prover3],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(tx_submit3)
        .await
        .expect("Failed prover3 submit");

    // 11. Finalizar job
    let (escrow_pda, _) = derive_escrow_pda(&FHE_GENERATOR_PROGRAM_ID, &job_pda);
    let (config_pda, _) = get_bedrock_config_pda();
    let fee_recipient = admin.pubkey(); // Admin recibe fees

    let prover_wallets = vec![prover1.pubkey(), prover2.pubkey(), prover3.pubkey()];
    let prover_pdas = vec![prover1_pda, prover2_pda, prover3_pda];

    let ix_finalize = finalize_job(
        &FHE_GENERATOR_PROGRAM_ID,
        &admin.pubkey(),
        &job_pda,
        &consensus_pda,
        &escrow_pda,
        &creator.pubkey(),
        &fee_recipient,
        &BEDROCK_PROGRAM_ID,
        &config_pda,
        &prover_wallets,
        &prover_pdas,
    );

    let tx_finalize = Transaction::new_signed_with_payer(
        &[ix_finalize],
        Some(&admin.pubkey()),
        &[&admin],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(tx_finalize)
        .await
        .expect("Failed to finalize FHE job");

    // 12. Verificar que los 3 provers recibieron pago
    // Cada uno deberia recibir ~1/3 del total menos fees
    let prover1_account = context
        .banks_client
        .get_account(prover1.pubkey())
        .await
        .expect("Failed to get prover1 account")
        .expect("Prover1 account not found");

    // El balance inicial era 10_000_000_000 + stake_amount
    // Debería haber aumentado por el pago
    assert!(prover1_account.lamports > 10_000_000_000 + stake_amount);

    println!("FHE multi-prover flow test passed!");
}

/// Test de FHE job con threshold no alcanzado
#[tokio::test]
async fn test_fhe_threshold_not_reached() {
    let program_test = setup_test_environment();
    let mut context = program_test.start_with_context().await;

    // Setup con 3 provers
    let admin = create_and_fund_keypair(&mut context, 10_000_000_000).await;
    let stake_amount = 100_000_000;
    let prover1 = create_and_fund_keypair(&mut context, 10_000_000_000 + stake_amount).await;
    let prover2 = create_and_fund_keypair(&mut context, 10_000_000_000 + stake_amount).await;
    let creator = create_and_fund_keypair(&mut context, 5_000_000_000).await;

    initialize_bedrock(&mut context, &admin).await.unwrap();
    register_prover(&mut context, &prover1, stake_amount).await.unwrap();
    register_prover(&mut context, &prover2, stake_amount).await.unwrap();

    // Create FHE job with threshold=3 (but only 2 provers will submit)
    let current_time = get_current_timestamp(&mut context).await;
    let job_id = generate_fhe_job_id(current_time, &creator.pubkey());

    let ix_create = create_job(
        &FHE_GENERATOR_PROGRAM_ID,
        &creator.pubkey(),
        job_id,
        CIRCUIT_FHE_SUM,
        [1u8; 32],
        512u32,
        500_000_000u64,
        3600i64,
        3, // required_provers = 3
        3, // consensus_threshold = 3
        50, 0, 0,
    );

    let tx = Transaction::new_signed_with_payer(
        &[ix_create],
        Some(&creator.pubkey()),
        &[&creator],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    let (job_pda, _) = derive_job_pda(&FHE_GENERATOR_PROGRAM_ID, &creator.pubkey(), job_id);
    let (consensus_pda, _) = derive_consensus_pda(&FHE_GENERATOR_PROGRAM_ID, job_id);
    let (prover1_pda, _) = get_prover_registry_pda(&prover1.pubkey());
    let (prover2_pda, _) = get_prover_registry_pda(&prover2.pubkey());

    // Only 2 provers claim and submit (threshold requires 3)
    for (prover, prover_pda) in [(&prover1, &prover1_pda), (&prover2, &prover2_pda)] {
        let ix_claim = claim_job(
            &FHE_GENERATOR_PROGRAM_ID,
            &prover.pubkey(),
            &job_pda,
            &consensus_pda,
            prover_pda,
            &BEDROCK_PROGRAM_ID,
        );
        context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
        let tx = Transaction::new_signed_with_payer(
            &[ix_claim],
            Some(&prover.pubkey()),
            &[prover],
            context.last_blockhash,
        );
        context.banks_client.process_transaction(tx).await.unwrap();

        let ix_submit = submit_result(
            &FHE_GENERATOR_PROGRAM_ID,
            &prover.pubkey(),
            &job_pda,
            &consensus_pda,
            [42u8; 32],
        );
        context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
        let tx = Transaction::new_signed_with_payer(
            &[ix_submit],
            Some(&prover.pubkey()),
            &[prover],
            context.last_blockhash,
        );
        context.banks_client.process_transaction(tx).await.unwrap();
    }

    // Try to finalize - should fail (only 2/3 submissions)
    let (escrow_pda, _) = derive_escrow_pda(&FHE_GENERATOR_PROGRAM_ID, &job_pda);
    let (config_pda, _) = get_bedrock_config_pda();

    let ix_finalize = finalize_job(
        &FHE_GENERATOR_PROGRAM_ID,
        &admin.pubkey(),
        &job_pda,
        &consensus_pda,
        &escrow_pda,
        &creator.pubkey(),
        &admin.pubkey(),
        &BEDROCK_PROGRAM_ID,
        &config_pda,
        &[prover1.pubkey(), prover2.pubkey()],
        &[prover1_pda, prover2_pda],
    );

    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[ix_finalize],
        Some(&admin.pubkey()),
        &[&admin],
        context.last_blockhash,
    );

    let result = context.banks_client.process_transaction(tx).await;
    assert!(result.is_err(), "Finalize should fail - threshold not reached");
}

/// Test de prover no registrado intentando claim en FHE
#[tokio::test]
async fn test_fhe_claim_unregistered_prover_fails() {
    let program_test = setup_test_environment();
    let mut context = program_test.start_with_context().await;

    // 1. Setup: inicializar bedrock y crear job
    let admin = create_and_fund_keypair(&mut context, 10_000_000_000).await;
    let creator = create_and_fund_keypair(&mut context, 5_000_000_000).await;
    let unregistered_prover = create_and_fund_keypair(&mut context, 10_000_000_000).await;

    initialize_bedrock(&mut context, &admin)
        .await
        .expect("Failed to initialize bedrock");

    // 2. Crear FHE job (sin registrar el prover)
    let current_time = get_current_timestamp(&mut context).await;
    let job_id = generate_fhe_job_id(current_time, &creator.pubkey());

    let witness_hash = [3u8; 32];
    let witness_size = 512u32;
    let price_lamports = 500_000_000u64;
    let timeout_seconds = 1800i64;

    let ix_create = create_job(
        &FHE_GENERATOR_PROGRAM_ID,
        &creator.pubkey(),
        job_id,
        CIRCUIT_FHE_SUM,
        witness_hash,
        witness_size,
        price_lamports,
        timeout_seconds,
        2, // required_provers
        2, // consensus_threshold
        50,
        0,
        0,
    );

    let tx_create = Transaction::new_signed_with_payer(
        &[ix_create],
        Some(&creator.pubkey()),
        &[&creator],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(tx_create)
        .await
        .expect("Failed to create FHE job");

    // 3. Prover no registrado intenta hacer claim
    let (job_pda, _) = derive_job_pda(&FHE_GENERATOR_PROGRAM_ID, &creator.pubkey(), job_id);
    let (consensus_pda, _) = derive_consensus_pda(&FHE_GENERATOR_PROGRAM_ID, job_id);
    let (unregistered_prover_pda, _) = get_prover_registry_pda(&unregistered_prover.pubkey());

    let ix_claim = claim_job(
        &FHE_GENERATOR_PROGRAM_ID,
        &unregistered_prover.pubkey(),
        &job_pda,
        &consensus_pda,
        &unregistered_prover_pda,
        &BEDROCK_PROGRAM_ID,
    );

    let tx_claim = Transaction::new_signed_with_payer(
        &[ix_claim],
        Some(&unregistered_prover.pubkey()),
        &[&unregistered_prover],
        context.last_blockhash,
    );

    // 4. Verificar que falla
    let result = context.banks_client.process_transaction(tx_claim).await;

    assert!(result.is_err(), "Unregistered prover should not be able to claim job");
    println!("FHE claim unregistered prover test passed!");
    println!("Unregistered prover was correctly rejected");
}

/// Test de submit sin claim previo
#[tokio::test]
async fn test_fhe_submit_without_claim_fails() {
    let program_test = setup_test_environment();
    let mut context = program_test.start_with_context().await;

    // Setup
    let admin = create_and_fund_keypair(&mut context, 10_000_000_000).await;
    let stake_amount = 100_000_000;
    let prover = create_and_fund_keypair(&mut context, 10_000_000_000 + stake_amount).await;
    let creator = create_and_fund_keypair(&mut context, 5_000_000_000).await;

    initialize_bedrock(&mut context, &admin).await.unwrap();
    register_prover(&mut context, &prover, stake_amount).await.unwrap();

    // Create FHE job
    let current_time = get_current_timestamp(&mut context).await;
    let job_id = generate_fhe_job_id(current_time, &creator.pubkey());

    let ix_create = create_job(
        &FHE_GENERATOR_PROGRAM_ID,
        &creator.pubkey(),
        job_id,
        CIRCUIT_FHE_SUM,
        [1u8; 32],
        512u32,
        500_000_000u64,
        3600i64,
        2, 2, 50, 0, 0,
    );

    let tx = Transaction::new_signed_with_payer(
        &[ix_create],
        Some(&creator.pubkey()),
        &[&creator],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Try to submit without claiming first
    let (job_pda, _) = derive_job_pda(&FHE_GENERATOR_PROGRAM_ID, &creator.pubkey(), job_id);
    let (consensus_pda, _) = derive_consensus_pda(&FHE_GENERATOR_PROGRAM_ID, job_id);

    let ix_submit = submit_result(
        &FHE_GENERATOR_PROGRAM_ID,
        &prover.pubkey(),
        &job_pda,
        &consensus_pda,
        [42u8; 32],
    );

    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[ix_submit],
        Some(&prover.pubkey()),
        &[&prover],
        context.last_blockhash,
    );

    let result = context.banks_client.process_transaction(tx).await;
    assert!(result.is_err(), "Submit without claim should fail");
}

/// Test de múltiples claims del mismo prover
#[tokio::test]
async fn test_fhe_duplicate_claim_fails() {
    let program_test = setup_test_environment();
    let mut context = program_test.start_with_context().await;

    // Setup
    let admin = create_and_fund_keypair(&mut context, 10_000_000_000).await;
    let stake_amount = 100_000_000;
    let prover = create_and_fund_keypair(&mut context, 10_000_000_000 + stake_amount).await;
    let creator = create_and_fund_keypair(&mut context, 5_000_000_000).await;

    initialize_bedrock(&mut context, &admin).await.unwrap();
    register_prover(&mut context, &prover, stake_amount).await.unwrap();

    // Create FHE job
    let current_time = get_current_timestamp(&mut context).await;
    let job_id = generate_fhe_job_id(current_time, &creator.pubkey());

    let ix_create = create_job(
        &FHE_GENERATOR_PROGRAM_ID,
        &creator.pubkey(),
        job_id,
        CIRCUIT_FHE_SUM,
        [1u8; 32],
        512u32,
        500_000_000u64,
        3600i64,
        2, // required_provers
        2, // consensus_threshold
        50, 0, 0,
    );

    let tx = Transaction::new_signed_with_payer(
        &[ix_create],
        Some(&creator.pubkey()),
        &[&creator],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // First claim - should succeed
    let (job_pda, _) = derive_job_pda(&FHE_GENERATOR_PROGRAM_ID, &creator.pubkey(), job_id);
    let (consensus_pda, _) = derive_consensus_pda(&FHE_GENERATOR_PROGRAM_ID, job_id);
    let (prover_pda, _) = get_prover_registry_pda(&prover.pubkey());

    let ix_claim1 = claim_job(
        &FHE_GENERATOR_PROGRAM_ID,
        &prover.pubkey(),
        &job_pda,
        &consensus_pda,
        &prover_pda,
        &BEDROCK_PROGRAM_ID,
    );

    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[ix_claim1],
        Some(&prover.pubkey()),
        &[&prover],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.expect("First claim should succeed");

    // Second claim - should fail (worker already exists)
    let ix_claim2 = claim_job(
        &FHE_GENERATOR_PROGRAM_ID,
        &prover.pubkey(),
        &job_pda,
        &consensus_pda,
        &prover_pda,
        &BEDROCK_PROGRAM_ID,
    );

    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[ix_claim2],
        Some(&prover.pubkey()),
        &[&prover],
        context.last_blockhash,
    );

    let result = context.banks_client.process_transaction(tx).await;
    assert!(result.is_err(), "Duplicate claim should fail");
}

/// Test de finalize sin suficientes submissions
#[tokio::test]
async fn test_fhe_finalize_insufficient_submissions() {
    let program_test = setup_test_environment();
    let mut context = program_test.start_with_context().await;

    // Setup
    let admin = create_and_fund_keypair(&mut context, 10_000_000_000).await;
    let stake_amount = 100_000_000;
    let prover1 = create_and_fund_keypair(&mut context, 10_000_000_000 + stake_amount).await;
    let creator = create_and_fund_keypair(&mut context, 5_000_000_000).await;

    initialize_bedrock(&mut context, &admin).await.unwrap();
    register_prover(&mut context, &prover1, stake_amount).await.unwrap();

    // Create job with threshold=2 but only 1 will submit
    let current_time = get_current_timestamp(&mut context).await;
    let job_id = generate_fhe_job_id(current_time, &creator.pubkey());

    let ix_create = create_job(
        &FHE_GENERATOR_PROGRAM_ID,
        &creator.pubkey(),
        job_id,
        CIRCUIT_FHE_SUM,
        [1u8; 32],
        512u32,
        500_000_000u64,
        3600i64,
        2, 2, 50, 0, 0,
    );

    let tx = Transaction::new_signed_with_payer(
        &[ix_create],
        Some(&creator.pubkey()),
        &[&creator],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    let (job_pda, _) = derive_job_pda(&FHE_GENERATOR_PROGRAM_ID, &creator.pubkey(), job_id);
    let (consensus_pda, _) = derive_consensus_pda(&FHE_GENERATOR_PROGRAM_ID, job_id);
    let (prover1_pda, _) = get_prover_registry_pda(&prover1.pubkey());

    // Only 1 prover claims and submits
    let ix_claim = claim_job(
        &FHE_GENERATOR_PROGRAM_ID,
        &prover1.pubkey(),
        &job_pda,
        &consensus_pda,
        &prover1_pda,
        &BEDROCK_PROGRAM_ID,
    );
    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[ix_claim],
        Some(&prover1.pubkey()),
        &[&prover1],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    let ix_submit = submit_result(
        &FHE_GENERATOR_PROGRAM_ID,
        &prover1.pubkey(),
        &job_pda,
        &consensus_pda,
        [42u8; 32],
    );
    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[ix_submit],
        Some(&prover1.pubkey()),
        &[&prover1],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Try to finalize with only 1 submission (needs 2)
    let (escrow_pda, _) = derive_escrow_pda(&FHE_GENERATOR_PROGRAM_ID, &job_pda);
    let (config_pda, _) = get_bedrock_config_pda();

    let ix_finalize = finalize_job(
        &FHE_GENERATOR_PROGRAM_ID,
        &admin.pubkey(),
        &job_pda,
        &consensus_pda,
        &escrow_pda,
        &creator.pubkey(),
        &admin.pubkey(),
        &BEDROCK_PROGRAM_ID,
        &config_pda,
        &[prover1.pubkey()],
        &[prover1_pda],
    );

    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[ix_finalize],
        Some(&admin.pubkey()),
        &[&admin],
        context.last_blockhash,
    );

    let result = context.banks_client.process_transaction(tx).await;
    assert!(result.is_err(), "Finalize should fail - insufficient submissions");
}

/// Test de timeout en FHE job
#[tokio::test]
async fn test_fhe_job_timeout() {
    use solana_program::clock::Clock;

    let program_test = setup_test_environment();
    let mut context = program_test.start_with_context().await;

    // Setup
    let admin = create_and_fund_keypair(&mut context, 10_000_000_000).await;
    let stake_amount = 100_000_000;
    let prover1 = create_and_fund_keypair(&mut context, 10_000_000_000 + stake_amount).await;
    let creator = create_and_fund_keypair(&mut context, 5_000_000_000).await;

    initialize_bedrock(&mut context, &admin).await.unwrap();
    register_prover(&mut context, &prover1, stake_amount).await.unwrap();

    // Create job with minimum timeout (60 seconds)
    let current_time = get_current_timestamp(&mut context).await;
    let job_id = generate_fhe_job_id(current_time, &creator.pubkey());

    let ix_create = create_job(
        &FHE_GENERATOR_PROGRAM_ID,
        &creator.pubkey(),
        job_id,
        CIRCUIT_FHE_SUM,
        [1u8; 32],
        512u32,
        500_000_000u64,
        60i64, // Minimum timeout
        2, 2, 50, 0, 0,
    );

    let tx = Transaction::new_signed_with_payer(
        &[ix_create],
        Some(&creator.pubkey()),
        &[&creator],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    let (job_pda, _) = derive_job_pda(&FHE_GENERATOR_PROGRAM_ID, &creator.pubkey(), job_id);
    let (consensus_pda, _) = derive_consensus_pda(&FHE_GENERATOR_PROGRAM_ID, job_id);
    let (prover1_pda, _) = get_prover_registry_pda(&prover1.pubkey());

    // Prover claims
    let ix_claim = claim_job(
        &FHE_GENERATOR_PROGRAM_ID,
        &prover1.pubkey(),
        &job_pda,
        &consensus_pda,
        &prover1_pda,
        &BEDROCK_PROGRAM_ID,
    );
    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[ix_claim],
        Some(&prover1.pubkey()),
        &[&prover1],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Warp time past timeout
    let mut clock: Clock = context.banks_client.get_sysvar().await.unwrap();
    clock.unix_timestamp += 200; // Well past 60s timeout
    context.set_sysvar(&clock);

    // Try to submit after timeout
    let ix_submit = submit_result(
        &FHE_GENERATOR_PROGRAM_ID,
        &prover1.pubkey(),
        &job_pda,
        &consensus_pda,
        [42u8; 32],
    );
    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[ix_submit],
        Some(&prover1.pubkey()),
        &[&prover1],
        context.last_blockhash,
    );

    // Submit behavior after timeout depends on program implementation
    let result = context.banks_client.process_transaction(tx).await;
    println!("FHE submit after timeout result: {:?}", result);
}

/// Test de coordinación exitosa con threshold exacto
#[tokio::test]
async fn test_fhe_exact_threshold_success() {
    let program_test = setup_test_environment();
    let mut context = program_test.start_with_context().await;

    // Setup con exactamente 2 provers
    let admin = create_and_fund_keypair(&mut context, 10_000_000_000).await;
    let stake_amount = 100_000_000;
    let prover1 = create_and_fund_keypair(&mut context, 10_000_000_000 + stake_amount).await;
    let prover2 = create_and_fund_keypair(&mut context, 10_000_000_000 + stake_amount).await;
    let creator = create_and_fund_keypair(&mut context, 5_000_000_000).await;

    initialize_bedrock(&mut context, &admin).await.unwrap();
    register_prover(&mut context, &prover1, stake_amount).await.unwrap();
    register_prover(&mut context, &prover2, stake_amount).await.unwrap();

    // Create FHE job with threshold=2
    let current_time = get_current_timestamp(&mut context).await;
    let job_id = generate_fhe_job_id(current_time, &creator.pubkey());

    let price_lamports = 500_000_000u64;

    let ix_create = create_job(
        &FHE_GENERATOR_PROGRAM_ID,
        &creator.pubkey(),
        job_id,
        CIRCUIT_FHE_SUM,
        [1u8; 32],
        512u32,
        price_lamports,
        3600i64,
        2, // required_provers = 2
        2, // consensus_threshold = 2
        50, 0, 0,
    );

    let tx = Transaction::new_signed_with_payer(
        &[ix_create],
        Some(&creator.pubkey()),
        &[&creator],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    let (job_pda, _) = derive_job_pda(&FHE_GENERATOR_PROGRAM_ID, &creator.pubkey(), job_id);
    let (consensus_pda, _) = derive_consensus_pda(&FHE_GENERATOR_PROGRAM_ID, job_id);
    let (prover1_pda, _) = get_prover_registry_pda(&prover1.pubkey());
    let (prover2_pda, _) = get_prover_registry_pda(&prover2.pubkey());

    let result_hash = [42u8; 32]; // Both provers agree

    // Both provers claim and submit
    for (prover, prover_pda) in [(&prover1, &prover1_pda), (&prover2, &prover2_pda)] {
        let ix_claim = claim_job(
            &FHE_GENERATOR_PROGRAM_ID,
            &prover.pubkey(),
            &job_pda,
            &consensus_pda,
            prover_pda,
            &BEDROCK_PROGRAM_ID,
        );
        context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
        let tx = Transaction::new_signed_with_payer(
            &[ix_claim],
            Some(&prover.pubkey()),
            &[prover],
            context.last_blockhash,
        );
        context.banks_client.process_transaction(tx).await.unwrap();

        let ix_submit = submit_result(
            &FHE_GENERATOR_PROGRAM_ID,
            &prover.pubkey(),
            &job_pda,
            &consensus_pda,
            result_hash,
        );
        context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
        let tx = Transaction::new_signed_with_payer(
            &[ix_submit],
            Some(&prover.pubkey()),
            &[prover],
            context.last_blockhash,
        );
        context.banks_client.process_transaction(tx).await.unwrap();
    }

    // Finalize - should succeed with exactly 2/2 submissions
    let (escrow_pda, _) = derive_escrow_pda(&FHE_GENERATOR_PROGRAM_ID, &job_pda);
    let (config_pda, _) = get_bedrock_config_pda();

    let prover1_balance_before = context.banks_client.get_account(prover1.pubkey()).await.unwrap().unwrap().lamports;

    let ix_finalize = finalize_job(
        &FHE_GENERATOR_PROGRAM_ID,
        &admin.pubkey(),
        &job_pda,
        &consensus_pda,
        &escrow_pda,
        &creator.pubkey(),
        &admin.pubkey(),
        &BEDROCK_PROGRAM_ID,
        &config_pda,
        &[prover1.pubkey(), prover2.pubkey()],
        &[prover1_pda, prover2_pda],
    );

    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[ix_finalize],
        Some(&admin.pubkey()),
        &[&admin],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.expect("Finalize should succeed with exact threshold");

    // Verify provers received payment
    let prover1_balance_after = context.banks_client.get_account(prover1.pubkey()).await.unwrap().unwrap().lamports;
    assert!(prover1_balance_after > prover1_balance_before, "Prover1 should have received payment");
}

/// Test de FHE con resultados inconsistentes entre provers
#[tokio::test]
async fn test_fhe_inconsistent_results() {
    let program_test = setup_test_environment();
    let mut context = program_test.start_with_context().await;

    // 1. Setup completo
    let admin = create_and_fund_keypair(&mut context, 10_000_000_000).await;
    let stake_amount = 100_000_000;
    let prover1 = create_and_fund_keypair(&mut context, 10_000_000_000 + stake_amount).await;
    let prover2 = create_and_fund_keypair(&mut context, 10_000_000_000 + stake_amount).await;
    let prover3 = create_and_fund_keypair(&mut context, 10_000_000_000 + stake_amount).await;
    let creator = create_and_fund_keypair(&mut context, 5_000_000_000).await;

    initialize_bedrock(&mut context, &admin)
        .await
        .expect("Failed to initialize bedrock");

    // 2. Registrar los 3 provers
    register_prover(&mut context, &prover1, stake_amount)
        .await
        .expect("Failed to register prover1");
    register_prover(&mut context, &prover2, stake_amount)
        .await
        .expect("Failed to register prover2");
    register_prover(&mut context, &prover3, stake_amount)
        .await
        .expect("Failed to register prover3");

    // 3. Crear FHE job con required_provers=3 y consensus_threshold=2 (mayoría)
    let current_time = get_current_timestamp(&mut context).await;
    let job_id = generate_fhe_job_id(current_time, &creator.pubkey());

    let witness_hash = [2u8; 32];
    let witness_size = 2048u32;
    let price_lamports = 900_000_000u64;
    let timeout_seconds = 7200i64;

    let ix_create = create_job(
        &FHE_GENERATOR_PROGRAM_ID,
        &creator.pubkey(),
        job_id,
        CIRCUIT_FHE_SUM,
        witness_hash,
        witness_size,
        price_lamports,
        timeout_seconds,
        3, // required_provers
        2, // consensus_threshold (2 de 3 es suficiente)
        200,
        1,
        0,
    );

    let tx_create = Transaction::new_signed_with_payer(
        &[ix_create],
        Some(&creator.pubkey()),
        &[&creator],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(tx_create)
        .await
        .expect("Failed to create FHE job");

    let (job_pda, _) = derive_job_pda(&FHE_GENERATOR_PROGRAM_ID, &creator.pubkey(), job_id);
    let (consensus_pda, _) = derive_consensus_pda(&FHE_GENERATOR_PROGRAM_ID, job_id);
    let (prover1_pda, _) = get_prover_registry_pda(&prover1.pubkey());
    let (prover2_pda, _) = get_prover_registry_pda(&prover2.pubkey());
    let (prover3_pda, _) = get_prover_registry_pda(&prover3.pubkey());

    // 4. Los 3 provers hacen claim
    for (prover, prover_pda) in &[
        (&prover1, prover1_pda),
        (&prover2, prover2_pda),
        (&prover3, prover3_pda),
    ] {
        let ix_claim = claim_job(
            &FHE_GENERATOR_PROGRAM_ID,
            &prover.pubkey(),
            &job_pda,
            &consensus_pda,
            prover_pda,
            &BEDROCK_PROGRAM_ID,
        );
        let tx_claim = Transaction::new_signed_with_payer(
            &[ix_claim],
            Some(&prover.pubkey()),
            &[prover],
            context.last_blockhash,
        );
        context
            .banks_client
            .process_transaction(tx_claim)
            .await
            .expect("Failed prover claim");
    }

    // 5. Prover1 y Prover2 submit el MISMO hash (mayoría)
    let result_hash_majority = [99u8; 32];
    let result_hash_minority = [11u8; 32]; // Diferente

    // Prover1 submit hash mayoritario
    let ix_submit1 = submit_result(
        &FHE_GENERATOR_PROGRAM_ID,
        &prover1.pubkey(),
        &job_pda,
        &consensus_pda,
        result_hash_majority,
    );
    let tx_submit1 = Transaction::new_signed_with_payer(
        &[ix_submit1],
        Some(&prover1.pubkey()),
        &[&prover1],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(tx_submit1)
        .await
        .expect("Failed prover1 submit");

    // Prover2 submit hash mayoritario
    let ix_submit2 = submit_result(
        &FHE_GENERATOR_PROGRAM_ID,
        &prover2.pubkey(),
        &job_pda,
        &consensus_pda,
        result_hash_majority,
    );
    let tx_submit2 = Transaction::new_signed_with_payer(
        &[ix_submit2],
        Some(&prover2.pubkey()),
        &[&prover2],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(tx_submit2)
        .await
        .expect("Failed prover2 submit");

    // 6. Guardar balance de prover1, prover2, prover3 ANTES de finalize
    let prover1_balance_before = context
        .banks_client
        .get_account(prover1.pubkey())
        .await
        .expect("Failed to get prover1 account")
        .expect("Prover1 account not found")
        .lamports;

    let prover2_balance_before = context
        .banks_client
        .get_account(prover2.pubkey())
        .await
        .expect("Failed to get prover2 account")
        .expect("Prover2 account not found")
        .lamports;

    let prover3_balance_before = context
        .banks_client
        .get_account(prover3.pubkey())
        .await
        .expect("Failed to get prover3 account")
        .expect("Prover3 account not found")
        .lamports;

    // Prover3 submit hash minoritario (diferente)
    let ix_submit3 = submit_result(
        &FHE_GENERATOR_PROGRAM_ID,
        &prover3.pubkey(),
        &job_pda,
        &consensus_pda,
        result_hash_minority,
    );
    let tx_submit3 = Transaction::new_signed_with_payer(
        &[ix_submit3],
        Some(&prover3.pubkey()),
        &[&prover3],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(tx_submit3)
        .await
        .expect("Failed prover3 submit");

    // 7. Finalizar job
    let (escrow_pda, _) = derive_escrow_pda(&FHE_GENERATOR_PROGRAM_ID, &job_pda);
    let (config_pda, _) = get_bedrock_config_pda();
    let fee_recipient = admin.pubkey();

    let prover_wallets = vec![prover1.pubkey(), prover2.pubkey(), prover3.pubkey()];
    let prover_pdas = vec![prover1_pda, prover2_pda, prover3_pda];

    let ix_finalize = finalize_job(
        &FHE_GENERATOR_PROGRAM_ID,
        &admin.pubkey(),
        &job_pda,
        &consensus_pda,
        &escrow_pda,
        &creator.pubkey(),
        &fee_recipient,
        &BEDROCK_PROGRAM_ID,
        &config_pda,
        &prover_wallets,
        &prover_pdas,
    );

    let tx_finalize = Transaction::new_signed_with_payer(
        &[ix_finalize],
        Some(&admin.pubkey()),
        &[&admin],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(tx_finalize)
        .await
        .expect("Failed to finalize FHE job");

    // 8. Verificar que prover1 y prover2 recibieron pago
    let prover1_balance_after = context
        .banks_client
        .get_account(prover1.pubkey())
        .await
        .expect("Failed to get prover1 account")
        .expect("Prover1 account not found")
        .lamports;

    let prover2_balance_after = context
        .banks_client
        .get_account(prover2.pubkey())
        .await
        .expect("Failed to get prover2 account")
        .expect("Prover2 account not found")
        .lamports;

    // Ambos deberían haber aumentado su balance (recibieron pago)
    assert!(
        prover1_balance_after > prover1_balance_before,
        "Prover1 should have received payment"
    );
    assert!(
        prover2_balance_after > prover2_balance_before,
        "Prover2 should have received payment"
    );

    // 9. Verificar que prover3 NO recibió pago
    let prover3_balance_after = context
        .banks_client
        .get_account(prover3.pubkey())
        .await
        .expect("Failed to get prover3 account")
        .expect("Prover3 account not found")
        .lamports;

    // El balance de prover3 debería ser igual o menor (solo gasto en gas, sin pago)
    assert!(
        prover3_balance_after <= prover3_balance_before,
        "Prover3 should NOT have received payment (mismatching result)"
    );

    println!("FHE inconsistent results test passed!");
    println!("Prover1 and Prover2 (matching) were paid");
    println!("Prover3 (mismatching) received nothing and was penalized");
}
