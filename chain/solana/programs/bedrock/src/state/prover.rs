//! Prover account state

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Seeds for prover PDA
pub const PROVER_SEED: &[u8] = b"prover";

/// Prover account - registered prover with stake
///
/// PDA: ["prover", prover_wallet]
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct ProverAccount {
    /// Prover's wallet address
    pub authority: Pubkey,

    /// Staked amount in lamports
    pub stake_lamports: u64,

    /// Total jobs completed successfully
    pub jobs_completed: u64,

    /// Total jobs failed (timeout, invalid)
    pub jobs_failed: u64,

    /// Total earnings in lamports
    pub total_earnings: u64,

    /// Is this prover currently active
    pub is_active: bool,

    /// Registration timestamp
    pub registered_at: i64,

    /// Last activity timestamp
    pub last_active_at: i64,

    /// Bump seed for PDA
    pub bump: u8,
}

impl ProverAccount {
    /// Size: 32 + 8 + 8 + 8 + 8 + 1 + 8 + 8 + 1 = 82 bytes
    /// But we allocate 114 for future expansion
    pub const SIZE: usize = 114;

    /// Minimum stake required to register (0.1 SOL)
    pub const MIN_STAKE: u64 = 100_000_000;

    pub fn new(authority: Pubkey, stake_lamports: u64, registered_at: i64, bump: u8) -> Self {
        Self {
            authority,
            stake_lamports,
            jobs_completed: 0,
            jobs_failed: 0,
            total_earnings: 0,
            is_active: true,
            registered_at,
            last_active_at: registered_at,
            bump,
        }
    }

    /// Calculate success rate (0-100)
    pub fn success_rate(&self) -> u8 {
        let total = self.jobs_completed + self.jobs_failed;
        if total == 0 {
            return 100;
        }
        ((self.jobs_completed * 100) / total) as u8
    }

    /// Record a completed job
    pub fn record_completion(&mut self, earnings: u64, current_time: i64) {
        self.jobs_completed += 1;
        self.total_earnings += earnings;
        self.last_active_at = current_time;
    }

    /// Record a failed job
    pub fn record_failure(&mut self, current_time: i64) {
        self.jobs_failed += 1;
        self.last_active_at = current_time;
    }

    /// Slash stake
    pub fn slash(&mut self, amount: u64) -> u64 {
        let slashed = amount.min(self.stake_lamports);
        self.stake_lamports -= slashed;

        // Deactivate if stake drops below minimum
        if self.stake_lamports < Self::MIN_STAKE {
            self.is_active = false;
        }

        slashed
    }

    /// Check if prover can take jobs
    pub fn can_take_jobs(&self) -> bool {
        self.is_active && self.stake_lamports >= Self::MIN_STAKE
    }
}
