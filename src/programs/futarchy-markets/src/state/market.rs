//! Market account state

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

use super::executable_action::ExecutableAction;

pub const MARKET_SEED: &[u8] = b"market";

/// Market account - represents a prediction market
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct Market {
    /// Market creator/authority
    pub authority: Pubkey,

    /// Unique market ID (incremental)
    pub market_id: u64,

    /// Oracle that can settle this market
    pub oracle: Pubkey,

    /// Market question/description (stored as hash to save space)
    pub question_hash: [u8; 32],

    /// Unix timestamp when betting closes
    pub end_time: i64,

    /// Market status
    pub status: MarketStatus,

    /// Maximum bet allowed per position
    pub max_bet: u64,

    /// Total amount bet on YES (transparent for MVP)
    pub total_yes_bets: u64,

    /// Total amount bet on NO (transparent for MVP)
    pub total_no_bets: u64,

    /// Encrypted pool for YES (FHE ciphertext)
    /// Empty vec if not using FHE
    pub encrypted_pool_yes: Vec<u8>,

    /// Encrypted pool for NO (FHE ciphertext)
    /// Empty vec if not using FHE
    pub encrypted_pool_no: Vec<u8>,

    /// Pending FHE job for pool update (if any)
    pub pending_pool_update_job: Option<u64>,

    /// Resolved outcome (None if not settled)
    /// true = YES won, false = NO won
    pub resolution: Option<bool>,

    /// Timestamp when market was settled
    pub settled_at: Option<i64>,

    /// Escrow PDA holding funds
    pub escrow: Pubkey,

    /// Used claim nullifiers (to prevent double claims)
    /// Stored as Vec for flexibility, can be migrated to Merkle tree later
    pub claim_nullifiers: Vec<[u8; 32]>,

    /// Governance: Executable action if market is used for decision-making
    pub executable_action: ExecutableAction,

    /// Governance: Execution threshold (0-100, percentage of YES votes needed)
    /// E.g., 50 = 50% of votes must be YES to execute action
    pub execution_threshold: u8,

    /// Governance: Timelock duration in seconds (delay before execution)
    pub timelock_duration: i64,

    /// Governance: When timelock expires (set after settlement if threshold met)
    pub timelock_expires_at: Option<i64>,

    /// Governance: Whether action has been executed
    pub action_executed: bool,

    /// Bump seed for PDA derivation
    pub bump: u8,

    /// Timestamp when market was created
    pub created_at: i64,
}

impl Market {
    /// Space needed for Market account
    ///
    /// Calculation:
    /// - authority: 32
    /// - market_id: 8
    /// - oracle: 32
    /// - question_hash: 32
    /// - end_time: 8
    /// - status: 1
    /// - max_bet: 8
    /// - total_yes_bets: 8
    /// - total_no_bets: 8
    /// - encrypted_pool_yes: 4 + max_ciphertext_size (4 + 512 = 516)
    /// - encrypted_pool_no: 4 + max_ciphertext_size (516)
    /// - pending_pool_update_job: 1 + 8 (Option<u64>) = 9
    /// - resolution: 1 + 1 (Option<bool>) = 2
    /// - settled_at: 1 + 8 (Option<i64>) = 9
    /// - escrow: 32
    /// - claim_nullifiers: 4 + (32 * max_nullifiers)
    /// - executable_action: ~600 (ExecutableAction::MAX_SIZE)
    /// - execution_threshold: 1
    /// - timelock_duration: 8
    /// - timelock_expires_at: 1 + 8 (Option<i64>) = 9
    /// - action_executed: 1
    /// - bump: 1
    /// - created_at: 8
    ///
    /// Base (without nullifiers): 184 + 1041 + 619 = 1844 bytes
    /// With 100 nullifiers: 1844 + 4 + 3200 = 5048 bytes
    pub const BASE_SPACE: usize = 1844;

    /// Max nullifiers we can store (adjustable based on rent budget)
    pub const MAX_NULLIFIERS: usize = 100;

