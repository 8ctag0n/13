//! Unit tests for Market state using Mollusk

use borsh::{BorshDeserialize, BorshSerialize};
use futarchy_markets::state::{ExecutableAction, Market, MarketStatus};
use solana_program::pubkey::Pubkey;

#[test]
fn test_market_serialization() {
    let market = Market {
        authority: Pubkey::new_unique(),
        market_id: 1,
        oracle: Pubkey::new_unique(),
        question_hash: [42u8; 32],
        end_time: 1700000000,
        status: MarketStatus::Active,
        max_bet: 1_000_000_000,
        total_yes_bets: 0,
        total_no_bets: 0,
        encrypted_pool_yes: Vec::new(),
        encrypted_pool_no: Vec::new(),
        pending_pool_update_job: None,
        resolution: None,
        settled_at: None,
        escrow: Pubkey::new_unique(),
        claim_nullifiers: Vec::new(),
        executable_action: ExecutableAction::None,
        execution_threshold: 0,
        timelock_duration: 0,
        timelock_expires_at: None,
        action_executed: false,
        bump: 255,
        created_at: 1600000000,
    };

    // Serialize
    let serialized = borsh::to_vec(&market).unwrap();

    // Deserialize
    let deserialized: Market = borsh::from_slice(&serialized).unwrap();

    // Verify all fields match
    assert_eq!(market.authority, deserialized.authority);
    assert_eq!(market.market_id, deserialized.market_id);
    assert_eq!(market.oracle, deserialized.oracle);
    assert_eq!(market.question_hash, deserialized.question_hash);
    assert_eq!(market.end_time, deserialized.end_time);
    assert_eq!(market.status, deserialized.status);
    assert_eq!(market.max_bet, deserialized.max_bet);
    assert_eq!(market.bump, deserialized.bump);
    assert_eq!(market.created_at, deserialized.created_at);
}

#[test]
fn test_market_status_transitions() {
    let mut market = create_test_market();

    // Initial state
    assert_eq!(market.status, MarketStatus::Active);
    assert!(market.is_active());
    assert!(!market.is_settled());

    // Settle market
    market.status = MarketStatus::Settled;
    market.resolution = Some(true);
    market.settled_at = Some(1700000000);

    assert!(!market.is_active());
    assert!(market.is_settled());
    assert_eq!(market.resolution, Some(true));

    // Cancel market
    let mut market2 = create_test_market();
    market2.status = MarketStatus::Cancelled;

    assert!(!market2.is_active());
    assert!(!market2.is_settled());
}

#[test]
fn test_market_is_betting_closed() {
    let market = create_test_market_with_end_time(1700000000);

    // Before end time
    assert!(!market.is_betting_closed(1699999999));

    // At end time
    assert!(market.is_betting_closed(1700000000));

    // After end time
    assert!(market.is_betting_closed(1700000001));
}

#[test]
fn test_market_add_nullifier() {
    let mut market = create_test_market();

    let nullifier1 = [1u8; 32];
    let nullifier2 = [2u8; 32];

    // Add first nullifier
    assert!(market.add_nullifier(nullifier1).is_ok());
    assert_eq!(market.claim_nullifiers.len(), 1);

    // Add second nullifier
    assert!(market.add_nullifier(nullifier2).is_ok());
    assert_eq!(market.claim_nullifiers.len(), 2);

    // Try to add duplicate - should fail
    let result = market.add_nullifier(nullifier1);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Nullifier already used");
    assert_eq!(market.claim_nullifiers.len(), 2);
}

