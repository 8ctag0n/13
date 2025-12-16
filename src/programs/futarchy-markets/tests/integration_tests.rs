//! Integration tests using solana-program-test
//!
//! These tests run with full Solana environment including CPIs.
//! They're slower than unit tests but necessary for testing
//! processor logic and cross-program interactions.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
    sysvar,
};
use solana_program_test::{processor, ProgramTest, ProgramTestContext};
use solana_sdk::{
    account::Account,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

use futarchy_markets::{
    entrypoint::process_instruction,
    instruction::FutarchyInstruction,
    state::{ExecutableAction, Market, MarketStatus, Position, UserEligibility, MARKET_SEED},
};

/// Test helpers module
mod helpers {
    use super::*;

    /// Setup ProgramTest with futarchy-markets program
    pub fn setup_program_test() -> ProgramTest {
        let mut program_test = ProgramTest::new(
            "futarchy_markets",
            futarchy_markets::id(),
            processor!(process_instruction),
        );

        // Add other programs if needed (zk-generator, fhe-generator, bedrock)
        // For now, we'll test without them and mock their behavior

        program_test
    }

    /// Derive market PDA
    pub fn derive_market_pda(program_id: &Pubkey, market_id: u64) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[MARKET_SEED, &market_id.to_le_bytes()],
            program_id,
        )
    }

    /// Derive escrow PDA
    pub fn derive_escrow_pda(program_id: &Pubkey, market_id: u64) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[b"escrow", &market_id.to_le_bytes()],
            program_id,
        )
    }

    /// Derive position PDA
    pub fn derive_position_pda(
        program_id: &Pubkey,
        user: &Pubkey,
        bet_commitment: &[u8; 32],
    ) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[b"position", user.as_ref(), bet_commitment],
            program_id,
        )
    }

    /// Derive user eligibility PDA
    pub fn derive_user_eligibility_pda(program_id: &Pubkey, user: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"user_eligibility", user.as_ref()], program_id)
    }

    /// Build CreateMarket instruction
    pub fn create_market_instruction(
        program_id: &Pubkey,
        authority: &Pubkey,
        oracle: &Pubkey,
        market_id: u64,
        question_hash: [u8; 32],
        end_time: i64,
        max_bet: u64,
    ) -> Instruction {
        let (market_pda, _) = derive_market_pda(program_id, market_id);
        let (escrow_pda, _) = derive_escrow_pda(program_id, market_id);

        let instruction = FutarchyInstruction::CreateMarket {
            market_id,
            question_hash,
            end_time,
            max_bet,
        };

        let data = instruction.pack().unwrap();

        Instruction {
            program_id: *program_id,
            accounts: vec![
                AccountMeta::new(*authority, true),
                AccountMeta::new(market_pda, false),
                AccountMeta::new(escrow_pda, false),
                AccountMeta::new_readonly(*oracle, false),
                AccountMeta::new_readonly(system_program::id(), false),
                AccountMeta::new_readonly(sysvar::clock::id(), false),
            ],
            data,
        }
    }

    /// Build CreateMarketWithGovernance instruction
    pub fn create_market_with_governance_instruction(
        program_id: &Pubkey,
        authority: &Pubkey,
        oracle: &Pubkey,
        market_id: u64,
        question_hash: [u8; 32],
        end_time: i64,
        max_bet: u64,
        executable_action: ExecutableAction,
        execution_threshold: u8,
        timelock_duration: i64,
    ) -> Instruction {
        let (market_pda, _) = derive_market_pda(program_id, market_id);
        let (escrow_pda, _) = derive_escrow_pda(program_id, market_id);

        let instruction = FutarchyInstruction::CreateMarketWithGovernance {
            market_id,
            question_hash,
            end_time,
            max_bet,
            executable_action,
            execution_threshold,
            timelock_duration,
        };

        let data = instruction.pack().unwrap();

        Instruction {
            program_id: *program_id,
            accounts: vec![
                AccountMeta::new(*authority, true),
                AccountMeta::new(market_pda, false),
                AccountMeta::new(escrow_pda, false),
                AccountMeta::new_readonly(*oracle, false),
                AccountMeta::new_readonly(system_program::id(), false),
                AccountMeta::new_readonly(sysvar::clock::id(), false),
            ],
            data,
        }
    }

    /// Build SettleMarket instruction
    pub fn settle_market_instruction(
        program_id: &Pubkey,
        oracle: &Pubkey,
        market_id: u64,
        outcome: bool,
    ) -> Instruction {
        let (market_pda, _) = derive_market_pda(program_id, market_id);

        let instruction = FutarchyInstruction::SettleMarket { market_id, outcome };

        let data = instruction.pack().unwrap();

        Instruction {
            program_id: *program_id,
            accounts: vec![
                AccountMeta::new(*oracle, true),
                AccountMeta::new(market_pda, false),
                AccountMeta::new_readonly(sysvar::clock::id(), false),
            ],
            data,
        }
    }

    /// Get account and deserialize as Market
    pub async fn get_market_account(
        context: &mut ProgramTestContext,
        market_pda: &Pubkey,
    ) -> Market {
        let account = context
            .banks_client
            .get_account(*market_pda)
            .await
            .unwrap()
            .expect("Market account not found");

        // Borsh deserialization reads only what it needs, but we need to handle the full account data
        let mut data_slice = &account.data[..];
        Market::deserialize(&mut data_slice).expect("Failed to deserialize Market")
    }

    /// Get account and deserialize as Position
    pub async fn get_position_account(
        context: &mut ProgramTestContext,
        position_pda: &Pubkey,
    ) -> Position {
        let account = context
            .banks_client
            .get_account(*position_pda)
            .await
            .unwrap()
            .expect("Position account not found");

        Position::try_from_slice(&account.data).expect("Failed to deserialize Position")
    }

    /// Get account and deserialize as UserEligibility
    pub async fn get_user_eligibility_account(
        context: &mut ProgramTestContext,
        eligibility_pda: &Pubkey,
    ) -> UserEligibility {
        let account = context
            .banks_client
            .get_account(*eligibility_pda)
            .await
            .unwrap()
            .expect("UserEligibility account not found");

        UserEligibility::try_from_slice(&account.data)
            .expect("Failed to deserialize UserEligibility")
    }

    /// Check if account exists
    pub async fn account_exists(context: &mut ProgramTestContext, pubkey: &Pubkey) -> bool {
        context
            .banks_client
            .get_account(*pubkey)
            .await
            .unwrap()
            .is_some()
    }

    /// Get account balance in lamports
    pub async fn get_balance(context: &mut ProgramTestContext, pubkey: &Pubkey) -> u64 {
        context
            .banks_client
            .get_balance(*pubkey)
            .await
            .unwrap()
    }

    /// Helper to force market end_time to the past (for testing settlement)
    pub async fn force_market_ended(
        context: &mut ProgramTestContext,
        program_id: &Pubkey,
        market_pda: &Pubkey,
    ) {
        let mut market = get_market_account(context, market_pda).await;
        market.end_time = 1; // Very old timestamp (1970)

        let account_data = context
            .banks_client
            .get_account(*market_pda)
            .await
            .unwrap()
            .unwrap();
        let mut data = account_data.data.clone();
        market.serialize(&mut &mut data[..]).unwrap();

        context.set_account(
            market_pda,
            &Account {
                lamports: account_data.lamports,
                data,
                owner: *program_id,
                executable: false,
                rent_epoch: account_data.rent_epoch,
            }
            .into(),
        );
    }
}