    pub const SPACE: usize = Self::BASE_SPACE + 4 + (32 * Self::MAX_NULLIFIERS);

    /// Check if market is active for betting
    pub fn is_active(&self) -> bool {
        matches!(self.status, MarketStatus::Active)
    }

    /// Check if market is settled
    pub fn is_settled(&self) -> bool {
        matches!(self.status, MarketStatus::Settled)
    }

    /// Check if betting period has ended
    pub fn is_betting_closed(&self, current_time: i64) -> bool {
        current_time >= self.end_time
    }

    /// Add a claim nullifier
    pub fn add_nullifier(&mut self, nullifier: [u8; 32]) -> Result<(), &'static str> {
        if self.claim_nullifiers.contains(&nullifier) {
            return Err("Nullifier already used");
        }

        if self.claim_nullifiers.len() >= Self::MAX_NULLIFIERS {
            return Err("Nullifier storage full");
        }

        self.claim_nullifiers.push(nullifier);
        Ok(())
    }

    /// Calculate total pool
    pub fn total_pool(&self) -> u64 {
        self.total_yes_bets.saturating_add(self.total_no_bets)
    }

    /// Calculate winning pool based on resolution
    pub fn winning_pool(&self) -> Option<u64> {
        self.resolution.map(|yes_won| {
            if yes_won {
                self.total_yes_bets
            } else {
                self.total_no_bets
            }
        })
    }

    /// Calculate losing pool based on resolution
    pub fn losing_pool(&self) -> Option<u64> {
        self.resolution.map(|yes_won| {
            if yes_won {
                self.total_no_bets
            } else {
                self.total_yes_bets
            }
        })
    }

    /// Check if this is a governance market (has executable action)
    pub fn is_governance_market(&self) -> bool {
        !self.executable_action.is_none()
    }

    /// Calculate YES vote percentage (0-100)
    pub fn yes_vote_percentage(&self) -> u8 {
        let total = self.total_pool();
        if total == 0 {
            return 0;
        }

        // Calculate (yes_bets * 100) / total
        // Use u128 to prevent overflow
        let percentage = (self.total_yes_bets as u128 * 100) / total as u128;
        percentage.min(100) as u8
    }

    /// Check if execution threshold was met
    /// Returns true if YES votes >= execution_threshold
    pub fn threshold_met(&self) -> bool {
        self.yes_vote_percentage() >= self.execution_threshold
    }

    /// Check if timelock has expired
    pub fn timelock_expired(&self, current_time: i64) -> bool {
        if let Some(expires_at) = self.timelock_expires_at {
            current_time >= expires_at
        } else {
            false
        }
    }

    /// Check if governance action can be executed
    /// Returns true if:
    /// - Market is settled
    /// - Is governance market
    /// - Threshold was met
    /// - Timelock expired
    /// - Action not already executed
    pub fn can_execute_action(&self, current_time: i64) -> bool {
        self.is_settled()
            && self.is_governance_market()
            && self.threshold_met()
            && self.timelock_expired(current_time)
            && !self.action_executed
    }
}

/// Market status enum
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[borsh(use_discriminant = true)]
pub enum MarketStatus {
    /// Market is active, accepting bets
    Active = 0,
    /// Market is paused (no new bets)
    Paused = 1,
    /// Market has been settled
    Settled = 2,
    /// Market was cancelled (refunds possible)
    Cancelled = 3,
}

// ============================================================================
// V2: Minimal Market (for PrivateBalance architecture)
// ============================================================================

pub const MARKET_V2_SEED: &[u8] = b"market_v2";

