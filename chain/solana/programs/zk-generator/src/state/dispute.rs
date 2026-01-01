//! Dispute escrow state for ZK Generator
//!
//! This module defines the escrow accounts used for the dispute system:
//! - DisputeBond: Holds disputor's bond during a dispute
//! - ProverStake: Holds prover's stake that can be slashed

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Seeds for dispute bond PDA
pub const DISPUTE_BOND_SEED: &[u8] = b"dispute_bond";

/// Seeds for prover stake PDA
pub const PROVER_STAKE_SEED: &[u8] = b"prover_stake";

/// Minimum stake required to be a prover (0.5 SOL)
pub const MIN_PROVER_STAKE: u64 = 500_000_000;

/// Dispute bond amount (0.1 SOL)
pub const DISPUTE_BOND_AMOUNT: u64 = 100_000_000;

/// Dispute reward percentage (50% of slash goes to disputor)
pub const DISPUTE_REWARD_BPS: u64 = 5000;

/// Dispute window in seconds (24 hours)
pub const DISPUTE_WINDOW_SECS: i64 = 24 * 60 * 60;

/// Status of a dispute
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[borsh(use_discriminant = true)]
pub enum DisputeStatus {
    /// No active dispute
    None = 0,
    /// Dispute initiated, awaiting resolution
    Pending = 1,
    /// Dispute resolved - proof was valid (disputor loses bond)
    RejectedProofValid = 2,
    /// Dispute resolved - proof was invalid (prover slashed)
    AcceptedProofInvalid = 3,
}

impl Default for DisputeStatus {
    fn default() -> Self {
        DisputeStatus::None
    }
}

/// Dispute Bond Account
///
/// Holds a disputor's bond while a dispute is active.
/// Created when a dispute is initiated, closed when resolved.
///
/// PDA: ["dispute_bond", job_pda, disputor]
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct DisputeBond {
    /// The job being disputed
    pub job: Pubkey,

    /// The disputor who initiated the dispute
    pub disputor: Pubkey,

    /// Amount of bond held (in lamports)
    pub amount: u64,

    /// Timestamp when dispute was initiated
    pub created_at: i64,

    /// Current status
    pub status: DisputeStatus,

    /// PDA bump
    pub bump: u8,
}

impl DisputeBond {
    /// Size: 32 + 32 + 8 + 8 + 1 + 1 = 82 bytes
    pub const SIZE: usize = 82;

    /// Create a new dispute bond
    pub fn new(job: Pubkey, disputor: Pubkey, amount: u64, created_at: i64, bump: u8) -> Self {
        Self {
            job,
            disputor,
            amount,
            created_at,
            status: DisputeStatus::Pending,
            bump,
        }
    }

    /// Check if dispute is still pending
    pub fn is_pending(&self) -> bool {
        self.status == DisputeStatus::Pending
    }

    /// Resolve dispute - proof was valid (disputor loses)
    pub fn reject(&mut self) {
        self.status = DisputeStatus::RejectedProofValid;
    }

    /// Resolve dispute - proof was invalid (prover loses)
    pub fn accept(&mut self) {
        self.status = DisputeStatus::AcceptedProofInvalid;
    }
}

/// Prover Stake Account
///
/// Holds a prover's stake that can be slashed if they submit invalid proofs.
/// Created when a prover registers, persists across jobs.
///
/// PDA: ["prover_stake", prover]
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct ProverStake {
    /// The prover's wallet
    pub prover: Pubkey,

    /// Total staked amount (in lamports)
    pub staked_amount: u64,

    /// Amount currently locked in active jobs
    pub locked_amount: u64,

    /// Total amount slashed historically
    pub total_slashed: u64,

    /// Number of successful jobs completed
    pub jobs_completed: u64,

    /// Number of times slashed
    pub times_slashed: u64,

    /// Timestamp when stake was created
    pub created_at: i64,

    /// PDA bump
    pub bump: u8,
}

impl ProverStake {
    /// Size: 32 + 8 + 8 + 8 + 8 + 8 + 8 + 1 = 81 bytes
    pub const SIZE: usize = 81;

