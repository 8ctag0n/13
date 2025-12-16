//! Unit tests for UserEligibility state

use borsh::{BorshDeserialize, BorshSerialize};
use futarchy_markets::state::UserEligibility;
use solana_program::pubkey::Pubkey;

#[test]
fn test_user_eligibility_serialization() {
    let eligibility = UserEligibility {
        user: Pubkey::new_unique(),
        poi_job_id: 123,
        blacklist_root: [42u8; 32],
        registered_at: 1600000000,
        expires_at: 1600086400, // +24h
        is_active: true,
        blacklist_version: 1,
        bump: 255,
    };

    // Serialize
    let serialized = borsh::to_vec(&eligibility).unwrap();

    // Verify size
    assert!(serialized.len() <= UserEligibility::SPACE);

    // Deserialize
    let deserialized: UserEligibility = borsh::from_slice(&serialized).unwrap();

    // Verify all fields
    assert_eq!(eligibility.user, deserialized.user);
    assert_eq!(eligibility.poi_job_id, deserialized.poi_job_id);
    assert_eq!(eligibility.blacklist_root, deserialized.blacklist_root);
    assert_eq!(eligibility.registered_at, deserialized.registered_at);
    assert_eq!(eligibility.expires_at, deserialized.expires_at);
    assert_eq!(eligibility.is_active, deserialized.is_active);
    assert_eq!(eligibility.blacklist_version, deserialized.blacklist_version);
    assert_eq!(eligibility.bump, deserialized.bump);
}

#[test]
fn test_user_eligibility_validity_window() {
    let registered_at = 1600000000;
    let expires_at = registered_at + UserEligibility::VALIDITY_WINDOW;

    let eligibility = UserEligibility {
        user: Pubkey::new_unique(),
        poi_job_id: 1,
        blacklist_root: [0u8; 32],
        registered_at,
        expires_at,
        is_active: true,
        blacklist_version: 1,
        bump: 255,
    };

    // Valid within window
    assert!(eligibility.is_valid(registered_at + 1)); // Just after registration
    assert!(eligibility.is_valid(registered_at + 3600)); // 1 hour later
    assert!(eligibility.is_valid(registered_at + 12 * 3600)); // 12 hours later
    assert!(eligibility.is_valid(expires_at - 1)); // Just before expiry

    // Invalid at expiry
    assert!(!eligibility.is_valid(expires_at));

    // Invalid after expiry
    assert!(!eligibility.is_valid(expires_at + 1));
    assert!(!eligibility.is_valid(expires_at + 86400));
}

#[test]
fn test_user_eligibility_inactive() {
    let eligibility = UserEligibility {
        user: Pubkey::new_unique(),
        poi_job_id: 1,
        blacklist_root: [0u8; 32],
        registered_at: 1600000000,
        expires_at: 1600086400,
        is_active: false, // Inactive
        blacklist_version: 1,
        bump: 255,
    };

    // Even if within time window, inactive = invalid
    let current_time = 1600000001; // Just after registration
    assert!(!eligibility.is_valid(current_time));
}

#[test]
fn test_user_eligibility_needs_renewal_expired() {
    let eligibility = UserEligibility {
        user: Pubkey::new_unique(),
        poi_job_id: 1,
        blacklist_root: [0u8; 32],
        registered_at: 1600000000,
        expires_at: 1600086400,
        is_active: true,
        blacklist_version: 1,
        bump: 255,
    };

    // Does not need renewal before expiry
    assert!(!eligibility.needs_renewal(1600086399, 1));

    // Needs renewal at expiry
    assert!(eligibility.needs_renewal(1600086400, 1));

    // Needs renewal after expiry
    assert!(eligibility.needs_renewal(1600086401, 1));
}

