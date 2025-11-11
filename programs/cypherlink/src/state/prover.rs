use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// On-chain prover account
/// Stores prover state and statistics
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct ProverAccount {
    /// Authority public key (controls this prover account)
    pub authority: Pubkey,

    /// Amount of SOL staked (for slashing if misbehaves)
    pub stake_amount: u64,

    /// Reputation score (0-1000, starts at 1000)
    pub reputation_score: u32,

    /// Total number of jobs successfully completed
    pub total_jobs_completed: u64,

    /// Total number of jobs failed (timeout or invalid proof)
    pub total_jobs_failed: u64,

    /// Average completion time in seconds (rolling average)
    pub avg_completion_time_secs: u32,

    /// Whether this prover is currently active
    pub is_active: bool,

    /// Timestamp when this prover registered
    pub registration_timestamp: i64,

    /// Total earnings in lamports (cumulative)
    pub total_earnings_lamports: u64,

    /// Bump seed for PDA derivation
    pub bump: u8,
}

impl ProverAccount {
    pub const LEN: usize = 32   // authority
        + 8                      // stake_amount
        + 4                      // reputation_score
        + 8                      // total_jobs_completed
        + 8                      // total_jobs_failed
        + 4                      // avg_completion_time_secs
        + 1                      // is_active
        + 8                      // registration_timestamp
        + 8                      // total_earnings_lamports
        + 1;                     // bump

    /// Create a new prover account
    pub fn new(
        authority: Pubkey,
        stake_amount: u64,
        registration_timestamp: i64,
        bump: u8,
    ) -> Self {
        Self {
            authority,
            stake_amount,
            reputation_score: 1000, // Start with perfect reputation
            total_jobs_completed: 0,
            total_jobs_failed: 0,
            avg_completion_time_secs: 0,
            is_active: true,
            registration_timestamp,
            total_earnings_lamports: 0,
            bump,
        }
    }

    /// Check if prover can claim jobs
    pub fn can_claim_jobs(&self, min_reputation: u32, min_stake: u64) -> bool {
        self.is_active
            && self.reputation_score >= min_reputation
            && self.stake_amount >= min_stake
    }

    /// Update stats after successful job completion
    pub fn on_job_completed(&mut self, completion_time_secs: u32, payout: u64) {
        self.total_jobs_completed += 1;
        self.total_earnings_lamports += payout;

        // Update rolling average completion time
        let total_jobs = self.total_jobs_completed + self.total_jobs_failed;
        if total_jobs == 1 {
            self.avg_completion_time_secs = completion_time_secs;
        } else {
            // Weighted average: 80% old, 20% new
            self.avg_completion_time_secs = ((self.avg_completion_time_secs as u64 * 4
                + completion_time_secs as u64)
                / 5) as u32;
        }

        // Slowly increase reputation on success (max 1000)
        if self.reputation_score < 1000 {
            self.reputation_score = (self.reputation_score + 10).min(1000);
        }
    }

    /// Update stats after job failure
    pub fn on_job_failed(&mut self) {
        self.total_jobs_failed += 1;

        // Reduce reputation
        if self.reputation_score > 50 {
            self.reputation_score -= 50;
        } else {
            self.reputation_score = 0;
        }
    }

    /// Apply slashing (reduce stake)
    pub fn slash(&mut self, amount: u64, min_stake: u64) {
        if amount >= self.stake_amount {
            self.stake_amount = 0;
            self.is_active = false; // Deactivate if completely slashed
        } else {
            self.stake_amount -= amount;
            // Deactivate if below minimum
            if self.stake_amount < min_stake {
                self.is_active = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prover_account_len() {
        let prover = ProverAccount::new(Pubkey::new_unique(), 10_000_000_000, 1000, 255);
        let serialized = borsh::to_vec(&prover).unwrap();
        assert_eq!(serialized.len(), ProverAccount::LEN);
    }

    #[test]
    fn test_prover_can_claim() {
        let prover = ProverAccount::new(Pubkey::new_unique(), 10_000_000_000, 1000, 255);
        assert!(prover.can_claim_jobs(500, 5_000_000_000));

        let mut low_rep = prover.clone();
        low_rep.reputation_score = 400;
        assert!(!low_rep.can_claim_jobs(500, 5_000_000_000));

        let mut low_stake = prover.clone();
        low_stake.stake_amount = 1_000_000_000;
        assert!(!low_stake.can_claim_jobs(500, 5_000_000_000));
    }

    #[test]
    fn test_prover_job_completion() {
        let mut prover = ProverAccount::new(Pubkey::new_unique(), 10_000_000_000, 1000, 255);

        prover.on_job_completed(15, 1_000_000);
        assert_eq!(prover.total_jobs_completed, 1);
        assert_eq!(prover.avg_completion_time_secs, 15);
        assert_eq!(prover.total_earnings_lamports, 1_000_000);
    }

    #[test]
    fn test_prover_slashing() {
        let mut prover = ProverAccount::new(Pubkey::new_unique(), 10_000_000_000, 1000, 255);

        prover.slash(1_000_000_000, 5_000_000_000); // Slash 1 SOL
        assert_eq!(prover.stake_amount, 9_000_000_000);
        assert!(prover.is_active);

        prover.slash(10_000_000_000, 5_000_000_000); // Slash more than available
        assert_eq!(prover.stake_amount, 0);
        assert!(!prover.is_active);
    }
}
