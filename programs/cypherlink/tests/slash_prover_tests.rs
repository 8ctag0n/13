mod common;

use borsh::BorshDeserialize;
use common::{initialize_marketplace, register_prover, setup_program_test};
use cypherlink::{
    instruction::MarketplaceInstruction, state::JobAccount, state::MarketplaceConfig,
    state::ProverAccount,
};
use cypherlink_types::CircuitType;
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};

/// Helper to create a job
async fn create_job(
    banks_client: &mut solana_program_test::BanksClient,
    payer: &Keypair,
    program_id: &Pubkey,
    config_pda: &Pubkey,
    job_id: u64,
) -> Result<Pubkey, Box<dyn std::error::Error>> {
    let job_creator = payer.pubkey();
    let job_id_bytes = job_id.to_le_bytes();
    let (job_pda, _) = Pubkey::find_program_address(
        &[b"job", job_creator.as_ref(), &job_id_bytes],
        program_id,
    );
    let (escrow_pda, _) = Pubkey::find_program_address(&[b"escrow", job_pda.as_ref()], program_id);

    let create_job_instruction = MarketplaceInstruction::CreateJob {
        circuit_type: CircuitType::ZcashOrchard,
        witness_commitment: [1u8; 32],
        witness_size: 2048,
        price_lamports: 1_000_000,
        timeout_seconds: 600,
    };

    let create_job_ix = solana_program::instruction::Instruction {
        program_id: *program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(job_creator, true),
            solana_program::instruction::AccountMeta::new(job_pda, false),
            solana_program::instruction::AccountMeta::new(*config_pda, false),
            solana_program::instruction::AccountMeta::new(escrow_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(
                solana_program::system_program::id(),
                false,
            ),
        ],
        data: create_job_instruction.pack()?,
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut create_job_tx = Transaction::new_with_payer(&[create_job_ix], Some(&payer.pubkey()));
    create_job_tx.sign(&[payer], recent_blockhash);
    banks_client.process_transaction(create_job_tx).await?;

    Ok(job_pda)
}

/// Helper to claim a job
async fn claim_job(
    banks_client: &mut solana_program_test::BanksClient,
    prover_keypair: &Keypair,
    program_id: &Pubkey,
    prover_pda: &Pubkey,
    job_pda: &Pubkey,
    config_pda: &Pubkey,
) -> Result<(), Box<dyn std::error::Error>> {
    let claim_job_instruction = MarketplaceInstruction::ClaimJob;

    let claim_job_ix = solana_program::instruction::Instruction {
        program_id: *program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(prover_keypair.pubkey(), true),
            solana_program::instruction::AccountMeta::new(*prover_pda, false),
            solana_program::instruction::AccountMeta::new(*job_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(*config_pda, false),
        ],
        data: claim_job_instruction.pack()?,
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut claim_job_tx =
        Transaction::new_with_payer(&[claim_job_ix], Some(&prover_keypair.pubkey()));
    claim_job_tx.sign(&[prover_keypair], recent_blockhash);
    banks_client.process_transaction(claim_job_tx).await?;

    Ok(())
}

#[tokio::test]
async fn test_slash_prover() {
    let program_id = Pubkey::new_unique();
    let mut program_test = setup_program_test(program_id);

    let (mut banks_client, payer, _recent_blockhash) = program_test.start().await;

    // Initialize marketplace
    let config_pda = initialize_marketplace(
        &mut banks_client,
        &payer,
        &program_id,
        1000,
        5_000_000_000, // Min stake: 5 SOL
        500,
        600,
    )
    .await
    .unwrap();

    // Register a prover with 6 SOL stake
    let prover_keypair = Keypair::new();
    let prover_pda = register_prover(
        &mut banks_client,
        &payer,
        &prover_keypair,
        &program_id,
        &config_pda,
        6_000_000_000, // 6 SOL stake
    )
    .await
    .unwrap();

    // Create and claim a job (as evidence)
    let job_pda = create_job(&mut banks_client, &payer, &program_id, &config_pda, 0)
        .await
        .unwrap();

    claim_job(
        &mut banks_client,
        &prover_keypair,
        &program_id,
        &prover_pda,
        &job_pda,
        &config_pda,
    )
    .await
    .unwrap();

    // Get config for protocol fee recipient
    let config = banks_client.get_account(config_pda).await.unwrap().unwrap();
    let config_data: MarketplaceConfig = borsh::from_slice(&config.data).unwrap();
    let protocol_fee_recipient = config_data.protocol_fee_recipient;

    // Get initial balances
    let prover_stake_before = banks_client
        .get_account(prover_pda)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    let protocol_balance_before = banks_client
        .get_account(protocol_fee_recipient)
        .await
        .unwrap()
        .map(|a| a.lamports)
        .unwrap_or(0);

    // Slash prover by 1 SOL
    let slash_amount = 1_000_000_000u64;
    let slash_instruction = MarketplaceInstruction::SlashProver { slash_amount };

    let slash_ix = solana_program::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(payer.pubkey(), true), // Authority
            solana_program::instruction::AccountMeta::new(prover_pda, false),
            solana_program::instruction::AccountMeta::new(job_pda, false), // Evidence
            solana_program::instruction::AccountMeta::new(protocol_fee_recipient, false),
            solana_program::instruction::AccountMeta::new_readonly(config_pda, false),
        ],
        data: slash_instruction.pack().unwrap(),
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut slash_tx = Transaction::new_with_payer(&[slash_ix], Some(&payer.pubkey()));
    slash_tx.sign(&[&payer], recent_blockhash);

    let result = banks_client.process_transaction(slash_tx).await;
    assert!(result.is_ok(), "SlashProver failed: {:?}", result.err());

    // Verify prover was slashed
    let prover_account = banks_client.get_account(prover_pda).await.unwrap().unwrap();
    let prover_data: ProverAccount = borsh::from_slice(&prover_account.data).unwrap();

    assert_eq!(prover_data.stake_amount, 5_000_000_000); // 6 SOL - 1 SOL
    assert_eq!(prover_data.total_jobs_failed, 1);
    assert_eq!(prover_data.reputation_score, 900); // 1000 - 100 penalty
    assert!(prover_data.is_active); // Still active (stake >= min)

    // Verify lamports were transferred
    let prover_lamports_after = prover_account.lamports;
    assert_eq!(
        prover_stake_before - prover_lamports_after,
        slash_amount,
        "Prover should lose slashed lamports"
    );

    let protocol_balance_after = banks_client
        .get_account(protocol_fee_recipient)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // Account for transaction fees
    let protocol_gain = protocol_balance_after - protocol_balance_before;
    assert!(
        protocol_gain >= slash_amount - 10_000,
        "Protocol should receive ~{} lamports, got {}",
        slash_amount,
        protocol_gain
    );
}

#[tokio::test]
async fn test_slash_prover_deactivation() {
    let program_id = Pubkey::new_unique();
    let mut program_test = setup_program_test(program_id);

    let (mut banks_client, payer, _recent_blockhash) = program_test.start().await;

    let config_pda = initialize_marketplace(
        &mut banks_client,
        &payer,
        &program_id,
        1000,
        5_000_000_000, // Min stake: 5 SOL
        500,
        600,
    )
    .await
    .unwrap();

    // Register prover with exactly minimum stake
    let prover_keypair = Keypair::new();
    let prover_pda = register_prover(
        &mut banks_client,
        &payer,
        &prover_keypair,
        &program_id,
        &config_pda,
        5_000_000_000, // Exactly 5 SOL
    )
    .await
    .unwrap();

    // Create and claim job
    let job_pda = create_job(&mut banks_client, &payer, &program_id, &config_pda, 0)
        .await
        .unwrap();

    claim_job(
        &mut banks_client,
        &prover_keypair,
        &program_id,
        &prover_pda,
        &job_pda,
        &config_pda,
    )
    .await
    .unwrap();

    let config = banks_client.get_account(config_pda).await.unwrap().unwrap();
    let config_data: MarketplaceConfig = borsh::from_slice(&config.data).unwrap();

    // Slash by 1 SOL - this should bring stake below minimum
    let slash_amount = 1_000_000_000u64;
    let slash_instruction = MarketplaceInstruction::SlashProver { slash_amount };

    let slash_ix = solana_program::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(payer.pubkey(), true),
            solana_program::instruction::AccountMeta::new(prover_pda, false),
            solana_program::instruction::AccountMeta::new(job_pda, false),
            solana_program::instruction::AccountMeta::new(config_data.protocol_fee_recipient, false),
            solana_program::instruction::AccountMeta::new_readonly(config_pda, false),
        ],
        data: slash_instruction.pack().unwrap(),
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut slash_tx = Transaction::new_with_payer(&[slash_ix], Some(&payer.pubkey()));
    slash_tx.sign(&[&payer], recent_blockhash);

    banks_client.process_transaction(slash_tx).await.unwrap();

    // Verify prover was deactivated
    let prover_account = banks_client.get_account(prover_pda).await.unwrap().unwrap();
    let prover_data: ProverAccount = borsh::from_slice(&prover_account.data).unwrap();

    assert!(!prover_data.is_active, "Prover should be deactivated when stake < min");
    assert_eq!(prover_data.stake_amount, 4_000_000_000); // 5 SOL - 1 SOL
}

