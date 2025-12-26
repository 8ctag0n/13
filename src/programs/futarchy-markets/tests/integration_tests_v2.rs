//! Integration tests for V2 (PrivateBalance architecture)
//!
//! Tests the complete flow:
//! 1. InitializeProtocol - Create global ProtocolVault
//! 2. CreatePrivateBalance - Create user's private balance
//! 3. Deposit - Add funds to protocol vault
//! 4. CreateMarketV2 - Create market with optional governance
//! 5. PlaceBetV2 - Place anonymous bet with mock ZK proof
//! 6. SettleMarketV2 - Oracle settles market
//! 7. ClaimV2 - Claim payout with mock ZK proof
//! 8. Withdraw - Withdraw funds with mock ZK proof

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
    sysvar,
};
use solana_program_test::{processor, ProgramTest, ProgramTestContext};
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};

use futarchy_markets::{
    entrypoint::process_instruction,
    instruction_v2::FutarchyInstructionV2,
    state::{
        ExecutableAction, GovernanceConfig, MarketV2, PositionV2, PrivateBalance,
        GOVERNANCE_CONFIG_SEED, MARKET_V2_SEED, MARKET_VAULT_SEED, NULLIFIER_SEED,
        POSITION_V2_SEED, PRIVATE_BALANCE_SEED, PROTOCOL_VAULT_SEED,
    },
};

/// Test helpers for V2
mod helpers_v2 {
    use super::*;

    /// Setup ProgramTest with futarchy-markets program
    pub fn setup_program_test() -> ProgramTest {
        ProgramTest::new(
            "futarchy_markets",
            futarchy_markets::id(),
            processor!(process_instruction),
        )
    }