#[test]
fn test_user_eligibility_needs_renewal_blacklist_updated() {
    let eligibility = UserEligibility {
        user: Pubkey::new_unique(),
        poi_job_id: 1,
        blacklist_root: [0u8; 32],
        registered_at: 1600000000,
        expires_at: 1600086400,
        is_active: true,
        blacklist_version: 1,
        bump: 255,
    };

    // Same blacklist version - no renewal needed
    assert!(!eligibility.needs_renewal(1600000001, 1));

    // Newer blacklist version - needs renewal
    assert!(eligibility.needs_renewal(1600000001, 2));
    assert!(eligibility.needs_renewal(1600000001, 5));
}

#[test]
fn test_user_eligibility_needs_renewal_both_conditions() {
    let eligibility = UserEligibility {
        user: Pubkey::new_unique(),
        poi_job_id: 1,
        blacklist_root: [0u8; 32],
        registered_at: 1600000000,
        expires_at: 1600086400,
        is_active: true,
        blacklist_version: 1,
        bump: 255,
    };

    // Both expired AND blacklist updated
    assert!(eligibility.needs_renewal(1600086401, 2));

    // Only blacklist updated (not expired)
    assert!(eligibility.needs_renewal(1600000001, 2));

    // Only expired (blacklist same)
    assert!(eligibility.needs_renewal(1600086401, 1));

    // Neither - no renewal needed
    assert!(!eligibility.needs_renewal(1600000001, 1));
}

#[test]
fn test_user_eligibility_validity_constant() {
    // VALIDITY_WINDOW should be 24 hours (86400 seconds)
    assert_eq!(UserEligibility::VALIDITY_WINDOW, 86400);
}

#[test]
fn test_user_eligibility_space_constant() {
    let eligibility = UserEligibility {
        user: Pubkey::new_unique(),
        poi_job_id: u64::MAX,
        blacklist_root: [255u8; 32],
        registered_at: i64::MAX,
        expires_at: i64::MAX,
        is_active: true,
        blacklist_version: u32::MAX,
        bump: 255,
    };

    let serialized = borsh::to_vec(&eligibility).unwrap();

    // Verify SPACE constant is sufficient
    assert!(serialized.len() <= UserEligibility::SPACE);

    // Verify actual size matches expected (94 bytes)
    // 32 (user) + 8 (poi_job_id) + 32 (blacklist_root) + 8 (registered_at) +
    // 8 (expires_at) + 1 (is_active) + 4 (blacklist_version) + 1 (bump) = 94
    assert_eq!(UserEligibility::SPACE, 94);
}

#[test]
fn test_user_eligibility_different_blacklist_roots() {
    let roots = vec![
        [0u8; 32],
        [255u8; 32],
        [42u8; 32],
        {
            let mut r = [0u8; 32];
            for i in 0..32 {
                r[i] = i as u8;
            }
            r
        },
    ];

    for root in roots {
        let eligibility = UserEligibility {
            user: Pubkey::new_unique(),
            poi_job_id: 1,
            blacklist_root: root,
            registered_at: 1600000000,
            expires_at: 1600086400,
            is_active: true,
            blacklist_version: 1,
            bump: 255,
        };

        let serialized = borsh::to_vec(&eligibility).unwrap();
        let deserialized: UserEligibility = borsh::from_slice(&serialized).unwrap();

        assert_eq!(eligibility.blacklist_root, deserialized.blacklist_root);
    }
}

#[test]
fn test_user_eligibility_multiple_versions() {
    let versions = vec![0, 1, 10, 100, u32::MAX];

    for version in versions {
        let eligibility = UserEligibility {
            user: Pubkey::new_unique(),
            poi_job_id: 1,
            blacklist_root: [0u8; 32],
            registered_at: 1600000000,
            expires_at: 1600086400,
            is_active: true,
            blacklist_version: version,
            bump: 255,
        };

        let serialized = borsh::to_vec(&eligibility).unwrap();
        let deserialized: UserEligibility = borsh::from_slice(&serialized).unwrap();

        assert_eq!(eligibility.blacklist_version, deserialized.blacklist_version);
    }
}