#[tokio::test]
async fn test_slash_prover_unauthorized() {
    let program_id = Pubkey::new_unique();
    let mut program_test = setup_program_test(program_id);

    let (mut banks_client, payer, _recent_blockhash) = program_test.start().await;

    let config_pda = initialize_marketplace(
        &mut banks_client,
        &payer,
        &program_id,
        1000,
        5_000_000_000,
        500,
        600,
    )
    .await
    .unwrap();

    let prover_keypair = Keypair::new();
    let prover_pda = register_prover(
        &mut banks_client,
        &payer,
        &prover_keypair,
        &program_id,
        &config_pda,
        6_000_000_000,
    )
    .await
    .unwrap();

    let job_pda = create_job(&mut banks_client, &payer, &program_id, &config_pda, 0)
        .await
        .unwrap();

    claim_job(
        &mut banks_client,
        &prover_keypair,
        &program_id,
        &prover_pda,
        &job_pda,
        &config_pda,
    )
    .await
    .unwrap();

    let config = banks_client.get_account(config_pda).await.unwrap().unwrap();
    let config_data: MarketplaceConfig = borsh::from_slice(&config.data).unwrap();

    // Try to slash as non-authority
    let fake_authority = Keypair::new();

    // Fund fake authority
    let fund_instruction = solana_program::system_instruction::transfer(
        &payer.pubkey(),
        &fake_authority.pubkey(),
        10_000_000,
    );
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut fund_tx = Transaction::new_with_payer(&[fund_instruction], Some(&payer.pubkey()));
    fund_tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(fund_tx).await.unwrap();

    let slash_instruction = MarketplaceInstruction::SlashProver {
        slash_amount: 1_000_000_000,
    };

    let slash_ix = solana_program::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(fake_authority.pubkey(), true),
            solana_program::instruction::AccountMeta::new(prover_pda, false),
            solana_program::instruction::AccountMeta::new(job_pda, false),
            solana_program::instruction::AccountMeta::new(config_data.protocol_fee_recipient, false),
            solana_program::instruction::AccountMeta::new_readonly(config_pda, false),
        ],
        data: slash_instruction.pack().unwrap(),
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut slash_tx =
        Transaction::new_with_payer(&[slash_ix], Some(&fake_authority.pubkey()));
    slash_tx.sign(&[&fake_authority], recent_blockhash);

    let result = banks_client.process_transaction(slash_tx).await;
    assert!(result.is_err(), "Only authority should be able to slash");
}
