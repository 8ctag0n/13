use futarchy_sdk::*;
use solana_program::pubkey::Pubkey;

#[test]
fn test_instruction_serialization() {
    let instruction = FutarchyInstruction::CreateMarket {
        market_id: 1,
        question_hash: [1u8; 32],
        end_time: 1234567890,
        max_bet: 1_000_000_000,
    };

    let packed = instruction.pack().unwrap();
    let unpacked = FutarchyInstruction::unpack(&packed).unwrap();

    assert_eq!(instruction, unpacked);
}

#[test]
fn test_place_bet_serialization() {
    let instruction = FutarchyInstruction::PlaceBet {
        market_id: 1,
        bet_commitment: [2u8; 32],
        proof: vec![0u8; 256],
        public_inputs: vec![1u8; 80],
        amount: 500_000_000,
        circuit_type: 30,
        encrypted_bet_amount: None,
        side: None,
    };

    let packed = instruction.pack().unwrap();
    let unpacked = FutarchyInstruction::unpack(&packed).unwrap();

    assert_eq!(instruction, unpacked);
}

#[test]
fn test_place_bet_with_fhe_serialization() {
    let instruction = FutarchyInstruction::PlaceBet {
        market_id: 1,
        bet_commitment: [2u8; 32],
        proof: vec![0u8; 256],
        public_inputs: vec![1u8; 80],
        amount: 500_000_000,
        circuit_type: 31,
        encrypted_bet_amount: Some(vec![3u8; 128]),
        side: Some(true),
    };

    let packed = instruction.pack().unwrap();
    let unpacked = FutarchyInstruction::unpack(&packed).unwrap();

    assert_eq!(instruction, unpacked);
}

#[test]
fn test_settle_market_serialization() {
    let instruction = FutarchyInstruction::SettleMarket {
        market_id: 1,
        outcome: true,
    };

    let packed = instruction.pack().unwrap();
    let unpacked = FutarchyInstruction::unpack(&packed).unwrap();

    assert_eq!(instruction, unpacked);
}

#[test]
fn test_claim_payout_serialization() {
    let instruction = FutarchyInstruction::ClaimPayout {
        market_id: 1,
        claim_nullifier: [3u8; 32],
        proof: vec![0u8; 256],
        public_inputs: vec![1u8; 105],
        payout_amount: 1_500_000_000,
    };

    let packed = instruction.pack().unwrap();
    let unpacked = FutarchyInstruction::unpack(&packed).unwrap();

    assert_eq!(instruction, unpacked);
}

#[test]
fn test_update_pool_serialization() {
    let instruction = FutarchyInstruction::UpdatePool {
        market_id: 1,
        fhe_job_id: 42,
        encrypted_result: vec![4u8; 128],
        side: true,
    };

    let packed = instruction.pack().unwrap();
    let unpacked = FutarchyInstruction::unpack(&packed).unwrap();

    assert_eq!(instruction, unpacked);
}

#[test]
fn test_register_user_serialization() {
    let instruction = FutarchyInstruction::RegisterUser {
        proof: vec![0u8; 256],
        public_inputs: vec![1u8; 80],
        blacklist_root: [5u8; 32],
        blacklist_version: 1,
    };

    let packed = instruction.pack().unwrap();
    let unpacked = FutarchyInstruction::unpack(&packed).unwrap();

    assert_eq!(instruction, unpacked);
}

#[test]
fn test_pda_consistency() {
    let program_id = Pubkey::new_unique();
    let market_id = 42u64;
    let user = Pubkey::new_unique();

    let market_pda_1 = find_market_pda(&program_id, market_id);
    let market_pda_2 = find_market_pda(&program_id, market_id);
    assert_eq!(market_pda_1.address, market_pda_2.address);
    assert_eq!(market_pda_1.bump, market_pda_2.bump);

    let position_pda_1 = find_position_pda(&program_id, market_id, &user);
    let position_pda_2 = find_position_pda(&program_id, market_id, &user);
    assert_eq!(position_pda_1.address, position_pda_2.address);
    assert_eq!(position_pda_1.bump, position_pda_2.bump);

    let escrow_pda_1 = find_escrow_pda(&program_id, market_id);
    let escrow_pda_2 = find_escrow_pda(&program_id, market_id);
    assert_eq!(escrow_pda_1.address, escrow_pda_2.address);
    assert_eq!(escrow_pda_1.bump, escrow_pda_2.bump);
}