/// Tests for CreateMarket instruction
#[cfg(test)]
mod create_market_tests {
    use super::*;
    use helpers::*;

    #[tokio::test]
    async fn test_create_market_success() {
        let program_id = futarchy_markets::id();
        let mut program_test = setup_program_test();

        // Create test accounts
        let authority = Keypair::new();
        let oracle = Keypair::new();

        // Fund authority
        program_test.add_account(
            authority.pubkey(),
            Account {
                lamports: 10_000_000_000, // 10 SOL
                ..Default::default()
            },
        );

        let mut context = program_test.start_with_context().await;

        // Market parameters
        let market_id = 1u64;
        let question_hash = [1u8; 32];
        let current_time = context.banks_client.get_sysvar::<solana_program::clock::Clock>()
            .await
            .unwrap()
            .unix_timestamp;
        let end_time = current_time + 86400; // 1 day from now
        let max_bet = 1_000_000_000; // 1 SOL

        // Derive PDAs
        let (market_pda, _) = derive_market_pda(&program_id, market_id);
        let (escrow_pda, _) = derive_escrow_pda(&program_id, market_id);

        // Build instruction
        let ix = create_market_instruction(
            &program_id,
            &authority.pubkey(),
            &oracle.pubkey(),
            market_id,
            question_hash,
            end_time,
            max_bet,
        );

        // Execute transaction
        let transaction = Transaction::new_signed_with_payer(
            &[ix],
            Some(&authority.pubkey()),
            &[&authority],
            context.last_blockhash,
        );

        context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        // Verify market account was created
        assert!(account_exists(&mut context, &market_pda).await);

        // Verify escrow account was created
        assert!(account_exists(&mut context, &escrow_pda).await);

        // Load and verify market state
        let market = get_market_account(&mut context, &market_pda).await;

        assert_eq!(market.authority, authority.pubkey());
        assert_eq!(market.market_id, market_id);
        assert_eq!(market.oracle, oracle.pubkey());
        assert_eq!(market.question_hash, question_hash);
        assert_eq!(market.end_time, end_time);
        assert_eq!(market.max_bet, max_bet);
        assert_eq!(market.status, MarketStatus::Active);
        assert_eq!(market.total_yes_bets, 0);
        assert_eq!(market.total_no_bets, 0);
        assert!(market.encrypted_pool_yes.is_empty());
        assert!(market.encrypted_pool_no.is_empty());
        assert_eq!(market.resolution, None);
        assert_eq!(market.settled_at, None);
        assert_eq!(market.escrow, escrow_pda);
        assert!(market.claim_nullifiers.is_empty());
        assert!(market.executable_action.is_none());
        assert_eq!(market.execution_threshold, 0);
        assert_eq!(market.timelock_duration, 0);
        assert_eq!(market.timelock_expires_at, None);
        assert!(!market.action_executed);
    }

