//! Unit tests for ExecutableAction using Mollusk

use borsh::{BorshDeserialize, BorshSerialize};
use futarchy_markets::state::{ActionAccount, ExecutableAction};
use solana_program::pubkey::Pubkey;

#[test]
fn test_action_account_serialization() {
    let account = ActionAccount::new(Pubkey::new_unique(), true, false);

    // Serialize
    let serialized = borsh::to_vec(&account).unwrap();

    // Verify size
    assert_eq!(serialized.len(), ActionAccount::SIZE);

    // Deserialize
    let deserialized: ActionAccount = borsh::from_slice(&serialized).unwrap();

    // Verify fields
    assert_eq!(account.pubkey, deserialized.pubkey);
    assert_eq!(account.is_signer, deserialized.is_signer);
    assert_eq!(account.is_writable, deserialized.is_writable);
}

#[test]
fn test_executable_action_none() {
    let action = ExecutableAction::None;

    assert!(action.is_none());
    assert_eq!(action.action_type(), "None");
    assert_eq!(action.size(), 1);
    assert!(action.validate_size());

    // Serialize/deserialize
    let serialized = borsh::to_vec(&action).unwrap();
    let deserialized: ExecutableAction = borsh::from_slice(&serialized).unwrap();
    assert_eq!(action, deserialized);
}

#[test]
fn test_executable_action_transfer_tokens() {
    let action = ExecutableAction::TransferTokens {
        token_mint: Pubkey::new_unique(),
        from_treasury: Pubkey::new_unique(),
        to: Pubkey::new_unique(),
        amount: 1_000_000_000,
    };

    assert!(!action.is_none());
    assert_eq!(action.action_type(), "TransferTokens");
    assert!(action.validate_size());

    // Verify size calculation
    let expected_size = 1 + 32 + 32 + 32 + 8; // discriminator + 3 pubkeys + u64
    assert_eq!(action.size(), expected_size);

    // Serialize/deserialize round trip
    let serialized = borsh::to_vec(&action).unwrap();
    let deserialized: ExecutableAction = borsh::from_slice(&serialized).unwrap();
    assert_eq!(action, deserialized);
}

#[test]
fn test_executable_action_update_parameter() {
    let action = ExecutableAction::UpdateParameter {
        target_program: Pubkey::new_unique(),
        parameter_key: b"max_bet".to_vec(),
        parameter_value: 5_000_000_000u64.to_le_bytes().to_vec(),
    };

    assert!(!action.is_none());
    assert_eq!(action.action_type(), "UpdateParameter");
    assert!(action.validate_size());

    // Serialize/deserialize
    let serialized = borsh::to_vec(&action).unwrap();
    let deserialized: ExecutableAction = borsh::from_slice(&serialized).unwrap();
    assert_eq!(action, deserialized);
}

#[test]
fn test_executable_action_custom_instruction() {
    let accounts = vec![
        ActionAccount::new(Pubkey::new_unique(), true, true),
        ActionAccount::new(Pubkey::new_unique(), false, true),
        ActionAccount::new(Pubkey::new_unique(), false, false),
    ];

    let data = vec![1, 2, 3, 4, 5, 6, 7, 8];

    let action = ExecutableAction::CustomInstruction {
        program_id: Pubkey::new_unique(),
        accounts: accounts.clone(),
        data: data.clone(),
    };

    assert!(!action.is_none());
    assert_eq!(action.action_type(), "CustomInstruction");
    assert!(action.validate_size());

    // Verify size calculation
    let expected_size = 1 + 32 + // discriminator + program_id
        4 + (accounts.len() * ActionAccount::SIZE) + // accounts vec
        4 + data.len(); // data vec
    assert_eq!(action.size(), expected_size);

    // Serialize/deserialize
    let serialized = borsh::to_vec(&action).unwrap();
    let deserialized: ExecutableAction = borsh::from_slice(&serialized).unwrap();
    assert_eq!(action, deserialized);
}

#[test]
fn test_executable_action_size_limit() {
    // Create action with data that's too large
    let large_data = vec![0u8; 1000]; // Exceeds max 512 bytes

    let action = ExecutableAction::CustomInstruction {
        program_id: Pubkey::new_unique(),
        accounts: vec![],
        data: large_data,
    };

    // Should fail validation
    assert!(!action.validate_size());
    assert!(action.size() > ExecutableAction::MAX_SIZE);
}

#[test]
fn test_executable_action_with_many_accounts() {
    // Create action with 10 accounts (max in our design)
    let accounts: Vec<ActionAccount> = (0..10)
        .map(|_| ActionAccount::new(Pubkey::new_unique(), false, false))
        .collect();

    let action = ExecutableAction::CustomInstruction {
        program_id: Pubkey::new_unique(),
        accounts,
        data: vec![1, 2, 3],
    };

    // Should still be valid
    assert!(action.validate_size());
}

#[test]
fn test_executable_action_with_too_many_accounts() {
    // Create action with more than 10 accounts
    let accounts: Vec<ActionAccount> = (0..15)
        .map(|_| ActionAccount::new(Pubkey::new_unique(), false, false))
        .collect();

    let action = ExecutableAction::CustomInstruction {
        program_id: Pubkey::new_unique(),
        accounts,
        data: vec![1, 2, 3],
    };

    // Might exceed size limit
    // Size check depends on data
    let size_ok = action.validate_size();

    // At least verify size is calculated correctly
    let expected_size = 1 + 32 + 4 + (15 * ActionAccount::SIZE) + 4 + 3;
    assert_eq!(action.size(), expected_size);

    // Should fail if exceeds MAX_SIZE
    assert_eq!(size_ok, action.size() <= ExecutableAction::MAX_SIZE);
}

#[test]
fn test_all_action_types_serialization() {
    let actions = vec![
        ExecutableAction::None,
        ExecutableAction::TransferTokens {
            token_mint: Pubkey::new_unique(),
            from_treasury: Pubkey::new_unique(),
            to: Pubkey::new_unique(),
            amount: 12345,
        },
        ExecutableAction::UpdateParameter {
            target_program: Pubkey::new_unique(),
            parameter_key: b"test_key".to_vec(),
            parameter_value: b"test_value".to_vec(),
        },
        ExecutableAction::CustomInstruction {
            program_id: Pubkey::new_unique(),
            accounts: vec![ActionAccount::new(Pubkey::new_unique(), true, false)],
            data: vec![42],
        },
    ];

    for action in actions {
        let serialized = borsh::to_vec(&action).unwrap();
        let deserialized: ExecutableAction = borsh::from_slice(&serialized).unwrap();
        assert_eq!(action, deserialized);
        assert!(action.validate_size());
    }
}
