//! Validator account state for threshold encryption

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Seeds for validator PDA
pub const VALIDATOR_SEED: &[u8] = b"validator";

/// Geographic regions for validator distribution (3-of-5 threshold requires geographic diversity)
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum ValidatorRegion {
    NorthAmerica,
    Europe,
    Asia,
    LatinAmerica,
    Africa,
}

impl Default for ValidatorRegion {
    fn default() -> Self {
        ValidatorRegion::NorthAmerica
    }
}

/// Validator account - registered validator for threshold encryption key shares
///
/// PDA: ["validator", validator_wallet]
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct ValidatorAccount {
    /// Validator's wallet address (authority)
    pub authority: Pubkey,

    /// Validator's public endpoint for key share requests
    pub endpoint: String,

    /// Geographic region (for distribution requirements)
    pub region: ValidatorRegion,

    /// Amount staked in lamports
    pub stake: u64,

    /// Total key shares provided
    pub total_shares_provided: u64,

    /// Failed/invalid shares (for slashing calculation)
    pub failed_shares: u64,

    /// Whether validator is currently active
    pub is_active: bool,

    /// Registration timestamp
    pub registered_at: i64,

    /// Last activity timestamp
    pub last_activity: i64,

    /// Bump seed for PDA
    pub bump: u8,
}

impl ValidatorAccount {
    /// Size: 32 + (4 + 256) + 1 + 8 + 8 + 8 + 1 + 8 + 8 + 1 = 335 bytes
    /// Allocate 400 for future expansion
    pub const SIZE: usize = 400;

    /// Maximum endpoint length
    pub const MAX_ENDPOINT_LENGTH: usize = 256;

    pub fn new(
        authority: Pubkey,
        endpoint: String,
        region: ValidatorRegion,
        stake: u64,
        registered_at: i64,
        bump: u8,
    ) -> Self {
        Self {
            authority,
            endpoint,
            region,
            stake,
            total_shares_provided: 0,
            failed_shares: 0,
            is_active: true,
            registered_at,
            last_activity: registered_at,
            bump,
        }
    }

    /// Calculate success rate (0-100)
    pub fn success_rate(&self) -> u8 {
        let total = self.total_shares_provided;
        if total == 0 {
            return 100;
        }
        let successful = total.saturating_sub(self.failed_shares);
        ((successful * 100) / total) as u8
    }

    /// Record a successful key share provided
    pub fn record_share_provided(&mut self, current_time: i64) {
        self.total_shares_provided += 1;
        self.last_activity = current_time;
    }

    /// Record a failed key share
    pub fn record_failure(&mut self, current_time: i64) {
        self.failed_shares += 1;
        self.last_activity = current_time;
    }

    /// Slash stake and return amount actually slashed
    pub fn slash(&mut self, amount: u64, min_stake: u64) -> u64 {
        let slashed = amount.min(self.stake);
        self.stake = self.stake.saturating_sub(slashed);

        // Deactivate if stake drops below minimum
        if self.stake < min_stake {
            self.is_active = false;
        }

        slashed
    }

    /// Check if validator can provide key shares
    pub fn can_provide_shares(&self, min_stake: u64) -> bool {
        self.is_active && self.stake >= min_stake
    }
}