    #[tokio::test]
    async fn test_create_market_invalid_end_time() {
        let program_id = futarchy_markets::id();
        let mut program_test = setup_program_test();

        let authority = Keypair::new();
        let oracle = Keypair::new();

        program_test.add_account(
            authority.pubkey(),
            Account {
                lamports: 10_000_000_000,
                ..Default::default()
            },
        );

        let mut context = program_test.start_with_context().await;

        let market_id = 1u64;
        let question_hash = [1u8; 32];
        let end_time = 1000000000; // Time in the past
        let max_bet = 1_000_000_000;

        let ix = create_market_instruction(
            &program_id,
            &authority.pubkey(),
            &oracle.pubkey(),
            market_id,
            question_hash,
            end_time,
            max_bet,
        );

        let transaction = Transaction::new_signed_with_payer(
            &[ix],
            Some(&authority.pubkey()),
            &[&authority],
            context.last_blockhash,
        );

        // Should fail because end_time is in the past
        let result = context.banks_client.process_transaction(transaction).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_market_invalid_pda() {
        let program_id = futarchy_markets::id();
        let mut program_test = setup_program_test();

        let authority = Keypair::new();
        let oracle = Keypair::new();

        program_test.add_account(
            authority.pubkey(),
            Account {
                lamports: 10_000_000_000,
                ..Default::default()
            },
        );

        let mut context = program_test.start_with_context().await;

        let market_id = 1u64;
        let question_hash = [1u8; 32];
        let current_time = context
            .banks_client
            .get_sysvar::<solana_program::clock::Clock>()
            .await
            .unwrap()
            .unix_timestamp;
        let end_time = current_time + 86400;
        let max_bet = 1_000_000_000;

        // Use wrong PDA (different market_id)
        let (wrong_market_pda, _) = derive_market_pda(&program_id, market_id + 1);
        let (escrow_pda, _) = derive_escrow_pda(&program_id, market_id);

        let instruction = FutarchyInstruction::CreateMarket {
            market_id,
            question_hash,
            end_time,
            max_bet,
        };

        let data = instruction.pack().unwrap();

        let ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(authority.pubkey(), true),
                AccountMeta::new(wrong_market_pda, false), // Wrong PDA
                AccountMeta::new(escrow_pda, false),
                AccountMeta::new_readonly(oracle.pubkey(), false),
                AccountMeta::new_readonly(system_program::id(), false),
                AccountMeta::new_readonly(sysvar::clock::id(), false),
            ],
            data,
        };

