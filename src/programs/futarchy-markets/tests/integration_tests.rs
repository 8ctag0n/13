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
    state::{ExecutableAction, Market, MarketStatus, Position, UserEligibility, UserEscrow, MARKET_SEED},
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
        market_id: u64,
        bet_commitment: &[u8; 32],
    ) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[b"position", user.as_ref(), &market_id.to_le_bytes(), bet_commitment],
            program_id,
        )
    }

    /// Derive user eligibility PDA
    pub fn derive_user_eligibility_pda(program_id: &Pubkey, user: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"user_eligibility", user.as_ref()], program_id)
    }

    /// Derive user escrow PDA
    pub fn derive_user_escrow_pda(program_id: &Pubkey, user: &Pubkey, market: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"user_escrow", user.as_ref(), market.as_ref()], program_id)
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

    /// Build CancelMarket instruction
    pub fn cancel_market_instruction(
        program_id: &Pubkey,
        authority: &Pubkey,
        market_id: u64,
    ) -> Instruction {
        let (market_pda, _) = derive_market_pda(program_id, market_id);

        let instruction = FutarchyInstruction::CancelMarket { market_id };

        let data = instruction.pack().unwrap();

        Instruction {
            program_id: *program_id,
            accounts: vec![
                AccountMeta::new(*authority, true),
                AccountMeta::new(market_pda, false),
            ],
            data,
        }
    }

    /// Build DepositToMarket instruction
    pub fn deposit_to_market_instruction(
        program_id: &Pubkey,
        user: &Pubkey,
        market_id: u64,
        amount: u64,
    ) -> Instruction {
        let (market_pda, _) = derive_market_pda(program_id, market_id);
        let (user_escrow_pda, _) = derive_user_escrow_pda(program_id, user, &market_pda);

        let instruction = FutarchyInstruction::DepositToMarket { market_id, amount };

        let data = instruction.pack().unwrap();

        Instruction {
            program_id: *program_id,
            accounts: vec![
                AccountMeta::new(*user, true),
                AccountMeta::new(user_escrow_pda, false),
                AccountMeta::new_readonly(market_pda, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data,
        }
    }

    /// Build WithdrawFromEscrow instruction
    pub fn withdraw_from_escrow_instruction(
        program_id: &Pubkey,
        user: &Pubkey,
        market_id: u64,
        amount: u64,
    ) -> Instruction {
        let (market_pda, _) = derive_market_pda(program_id, market_id);
        let (user_escrow_pda, _) = derive_user_escrow_pda(program_id, user, &market_pda);

        let instruction = FutarchyInstruction::WithdrawFromEscrow { market_id, amount };

        let data = instruction.pack().unwrap();

        Instruction {
            program_id: *program_id,
            accounts: vec![
                AccountMeta::new(*user, true),
                AccountMeta::new(user_escrow_pda, false),
                AccountMeta::new_readonly(market_pda, false),
                AccountMeta::new_readonly(system_program::id(), false),
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

    /// Get account and deserialize as UserEscrow
    pub async fn get_user_escrow_account(
        context: &mut ProgramTestContext,
        user_escrow_pda: &Pubkey,
    ) -> UserEscrow {
        let account = context
            .banks_client
            .get_account(*user_escrow_pda)
            .await
            .unwrap()
            .expect("UserEscrow account not found");

        UserEscrow::try_from_slice(&account.data)
            .expect("Failed to deserialize UserEscrow")
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

/// Tests for CancelMarket instruction
#[cfg(test)]
mod cancel_market_tests {
    use super::*;
    use helpers::*;

    #[tokio::test]
    async fn test_cancel_market_success() {
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

        // Cancel market
        let cancel_ix = cancel_market_instruction(&program_id, &authority.pubkey(), market_id);

        let tx = Transaction::new_signed_with_payer(
            &[cancel_ix],
            Some(&authority.pubkey()),
            &[&authority],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        // Verify market is cancelled
        let market = get_market_account(&mut context, &market_pda).await;
        assert_eq!(market.status, MarketStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_cancel_market_unauthorized() {
        let program_id = futarchy_markets::id();
        let mut program_test = setup_program_test();

        let authority = Keypair::new();
        let oracle = Keypair::new();
        let fake_authority = Keypair::new();

        program_test.add_account(
            authority.pubkey(),
            Account {
                lamports: 10_000_000_000,
                ..Default::default()
            },
        );

        program_test.add_account(
            fake_authority.pubkey(),
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

        // Try to cancel with wrong authority
        let cancel_ix = cancel_market_instruction(&program_id, &fake_authority.pubkey(), market_id);

        let tx = Transaction::new_signed_with_payer(
            &[cancel_ix],
            Some(&fake_authority.pubkey()),
            &[&fake_authority],
            context.last_blockhash,
        );

        // Should fail
        let result = context.banks_client.process_transaction(tx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cancel_market_missing_signature() {
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
        let end_time = current_time + 86400;
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

        // Try to cancel without authority signature
        let (market_pda, _) = derive_market_pda(&program_id, market_id);

        let instruction = FutarchyInstruction::CancelMarket { market_id };

        let data = instruction.pack().unwrap();

        let ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(authority.pubkey(), false), // Not a signer
                AccountMeta::new(market_pda, false),
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
    async fn test_cancel_market_already_settled() {
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

        // Force market to be ended and settle it
        force_market_ended(&mut context, &program_id, &market_pda).await;

        let settle_ix = settle_market_instruction(&program_id, &oracle.pubkey(), market_id, true);

        let tx = Transaction::new_signed_with_payer(
            &[settle_ix],
            Some(&oracle.pubkey()),
            &[&oracle],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        // Try to cancel settled market
        let cancel_ix = cancel_market_instruction(&program_id, &authority.pubkey(), market_id);

        let tx = Transaction::new_signed_with_payer(
            &[cancel_ix],
            Some(&authority.pubkey()),
            &[&authority],
            context.last_blockhash,
        );

        // Should fail
        let result = context.banks_client.process_transaction(tx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cancel_market_invalid_pda() {
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

        // Try to cancel with wrong market PDA
        let (wrong_market_pda, _) = derive_market_pda(&program_id, market_id + 1);

        let instruction = FutarchyInstruction::CancelMarket { market_id };

        let data = instruction.pack().unwrap();

        let ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(authority.pubkey(), true),
                AccountMeta::new(wrong_market_pda, false), // Wrong PDA
            ],
            data,
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&authority.pubkey()),
            &[&authority],
            context.last_blockhash,
        );

        // Should fail
        let result = context.banks_client.process_transaction(tx).await;
        assert!(result.is_err());
    }
}

/// Tests for CreateMarketWithGovernance instruction
#[cfg(test)]
mod create_governance_market_tests {
    use super::*;
    use helpers::*;

    #[tokio::test]
    async fn test_create_governance_market_success() {
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

        let executable_action = ExecutableAction::TransferTokens {
            token_mint: Pubkey::new_unique(),
            from_treasury: Pubkey::new_unique(),
            to: Pubkey::new_unique(),
            amount: 1_000_000,
        };

        let execution_threshold = 50;
        let timelock_duration = 3600;

        let (market_pda, _) = derive_market_pda(&program_id, market_id);

        let ix = create_market_with_governance_instruction(
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
            &[ix],
            Some(&authority.pubkey()),
            &[&authority],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        // Verify governance fields
        let market = get_market_account(&mut context, &market_pda).await;
        assert_eq!(market.status, MarketStatus::Active);
        assert!(market.is_governance_market());
        assert_eq!(market.execution_threshold, execution_threshold);
        assert_eq!(market.timelock_duration, timelock_duration);
        assert_eq!(market.timelock_expires_at, None);
        assert!(!market.action_executed);
        assert_eq!(market.executable_action.action_type(), "TransferTokens");
    }

    #[tokio::test]
    async fn test_create_governance_market_invalid_threshold() {
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

        let executable_action = ExecutableAction::TransferTokens {
            token_mint: Pubkey::new_unique(),
            from_treasury: Pubkey::new_unique(),
            to: Pubkey::new_unique(),
            amount: 1_000_000,
        };

        let execution_threshold = 101; // Invalid (>100)
        let timelock_duration = 3600;

        let ix = create_market_with_governance_instruction(
            &program_id,
            &authority.pubkey(),
            &oracle.pubkey(),
            market_id,
            question_hash,
            end_time,
            max_bet,
            executable_action,
            execution_threshold,
            timelock_duration,
        );

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&authority.pubkey()),
            &[&authority],
            context.last_blockhash,
        );

        // Should fail
        let result = context.banks_client.process_transaction(tx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_governance_market_negative_timelock() {
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

        let executable_action = ExecutableAction::TransferTokens {
            token_mint: Pubkey::new_unique(),
            from_treasury: Pubkey::new_unique(),
            to: Pubkey::new_unique(),
            amount: 1_000_000,
        };

        let execution_threshold = 50;
        let timelock_duration = -1; // Invalid (negative)

        let ix = create_market_with_governance_instruction(
            &program_id,
            &authority.pubkey(),
            &oracle.pubkey(),
            market_id,
            question_hash,
            end_time,
            max_bet,
            executable_action,
            execution_threshold,
            timelock_duration,
        );

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&authority.pubkey()),
            &[&authority],
            context.last_blockhash,
        );

        // Should fail
        let result = context.banks_client.process_transaction(tx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_governance_market_action_none() {
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

        let executable_action = ExecutableAction::None; // No action

        let execution_threshold = 50;
        let timelock_duration = 3600;

        let (market_pda, _) = derive_market_pda(&program_id, market_id);

        let ix = create_market_with_governance_instruction(
            &program_id,
            &authority.pubkey(),
            &oracle.pubkey(),
            market_id,
            question_hash,
            end_time,
            max_bet,
            executable_action,
            execution_threshold,
            timelock_duration,
        );

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&authority.pubkey()),
            &[&authority],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        // Verify it's not really a governance market (action is None)
        let market = get_market_account(&mut context, &market_pda).await;
        assert!(!market.is_governance_market());
        assert!(market.executable_action.is_none());
    }
}

/// Tests for User Escrow System (DepositToMarket, WithdrawFromEscrow)
#[cfg(test)]
mod user_escrow_tests {
    use super::*;
    use helpers::*;

    #[tokio::test]
    async fn test_deposit_creates_new_escrow() {
        let program_id = futarchy_markets::id();
        let mut program_test = setup_program_test();

        let authority = Keypair::new();
        let oracle = Keypair::new();
        let user = Keypair::new();

        program_test.add_account(
            authority.pubkey(),
            Account {
                lamports: 10_000_000_000,
                ..Default::default()
            },
        );

        program_test.add_account(
            user.pubkey(),
            Account {
                lamports: 10_000_000_000,
                ..Default::default()
            },
        );

        let mut context = program_test.start_with_context().await;

        // Create market first
        let market_id = 1u64;
        let question_hash = [1u8; 32];
        let current_time = context
            .banks_client
            .get_sysvar::<solana_program::clock::Clock>()
            .await
            .unwrap()
            .unix_timestamp;
        let end_time = current_time + 86400;
        let max_bet = 2_000_000_000;

        let (market_pda, _) = derive_market_pda(&program_id, market_id);
        let (user_escrow_pda, _) = derive_user_escrow_pda(&program_id, &user.pubkey(), &market_pda);

        let create_market_ix = create_market_instruction(
            &program_id,
            &authority.pubkey(),
            &oracle.pubkey(),
            market_id,
            question_hash,
            end_time,
            max_bet,
        );

        let tx = Transaction::new_signed_with_payer(
            &[create_market_ix],
            Some(&authority.pubkey()),
            &[&authority],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        // Deposit 5 SOL
        let deposit_amount = 5_000_000_000;

        let deposit_ix = deposit_to_market_instruction(
            &program_id,
            &user.pubkey(),
            market_id,
            deposit_amount,
        );

        let tx = Transaction::new_signed_with_payer(
            &[deposit_ix],
            Some(&user.pubkey()),
            &[&user],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        // Verify escrow created with correct balances
        let escrow = get_user_escrow_account(&mut context, &user_escrow_pda).await;
        assert_eq!(escrow.user, user.pubkey());
        assert_eq!(escrow.market, market_pda);
        assert_eq!(escrow.deposited, deposit_amount);
        assert_eq!(escrow.available, deposit_amount);
        assert_eq!(escrow.reserved, 0);
        assert!(escrow.verify_invariant());
    }

    #[tokio::test]
    async fn test_deposit_adds_to_existing_escrow() {
        let program_id = futarchy_markets::id();
        let mut program_test = setup_program_test();

        let authority = Keypair::new();
        let oracle = Keypair::new();
        let user = Keypair::new();

        program_test.add_account(
            authority.pubkey(),
            Account {
                lamports: 10_000_000_000,
                ..Default::default()
            },
        );

        program_test.add_account(
            user.pubkey(),
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
        let max_bet = 2_000_000_000;

        let (market_pda, _) = derive_market_pda(&program_id, market_id);
        let (user_escrow_pda, _) = derive_user_escrow_pda(&program_id, &user.pubkey(), &market_pda);

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

        // First deposit: 3 SOL
        let deposit1 = 3_000_000_000;
        let deposit_ix1 = deposit_to_market_instruction(&program_id, &user.pubkey(), market_id, deposit1);

        let tx = Transaction::new_signed_with_payer(
            &[deposit_ix1],
            Some(&user.pubkey()),
            &[&user],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        // Second deposit: 2 SOL
        let deposit2 = 2_000_000_000;
        let deposit_ix2 = deposit_to_market_instruction(&program_id, &user.pubkey(), market_id, deposit2);

        let tx = Transaction::new_signed_with_payer(
            &[deposit_ix2],
            Some(&user.pubkey()),
            &[&user],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        // Verify total is 5 SOL
        let escrow = get_user_escrow_account(&mut context, &user_escrow_pda).await;
        assert_eq!(escrow.deposited, 5_000_000_000);
        assert_eq!(escrow.available, 5_000_000_000);
        assert_eq!(escrow.reserved, 0);
        assert!(escrow.verify_invariant());
    }

    #[tokio::test]
    async fn test_withdraw_from_escrow() {
        let program_id = futarchy_markets::id();
        let mut program_test = setup_program_test();

        let authority = Keypair::new();
        let oracle = Keypair::new();
        let user = Keypair::new();

        program_test.add_account(
            authority.pubkey(),
            Account {
                lamports: 10_000_000_000,
                ..Default::default()
            },
        );

        program_test.add_account(
            user.pubkey(),
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
        let max_bet = 2_000_000_000;

        let (market_pda, _) = derive_market_pda(&program_id, market_id);
        let (user_escrow_pda, _) = derive_user_escrow_pda(&program_id, &user.pubkey(), &market_pda);

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

        // Deposit 5 SOL
        let deposit_amount = 5_000_000_000;
        let deposit_ix = deposit_to_market_instruction(&program_id, &user.pubkey(), market_id, deposit_amount);

        let tx = Transaction::new_signed_with_payer(
            &[deposit_ix],
            Some(&user.pubkey()),
            &[&user],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        // Withdraw 2 SOL
        let withdraw_amount = 2_000_000_000;
        let withdraw_ix = withdraw_from_escrow_instruction(&program_id, &user.pubkey(), market_id, withdraw_amount);

        let tx = Transaction::new_signed_with_payer(
            &[withdraw_ix],
            Some(&user.pubkey()),
            &[&user],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        // Verify escrow updated
        let escrow = get_user_escrow_account(&mut context, &user_escrow_pda).await;
        assert_eq!(escrow.deposited, 3_000_000_000); // 5 - 2 = 3 SOL
        assert_eq!(escrow.available, 3_000_000_000);
        assert_eq!(escrow.reserved, 0);
        assert!(escrow.verify_invariant());
    }

    #[tokio::test]
    async fn test_withdraw_insufficient_available() {
        let program_id = futarchy_markets::id();
        let mut program_test = setup_program_test();

        let authority = Keypair::new();
        let oracle = Keypair::new();
        let user = Keypair::new();

        program_test.add_account(
            authority.pubkey(),
            Account {
                lamports: 10_000_000_000,
                ..Default::default()
            },
        );

        program_test.add_account(
            user.pubkey(),
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
        let max_bet = 2_000_000_000;

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

        // Deposit 2 SOL
        let deposit_amount = 2_000_000_000;
        let deposit_ix = deposit_to_market_instruction(&program_id, &user.pubkey(), market_id, deposit_amount);

        let tx = Transaction::new_signed_with_payer(
            &[deposit_ix],
            Some(&user.pubkey()),
            &[&user],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        // Try to withdraw 5 SOL (more than deposited)
        let withdraw_amount = 5_000_000_000;
        let withdraw_ix = withdraw_from_escrow_instruction(&program_id, &user.pubkey(), market_id, withdraw_amount);

        let tx = Transaction::new_signed_with_payer(
            &[withdraw_ix],
            Some(&user.pubkey()),
            &[&user],
            context.last_blockhash,
        );

        // Should fail
        let result = context.banks_client.process_transaction(tx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_deposit_to_inactive_market_fails() {
        let program_id = futarchy_markets::id();
        let mut program_test = setup_program_test();

        let authority = Keypair::new();
        let oracle = Keypair::new();
        let user = Keypair::new();

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

        program_test.add_account(
            user.pubkey(),
            Account {
                lamports: 10_000_000_000,
                ..Default::default()
            },
        );

        let mut context = program_test.start_with_context().await;

        // Create and cancel market
        let market_id = 1u64;
        let question_hash = [1u8; 32];
        let current_time = context
            .banks_client
            .get_sysvar::<solana_program::clock::Clock>()
            .await
            .unwrap()
            .unix_timestamp;
        let end_time = current_time + 86400;
        let max_bet = 2_000_000_000;

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

        // Cancel market
        let cancel_ix = cancel_market_instruction(&program_id, &authority.pubkey(), market_id);

        let tx = Transaction::new_signed_with_payer(
            &[cancel_ix],
            Some(&authority.pubkey()),
            &[&authority],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        // Try to deposit to cancelled market
        let deposit_amount = 2_000_000_000;
        let deposit_ix = deposit_to_market_instruction(&program_id, &user.pubkey(), market_id, deposit_amount);

        let tx = Transaction::new_signed_with_payer(
            &[deposit_ix],
            Some(&user.pubkey()),
            &[&user],
            context.last_blockhash,
        );

        // Should fail
        let result = context.banks_client.process_transaction(tx).await;
        assert!(result.is_err());
    }
}

/// Tests for PlaceBet instruction with user escrow
#[cfg(test)]
mod place_bet_tests {
    use super::*;
    use helpers::*;

    /// Helper to build PlaceBet instruction
    fn place_bet_instruction(
        program_id: &Pubkey,
        user: &Pubkey,
        market_id: u64,
        bet_commitment: [u8; 32],
        amount: u64,
    ) -> Instruction {
        let (market_pda, _) = derive_market_pda(program_id, market_id);
        let (position_pda, _) = Pubkey::find_program_address(
            &[b"position", user.as_ref(), &market_id.to_le_bytes(), &bet_commitment],
            program_id,
        );
        let (user_escrow_pda, _) = derive_user_escrow_pda(program_id, user, &market_pda);
        let (market_escrow_pda, _) = derive_escrow_pda(program_id, market_id);

        // Mock ZK proof (256 bytes for Groth16)
        let proof = vec![0u8; 256];
        // Mock public inputs for Circuit 30 (MarketBet): 80 bytes
        let public_inputs = vec![0u8; 80];

        let instruction = FutarchyInstruction::PlaceBet {
            market_id,
            bet_commitment,
            proof,
            public_inputs,
            amount,
            circuit_type: 30, // MarketBet circuit
            encrypted_bet_amount: None,
            side: None,
        };

        let data = instruction.pack().unwrap();

        // Mock ZK program
        let zk_program = Pubkey::new_unique();

        Instruction {
            program_id: *program_id,
            accounts: vec![
                AccountMeta::new(*user, true),
                AccountMeta::new(market_pda, false),
                AccountMeta::new(position_pda, false),
                AccountMeta::new(user_escrow_pda, false),
                AccountMeta::new(market_escrow_pda, false),
                AccountMeta::new_readonly(zk_program, false),
                AccountMeta::new_readonly(system_program::id(), false),
                AccountMeta::new_readonly(sysvar::clock::id(), false),
            ],
            data,
        }
    }

    #[tokio::test]
    async fn test_place_bet_with_escrow() {
        let program_id = futarchy_markets::id();
        let mut program_test = setup_program_test();

        let authority = Keypair::new();
        let oracle = Keypair::new();
        let user = Keypair::new();

        program_test.add_account(
            authority.pubkey(),
            Account {
                lamports: 10_000_000_000,
                ..Default::default()
            },
        );

        program_test.add_account(
            user.pubkey(),
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
        let max_bet = 2_000_000_000;

        let (market_pda, _) = derive_market_pda(&program_id, market_id);
        let (user_escrow_pda, _) = derive_user_escrow_pda(&program_id, &user.pubkey(), &market_pda);
        let (market_escrow_pda, _) = derive_escrow_pda(&program_id, market_id);

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

        // Deposit 5 SOL to escrow
        let deposit_amount = 5_000_000_000;
        let deposit_ix = deposit_to_market_instruction(&program_id, &user.pubkey(), market_id, deposit_amount);

        let tx = Transaction::new_signed_with_payer(
            &[deposit_ix],
            Some(&user.pubkey()),
            &[&user],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        // Verify escrow has 5 SOL
        let escrow_before = get_user_escrow_account(&mut context, &user_escrow_pda).await;
        assert_eq!(escrow_before.deposited, 5_000_000_000);
        assert_eq!(escrow_before.available, 5_000_000_000);
        assert_eq!(escrow_before.reserved, 0);

        // Place bet for 2 SOL
        let bet_amount = 2_000_000_000;
        let bet_commitment = [7u8; 32];
        let bet_ix = place_bet_instruction(&program_id, &user.pubkey(), market_id, bet_commitment, bet_amount);

        let tx = Transaction::new_signed_with_payer(
            &[bet_ix],
            Some(&user.pubkey()),
            &[&user],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        // Verify escrow state after bet
        let escrow_after = get_user_escrow_account(&mut context, &user_escrow_pda).await;
        assert_eq!(escrow_after.deposited, 3_000_000_000); // 5 - 2 = 3 SOL (funds left escrow)
        assert_eq!(escrow_after.available, 3_000_000_000); // Remaining available
        assert_eq!(escrow_after.reserved, 0); // No longer reserved (transferred)
        assert!(escrow_after.verify_invariant());

        // Verify market escrow received the funds
        let market_escrow_balance = get_balance(&mut context, &market_escrow_pda).await;
        assert!(market_escrow_balance >= bet_amount); // Should have at least the bet amount

        // Verify position was created
        let (position_pda, _) = Pubkey::find_program_address(
            &[b"position", user.pubkey().as_ref(), &market_id.to_le_bytes(), &bet_commitment],
            &program_id,
        );
        assert!(account_exists(&mut context, &position_pda).await);
    }

    #[tokio::test]
    async fn test_place_bet_insufficient_escrow() {
        let program_id = futarchy_markets::id();
        let mut program_test = setup_program_test();

        let authority = Keypair::new();
        let oracle = Keypair::new();
        let user = Keypair::new();

        program_test.add_account(
            authority.pubkey(),
            Account {
                lamports: 10_000_000_000,
                ..Default::default()
            },
        );

        program_test.add_account(
            user.pubkey(),
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
        let max_bet = 5_000_000_000;

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

        // Deposit only 1 SOL to escrow
        let deposit_amount = 1_000_000_000;
        let deposit_ix = deposit_to_market_instruction(&program_id, &user.pubkey(), market_id, deposit_amount);

        let tx = Transaction::new_signed_with_payer(
            &[deposit_ix],
            Some(&user.pubkey()),
            &[&user],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        // Try to place bet for 2 SOL (more than available)
        let bet_amount = 2_000_000_000;
        let bet_commitment = [7u8; 32];
        let bet_ix = place_bet_instruction(&program_id, &user.pubkey(), market_id, bet_commitment, bet_amount);

        let tx = Transaction::new_signed_with_payer(
            &[bet_ix],
            Some(&user.pubkey()),
            &[&user],
            context.last_blockhash,
        );

        // Should fail with insufficient funds
        let result = context.banks_client.process_transaction(tx).await;
        assert!(result.is_err());
    }
}

/// Tests for complete bet lifecycle: Deposit → PlaceBet → Settle → ClaimPayout
#[cfg(test)]
mod full_lifecycle_tests {
    use super::*;
    use helpers::*;

    /// Helper to build PlaceBet instruction
    fn place_bet_instruction(
        program_id: &Pubkey,
        user: &Pubkey,
        market_id: u64,
        bet_commitment: [u8; 32],
        amount: u64,
    ) -> Instruction {
        let (market_pda, _) = derive_market_pda(program_id, market_id);
        let (position_pda, _) = Pubkey::find_program_address(
            &[b"position", user.as_ref(), &market_id.to_le_bytes(), &bet_commitment],
            program_id,
        );
        let (user_escrow_pda, _) = derive_user_escrow_pda(program_id, user, &market_pda);
        let (market_escrow_pda, _) = derive_escrow_pda(program_id, market_id);

        let proof = vec![0u8; 256];
        let public_inputs = vec![0u8; 80];

        let instruction = FutarchyInstruction::PlaceBet {
            market_id,
            bet_commitment,
            proof,
            public_inputs,
            amount,
            circuit_type: 30,
            encrypted_bet_amount: None,
            side: None,
        };

        let data = instruction.pack().unwrap();
        let zk_program = Pubkey::new_unique();

        Instruction {
            program_id: *program_id,
            accounts: vec![
                AccountMeta::new(*user, true),
                AccountMeta::new(market_pda, false),
                AccountMeta::new(position_pda, false),
                AccountMeta::new(user_escrow_pda, false),
                AccountMeta::new(market_escrow_pda, false),
                AccountMeta::new_readonly(zk_program, false),
                AccountMeta::new_readonly(system_program::id(), false),
                AccountMeta::new_readonly(sysvar::clock::id(), false),
            ],
            data,
        }
    }

    /// Helper to build ClaimPayout instruction
    fn claim_payout_instruction(
        program_id: &Pubkey,
        user: &Pubkey,
        market_id: u64,
        claim_nullifier: [u8; 32],
        payout_amount: u64,
    ) -> Instruction {
        let (market_pda, _) = derive_market_pda(program_id, market_id);
        let (escrow_pda, _) = derive_escrow_pda(program_id, market_id);

        let proof = vec![0u8; 256];
        let public_inputs = vec![0u8; 105]; // Circuit 32 needs 105 bytes

        let instruction = FutarchyInstruction::ClaimPayout {
            market_id,
            claim_nullifier,
            proof,
            public_inputs,
            payout_amount,
        };

        let data = instruction.pack().unwrap();
        let zk_program = Pubkey::new_unique();

        // Position is optional but still needs to be passed (can use a dummy)
        let dummy_position = Pubkey::new_unique();

        Instruction {
            program_id: *program_id,
            accounts: vec![
                AccountMeta::new(*user, true),
                AccountMeta::new(market_pda, false),
                AccountMeta::new(dummy_position, false), // Position (optional)
                AccountMeta::new(escrow_pda, false),
                AccountMeta::new_readonly(zk_program, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data,
        }
    }

    #[tokio::test]
    async fn test_full_bet_lifecycle() {
        let program_id = futarchy_markets::id();
        let mut program_test = setup_program_test();

        let authority = Keypair::new();
        let oracle = Keypair::new();
        let user = Keypair::new();

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

        program_test.add_account(
            user.pubkey(),
            Account {
                lamports: 10_000_000_000,
                ..Default::default()
            },
        );

        let mut context = program_test.start_with_context().await;

        // Step 1: Create market
        let market_id = 1u64;
        let question_hash = [1u8; 32];
        let current_time = context
            .banks_client
            .get_sysvar::<solana_program::clock::Clock>()
            .await
            .unwrap()
            .unix_timestamp;
        let end_time = current_time + 86400;
        let max_bet = 5_000_000_000;

        let (market_pda, _) = derive_market_pda(&program_id, market_id);
        let (user_escrow_pda, _) = derive_user_escrow_pda(&program_id, &user.pubkey(), &market_pda);
        let (market_escrow_pda, _) = derive_escrow_pda(&program_id, market_id);

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

        // Step 2: Deposit 5 SOL to escrow
        let deposit_amount = 5_000_000_000;
        let deposit_ix = deposit_to_market_instruction(&program_id, &user.pubkey(), market_id, deposit_amount);

        let tx = Transaction::new_signed_with_payer(
            &[deposit_ix],
            Some(&user.pubkey()),
            &[&user],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        let user_balance_before_bet = get_balance(&mut context, &user.pubkey()).await;

        // Step 3: Place bet for 2 SOL
        let bet_amount = 2_000_000_000;
        let bet_commitment = [7u8; 32];
        let bet_ix = place_bet_instruction(&program_id, &user.pubkey(), market_id, bet_commitment, bet_amount);

        let tx = Transaction::new_signed_with_payer(
            &[bet_ix],
            Some(&user.pubkey()),
            &[&user],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        // Verify escrow state after bet
        let escrow_after_bet = get_user_escrow_account(&mut context, &user_escrow_pda).await;
        assert_eq!(escrow_after_bet.deposited, 3_000_000_000); // 5 - 2 = 3 SOL
        assert_eq!(escrow_after_bet.available, 3_000_000_000);
        assert_eq!(escrow_after_bet.reserved, 0);
        assert!(escrow_after_bet.verify_invariant());

        // Step 4: Settle market (YES wins)
        force_market_ended(&mut context, &program_id, &market_pda).await;

        let settle_ix = settle_market_instruction(&program_id, &oracle.pubkey(), market_id, true);

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

        // Step 5: Claim payout (payout <= escrow balance)
        let payout_amount = 2_000_000_000; // Same as bet amount
        let claim_nullifier = [9u8; 32];
        let claim_ix = claim_payout_instruction(
            &program_id,
            &user.pubkey(),
            market_id,
            claim_nullifier,
            payout_amount,
        );

        let tx = Transaction::new_signed_with_payer(
            &[claim_ix],
            Some(&user.pubkey()),
            &[&user],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        // Verify user received payout
        let user_balance_after_claim = get_balance(&mut context, &user.pubkey()).await;
        assert!(user_balance_after_claim > user_balance_before_bet);

        // Verify market escrow decreased
        let market_escrow_balance = get_balance(&mut context, &market_escrow_pda).await;
        assert!(market_escrow_balance < bet_amount); // Should have paid out

        // Verify claim nullifier was recorded
        let market_after_claim = get_market_account(&mut context, &market_pda).await;
        assert!(market_after_claim.claim_nullifiers.contains(&claim_nullifier));
    }

    #[tokio::test]
    async fn test_cannot_claim_twice() {
        let program_id = futarchy_markets::id();
        let mut program_test = setup_program_test();

        let authority = Keypair::new();
        let oracle = Keypair::new();
        let user = Keypair::new();

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

        program_test.add_account(
            user.pubkey(),
            Account {
                lamports: 10_000_000_000,
                ..Default::default()
            },
        );

        let mut context = program_test.start_with_context().await;

        // Create and settle market
        let market_id = 1u64;
        let question_hash = [1u8; 32];
        let current_time = context
            .banks_client
            .get_sysvar::<solana_program::clock::Clock>()
            .await
            .unwrap()
            .unix_timestamp;
        let end_time = current_time + 86400;
        let max_bet = 5_000_000_000;

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

        // Deposit and place bet so escrow has funds
        let deposit_amount = 2_000_000_000;
        let deposit_ix = deposit_to_market_instruction(&program_id, &user.pubkey(), market_id, deposit_amount);

        let tx = Transaction::new_signed_with_payer(
            &[deposit_ix],
            Some(&user.pubkey()),
            &[&user],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        // Place bet
        let bet_commitment = [7u8; 32];
        let bet_ix = place_bet_instruction(&program_id, &user.pubkey(), market_id, bet_commitment, deposit_amount);

        let tx = Transaction::new_signed_with_payer(
            &[bet_ix],
            Some(&user.pubkey()),
            &[&user],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        // Force settle
        force_market_ended(&mut context, &program_id, &market_pda).await;

        let settle_ix = settle_market_instruction(&program_id, &oracle.pubkey(), market_id, true);

        let tx = Transaction::new_signed_with_payer(
            &[settle_ix],
            Some(&oracle.pubkey()),
            &[&oracle],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        // Claim once (payout <= escrow balance)
        let payout_amount = 1_000_000_000;
        let claim_nullifier = [9u8; 32];
        let claim_ix = claim_payout_instruction(
            &program_id,
            &user.pubkey(),
            market_id,
            claim_nullifier,
            payout_amount,
        );

        let tx = Transaction::new_signed_with_payer(
            &[claim_ix],
            Some(&user.pubkey()),
            &[&user],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        // Try to claim again with same nullifier
        let claim_ix2 = claim_payout_instruction(
            &program_id,
            &user.pubkey(),
            market_id,
            claim_nullifier,
            payout_amount,
        );

        let tx2 = Transaction::new_signed_with_payer(
            &[claim_ix2],
            Some(&user.pubkey()),
            &[&user],
            context.last_blockhash,
        );

        // Should fail with double claim
        let result = context.banks_client.process_transaction(tx2).await;
        assert!(result.is_err());
    }
}