/// MarketV2 - Minimal market structure with vote counting
/// Seeds: ["market_v2", market_id]
///
/// V2 Simple: No on-chain amount pools. Only vote counts are public.
/// Payout = bet_amount + (vault_total / winner_count)
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct MarketV2 {
    /// Market creator/authority
    pub authority: Pubkey,

    /// Unique market ID
    pub market_id: u64,

    /// Oracle that can settle this market
    pub oracle: Pubkey,

    /// Market question hash
    pub question_hash: [u8; 32],

    /// Unix timestamp when betting closes
    pub end_time: i64,

    /// Market status
    pub status: MarketStatus,

    /// Maximum bet allowed per position
    pub max_bet: u64,

    /// Resolved outcome (None if not settled)
    pub resolution: Option<bool>,

    /// Number of bets on YES (public counter)
    pub bet_count_yes: u64,

    /// Number of bets on NO (public counter)
    pub bet_count_no: u64,

    /// Commitment to off-chain FHE pool state (for V3)
    /// pool_state_root = Poseidon(encrypted_yes || encrypted_no || nonce)
    pub pool_state_root: [u8; 32],

    /// Whether this market has governance (GovernanceConfig PDA exists)
    pub has_governance: bool,

    /// Bump seed for PDA derivation
    pub bump: u8,

    /// Timestamp when market was created
    pub created_at: i64,
}

impl MarketV2 {
    /// Space: 32 + 8 + 32 + 32 + 8 + 1 + 8 + 2 + 8 + 8 + 32 + 1 + 1 + 8 = 181 bytes
    /// - authority: 32
    /// - market_id: 8
    /// - oracle: 32
    /// - question_hash: 32
    /// - end_time: 8
    /// - status: 1
    /// - max_bet: 8
    /// - resolution: 2 (Option<bool>)
    /// - bet_count_yes: 8
    /// - bet_count_no: 8
    /// - pool_state_root: 32
    /// - has_governance: 1
    /// - bump: 1
    /// - created_at: 8
    pub const SPACE: usize = 181;

    pub fn new(
        authority: Pubkey,
        market_id: u64,
        oracle: Pubkey,
        question_hash: [u8; 32],
        end_time: i64,
        max_bet: u64,
        has_governance: bool,
        bump: u8,
        created_at: i64,
    ) -> Self {
        Self {
            authority,
            market_id,
            oracle,
            question_hash,
            end_time,
            status: MarketStatus::Active,
            max_bet,
            resolution: None,
            bet_count_yes: 0,
            bet_count_no: 0,
            pool_state_root: [0u8; 32],
            has_governance,
            bump,
            created_at,
        }
    }

    /// Increment bet counter for the given side
    pub fn increment_bet_count(&mut self, side: bool) {
        if side {
            self.bet_count_yes = self.bet_count_yes.saturating_add(1);
        } else {
            self.bet_count_no = self.bet_count_no.saturating_add(1);
        }
    }

    /// Get winner count based on resolution
    pub fn winner_count(&self) -> Option<u64> {
        self.resolution.map(|yes_won| {
            if yes_won {
                self.bet_count_yes
            } else {
                self.bet_count_no
            }
        })
    }

    /// Get total bet count
    pub fn total_bet_count(&self) -> u64 {
        self.bet_count_yes.saturating_add(self.bet_count_no)
    }

    pub fn seeds_with_bump<'a>(market_id: &'a [u8; 8], bump: &'a [u8]) -> [&'a [u8]; 3] {
        [MARKET_V2_SEED, market_id, bump]
    }

    pub fn is_active(&self) -> bool {
        matches!(self.status, MarketStatus::Active)
    }

    pub fn is_settled(&self) -> bool {
        matches!(self.status, MarketStatus::Settled)
    }

    pub fn is_betting_closed(&self, current_time: i64) -> bool {
        current_time >= self.end_time
    }

    /// Update pool state root (called after FHE prover consensus)
    pub fn update_pool_state_root(&mut self, new_root: [u8; 32]) {
        self.pool_state_root = new_root;
    }

    /// Settle the market
    pub fn settle(&mut self, resolution: bool) {
        self.resolution = Some(resolution);
        self.status = MarketStatus::Settled;
    }
}