        let transaction = Transaction::new_signed_with_payer(
            &[ix],
            Some(&authority.pubkey()),
            &[&authority],
            context.last_blockhash,
        );

        // Should fail due to invalid PDA
        let result = context.banks_client.process_transaction(transaction).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_market_missing_signature() {
        let program_id = futarchy_markets::id();
        let mut program_test = setup_program_test();

        let authority = Keypair::new();
        let oracle = Keypair::new();
        let fake_payer = Keypair::new(); // Different payer to avoid authority signature

        program_test.add_account(
            authority.pubkey(),
            Account {
                lamports: 10_000_000_000,
                ..Default::default()
            },
        );

        program_test.add_account(
            fake_payer.pubkey(),
            Account {
                lamports: 10_000_000_000,
                ..Default::default()
            },
        );

        let mut context = program_test.start_with_context().await;

        let market_id = 1u64;
        let question_hash = [1u8; 32];
        let current_time = context
            .banks_client
            .get_sysvar::<solana_program::clock::Clock>()
            .await
            .unwrap()
            .unix_timestamp;
        let end_time = current_time + 86400;
        let max_bet = 1_000_000_000;

        let (market_pda, _) = derive_market_pda(&program_id, market_id);
        let (escrow_pda, _) = derive_escrow_pda(&program_id, market_id);

        let instruction = FutarchyInstruction::CreateMarket {
            market_id,
            question_hash,
            end_time,
            max_bet,
        };

        let data = instruction.pack().unwrap();

        let ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(authority.pubkey(), false), // Not a signer
                AccountMeta::new(market_pda, false),
                AccountMeta::new(escrow_pda, false),
                AccountMeta::new_readonly(oracle.pubkey(), false),
                AccountMeta::new_readonly(system_program::id(), false),
                AccountMeta::new_readonly(sysvar::clock::id(), false),
            ],
            data,
        };

        // Sign with fake_payer only, not with authority
        let transaction = Transaction::new_signed_with_payer(
            &[ix],
            Some(&fake_payer.pubkey()),
            &[&fake_payer], // Authority is NOT signing
            context.last_blockhash,
        );

        // Should fail due to missing signature
        let result = context.banks_client.process_transaction(transaction).await;
        assert!(result.is_err());
    }
}

/// Tests for SettleMarket instruction
#[cfg(test)]
mod settle_market_tests {
    use super::*;
    use helpers::*;

