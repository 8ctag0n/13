//! Integration tests for CreateMarket instruction

use borsh::{BorshDeserialize, BorshSerialize};
use futarchy_markets::{
    id,
    instruction::FutarchyInstruction,
    processor::process,
    state::{Market, MarketStatus, MARKET_SEED},
};
use solana_program::{clock::Clock, instruction::Instruction, pubkey::Pubkey, system_instruction};
use solana_program_test::{tokio, ProgramTest};
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};

/// Helper to create program test
fn create_program_test() -> ProgramTest {
    ProgramTest::new(
        "futarchy_markets",
        id(),
        solana_program_test::processor!(process),
    )
}

#[tokio::test]
async fn test_create_market_success() {
    let mut program_test = create_program_test();
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let authority = Keypair::new();
    let oracle = Pubkey::new_unique();
    let market_id = 1u64;
    let question_hash = [42u8; 32];
    let end_time = 2000000000i64; // Future time
    let max_bet = 1_000_000_000u64;

    // Airdrop to authority
    let airdrop_ix = system_instruction::transfer(&payer.pubkey(), &authority.pubkey(), 100_000_000_000);
    let airdrop_tx = Transaction::new_signed_with_payer(
        &[airdrop_ix],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    banks_client.process_transaction(airdrop_tx).await.unwrap();

    // Derive PDAs
    let (market_pda, _market_bump) =
        Pubkey::find_program_address(&[MARKET_SEED, &market_id.to_le_bytes()], &id());

    let (escrow_pda, _escrow_bump) =
        Pubkey::find_program_address(&[b"escrow", &market_id.to_le_bytes()], &id());

    // Create instruction
    let instruction = FutarchyInstruction::CreateMarket {
        market_id,
        question_hash,
        end_time,
        max_bet,
    };

    let instruction_data = borsh::to_vec(&instruction).unwrap();

    let create_market_ix = Instruction {
        program_id: id(),
        accounts: vec![
            solana_program::instruction::AccountMeta::new(authority.pubkey(), true),
            solana_program::instruction::AccountMeta::new(market_pda, false),
            solana_program::instruction::AccountMeta::new(escrow_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(oracle, false),
            solana_program::instruction::AccountMeta::new_readonly(
                solana_program::system_program::ID,
                false,
            ),
            solana_program::instruction::AccountMeta::new_readonly(
                solana_program::sysvar::clock::ID,
                false,
            ),
        ],
        data: instruction_data,
    };

    let tx = Transaction::new_signed_with_payer(
        &[create_market_ix],
        Some(&payer.pubkey()),
        &[&payer, &authority],
        recent_blockhash,
    );

    banks_client.process_transaction(tx).await.unwrap();

    // Verify market account was created
    let market_account = banks_client.get_account(market_pda).await.unwrap().unwrap();

    assert!(market_account.lamports > 0);
    assert_eq!(market_account.owner, id());

    // Deserialize and verify market state
    let market: Market = Market::try_from_slice(&market_account.data[..]).unwrap_or_else(|_| {
        // If try_from_slice fails due to extra bytes, use deserialize
        Market::deserialize(&mut &market_account.data[..]).unwrap()
    });

    assert_eq!(market.authority, authority.pubkey());
    assert_eq!(market.market_id, market_id);
    assert_eq!(market.oracle, oracle);
    assert_eq!(market.question_hash, question_hash);
    assert_eq!(market.end_time, end_time);
    assert_eq!(market.max_bet, max_bet);
    assert_eq!(market.status, MarketStatus::Active);
    assert_eq!(market.total_yes_bets, 0);
    assert_eq!(market.total_no_bets, 0);
    assert_eq!(market.escrow, escrow_pda);

    // Verify escrow account was created
    let escrow_account = banks_client.get_account(escrow_pda).await.unwrap().unwrap();
    assert!(escrow_account.lamports > 0);
    assert_eq!(escrow_account.owner, id());
}

#[tokio::test]
async fn test_create_market_end_time_in_past() {
    let mut program_test = create_program_test();
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let authority = Keypair::new();
    let oracle = Pubkey::new_unique();
    let market_id = 2u64;
    let question_hash = [1u8; 32];
    let end_time = 1500000000i64; // Past time
    let max_bet = 1_000_000_000u64;

    // Airdrop to authority
    let airdrop_ix = system_instruction::transfer(&payer.pubkey(), &authority.pubkey(), 100_000_000_000);
    let airdrop_tx = Transaction::new_signed_with_payer(
        &[airdrop_ix],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    banks_client.process_transaction(airdrop_tx).await.unwrap();

    // Derive PDAs
    let (market_pda, _) =
        Pubkey::find_program_address(&[MARKET_SEED, &market_id.to_le_bytes()], &id());

    let (escrow_pda, _) =
        Pubkey::find_program_address(&[b"escrow", &market_id.to_le_bytes()], &id());

    // Create instruction
    let instruction = FutarchyInstruction::CreateMarket {
        market_id,
        question_hash,
        end_time,
        max_bet,
    };

    let instruction_data = borsh::to_vec(&instruction).unwrap();

    let create_market_ix = Instruction {
        program_id: id(),
        accounts: vec![
            solana_program::instruction::AccountMeta::new(authority.pubkey(), true),
            solana_program::instruction::AccountMeta::new(market_pda, false),
            solana_program::instruction::AccountMeta::new(escrow_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(oracle, false),
            solana_program::instruction::AccountMeta::new_readonly(
                solana_program::system_program::ID,
                false,
            ),
            solana_program::instruction::AccountMeta::new_readonly(
                solana_program::sysvar::clock::ID,
                false,
            ),
        ],
        data: instruction_data,
    };

    let tx = Transaction::new_signed_with_payer(
        &[create_market_ix],
        Some(&payer.pubkey()),
        &[&payer, &authority],
        recent_blockhash,
    );

    let result = banks_client.process_transaction(tx).await;

    // Should fail
    assert!(result.is_err(), "Expected transaction to fail with end time in past");
}
