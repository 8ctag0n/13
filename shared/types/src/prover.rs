use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;

/// Represents a prover node registered in the marketplace
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct Prover {
    /// Authority public key (controls this prover account)
    pub authority: Pubkey,

    /// Amount of SOL staked (for slashing if misbehaves)
    pub stake_amount: u64,

    /// Reputation score (0-1000, starts at 1000)
    /// Used by clients to select reliable provers
    pub reputation_score: u32,

    /// Total number of jobs successfully completed
    pub total_jobs_completed: u64,

    /// Total number of jobs failed (timeout or invalid proof)
    pub total_jobs_failed: u64,

    /// Average completion time in seconds (rolling average)
    pub avg_completion_time_secs: u32,

    /// Whether this prover is currently active and accepting jobs
    pub is_active: bool,

    /// Timestamp when this prover registered
    pub registration_timestamp: i64,

    /// Total earnings in lamports (cumulative)
    pub total_earnings_lamports: u64,
}

impl Prover {
    /// Calculate success rate as a percentage (0-100)
    pub fn success_rate_percent(&self) -> f64 {
        let total = self.total_jobs_completed + self.total_jobs_failed;
        if total == 0 {
            return 100.0; // New prover, optimistic
        }
        (self.total_jobs_completed as f64 / total as f64) * 100.0
    }

    /// Check if prover has sufficient reputation to claim jobs
    pub fn has_sufficient_reputation(&self, min_score: u32) -> bool {
        self.reputation_score >= min_score
    }

    /// Check if prover has sufficient stake
    pub fn has_sufficient_stake(&self, min_stake: u64) -> bool {
        self.stake_amount >= min_stake
    }

    /// Check if prover is eligible to claim jobs
    pub fn can_claim_jobs(&self, min_reputation: u32, min_stake: u64) -> bool {
        self.is_active
            && self.has_sufficient_reputation(min_reputation)
            && self.has_sufficient_stake(min_stake)
    }

    /// Update statistics after job completion
    pub fn update_on_job_completed(&mut self, completion_time_secs: u32) {
        self.total_jobs_completed += 1;

        // Update rolling average (weighted toward recent jobs)
        let total_jobs = self.total_jobs_completed + self.total_jobs_failed;
        if total_jobs == 1 {
            self.avg_completion_time_secs = completion_time_secs;
        } else {
            // Weighted average: 80% old average, 20% new sample
            self.avg_completion_time_secs =
                ((self.avg_completion_time_secs as u64 * 4 + completion_time_secs as u64) / 5) as u32;
        }
    }

    /// Update statistics after job failure
    pub fn update_on_job_failed(&mut self) {
        self.total_jobs_failed += 1;

        // Reduce reputation on failure
        if self.reputation_score > 50 {
            self.reputation_score -= 50;
        } else {
            self.reputation_score = 0;
        }
    }

    /// Apply slashing (reduce stake)
    pub fn slash(&mut self, amount: u64) {
        if amount >= self.stake_amount {
            self.stake_amount = 0;
            self.is_active = false; // Deactivate if completely slashed
        } else {
            self.stake_amount -= amount;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_prover() -> Prover {
        Prover {
            authority: Pubkey::new_unique(),
            stake_amount: 10_000_000_000, // 10 SOL
            reputation_score: 1000,
            total_jobs_completed: 0,
            total_jobs_failed: 0,
            avg_completion_time_secs: 0,
            is_active: true,
            registration_timestamp: 1000,
            total_earnings_lamports: 0,
        }
    }

    #[test]
    fn test_prover_success_rate() {
        let mut prover = create_test_prover();

        // New prover
        assert_eq!(prover.success_rate_percent(), 100.0);

        // After some successes
        prover.total_jobs_completed = 8;
        prover.total_jobs_failed = 2;
        assert_eq!(prover.success_rate_percent(), 80.0);
    }

    #[test]
    fn test_prover_eligibility() {
        let mut prover = create_test_prover();

        assert!(prover.can_claim_jobs(500, 5_000_000_000));

        // Low reputation
        prover.reputation_score = 400;
        assert!(!prover.can_claim_jobs(500, 5_000_000_000));
        prover.reputation_score = 1000;

        // Insufficient stake
        prover.stake_amount = 1_000_000_000;
        assert!(!prover.can_claim_jobs(500, 5_000_000_000));
        prover.stake_amount = 10_000_000_000;

        // Inactive
        prover.is_active = false;
        assert!(!prover.can_claim_jobs(500, 5_000_000_000));
    }

    #[test]
    fn test_prover_job_completion() {
        let mut prover = create_test_prover();

        prover.update_on_job_completed(15);
        assert_eq!(prover.total_jobs_completed, 1);
        assert_eq!(prover.avg_completion_time_secs, 15);

        prover.update_on_job_completed(20);
        assert_eq!(prover.total_jobs_completed, 2);
        // (15 * 4 + 20) / 5 = 16
        assert_eq!(prover.avg_completion_time_secs, 16);
    }

    #[test]
    fn test_prover_job_failure() {
        let mut prover = create_test_prover();

        prover.update_on_job_failed();
        assert_eq!(prover.total_jobs_failed, 1);
        assert_eq!(prover.reputation_score, 950); // 1000 - 50
    }

    #[test]
    fn test_prover_slashing() {
        let mut prover = create_test_prover();

        // Partial slash
        prover.slash(1_000_000_000); // 1 SOL
        assert_eq!(prover.stake_amount, 9_000_000_000);
        assert!(prover.is_active);

        // Complete slash
        prover.slash(10_000_000_000);
        assert_eq!(prover.stake_amount, 0);
        assert!(!prover.is_active);
    }

    #[test]
    fn test_prover_serialization() {
        let prover = create_test_prover();
        let serialized = borsh::to_vec(&prover).unwrap();
        let deserialized: Prover = borsh::from_slice(&serialized).unwrap();

        assert_eq!(prover.authority, deserialized.authority);
        assert_eq!(prover.reputation_score, deserialized.reputation_score);
    }
}