    #[tokio::test]
    async fn test_settle_market_success() {
        let program_id = futarchy_markets::id();
        let mut program_test = setup_program_test();

        let authority = Keypair::new();
        let oracle = Keypair::new();

        program_test.add_account(
            authority.pubkey(),
            Account {
                lamports: 10_000_000_000,
                ..Default::default()
            },
        );

        program_test.add_account(
            oracle.pubkey(),
            Account {
                lamports: 10_000_000_000,
                ..Default::default()
            },
        );

        let mut context = program_test.start_with_context().await;

        // Create market first
        let market_id = 1u64;
        let question_hash = [1u8; 32];
        let max_bet = 1_000_000_000;
        let (market_pda, _) = derive_market_pda(&program_id, market_id);

        // First create market with valid end_time
        let current_time = context
            .banks_client
            .get_sysvar::<solana_program::clock::Clock>()
            .await
            .unwrap()
            .unix_timestamp;
        let end_time = current_time + 86400; // 1 day in future (will pass validation)

        let create_ix = create_market_instruction(
            &program_id,
            &authority.pubkey(),
            &oracle.pubkey(),
            market_id,
            question_hash,
            end_time,
            max_bet,
        );

        let tx = Transaction::new_signed_with_payer(
            &[create_ix],
            Some(&authority.pubkey()),
            &[&authority],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        // Force market end_time to be in the past
        force_market_ended(&mut context, &program_id, &market_pda).await;

        // Settle market
        let outcome = true; // YES wins
        let settle_ix = settle_market_instruction(&program_id, &oracle.pubkey(), market_id, outcome);

        let tx = Transaction::new_signed_with_payer(
            &[settle_ix],
            Some(&oracle.pubkey()),
            &[&oracle],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        // Verify market is settled
        let market = get_market_account(&mut context, &market_pda).await;
        assert_eq!(market.status, MarketStatus::Settled);
        assert_eq!(market.resolution, Some(true));
        assert!(market.settled_at.is_some());
    }

    #[tokio::test]
    async fn test_settle_market_invalid_oracle() {
        let program_id = futarchy_markets::id();
        let mut program_test = setup_program_test();

        let authority = Keypair::new();
        let oracle = Keypair::new();
        let fake_oracle = Keypair::new();

        program_test.add_account(
            authority.pubkey(),
            Account {
                lamports: 10_000_000_000,
                ..Default::default()
            },
        );

        program_test.add_account(
            fake_oracle.pubkey(),
            Account {
                lamports: 10_000_000_000,
                ..Default::default()
            },
        );

        let mut context = program_test.start_with_context().await;

        // Create market
        let market_id = 1u64;
        let question_hash = [1u8; 32];
        let current_time = context
            .banks_client
            .get_sysvar::<solana_program::clock::Clock>()
            .await
            .unwrap()
            .unix_timestamp;
        let end_time = current_time + 1;
        let max_bet = 1_000_000_000;

        let create_ix = create_market_instruction(
            &program_id,
            &authority.pubkey(),
            &oracle.pubkey(),
            market_id,
            question_hash,
            end_time,
            max_bet,
        );

        let tx = Transaction::new_signed_with_payer(
            &[create_ix],
            Some(&authority.pubkey()),
            &[&authority],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        // Warp time
        context.warp_to_slot(100).unwrap();

        // Try to settle with wrong oracle
        let settle_ix = settle_market_instruction(&program_id, &fake_oracle.pubkey(), market_id, true);

        let tx = Transaction::new_signed_with_payer(
            &[settle_ix],
            Some(&fake_oracle.pubkey()),
            &[&fake_oracle],
            context.last_blockhash,
        );

        // Should fail
        let result = context.banks_client.process_transaction(tx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_settle_market_missing_signature() {
        let program_id = futarchy_markets::id();
        let mut program_test = setup_program_test();

        let authority = Keypair::new();
        let oracle = Keypair::new();
        let fake_payer = Keypair::new();

        program_test.add_account(
            authority.pubkey(),
            Account {
                lamports: 10_000_000_000,
                ..Default::default()
            },
        );

        program_test.add_account(
            fake_payer.pubkey(),
            Account {
                lamports: 10_000_000_000,
                ..Default::default()
            },
        );

        let mut context = program_test.start_with_context().await;

        // Create market
        let market_id = 1u64;
        let question_hash = [1u8; 32];
        let current_time = context
            .banks_client
            .get_sysvar::<solana_program::clock::Clock>()
            .await
            .unwrap()
            .unix_timestamp;
        let end_time = current_time + 1;
        let max_bet = 1_000_000_000;

        let create_ix = create_market_instruction(
            &program_id,
            &authority.pubkey(),
            &oracle.pubkey(),
            market_id,
            question_hash,
            end_time,
            max_bet,
        );

        let tx = Transaction::new_signed_with_payer(
            &[create_ix],
            Some(&authority.pubkey()),
            &[&authority],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        // Warp time
        context.warp_to_slot(100).unwrap();

        // Try to settle without oracle signature
        let (market_pda, _) = derive_market_pda(&program_id, market_id);

        let instruction = FutarchyInstruction::SettleMarket {
            market_id,
            outcome: true,
        };

        let data = instruction.pack().unwrap();

        let ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(oracle.pubkey(), false), // Not a signer
                AccountMeta::new(market_pda, false),
                AccountMeta::new_readonly(sysvar::clock::id(), false),
            ],
            data,
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&fake_payer.pubkey()),
            &[&fake_payer],
            context.last_blockhash,
        );

        // Should fail
        let result = context.banks_client.process_transaction(tx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_settle_market_already_settled() {
        let program_id = futarchy_markets::id();
        let mut program_test = setup_program_test();

        let authority = Keypair::new();
        let oracle = Keypair::new();

        program_test.add_account(
            authority.pubkey(),
            Account {
                lamports: 10_000_000_000,
                ..Default::default()
            },
        );

        program_test.add_account(
            oracle.pubkey(),
            Account {
                lamports: 10_000_000_000,
                ..Default::default()
            },
        );

        let mut context = program_test.start_with_context().await;

        // Create market
        let market_id = 1u64;
        let question_hash = [1u8; 32];
        let current_time = context
            .banks_client
            .get_sysvar::<solana_program::clock::Clock>()
            .await
            .unwrap()
            .unix_timestamp;
        let end_time = current_time + 86400;
        let max_bet = 1_000_000_000;

        let (market_pda, _) = derive_market_pda(&program_id, market_id);

        let create_ix = create_market_instruction(
            &program_id,
            &authority.pubkey(),
            &oracle.pubkey(),
            market_id,
            question_hash,
            end_time,
            max_bet,
        );

        let tx = Transaction::new_signed_with_payer(
            &[create_ix],
            Some(&authority.pubkey()),
            &[&authority],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        // Force market to be ended
        force_market_ended(&mut context, &program_id, &market_pda).await;

        // Settle once
        let settle_ix = settle_market_instruction(&program_id, &oracle.pubkey(), market_id, true);

        let tx = Transaction::new_signed_with_payer(
            &[settle_ix],
            Some(&oracle.pubkey()),
            &[&oracle],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        // Try to settle again
        let settle_ix2 = settle_market_instruction(&program_id, &oracle.pubkey(), market_id, false);

        let tx2 = Transaction::new_signed_with_payer(
            &[settle_ix2],
            Some(&oracle.pubkey()),
            &[&oracle],
            context.last_blockhash,
        );

        // Should fail
        let result = context.banks_client.process_transaction(tx2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_settle_market_betting_not_closed() {
        let program_id = futarchy_markets::id();
        let mut program_test = setup_program_test();

        let authority = Keypair::new();
        let oracle = Keypair::new();

        program_test.add_account(
            authority.pubkey(),
            Account {
                lamports: 10_000_000_000,
                ..Default::default()
            },
        );

        program_test.add_account(
            oracle.pubkey(),
            Account {
                lamports: 10_000_000_000,
                ..Default::default()
            },
        );

        let mut context = program_test.start_with_context().await;

        // Create market
        let market_id = 1u64;
        let question_hash = [1u8; 32];
        let current_time = context
            .banks_client
            .get_sysvar::<solana_program::clock::Clock>()
            .await
            .unwrap()
            .unix_timestamp;
        let end_time = current_time + 86400; // Far in future
        let max_bet = 1_000_000_000;

        let create_ix = create_market_instruction(
            &program_id,
            &authority.pubkey(),
            &oracle.pubkey(),
            market_id,
            question_hash,
            end_time,
            max_bet,
        );

        let tx = Transaction::new_signed_with_payer(
            &[create_ix],
            Some(&authority.pubkey()),
            &[&authority],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        // Try to settle immediately without warping time
        let settle_ix = settle_market_instruction(&program_id, &oracle.pubkey(), market_id, true);

        let tx = Transaction::new_signed_with_payer(
            &[settle_ix],
            Some(&oracle.pubkey()),
            &[&oracle],
            context.last_blockhash,
        );

        // Should fail
        let result = context.banks_client.process_transaction(tx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_settle_market_governance_threshold_met() {
        let program_id = futarchy_markets::id();
        let mut program_test = setup_program_test();

        let authority = Keypair::new();
        let oracle = Keypair::new();

        program_test.add_account(
            authority.pubkey(),
            Account {
                lamports: 10_000_000_000,
                ..Default::default()
            },
        );

        program_test.add_account(
            oracle.pubkey(),
            Account {
                lamports: 10_000_000_000,
                ..Default::default()
            },
        );

        let mut context = program_test.start_with_context().await;

        // Create governance market
        let market_id = 1u64;
        let question_hash = [1u8; 32];
        let current_time = context
            .banks_client
            .get_sysvar::<solana_program::clock::Clock>()
            .await
            .unwrap()
            .unix_timestamp;
        let end_time = current_time + 86400;
        let max_bet = 1_000_000_000;

        let executable_action = ExecutableAction::TransferTokens {
            token_mint: Pubkey::new_unique(),
            from_treasury: Pubkey::new_unique(),
            to: Pubkey::new_unique(),
            amount: 1_000_000,
        };

        let execution_threshold = 50; // 50%
        let timelock_duration = 3600; // 1 hour

        let (market_pda, _) = derive_market_pda(&program_id, market_id);

        let create_ix = create_market_with_governance_instruction(
            &program_id,
            &authority.pubkey(),
            &oracle.pubkey(),
            market_id,
            question_hash,
            end_time,
            max_bet,
            executable_action.clone(),
            execution_threshold,
            timelock_duration,
        );

        let tx = Transaction::new_signed_with_payer(
            &[create_ix],
            Some(&authority.pubkey()),
            &[&authority],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        // Manually update market with some votes (simulating PlaceBet)
        let mut market = get_market_account(&mut context, &market_pda).await;
        market.total_yes_bets = 60_000_000_000; // 60 SOL
        market.total_no_bets = 40_000_000_000; // 40 SOL
        market.end_time = 1; // Force to past so settlement works

        // Write back
        let account_data = context
            .banks_client
            .get_account(market_pda)
            .await
            .unwrap()
            .unwrap();
        let mut data = account_data.data.clone();
        market.serialize(&mut &mut data[..]).unwrap();

        // Update account with new data
        context.set_account(
            &market_pda,
            &Account {
                lamports: account_data.lamports,
                data,
                owner: program_id,
                executable: false,
                rent_epoch: account_data.rent_epoch,
            }
            .into(),
        );

        // Settle market
        let settle_ix = settle_market_instruction(&program_id, &oracle.pubkey(), market_id, true);

        let tx = Transaction::new_signed_with_payer(
            &[settle_ix],
            Some(&oracle.pubkey()),
            &[&oracle],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        // Verify governance timelock was set
        let market = get_market_account(&mut context, &market_pda).await;
        assert_eq!(market.status, MarketStatus::Settled);
        assert!(market.timelock_expires_at.is_some());
        assert!(!market.action_executed);
        assert!(market.threshold_met());
    }
}
