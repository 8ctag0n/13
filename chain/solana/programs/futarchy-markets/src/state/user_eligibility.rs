//! UserEligibility state - tracks PoI verification

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

pub const USER_ELIGIBILITY_SEED: &[u8] = b"user_eligibility";

/// UserEligibility account - proves user passed Proof of Innocence check
///
/// This account is created when a user submits a valid PoI proof (Circuit 10).
/// It's referenced when placing bets with Circuit 31 (MarketBetWithPoI)
/// to prove the user is not on any blacklist/sanctions list.
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct UserEligibility {
    /// User's wallet address
    pub user: Pubkey,

    /// ZK job ID that verified the PoI proof
    /// This references a ZkJob in zk-generator that used Circuit 10
    pub poi_job_id: u64,

    /// Blacklist merkle root that was used for verification
    /// Allows re-verification if blacklist updates
    pub blacklist_root: [u8; 32],

    /// Timestamp when user was registered (proof generated)
    pub registered_at: i64,

    /// Proof expiry timestamp (PoI proofs valid for 24h)
    pub expires_at: i64,

    /// Whether eligibility is still active
    pub is_active: bool,

    /// Blacklist version (for tracking updates)
    pub blacklist_version: u32,

    /// Bump seed for PDA derivation
    pub bump: u8,
}

impl UserEligibility {
    /// Space needed for UserEligibility account
    ///
    /// Calculation:
    /// - user: 32
    /// - poi_job_id: 8
    /// - blacklist_root: 32
    /// - registered_at: 8
    /// - expires_at: 8
    /// - is_active: 1
    /// - blacklist_version: 4
    /// - bump: 1
    ///
    /// Total: 94 bytes
    pub const SPACE: usize = 94;

    /// PoI proof validity window (24 hours)
    pub const VALIDITY_WINDOW: i64 = 24 * 60 * 60;

    /// Check if eligibility is still valid
    pub fn is_valid(&self, current_time: i64) -> bool {
        self.is_active && current_time < self.expires_at
    }

    /// Check if eligibility needs renewal
    pub fn needs_renewal(&self, current_time: i64, blacklist_version: u32) -> bool {
        // Need renewal if expired OR blacklist updated
        current_time >= self.expires_at || self.blacklist_version < blacklist_version
    }

    /// Mark as inactive (for revocation)
    pub fn revoke(&mut self) {
        self.is_active = false;
    }

    /// Renew eligibility with new PoI proof
    pub fn renew(
        &mut self,
        new_poi_job_id: u64,
        new_blacklist_root: [u8; 32],
        new_blacklist_version: u32,
        current_time: i64,
    ) {
        self.poi_job_id = new_poi_job_id;
        self.blacklist_root = new_blacklist_root;
        self.blacklist_version = new_blacklist_version;
        self.registered_at = current_time;
        self.expires_at = current_time + Self::VALIDITY_WINDOW;
        self.is_active = true;
    }
}