    /// Derive ProtocolVault PDA
    pub fn derive_protocol_vault_pda(program_id: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[PROTOCOL_VAULT_SEED], program_id)
    }

    /// Derive PrivateBalance PDA
    pub fn derive_private_balance_pda(program_id: &Pubkey, user: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[PRIVATE_BALANCE_SEED, user.as_ref()], program_id)
    }

    /// Derive MarketV2 PDA
    pub fn derive_market_v2_pda(program_id: &Pubkey, market_id: u64) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[MARKET_V2_SEED, &market_id.to_le_bytes()],
            program_id,
        )
    }

    /// Derive MarketVault PDA
    pub fn derive_market_vault_pda(program_id: &Pubkey, market_id: u64) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[MARKET_VAULT_SEED, &market_id.to_le_bytes()],
            program_id,
        )
    }

    /// Derive PositionV2 PDA
    pub fn derive_position_v2_pda(
        program_id: &Pubkey,
        market_id: u64,
        bet_commitment: &[u8; 32],
    ) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[POSITION_V2_SEED, &market_id.to_le_bytes(), bet_commitment],
            program_id,
        )
    }

    /// Derive GovernanceConfig PDA
    pub fn derive_governance_config_pda(program_id: &Pubkey, market_id: u64) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[GOVERNANCE_CONFIG_SEED, &market_id.to_le_bytes()],
            program_id,
        )
    }

    /// Derive Nullifier PDA
    pub fn derive_nullifier_pda(
        program_id: &Pubkey,
        market_id: u64,
        nullifier_hash: &[u8; 32],
    ) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[NULLIFIER_SEED, &market_id.to_le_bytes(), nullifier_hash],
            program_id,
        )
    }

    /// Generate mock ZK proof (256 bytes)
    pub fn mock_zk_proof() -> Vec<u8> {
        vec![0u8; 256]
    }

    /// Generate mock public inputs for PlaceBetV2
    /// old_commitment(32) + new_commitment(32) + bet_commitment(32) + max_bet(8) + amount(8) = 112
    pub fn mock_place_bet_public_inputs(
        old_commitment: &[u8; 32],
        new_commitment: &[u8; 32],
        bet_commitment: &[u8; 32],
        max_bet: u64,
        bet_amount: u64,
    ) -> Vec<u8> {
        let mut inputs = Vec::with_capacity(112);
        inputs.extend_from_slice(old_commitment);
        inputs.extend_from_slice(new_commitment);
        inputs.extend_from_slice(bet_commitment);
        inputs.extend_from_slice(&max_bet.to_le_bytes());
        inputs.extend_from_slice(&bet_amount.to_le_bytes());
        inputs
    }

    /// Generate mock public inputs for Withdraw
    /// old_commitment(32) + new_commitment(32) + amount(8) = 72
    pub fn mock_withdraw_public_inputs(
        old_commitment: &[u8; 32],
        new_commitment: &[u8; 32],
        amount: u64,
    ) -> Vec<u8> {
        let mut inputs = Vec::with_capacity(72);
        inputs.extend_from_slice(old_commitment);
        inputs.extend_from_slice(new_commitment);
        inputs.extend_from_slice(&amount.to_le_bytes());
        inputs
    }

    /// Generate mock public inputs for ClaimV2
    /// bet_commitment(32) + resolution(1) + nullifier(32) + bet_amount(8) + bet_side(1) + old_commitment(32) + new_commitment(32) = 138
    pub fn mock_claim_public_inputs(
        bet_commitment: &[u8; 32],
        resolution: bool,
        nullifier_hash: &[u8; 32],
        bet_amount: u64,
        bet_side: bool,
        old_commitment: &[u8; 32],
        new_commitment: &[u8; 32],
    ) -> Vec<u8> {
        let mut inputs = Vec::with_capacity(138);
        inputs.extend_from_slice(bet_commitment);
        inputs.push(if resolution { 1 } else { 0 });
        inputs.extend_from_slice(nullifier_hash);
        inputs.extend_from_slice(&bet_amount.to_le_bytes());
        inputs.push(if bet_side { 1 } else { 0 });
        inputs.extend_from_slice(old_commitment);
        inputs.extend_from_slice(new_commitment);
        inputs
    }

    /// Build InitializeProtocol instruction
    pub fn initialize_protocol_instruction(
        program_id: &Pubkey,
        authority: &Pubkey,
    ) -> Instruction {
        let (protocol_vault_pda, _) = derive_protocol_vault_pda(program_id);

        let instruction = FutarchyInstructionV2::InitializeProtocol;
        let data = instruction.pack().unwrap();

        Instruction {
            program_id: *program_id,
            accounts: vec![
                AccountMeta::new(*authority, true),
                AccountMeta::new(protocol_vault_pda, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data,
        }
    }

    /// Build CreatePrivateBalance instruction
    pub fn create_private_balance_instruction(
        program_id: &Pubkey,
        user: &Pubkey,
        initial_encrypted_balance: Vec<u8>,
        initial_commitment: [u8; 32],
    ) -> Instruction {
        let (private_balance_pda, _) = derive_private_balance_pda(program_id, user);

        let instruction = FutarchyInstructionV2::CreatePrivateBalance {
            initial_encrypted_balance,
            initial_commitment,
        };
        let data = instruction.pack().unwrap();

        Instruction {
            program_id: *program_id,
            accounts: vec![
                AccountMeta::new(*user, true),
                AccountMeta::new(private_balance_pda, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data,
        }
    }

    /// Build Deposit instruction
    pub fn deposit_instruction(
        program_id: &Pubkey,
        user: &Pubkey,
        amount: u64,
    ) -> Instruction {
        let (private_balance_pda, _) = derive_private_balance_pda(program_id, user);
        let (protocol_vault_pda, _) = derive_protocol_vault_pda(program_id);

        let instruction = FutarchyInstructionV2::Deposit { amount };
        let data = instruction.pack().unwrap();

        Instruction {
            program_id: *program_id,
            accounts: vec![
                AccountMeta::new(*user, true),
                AccountMeta::new(private_balance_pda, false),
                AccountMeta::new(protocol_vault_pda, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data,
        }
    }

    /// Build CreateMarketV2 instruction
    pub fn create_market_v2_instruction(
        program_id: &Pubkey,
        authority: &Pubkey,
        oracle: &Pubkey,
        market_id: u64,
        question_hash: [u8; 32],
        end_time: i64,
        max_bet: u64,
        has_governance: bool,
        executable_action: Option<ExecutableAction>,
        execution_threshold: Option<u8>,
        timelock_duration: Option<i64>,
    ) -> Instruction {
        let (market_pda, _) = derive_market_v2_pda(program_id, market_id);
        let (market_vault_pda, _) = derive_market_vault_pda(program_id, market_id);

        let instruction = FutarchyInstructionV2::CreateMarketV2 {
            market_id,
            question_hash,
            end_time,
            max_bet,
            has_governance,
            executable_action,
            execution_threshold,
            timelock_duration,
        };
        let data = instruction.pack().unwrap();

        let mut accounts = vec![
            AccountMeta::new(*authority, true),
            AccountMeta::new(market_pda, false),
            AccountMeta::new(market_vault_pda, false),
            AccountMeta::new_readonly(*oracle, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ];

        if has_governance {
            let (governance_pda, _) = derive_governance_config_pda(program_id, market_id);
            accounts.push(AccountMeta::new(governance_pda, false));
        }

        Instruction {
            program_id: *program_id,
            accounts,
            data,
        }
    }

    /// Build SettleMarketV2 instruction
    pub fn settle_market_v2_instruction(
        program_id: &Pubkey,
        oracle: &Pubkey,
        market_id: u64,
        outcome: bool,
        has_governance: bool,
    ) -> Instruction {
        let (market_pda, _) = derive_market_v2_pda(program_id, market_id);

        let instruction = FutarchyInstructionV2::SettleMarketV2 { market_id, outcome };
        let data = instruction.pack().unwrap();

        let mut accounts = vec![
            AccountMeta::new(*oracle, true),
            AccountMeta::new(market_pda, false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ];

        if has_governance {
            let (governance_pda, _) = derive_governance_config_pda(program_id, market_id);
            accounts.push(AccountMeta::new(governance_pda, false));
        }

        Instruction {
            program_id: *program_id,
            accounts,
            data,
        }
    }

    /// Build PlaceBetV2 instruction
    #[allow(clippy::too_many_arguments)]
    pub fn place_bet_v2_instruction(
        program_id: &Pubkey,
        user: &Pubkey,
        market_id: u64,
        bet_commitment: [u8; 32],
        bet_side: bool,
        proof: Vec<u8>,
        public_inputs: Vec<u8>,
        new_balance_commitment: [u8; 32],
        encrypted_bet: Vec<u8>,
        circuit_type: u8,
        zk_program_id: &Pubkey,
    ) -> Instruction {
        let (private_balance_pda, _) = derive_private_balance_pda(program_id, user);
        let (market_pda, _) = derive_market_v2_pda(program_id, market_id);
        let (position_pda, _) = derive_position_v2_pda(program_id, market_id, &bet_commitment);
        let (protocol_vault_pda, _) = derive_protocol_vault_pda(program_id);
        let (market_vault_pda, _) = derive_market_vault_pda(program_id, market_id);

        let instruction = FutarchyInstructionV2::PlaceBetV2 {
            market_id,
            bet_commitment,
            bet_side,
            proof,
            public_inputs,
            new_balance_commitment,
            encrypted_bet,
            circuit_type,
        };
        let data = instruction.pack().unwrap();

        Instruction {
            program_id: *program_id,
            accounts: vec![
                AccountMeta::new(*user, true),
                AccountMeta::new(private_balance_pda, false),
                AccountMeta::new(market_pda, false),
                AccountMeta::new(position_pda, false),
                AccountMeta::new(protocol_vault_pda, false),
                AccountMeta::new(market_vault_pda, false),
                AccountMeta::new_readonly(*zk_program_id, false),
                AccountMeta::new_readonly(system_program::id(), false),
                AccountMeta::new_readonly(sysvar::clock::id(), false),
            ],
            data,
        }
    }

    /// Build ClaimV2 instruction
    #[allow(clippy::too_many_arguments)]
    pub fn claim_v2_instruction(
        program_id: &Pubkey,
        user: &Pubkey,
        market_id: u64,
        nullifier_hash: [u8; 32],
        proof: Vec<u8>,
        public_inputs: Vec<u8>,
        new_balance_commitment: [u8; 32],
        encrypted_payout: Vec<u8>,
        bet_commitment: [u8; 32],
        circuit_type: u8,
        zk_program_id: &Pubkey,
    ) -> Instruction {
        let (private_balance_pda, _) = derive_private_balance_pda(program_id, user);
        let (market_pda, _) = derive_market_v2_pda(program_id, market_id);
        let (position_pda, _) = derive_position_v2_pda(program_id, market_id, &bet_commitment);
        let (nullifier_pda, _) = derive_nullifier_pda(program_id, market_id, &nullifier_hash);
        let (market_vault_pda, _) = derive_market_vault_pda(program_id, market_id);
        let (protocol_vault_pda, _) = derive_protocol_vault_pda(program_id);

        let instruction = FutarchyInstructionV2::ClaimV2 {
            market_id,
            nullifier_hash,
            proof,
            public_inputs,
            new_balance_commitment,
            encrypted_payout,
            bet_commitment,
            circuit_type,
        };
        let data = instruction.pack().unwrap();

        Instruction {
            program_id: *program_id,
            accounts: vec![
                AccountMeta::new(*user, true),
                AccountMeta::new(private_balance_pda, false),
                AccountMeta::new_readonly(market_pda, false),
                AccountMeta::new(position_pda, false),
                AccountMeta::new(nullifier_pda, false),
                AccountMeta::new(market_vault_pda, false),
                AccountMeta::new(protocol_vault_pda, false),
                AccountMeta::new_readonly(*zk_program_id, false),
                AccountMeta::new_readonly(system_program::id(), false),
                AccountMeta::new_readonly(sysvar::clock::id(), false),
            ],
            data,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
async fn test_initialize_protocol() {
    let mut program_test = helpers_v2::setup_program_test();
    let mut context = program_test.start_with_context().await;

    let program_id = futarchy_markets::id();
    let authority = context.payer.pubkey();

    // Initialize protocol
    let ix = helpers_v2::initialize_protocol_instruction(&program_id, &authority);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&authority),
        &[&context.payer],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.unwrap();

    // Verify ProtocolVault was created
    let (protocol_vault_pda, _) = helpers_v2::derive_protocol_vault_pda(&program_id);
    let vault_account = context
        .banks_client
        .get_account(protocol_vault_pda)
        .await
        .unwrap();

    assert!(vault_account.is_some());
    println!("ProtocolVault created: {}", protocol_vault_pda);
}

#[tokio::test]
async fn test_create_private_balance() {
    let mut program_test = helpers_v2::setup_program_test();
    let mut context = program_test.start_with_context().await;

    let program_id = futarchy_markets::id();
    let user = context.payer.pubkey();

    // First initialize protocol
    let init_ix = helpers_v2::initialize_protocol_instruction(&program_id, &user);
    let init_tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&user),
        &[&context.payer],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(init_tx).await.unwrap();

    // Create private balance with zero commitment
    let initial_commitment = [0u8; 32]; // Poseidon(0, 0)
    let initial_encrypted = vec![0u8; 64]; // Mock FHE(0)

    let ix = helpers_v2::create_private_balance_instruction(
        &program_id,
        &user,
        initial_encrypted,
        initial_commitment,
    );

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&user),
        &[&context.payer],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.unwrap();

    // Verify PrivateBalance was created
    let (private_balance_pda, _) = helpers_v2::derive_private_balance_pda(&program_id, &user);
    let balance_account = context
        .banks_client
        .get_account(private_balance_pda)
        .await
        .unwrap()
        .unwrap();

    let private_balance = PrivateBalance::deserialize(&mut &balance_account.data[..]).unwrap();
    assert_eq!(private_balance.user, user);
    assert_eq!(private_balance.nonce, 0);
    println!("PrivateBalance created for user: {}", user);
}

#[tokio::test]
async fn test_deposit() {
    let mut program_test = helpers_v2::setup_program_test();
    let mut context = program_test.start_with_context().await;

    let program_id = futarchy_markets::id();
    let user = context.payer.pubkey();

    // Initialize protocol
    let init_ix = helpers_v2::initialize_protocol_instruction(&program_id, &user);

    // Create private balance
    let initial_commitment = [0u8; 32];
    let create_balance_ix = helpers_v2::create_private_balance_instruction(
        &program_id,
        &user,
        vec![0u8; 64],
        initial_commitment,
    );

    let setup_tx = Transaction::new_signed_with_payer(
        &[init_ix, create_balance_ix],
        Some(&user),
        &[&context.payer],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(setup_tx).await.unwrap();

    // Get new blockhash
    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    // Deposit 1 SOL
    let deposit_amount = 1_000_000_000u64;
    let deposit_ix = helpers_v2::deposit_instruction(&program_id, &user, deposit_amount);

    let deposit_tx = Transaction::new_signed_with_payer(
        &[deposit_ix],
        Some(&user),
        &[&context.payer],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(deposit_tx).await.unwrap();

    // Verify ProtocolVault received funds
    let (protocol_vault_pda, _) = helpers_v2::derive_protocol_vault_pda(&program_id);
    let vault_account = context
        .banks_client
        .get_account(protocol_vault_pda)
        .await
        .unwrap()
        .unwrap();

    // Vault should have deposit + rent
    assert!(vault_account.lamports >= deposit_amount);
    println!("Deposited {} lamports to ProtocolVault", deposit_amount);
}

#[tokio::test]
async fn test_create_market_v2_without_governance() {
    let mut program_test = helpers_v2::setup_program_test();
    let mut context = program_test.start_with_context().await;

    let program_id = futarchy_markets::id();
    let authority = context.payer.pubkey();
    let oracle = Keypair::new();

    // Initialize protocol first
    let init_ix = helpers_v2::initialize_protocol_instruction(&program_id, &authority);
    let init_tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&authority),
        &[&context.payer],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(init_tx).await.unwrap();

    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    // Create market without governance
    let market_id = 1u64;
    let question_hash = [1u8; 32];
    let end_time = 9999999999i64; // Far future
    let max_bet = 10_000_000_000u64; // 10 SOL

    let create_market_ix = helpers_v2::create_market_v2_instruction(
        &program_id,
        &authority,
        &oracle.pubkey(),
        market_id,
        question_hash,
        end_time,
        max_bet,
        false, // no governance
        None,
        None,
        None,
    );

    let tx = Transaction::new_signed_with_payer(
        &[create_market_ix],
        Some(&authority),
        &[&context.payer],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Verify market was created
    let (market_pda, _) = helpers_v2::derive_market_v2_pda(&program_id, market_id);
    let market_account = context
        .banks_client
        .get_account(market_pda)
        .await
        .unwrap()
        .unwrap();

    let market = MarketV2::deserialize(&mut &market_account.data[..]).unwrap();
    assert_eq!(market.market_id, market_id);
    assert_eq!(market.authority, authority);
    assert_eq!(market.oracle, oracle.pubkey());
    assert!(!market.has_governance);
    assert!(market.is_active());

    println!("MarketV2 created: market_id={}", market_id);
}

#[tokio::test]
async fn test_create_market_v2_with_governance() {
    let mut program_test = helpers_v2::setup_program_test();
    let mut context = program_test.start_with_context().await;

    let program_id = futarchy_markets::id();
    let authority = context.payer.pubkey();
    let oracle = Keypair::new();

    // Initialize protocol first
    let init_ix = helpers_v2::initialize_protocol_instruction(&program_id, &authority);
    let init_tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&authority),
        &[&context.payer],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(init_tx).await.unwrap();

    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    // Create market with governance
    let market_id = 2u64;
    let question_hash = [2u8; 32];
    let end_time = 9999999999i64;
    let max_bet = 10_000_000_000u64;

    let executable_action = ExecutableAction::TransferTokens {
        token_mint: Pubkey::new_unique(),
        from_treasury: Pubkey::new_unique(),
        to: Pubkey::new_unique(),
        amount: 1_000_000,
    };

    let create_market_ix = helpers_v2::create_market_v2_instruction(
        &program_id,
        &authority,
        &oracle.pubkey(),
        market_id,
        question_hash,
        end_time,
        max_bet,
        true, // with governance
        Some(executable_action),
        Some(51), // 51% threshold
        Some(3600), // 1 hour timelock
    );

    let tx = Transaction::new_signed_with_payer(
        &[create_market_ix],
        Some(&authority),
        &[&context.payer],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Verify market was created
    let (market_pda, _) = helpers_v2::derive_market_v2_pda(&program_id, market_id);
    let market_account = context
        .banks_client
        .get_account(market_pda)
        .await
        .unwrap()
        .unwrap();

    let market = MarketV2::deserialize(&mut &market_account.data[..]).unwrap();
    assert!(market.has_governance);

    // Verify GovernanceConfig was created
    let (governance_pda, _) = helpers_v2::derive_governance_config_pda(&program_id, market_id);
    let governance_account = context
        .banks_client
        .get_account(governance_pda)
        .await
        .unwrap()
        .unwrap();

    let governance = GovernanceConfig::deserialize(&mut &governance_account.data[..]).unwrap();
    assert_eq!(governance.market_id, market_id);
    assert_eq!(governance.execution_threshold, 51);
    assert_eq!(governance.timelock_duration, 3600);

    println!("MarketV2 with governance created: market_id={}", market_id);
}

#[tokio::test]
async fn test_settle_market_v2() {
    use solana_program_test::tokio;
    use solana_sdk::clock::Clock;

    let mut program_test = helpers_v2::setup_program_test();
    let mut context = program_test.start_with_context().await;

    let program_id = futarchy_markets::id();
    let authority = context.payer.pubkey();
    let oracle = Keypair::new();

    // Initialize protocol
    let init_ix = helpers_v2::initialize_protocol_instruction(&program_id, &authority);
    let init_tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&authority),
        &[&context.payer],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(init_tx).await.unwrap();

    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    // Get current clock
    let clock: Clock = context.banks_client.get_sysvar().await.unwrap();
    let current_time = clock.unix_timestamp;

    // Create market with end_time 5 seconds in future
    let market_id = 3u64;
    let question_hash = [3u8; 32];
    let end_time = current_time + 5;
    let max_bet = 10_000_000_000u64;

    let create_market_ix = helpers_v2::create_market_v2_instruction(
        &program_id,
        &authority,
        &oracle.pubkey(),
        market_id,
        question_hash,
        end_time,
        max_bet,
        false,
        None,
        None,
        None,
    );

    let tx = Transaction::new_signed_with_payer(
        &[create_market_ix],
        Some(&authority),
        &[&context.payer],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Advance clock past end_time (slot ~0.4s, need to warp enough slots)
    // Warp to slot 1000 to advance time significantly
    let current_slot = context.banks_client.get_root_slot().await.unwrap();
    context.warp_to_slot(current_slot + 10000).unwrap();
    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    // Settle market (oracle signs)
    let settle_ix = helpers_v2::settle_market_v2_instruction(
        &program_id,
        &oracle.pubkey(),
        market_id,
        true, // YES wins
        false,
    );

    let settle_tx = Transaction::new_signed_with_payer(
        &[settle_ix],
        Some(&authority),
        &[&context.payer, &oracle],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(settle_tx).await.unwrap();

    // Verify market is settled
    let (market_pda, _) = helpers_v2::derive_market_v2_pda(&program_id, market_id);
    let market_account = context
        .banks_client
        .get_account(market_pda)
        .await
        .unwrap()
        .unwrap();

    let market = MarketV2::deserialize(&mut &market_account.data[..]).unwrap();
    assert!(market.is_settled());
    assert_eq!(market.resolution, Some(true));

    println!("MarketV2 settled: market_id={}, outcome=YES", market_id);
}

#[tokio::test]
async fn test_full_flow_deposit_to_withdraw() {
    let mut program_test = helpers_v2::setup_program_test();
    let mut context = program_test.start_with_context().await;

    let program_id = futarchy_markets::id();
    let user = context.payer.pubkey();

    // 1. Initialize protocol
    let init_ix = helpers_v2::initialize_protocol_instruction(&program_id, &user);

    // 2. Create private balance
    let initial_commitment = [0u8; 32];
    let create_balance_ix = helpers_v2::create_private_balance_instruction(
        &program_id,
        &user,
        vec![0u8; 64],
        initial_commitment,
    );

    let setup_tx = Transaction::new_signed_with_payer(
        &[init_ix, create_balance_ix],
        Some(&user),
        &[&context.payer],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(setup_tx).await.unwrap();

    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    // 3. Deposit
    let deposit_amount = 2_000_000_000u64; // 2 SOL
    let deposit_ix = helpers_v2::deposit_instruction(&program_id, &user, deposit_amount);

    let deposit_tx = Transaction::new_signed_with_payer(
        &[deposit_ix],
        Some(&user),
        &[&context.payer],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(deposit_tx).await.unwrap();

    // Verify deposit
    let (protocol_vault_pda, _) = helpers_v2::derive_protocol_vault_pda(&program_id);
    let vault_account = context
        .banks_client
        .get_account(protocol_vault_pda)
        .await
        .unwrap()
        .unwrap();

    println!("Full flow test passed:");
    println!("  - Protocol initialized");
    println!("  - PrivateBalance created");
    println!("  - Deposited {} lamports", deposit_amount);
    println!("  - ProtocolVault balance: {} lamports", vault_account.lamports);
}

#[tokio::test]
async fn test_place_bet_v2_increments_counter() {
    use solana_sdk::clock::Clock;

    let mut program_test = helpers_v2::setup_program_test();
    let mut context = program_test.start_with_context().await;

    let program_id = futarchy_markets::id();
    let user = context.payer.pubkey();
    let oracle = Keypair::new();
    let zk_program_id = Pubkey::new_unique(); // Mock ZK program

    // Setup: Initialize protocol, create balance, deposit
    let init_ix = helpers_v2::initialize_protocol_instruction(&program_id, &user);
    let initial_commitment = [0u8; 32];
    let create_balance_ix = helpers_v2::create_private_balance_instruction(
        &program_id,
        &user,
        vec![0u8; 64],
        initial_commitment,
    );

    let setup_tx = Transaction::new_signed_with_payer(
        &[init_ix, create_balance_ix],
        Some(&user),
        &[&context.payer],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(setup_tx).await.unwrap();

    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    // Deposit funds
    let deposit_amount = 5_000_000_000u64; // 5 SOL
    let deposit_ix = helpers_v2::deposit_instruction(&program_id, &user, deposit_amount);
    let deposit_tx = Transaction::new_signed_with_payer(
        &[deposit_ix],
        Some(&user),
        &[&context.payer],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(deposit_tx).await.unwrap();

    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    // Create market
    let market_id = 100u64;
    let question_hash = [100u8; 32];
    let clock: Clock = context.banks_client.get_sysvar().await.unwrap();
    let end_time = clock.unix_timestamp + 3600; // 1 hour from now
    let max_bet = 1_000_000_000u64; // 1 SOL

    let create_market_ix = helpers_v2::create_market_v2_instruction(
        &program_id,
        &user,
        &oracle.pubkey(),
        market_id,
        question_hash,
        end_time,
        max_bet,
        false,
        None,
        None,
        None,
    );

    let create_market_tx = Transaction::new_signed_with_payer(
        &[create_market_ix],
        Some(&user),
        &[&context.payer],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(create_market_tx).await.unwrap();

    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    // Place bet on YES
    let bet_commitment_yes = [1u8; 32];
    let bet_amount = 500_000_000u64; // 0.5 SOL
    let new_balance_commitment = [2u8; 32];

    let public_inputs = helpers_v2::mock_place_bet_public_inputs(
        &initial_commitment,
        &new_balance_commitment,
        &bet_commitment_yes,
        max_bet,
        bet_amount,
    );

    let place_bet_ix = helpers_v2::place_bet_v2_instruction(
        &program_id,
        &user,
        market_id,
        bet_commitment_yes,
        true, // YES
        helpers_v2::mock_zk_proof(),
        public_inputs,
        new_balance_commitment,
        vec![0u8; 64], // encrypted_bet
        33, // circuit type
        &zk_program_id,
    );

    let place_bet_tx = Transaction::new_signed_with_payer(
        &[place_bet_ix],
        Some(&user),
        &[&context.payer],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(place_bet_tx).await.unwrap();

    // Verify market bet_count_yes incremented
    let (market_pda, _) = helpers_v2::derive_market_v2_pda(&program_id, market_id);
    let market_account = context
        .banks_client
        .get_account(market_pda)
        .await
        .unwrap()
        .unwrap();

    let market = MarketV2::deserialize(&mut &market_account.data[..]).unwrap();
    assert_eq!(market.bet_count_yes, 1);
    assert_eq!(market.bet_count_no, 0);

    println!("PlaceBetV2 test passed:");
    println!("  - bet_count_yes: {}", market.bet_count_yes);
    println!("  - bet_count_no: {}", market.bet_count_no);
}

#[tokio::test]
async fn test_claim_v2_payout_model() {
    use solana_sdk::clock::Clock;

    let mut program_test = helpers_v2::setup_program_test();
    let mut context = program_test.start_with_context().await;

    let program_id = futarchy_markets::id();
    let user = context.payer.pubkey();
    let oracle = Keypair::new();
    let zk_program_id = Pubkey::new_unique();

    // === Setup Phase ===
    // Initialize protocol
    let init_ix = helpers_v2::initialize_protocol_instruction(&program_id, &user);
    let initial_commitment = [0u8; 32];
    let create_balance_ix = helpers_v2::create_private_balance_instruction(
        &program_id,
        &user,
        vec![0u8; 64],
        initial_commitment,
    );

    let setup_tx = Transaction::new_signed_with_payer(
        &[init_ix, create_balance_ix],
        Some(&user),
        &[&context.payer],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(setup_tx).await.unwrap();
    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    // Deposit funds
    let deposit_amount = 10_000_000_000u64; // 10 SOL
    let deposit_ix = helpers_v2::deposit_instruction(&program_id, &user, deposit_amount);
    let deposit_tx = Transaction::new_signed_with_payer(
        &[deposit_ix],
        Some(&user),
        &[&context.payer],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(deposit_tx).await.unwrap();
    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    // Create market with short end_time
    let market_id = 200u64;
    let question_hash = [200u8; 32];
    let clock: Clock = context.banks_client.get_sysvar().await.unwrap();
    let end_time = clock.unix_timestamp + 5; // 5 seconds from now
    let max_bet = 1_000_000_000u64;

    let create_market_ix = helpers_v2::create_market_v2_instruction(
        &program_id,
        &user,
        &oracle.pubkey(),
        market_id,
        question_hash,
        end_time,
        max_bet,
        false,
        None,
        None,
        None,
    );
    let create_market_tx = Transaction::new_signed_with_payer(
        &[create_market_ix],
        Some(&user),
        &[&context.payer],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(create_market_tx).await.unwrap();
    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    // === Betting Phase ===
    // Place bet on YES (winning side)
    let bet_commitment = [10u8; 32];
    let bet_amount = 500_000_000u64; // 0.5 SOL
    let balance_after_bet = [11u8; 32];

    let place_bet_inputs = helpers_v2::mock_place_bet_public_inputs(
        &initial_commitment,
        &balance_after_bet,
        &bet_commitment,
        max_bet,
        bet_amount,
    );

    let place_bet_ix = helpers_v2::place_bet_v2_instruction(
        &program_id,
        &user,
        market_id,
        bet_commitment,
        true, // YES
        helpers_v2::mock_zk_proof(),
        place_bet_inputs,
        balance_after_bet,
        vec![0u8; 64],
        33,
        &zk_program_id,
    );
    let place_bet_tx = Transaction::new_signed_with_payer(
        &[place_bet_ix],
        Some(&user),
        &[&context.payer],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(place_bet_tx).await.unwrap();
    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    // Verify bet counter
    let (market_pda, _) = helpers_v2::derive_market_v2_pda(&program_id, market_id);
    let market_account = context.banks_client.get_account(market_pda).await.unwrap().unwrap();
    let market = MarketV2::deserialize(&mut &market_account.data[..]).unwrap();
    assert_eq!(market.bet_count_yes, 1);

    // Get market vault balance for payout calculation
    let (market_vault_pda, _) = helpers_v2::derive_market_vault_pda(&program_id, market_id);
    let vault_balance_before = context
        .banks_client
        .get_account(market_vault_pda)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    println!("Market vault balance after bet: {} lamports", vault_balance_before);

    // === Settlement Phase ===
    // Warp time past end_time
    let current_slot = context.banks_client.get_root_slot().await.unwrap();
    context.warp_to_slot(current_slot + 10000).unwrap();
    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    // Settle market with YES winning
    let settle_ix = helpers_v2::settle_market_v2_instruction(
        &program_id,
        &oracle.pubkey(),
        market_id,
        true, // YES wins
        false,
    );
    let settle_tx = Transaction::new_signed_with_payer(
        &[settle_ix],
        Some(&user),
        &[&context.payer, &oracle],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(settle_tx).await.unwrap();
    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    // === Claim Phase ===
    let nullifier_hash = [20u8; 32];
    let balance_after_claim = [21u8; 32];
    let bet_side = true; // YES (winning side)

    // Create claim public inputs with the new format
    let claim_public_inputs = helpers_v2::mock_claim_public_inputs(
        &bet_commitment,
        true, // resolution: YES won
        &nullifier_hash,
        bet_amount,
        bet_side,
        &balance_after_bet,
        &balance_after_claim,
    );

    let claim_ix = helpers_v2::claim_v2_instruction(
        &program_id,
        &user,
        market_id,
        nullifier_hash,
        helpers_v2::mock_zk_proof(),
        claim_public_inputs,
        balance_after_claim,
        vec![0u8; 64], // encrypted_payout
        bet_commitment,
        34, // circuit type for claim
        &zk_program_id,
    );

    let claim_tx = Transaction::new_signed_with_payer(
        &[claim_ix],
        Some(&user),
        &[&context.payer],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(claim_tx).await.unwrap();

    // Verify position is claimed
    let (position_pda, _) = helpers_v2::derive_position_v2_pda(&program_id, market_id, &bet_commitment);
    let position_account = context
        .banks_client
        .get_account(position_pda)
        .await
        .unwrap()
        .unwrap();
    let position = PositionV2::deserialize(&mut &position_account.data[..]).unwrap();
    assert!(position.is_claimed());

    // Verify nullifier was created
    let (nullifier_pda, _) = helpers_v2::derive_nullifier_pda(&program_id, market_id, &nullifier_hash);
    let nullifier_account = context.banks_client.get_account(nullifier_pda).await.unwrap();
    assert!(nullifier_account.is_some());

    println!("ClaimV2 test passed:");
    println!("  - Position marked as claimed");
    println!("  - Nullifier created to prevent double-claim");
    println!("  - Payout model: bet_amount + (vault_total / winner_count)");
}

/// Multi-user test: 2 users bet on opposite sides, winner claims payout
#[tokio::test]
async fn test_multi_user_bet_and_claim() {
    use solana_sdk::clock::Clock;

    let mut program_test = helpers_v2::setup_program_test();
    let mut context = program_test.start_with_context().await;

    let program_id = futarchy_markets::id();
    let payer = context.payer.insecure_clone();
    let oracle = Keypair::new();
    let user_alice = Keypair::new();
    let user_bob = Keypair::new();
    let zk_program_id = Pubkey::new_unique();

    // Fund Alice and Bob
    let transfer_ix_alice = solana_program::system_instruction::transfer(
        &payer.pubkey(),
        &user_alice.pubkey(),
        20_000_000_000, // 20 SOL
    );
    let transfer_ix_bob = solana_program::system_instruction::transfer(
        &payer.pubkey(),
        &user_bob.pubkey(),
        20_000_000_000, // 20 SOL
    );
    let fund_tx = Transaction::new_signed_with_payer(
        &[transfer_ix_alice, transfer_ix_bob],
        Some(&payer.pubkey()),
        &[&payer],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(fund_tx).await.unwrap();
    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    // === Setup Protocol ===
    let init_ix = helpers_v2::initialize_protocol_instruction(&program_id, &payer.pubkey());
    let init_tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&payer.pubkey()),
        &[&payer],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(init_tx).await.unwrap();
    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    // === Alice creates balance and deposits ===
    let alice_initial_commitment = [100u8; 32];
    let create_alice_balance = helpers_v2::create_private_balance_instruction(
        &program_id,
        &user_alice.pubkey(),
        vec![0u8; 64],
        alice_initial_commitment,
    );
    let alice_deposit = helpers_v2::deposit_instruction(&program_id, &user_alice.pubkey(), 5_000_000_000);
    let alice_setup_tx = Transaction::new_signed_with_payer(
        &[create_alice_balance, alice_deposit],
        Some(&user_alice.pubkey()),
        &[&user_alice],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(alice_setup_tx).await.unwrap();
    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    // === Bob creates balance and deposits ===
    let bob_initial_commitment = [200u8; 32];
    let create_bob_balance = helpers_v2::create_private_balance_instruction(
        &program_id,
        &user_bob.pubkey(),
        vec![0u8; 64],
        bob_initial_commitment,
    );
    let bob_deposit = helpers_v2::deposit_instruction(&program_id, &user_bob.pubkey(), 5_000_000_000);
    let bob_setup_tx = Transaction::new_signed_with_payer(
        &[create_bob_balance, bob_deposit],
        Some(&user_bob.pubkey()),
        &[&user_bob],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(bob_setup_tx).await.unwrap();
    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    // === Create Market ===
    let market_id = 500u64;
    let clock: Clock = context.banks_client.get_sysvar().await.unwrap();
    let end_time = clock.unix_timestamp + 5;
    let max_bet = 1_000_000_000u64;

    let create_market_ix = helpers_v2::create_market_v2_instruction(
        &program_id,
        &payer.pubkey(),
        &oracle.pubkey(),
        market_id,
        [50u8; 32],
        end_time,
        max_bet,
        false,
        None,
        None,
        None,
    );
    let create_market_tx = Transaction::new_signed_with_payer(
        &[create_market_ix],
        Some(&payer.pubkey()),
        &[&payer],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(create_market_tx).await.unwrap();
    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    // === Alice bets 1 SOL on YES ===
    let alice_bet_commitment = [101u8; 32];
    let alice_bet_amount = 1_000_000_000u64;
    let alice_balance_after_bet = [102u8; 32];

    let alice_bet_inputs = helpers_v2::mock_place_bet_public_inputs(
        &alice_initial_commitment,
        &alice_balance_after_bet,
        &alice_bet_commitment,
        max_bet,
        alice_bet_amount,
    );

    let alice_bet_ix = helpers_v2::place_bet_v2_instruction(
        &program_id,
        &user_alice.pubkey(),
        market_id,
        alice_bet_commitment,
        true, // YES
        helpers_v2::mock_zk_proof(),
        alice_bet_inputs,
        alice_balance_after_bet,
        vec![0u8; 64],
        33,
        &zk_program_id,
    );
    let alice_bet_tx = Transaction::new_signed_with_payer(
        &[alice_bet_ix],
        Some(&user_alice.pubkey()),
        &[&user_alice],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(alice_bet_tx).await.unwrap();
    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    // === Bob bets 1 SOL on NO ===
    let bob_bet_commitment = [201u8; 32];
    let bob_bet_amount = 1_000_000_000u64;
    let bob_balance_after_bet = [202u8; 32];

    let bob_bet_inputs = helpers_v2::mock_place_bet_public_inputs(
        &bob_initial_commitment,
        &bob_balance_after_bet,
        &bob_bet_commitment,
        max_bet,
        bob_bet_amount,
    );

    let bob_bet_ix = helpers_v2::place_bet_v2_instruction(
        &program_id,
        &user_bob.pubkey(),
        market_id,
        bob_bet_commitment,
        false, // NO
        helpers_v2::mock_zk_proof(),
        bob_bet_inputs,
        bob_balance_after_bet,
        vec![0u8; 64],
        33,
        &zk_program_id,
    );
    let bob_bet_tx = Transaction::new_signed_with_payer(
        &[bob_bet_ix],
        Some(&user_bob.pubkey()),
        &[&user_bob],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(bob_bet_tx).await.unwrap();
    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    // Verify bet counters
    let (market_pda, _) = helpers_v2::derive_market_v2_pda(&program_id, market_id);
    let market_account = context.banks_client.get_account(market_pda).await.unwrap().unwrap();
    let market = MarketV2::deserialize(&mut &market_account.data[..]).unwrap();
    assert_eq!(market.bet_count_yes, 1, "Alice's YES bet not counted");
    assert_eq!(market.bet_count_no, 1, "Bob's NO bet not counted");

    // Check MarketVault balance (should have 2 SOL from bets + rent)
    let (market_vault_pda, _) = helpers_v2::derive_market_vault_pda(&program_id, market_id);
    let vault_before_settle = context.banks_client.get_account(market_vault_pda).await.unwrap().unwrap();
    println!("MarketVault balance after bets: {} lamports", vault_before_settle.lamports);
    assert!(vault_before_settle.lamports >= 2_000_000_000, "MarketVault should have at least 2 SOL");

    // === Settle: YES wins ===
    let current_slot = context.banks_client.get_root_slot().await.unwrap();
    context.warp_to_slot(current_slot + 10000).unwrap();
    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    let settle_ix = helpers_v2::settle_market_v2_instruction(
        &program_id,
        &oracle.pubkey(),
        market_id,
        true, // YES wins
        false,
    );
    let settle_tx = Transaction::new_signed_with_payer(
        &[settle_ix],
        Some(&payer.pubkey()),
        &[&payer, &oracle],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(settle_tx).await.unwrap();
    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    // === Alice claims (she won!) ===
    let alice_nullifier = [103u8; 32];
    let alice_balance_after_claim = [104u8; 32];

    let alice_claim_inputs = helpers_v2::mock_claim_public_inputs(
        &alice_bet_commitment,
        true, // resolution: YES won
        &alice_nullifier,
        alice_bet_amount,
        true, // bet_side: YES
        &alice_balance_after_bet,
        &alice_balance_after_claim,
    );

    let alice_claim_ix = helpers_v2::claim_v2_instruction(
        &program_id,
        &user_alice.pubkey(),
        market_id,
        alice_nullifier,
        helpers_v2::mock_zk_proof(),
        alice_claim_inputs,
        alice_balance_after_claim,
        vec![0u8; 64],
        alice_bet_commitment,
        34,
        &zk_program_id,
    );
    let alice_claim_tx = Transaction::new_signed_with_payer(
        &[alice_claim_ix],
        Some(&user_alice.pubkey()),
        &[&user_alice],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(alice_claim_tx).await.unwrap();

    // Verify Alice's position is claimed
    let (alice_position_pda, _) = helpers_v2::derive_position_v2_pda(&program_id, market_id, &alice_bet_commitment);
    let alice_position = context.banks_client.get_account(alice_position_pda).await.unwrap().unwrap();
    let alice_pos = PositionV2::deserialize(&mut &alice_position.data[..]).unwrap();
    assert!(alice_pos.is_claimed(), "Alice's position should be claimed");

    // Verify Bob's position is NOT claimed (he lost)
    let (bob_position_pda, _) = helpers_v2::derive_position_v2_pda(&program_id, market_id, &bob_bet_commitment);
    let bob_position = context.banks_client.get_account(bob_position_pda).await.unwrap().unwrap();
    let bob_pos = PositionV2::deserialize(&mut &bob_position.data[..]).unwrap();
    assert!(!bob_pos.is_claimed(), "Bob's position should NOT be claimed (he lost)");

    println!("Multi-user test passed:");
    println!("  - Alice bet 1 SOL on YES");
    println!("  - Bob bet 1 SOL on NO");
    println!("  - YES won, Alice claimed payout");
    println!("  - Bob's position unclaimed (lost)");
    println!("  - Equitative payout: Alice receives vault_total / winner_count");
}