#[test]
fn test_pda_uniqueness() {
    let program_id = Pubkey::new_unique();

    let market_pda_1 = find_market_pda(&program_id, 1);
    let market_pda_2 = find_market_pda(&program_id, 2);
    assert_ne!(market_pda_1.address, market_pda_2.address);

    let user1 = Pubkey::new_unique();
    let user2 = Pubkey::new_unique();
    let position_pda_1 = find_position_pda(&program_id, 1, &user1);
    let position_pda_2 = find_position_pda(&program_id, 1, &user2);
    assert_ne!(position_pda_1.address, position_pda_2.address);
}

#[test]
fn test_instruction_builder() {
    let program_id = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let oracle = Pubkey::new_unique();

    let ix = build_create_market_ix(
        &program_id,
        &authority,
        1,
        [0u8; 32],
        &oracle,
        1234567890,
        1_000_000_000,
    )
    .unwrap();

    assert_eq!(ix.program_id, program_id);
    assert_eq!(ix.accounts.len(), 6);
    assert!(!ix.data.is_empty());

    assert_eq!(ix.accounts[0].pubkey, authority);
    assert!(ix.accounts[0].is_signer);
    assert!(ix.accounts[0].is_writable);

    let unpacked = FutarchyInstruction::unpack(&ix.data).unwrap();
    match unpacked {
        FutarchyInstruction::CreateMarket { market_id, .. } => {
            assert_eq!(market_id, 1);
        }
        _ => panic!("Wrong instruction variant"),
    }
}

#[test]
fn test_client_creation() {
    let program_id = Pubkey::new_unique();
    let client = FutarchyClient::new("http://localhost:8899", program_id);

    assert_eq!(client.program_id(), &program_id);
}

#[test]
fn test_client_instruction_building() {
    let program_id = Pubkey::new_unique();
    let client = FutarchyClient::new("http://localhost:8899", program_id);

    let authority = Pubkey::new_unique();
    let oracle = Pubkey::new_unique();

    let ix = client
        .create_market(&authority, 1, [0u8; 32], &oracle, 1234567890, 1_000_000_000)
        .unwrap();

    assert_eq!(ix.program_id, program_id);
    assert_eq!(ix.accounts.len(), 6);
}

#[test]
fn test_market_state_helpers() {
    let market = Market {
        authority: Pubkey::new_unique(),
        market_id: 1,
        oracle: Pubkey::new_unique(),
        question_hash: [0u8; 32],
        end_time: 1234567890,
        status: MarketStatus::Active,
        max_bet: 1_000_000_000,
        total_yes_bets: 600,
        total_no_bets: 400,
        encrypted_pool_yes: vec![],
        encrypted_pool_no: vec![],
        pending_pool_update_job: None,
        resolution: None,
        settled_at: None,
        escrow: Pubkey::new_unique(),
        claim_nullifiers: vec![],
        executable_action: ExecutableAction::None,
        execution_threshold: 50,
        timelock_duration: 3600,
        timelock_expires_at: None,
        action_executed: false,
        bump: 255,
        created_at: 1234567890,
    };

    assert!(market.is_active());
    assert!(!market.is_settled());
    assert_eq!(market.total_pool(), 1000);
    assert_eq!(market.yes_vote_percentage(), 60);
    assert!(market.threshold_met());
}

#[test]
fn test_executable_action_variants() {
    let action_none = ExecutableAction::None;
    assert!(action_none.is_none());

    let action_transfer = ExecutableAction::TransferLamports {
        recipient: Pubkey::new_unique(),
        amount: 1_000_000,
    };
    assert!(!action_transfer.is_none());

    let action_custom = ExecutableAction::ExecuteCustom {
        target_program: Pubkey::new_unique(),
        instruction_data: vec![1, 2, 3],
    };
    assert!(!action_custom.is_none());
}
