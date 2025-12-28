use solana_program::pubkey::Pubkey;
use solana_program_test::{processor, BanksClient, ProgramTest};
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use zyberlink::instruction::MarketplaceInstruction;

/// Initialize a new program test environment
pub fn setup_program_test(program_id: Pubkey) -> ProgramTest {
    ProgramTest::new(
        "zyberlink",
        program_id,
        processor!(zyberlink::process_instruction),
    )
}

/// Helper to initialize the marketplace
pub async fn initialize_marketplace(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    program_id: &Pubkey,
    fee_basis_points: u16,
    min_stake_amount: u64,
    min_reputation_score: u32,
    default_job_timeout_seconds: i64,
) -> Result<Pubkey, Box<dyn std::error::Error>> {
    let (config_pda, _) = Pubkey::find_program_address(&[b"config"], program_id);

    let init_instruction = MarketplaceInstruction::Initialize {
        fee_basis_points,
        min_stake_amount,
        min_reputation_score,
        default_job_timeout_seconds,
    };

    let init_ix = solana_program::instruction::Instruction {
        program_id: *program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(payer.pubkey(), true),
            solana_program::instruction::AccountMeta::new(config_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(
                solana_program::system_program::id(),
                false,
            ),
        ],
        data: init_instruction.pack()?,
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut init_tx = Transaction::new_with_payer(&[init_ix], Some(&payer.pubkey()));
    init_tx.sign(&[payer], recent_blockhash);
    banks_client.process_transaction(init_tx).await?;

    Ok(config_pda)
}

/// Helper to register a prover
pub async fn register_prover(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    prover_keypair: &Keypair,
    program_id: &Pubkey,
    config_pda: &Pubkey,
    stake_amount: u64,
) -> Result<Pubkey, Box<dyn std::error::Error>> {
    let prover_authority = prover_keypair.pubkey();
    let (prover_pda, _) =
        Pubkey::find_program_address(&[b"prover", prover_authority.as_ref()], program_id);

    // Fund the prover
    let fund_instruction = solana_program::system_instruction::transfer(
        &payer.pubkey(),
        &prover_authority,
        stake_amount + 10_000_000, // Extra for fees
    );
    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut fund_tx = Transaction::new_with_payer(&[fund_instruction], Some(&payer.pubkey()));
    fund_tx.sign(&[payer], recent_blockhash);
    banks_client.process_transaction(fund_tx).await?;

    // Register prover
    let register_instruction = MarketplaceInstruction::RegisterProver {
        stake_amount,
        encryption_pubkey: [1u8; 32],
    };

    let register_ix = solana_program::instruction::Instruction {
        program_id: *program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(prover_authority, true),
            solana_program::instruction::AccountMeta::new(prover_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(*config_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(
                solana_program::system_program::id(),
                false,
            ),
        ],
        data: register_instruction.pack()?,
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut register_tx = Transaction::new_with_payer(&[register_ix], Some(&prover_authority));
    register_tx.sign(&[prover_keypair], recent_blockhash);
    banks_client.process_transaction(register_tx).await?;

    Ok(prover_pda)
}