#[test]
fn test_market_nullifier_storage_limit() {
    let mut market = create_test_market();

    // Fill up to MAX_NULLIFIERS
    for i in 0..Market::MAX_NULLIFIERS {
        let mut nullifier = [0u8; 32];
        nullifier[0] = i as u8;
        assert!(market.add_nullifier(nullifier).is_ok());
    }

    assert_eq!(market.claim_nullifiers.len(), Market::MAX_NULLIFIERS);

    // Try to add one more - should fail
    let result = market.add_nullifier([255u8; 32]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Nullifier storage full");
}

#[test]
fn test_market_total_pool() {
    let mut market = create_test_market();

    market.total_yes_bets = 1_000_000_000;
    market.total_no_bets = 500_000_000;

    assert_eq!(market.total_pool(), 1_500_000_000);
}

#[test]
fn test_market_winning_and_losing_pools() {
    let mut market = create_test_market();

    market.total_yes_bets = 1_000_000_000;
    market.total_no_bets = 500_000_000;

    // Before settlement
    assert_eq!(market.winning_pool(), None);
    assert_eq!(market.losing_pool(), None);

    // YES wins
    market.resolution = Some(true);
    assert_eq!(market.winning_pool(), Some(1_000_000_000));
    assert_eq!(market.losing_pool(), Some(500_000_000));

    // NO wins
    market.resolution = Some(false);
    assert_eq!(market.winning_pool(), Some(500_000_000));
    assert_eq!(market.losing_pool(), Some(1_000_000_000));
}

#[test]
fn test_market_is_governance_market() {
    // Simple market (no action)
    let market1 = create_test_market();
    assert!(!market1.is_governance_market());

    // Governance market (with action)
    let market2 = create_test_governance_market();
    assert!(market2.is_governance_market());
}

#[test]
fn test_market_yes_vote_percentage() {
    let mut market = create_test_market();

    // No votes yet
    assert_eq!(market.yes_vote_percentage(), 0);

    // 60 YES, 40 NO = 60%
    market.total_yes_bets = 60_000_000_000;
    market.total_no_bets = 40_000_000_000;
    assert_eq!(market.yes_vote_percentage(), 60);

    // 50 YES, 50 NO = 50%
    market.total_yes_bets = 50_000_000_000;
    market.total_no_bets = 50_000_000_000;
    assert_eq!(market.yes_vote_percentage(), 50);

    // 100 YES, 0 NO = 100%
    market.total_yes_bets = 100_000_000_000;
    market.total_no_bets = 0;
    assert_eq!(market.yes_vote_percentage(), 100);

    // 0 YES, 100 NO = 0%
    market.total_yes_bets = 0;
    market.total_no_bets = 100_000_000_000;
    assert_eq!(market.yes_vote_percentage(), 0);

    // 75 YES, 25 NO = 75%
    market.total_yes_bets = 75_000_000_000;
    market.total_no_bets = 25_000_000_000;
    assert_eq!(market.yes_vote_percentage(), 75);
}

#[test]
fn test_market_threshold_met() {
    let mut market = create_test_governance_market();
    market.execution_threshold = 50; // 50% threshold

    // 60% YES - threshold met
    market.total_yes_bets = 60_000_000_000;
    market.total_no_bets = 40_000_000_000;
    assert!(market.threshold_met());

    // 40% YES - threshold NOT met
    market.total_yes_bets = 40_000_000_000;
    market.total_no_bets = 60_000_000_000;
    assert!(!market.threshold_met());

    // 50% YES - threshold met (equal)
    market.total_yes_bets = 50_000_000_000;
    market.total_no_bets = 50_000_000_000;
    assert!(market.threshold_met());

    // Higher threshold (80%)
    market.execution_threshold = 80;
    market.total_yes_bets = 75_000_000_000;
    market.total_no_bets = 25_000_000_000;
    assert!(!market.threshold_met()); // 75% < 80%

    market.total_yes_bets = 85_000_000_000;
    market.total_no_bets = 15_000_000_000;
    assert!(market.threshold_met()); // 85% >= 80%
}

#[test]
fn test_market_timelock_expired() {
    let mut market = create_test_governance_market();

    // No timelock set
    assert!(!market.timelock_expired(1700000000));

    // Set timelock to expire at 1700000000
    market.timelock_expires_at = Some(1700000000);

    // Before expiry
    assert!(!market.timelock_expired(1699999999));

    // At expiry
    assert!(market.timelock_expired(1700000000));

    // After expiry
    assert!(market.timelock_expired(1700000001));
}

#[test]
fn test_market_can_execute_action() {
    let mut market = create_test_governance_market();

    // Setup
    market.status = MarketStatus::Settled;
    market.execution_threshold = 50;
    market.total_yes_bets = 60_000_000_000;
    market.total_no_bets = 40_000_000_000;
    market.timelock_expires_at = Some(1700000000);
    market.action_executed = false;

    let current_time = 1700000001; // After timelock

    // All conditions met - can execute
    assert!(market.can_execute_action(current_time));

    // Not settled - cannot execute
    let mut market2 = market.clone();
    market2.status = MarketStatus::Active;
    assert!(!market2.can_execute_action(current_time));

    // Threshold not met - cannot execute
    let mut market3 = market.clone();
    market3.total_yes_bets = 40_000_000_000;
    market3.total_no_bets = 60_000_000_000;
    assert!(!market3.can_execute_action(current_time));

    // Timelock not expired - cannot execute
    assert!(!market.can_execute_action(1699999999));

    // Already executed - cannot execute
    let mut market4 = market.clone();
    market4.action_executed = true;
    assert!(!market4.can_execute_action(current_time));
}

// Helper functions

fn create_test_market() -> Market {
    Market {
        authority: Pubkey::new_unique(),
        market_id: 1,
        oracle: Pubkey::new_unique(),
        question_hash: [0u8; 32],
        end_time: 1700000000,
        status: MarketStatus::Active,
        max_bet: 1_000_000_000,
        total_yes_bets: 0,
        total_no_bets: 0,
        encrypted_pool_yes: Vec::new(),
        encrypted_pool_no: Vec::new(),
        pending_pool_update_job: None,
        resolution: None,
        settled_at: None,
        escrow: Pubkey::new_unique(),
        claim_nullifiers: Vec::new(),
        executable_action: ExecutableAction::None,
        execution_threshold: 0,
        timelock_duration: 0,
        timelock_expires_at: None,
        action_executed: false,
        bump: 255,
        created_at: 1600000000,
    }
}

fn create_test_market_with_end_time(end_time: i64) -> Market {
    let mut market = create_test_market();
    market.end_time = end_time;
    market
}

fn create_test_governance_market() -> Market {
    let mut market = create_test_market();
    market.executable_action = ExecutableAction::TransferTokens {
        token_mint: Pubkey::new_unique(),
        from_treasury: Pubkey::new_unique(),
        to: Pubkey::new_unique(),
        amount: 100_000_000,
    };
    market.execution_threshold = 50;
    market.timelock_duration = 86400; // 24h
    market
}
