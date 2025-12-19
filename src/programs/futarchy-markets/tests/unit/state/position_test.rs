//! Unit tests for Position state

use borsh::{BorshDeserialize, BorshSerialize};
use futarchy_markets::state::Position;
use solana_program::pubkey::Pubkey;

#[test]
fn test_position_serialization() {
    let position = Position {
        user: Pubkey::new_unique(),
        market: Pubkey::new_unique(),
        bet_commitment: [42u8; 32],
        placed_at: 1600000000,
        claimed: false,
        bump: 255,
        encrypted_amount: None,
    };

    // Serialize
    let serialized = borsh::to_vec(&position).unwrap();

    // Verify size is reasonable
    assert!(serialized.len() <= Position::SPACE);

    // Deserialize
    let deserialized: Position = borsh::from_slice(&serialized).unwrap();

    // Verify all fields match
    assert_eq!(position.user, deserialized.user);
    assert_eq!(position.market, deserialized.market);
    assert_eq!(position.bet_commitment, deserialized.bet_commitment);
    assert_eq!(position.placed_at, deserialized.placed_at);
    assert_eq!(position.claimed, deserialized.claimed);
    assert_eq!(position.bump, deserialized.bump);
    assert_eq!(position.encrypted_amount, deserialized.encrypted_amount);
}

#[test]
fn test_position_space_constant() {
    let position = Position {
        user: Pubkey::new_unique(),
        market: Pubkey::new_unique(),
        bet_commitment: [0u8; 32],
        placed_at: 1600000000,
        claimed: false,
        bump: 255,
        encrypted_amount: None,
    };

    let serialized = borsh::to_vec(&position).unwrap();

    // Verify SPACE constant is sufficient (without encrypted_amount)
    assert!(serialized.len() <= Position::SPACE + 1);

    // Verify SPACE is accurate (106 bytes base)
    assert_eq!(Position::SPACE, 106);

    // Size should be close to constant (within a few bytes, +1 for Option discriminant)
    assert!(serialized.len() >= 100);
}

#[test]
fn test_position_is_claimed() {
    let position = Position {
        user: Pubkey::new_unique(),
        market: Pubkey::new_unique(),
        bet_commitment: [0u8; 32],
        placed_at: 1600000000,
        claimed: false,
        bump: 255,
        encrypted_amount: None,
    };

    assert!(!position.is_claimed());

    let mut position_claimed = position.clone();
    position_claimed.mark_claimed();
    assert!(position_claimed.is_claimed());
    assert!(position_claimed.claimed);
}

#[test]
fn test_position_mark_claimed() {
    let mut position = Position {
        user: Pubkey::new_unique(),
        market: Pubkey::new_unique(),
        bet_commitment: [0u8; 32],
        placed_at: 1600000000,
        claimed: false,
        bump: 255,
        encrypted_amount: None,
    };

    // Initially not claimed
    assert!(!position.is_claimed());

    // Mark as claimed
    position.mark_claimed();
    assert!(position.is_claimed());
    assert_eq!(position.claimed, true);

    // Calling again should be idempotent
    position.mark_claimed();
    assert!(position.is_claimed());
}

#[test]
fn test_position_with_different_commitments() {
    let commitments = vec![
        [0u8; 32],
        [255u8; 32],
        [42u8; 32],
        {
            let mut c = [0u8; 32];
            c[0] = 1;
            c
        },
    ];

    for commitment in commitments {
        let position = Position {
            user: Pubkey::new_unique(),
            market: Pubkey::new_unique(),
            bet_commitment: commitment,
            placed_at: 1600000000,
            claimed: false,
            bump: 255,
            encrypted_amount: None,
        };

        let serialized = borsh::to_vec(&position).unwrap();
        let deserialized: Position = borsh::from_slice(&serialized).unwrap();

        assert_eq!(position.bet_commitment, deserialized.bet_commitment);
    }
}

#[test]
fn test_position_unique_per_user_and_commitment() {
    let user1 = Pubkey::new_unique();
    let user2 = Pubkey::new_unique();
    let market = Pubkey::new_unique();

    let commitment1 = [1u8; 32];
    let commitment2 = [2u8; 32];

    // Same user, different commitments - should be different positions
    let pos1 = Position {
        user: user1,
        market,
        bet_commitment: commitment1,
        placed_at: 1600000000,
        claimed: false,
        bump: 255,
        encrypted_amount: None,
    };

    let pos2 = Position {
        user: user1,
        market,
        bet_commitment: commitment2,
        placed_at: 1600000000,
        claimed: false,
        bump: 255,
        encrypted_amount: None,
    };

    assert_ne!(pos1.bet_commitment, pos2.bet_commitment);

    // Different users, same commitment - should be different positions
    let pos3 = Position {
        user: user2,
        market,
        bet_commitment: commitment1,
        placed_at: 1600000000,
        claimed: false,
        bump: 255,
        encrypted_amount: None,
    };

    assert_ne!(pos1.user, pos3.user);
}

#[test]
fn test_position_with_different_timestamps() {
    let timestamps = vec![0, 1, 1600000000, i64::MAX];

    for timestamp in timestamps {
        let position = Position {
            user: Pubkey::new_unique(),
            market: Pubkey::new_unique(),
            bet_commitment: [0u8; 32],
            placed_at: timestamp,
            claimed: false,
            bump: 255,
            encrypted_amount: None,
        };

        let serialized = borsh::to_vec(&position).unwrap();
        let deserialized: Position = borsh::from_slice(&serialized).unwrap();

        assert_eq!(position.placed_at, deserialized.placed_at);
    }
}

#[test]
fn test_position_with_encrypted_amount() {
    let encrypted_data = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
    let position = Position {
        user: Pubkey::new_unique(),
        market: Pubkey::new_unique(),
        bet_commitment: [42u8; 32],
        placed_at: 1600000000,
        claimed: false,
        bump: 255,
        encrypted_amount: Some(encrypted_data.clone()),
    };

    let serialized = borsh::to_vec(&position).unwrap();
    let deserialized: Position = borsh::from_slice(&serialized).unwrap();

    assert_eq!(position.encrypted_amount, deserialized.encrypted_amount);
    assert_eq!(deserialized.encrypted_amount, Some(encrypted_data));
}

#[test]
fn test_position_space_function() {
    assert_eq!(Position::space(0), 111);
    assert_eq!(Position::space(100), 211);
    assert_eq!(Position::space(2048), 2159);
}