    /// Create a new prover stake
    pub fn new(prover: Pubkey, staked_amount: u64, created_at: i64, bump: u8) -> Self {
        Self {
            prover,
            staked_amount,
            locked_amount: 0,
            total_slashed: 0,
            jobs_completed: 0,
            times_slashed: 0,
            created_at,
            bump,
        }
    }

    /// Available stake (not locked in jobs)
    pub fn available_stake(&self) -> u64 {
        self.staked_amount.saturating_sub(self.locked_amount)
    }

    /// Check if prover has minimum stake
    pub fn has_minimum_stake(&self) -> bool {
        self.available_stake() >= MIN_PROVER_STAKE
    }

    /// Lock stake for a job
    pub fn lock(&mut self, amount: u64) -> bool {
        if self.available_stake() >= amount {
            self.locked_amount = self.locked_amount.saturating_add(amount);
            true
        } else {
            false
        }
    }

    /// Unlock stake after job completion
    pub fn unlock(&mut self, amount: u64) {
        self.locked_amount = self.locked_amount.saturating_sub(amount);
        self.jobs_completed = self.jobs_completed.saturating_add(1);
    }

    /// Slash stake due to invalid proof
    pub fn slash(&mut self, amount: u64) -> u64 {
        let slash = amount.min(self.staked_amount);
        self.staked_amount = self.staked_amount.saturating_sub(slash);
        self.locked_amount = self.locked_amount.saturating_sub(amount);
        self.total_slashed = self.total_slashed.saturating_add(slash);
        self.times_slashed = self.times_slashed.saturating_add(1);
        slash
    }

    /// Add more stake
    pub fn deposit(&mut self, amount: u64) {
        self.staked_amount = self.staked_amount.saturating_add(amount);
    }

    /// Withdraw available stake
    pub fn withdraw(&mut self, amount: u64) -> u64 {
        let withdrawable = amount.min(self.available_stake());
        self.staked_amount = self.staked_amount.saturating_sub(withdrawable);
        withdrawable
    }

    /// Reputation score (0-100)
    pub fn reputation_score(&self) -> u8 {
        if self.jobs_completed == 0 {
            return 50; // Neutral for new provers
        }

        let total_jobs = self.jobs_completed + self.times_slashed;
        let success_rate = (self.jobs_completed * 100) / total_jobs;
        success_rate.min(100) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispute_bond_size() {
        let bond = DisputeBond::new(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            DISPUTE_BOND_AMOUNT,
            1000,
            255,
        );
        let serialized = borsh::to_vec(&bond).unwrap();
        assert_eq!(serialized.len(), DisputeBond::SIZE);
    }

    #[test]
    fn test_prover_stake_size() {
        let stake = ProverStake::new(Pubkey::new_unique(), MIN_PROVER_STAKE, 1000, 255);
        let serialized = borsh::to_vec(&stake).unwrap();
        assert_eq!(serialized.len(), ProverStake::SIZE);
    }

    #[test]
    fn test_prover_stake_operations() {
        let mut stake = ProverStake::new(Pubkey::new_unique(), 1_000_000_000, 1000, 255);

        // Lock some stake
        assert!(stake.lock(500_000_000));
        assert_eq!(stake.available_stake(), 500_000_000);

        // Try to lock more than available
        assert!(!stake.lock(600_000_000));

        // Unlock
        stake.unlock(500_000_000);
        assert_eq!(stake.available_stake(), 1_000_000_000);
        assert_eq!(stake.jobs_completed, 1);

        // Slash
        let slashed = stake.slash(300_000_000);
        assert_eq!(slashed, 300_000_000);
        assert_eq!(stake.staked_amount, 700_000_000);
        assert_eq!(stake.times_slashed, 1);
    }

    #[test]
    fn test_reputation_score() {
        let mut stake = ProverStake::new(Pubkey::new_unique(), 1_000_000_000, 1000, 255);

        // New prover
        assert_eq!(stake.reputation_score(), 50);

        // After some successful jobs
        stake.jobs_completed = 10;
        assert_eq!(stake.reputation_score(), 100);

        // After being slashed once
        stake.times_slashed = 1;
        assert_eq!(stake.reputation_score(), 90); // 10/11 = 90%
    }
}
