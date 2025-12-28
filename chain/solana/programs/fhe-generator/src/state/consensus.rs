//! FHE Consensus Data - multi-prover consensus state
//!
//! This is copied from the original monolith with minimal changes.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Seeds for FHE consensus PDA
pub const FHE_CONSENSUS_SEED: &[u8] = b"fhe_consensus";

/// Maximum number of provers for FHE consensus
pub const MAX_FHE_PROVERS: usize = 5;

/// FHE Consensus Data - separate account for multi-prover jobs
///
/// PDA: ["fhe_consensus", job_id.to_le_bytes()]
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct FheConsensusData {
    /// Job ID this consensus data belongs to
    pub job_id: u64,

    /// FHE operation type (matches circuit_type in FheJob)
    pub operation_type: u8,

    /// Packed operation parameters
    pub operation_param1: u16,
    pub operation_param2: u8,
    pub operation_param3: u8,

    /// Number of provers required for this job
    pub required_provers: u8,

    /// Minimum matching results for consensus
    pub consensus_threshold: u8,

    /// Timeout for submissions (Unix timestamp)
    pub submission_timeout: i64,

    /// Provers who have claimed this job (fixed array)
    pub claimed_provers: [Pubkey; MAX_FHE_PROVERS],

    /// Number of provers who have claimed
    pub claimed_count: u8,

    /// Result hashes submitted by provers (fixed array)
    pub result_hashes: [[u8; 32]; MAX_FHE_PROVERS],

    /// Which provers have submitted results
    pub result_submitted: [bool; MAX_FHE_PROVERS],

    /// Number of results submitted
    pub results_count: u8,

    /// Consensus hash (once achieved)
    pub consensus_hash: Option<[u8; 32]>,

    /// Bump seed for PDA derivation
    pub bump: u8,
}

impl FheConsensusData {
    /// Fixed size: 384 bytes (same as original monolith)
    pub const SIZE: usize = 384;

    /// Create new FHE consensus data
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        job_id: u64,
        operation_type: u8,
        operation_param1: u16,
        operation_param2: u8,
        operation_param3: u8,
        required_provers: u8,
        consensus_threshold: u8,
        submission_timeout: i64,
        bump: u8,
    ) -> Self {
        Self {
            job_id,
            operation_type,
            operation_param1,
            operation_param2,
            operation_param3,
            required_provers,
            consensus_threshold,
            submission_timeout,
            claimed_provers: [Pubkey::default(); MAX_FHE_PROVERS],
            claimed_count: 0,
            result_hashes: [[0u8; 32]; MAX_FHE_PROVERS],
            result_submitted: [false; MAX_FHE_PROVERS],
            results_count: 0,
            consensus_hash: None,
            bump,
        }
    }

    /// Add a prover to the claimed list
    pub fn add_prover(&mut self, prover: Pubkey) -> Result<usize, &'static str> {
        if self.claimed_count >= self.required_provers {
            return Err("All prover slots filled");
        }
        if (self.claimed_count as usize) >= MAX_FHE_PROVERS {
            return Err("Maximum provers reached");
        }

        // Check if prover already claimed
        for i in 0..self.claimed_count as usize {
            if self.claimed_provers[i] == prover {
                return Err("Prover already claimed this job");
            }
        }

        let idx = self.claimed_count as usize;
        self.claimed_provers[idx] = prover;
        self.claimed_count += 1;
        Ok(idx)
    }

    /// Submit a result for a prover
    pub fn submit_result(
        &mut self,
        prover: &Pubkey,
        result_hash: [u8; 32],
    ) -> Result<(), &'static str> {
        let mut prover_idx = None;
        for i in 0..self.claimed_count as usize {
            if &self.claimed_provers[i] == prover {
                prover_idx = Some(i);
                break;
            }
        }

        let idx = prover_idx.ok_or("Prover has not claimed this job")?;

        if self.result_submitted[idx] {
            return Err("Prover already submitted result");
        }

        self.result_hashes[idx] = result_hash;
        self.result_submitted[idx] = true;
        self.results_count += 1;
        Ok(())
    }

    /// Check if consensus has been reached and return the consensus hash
    pub fn check_consensus(&mut self) -> Option<[u8; 32]> {
        if self.results_count < self.consensus_threshold {
            return None;
        }

        let mut best_hash: Option<[u8; 32]> = None;
        let mut best_count: u8 = 0;

        for i in 0..self.results_count as usize {
            if !self.result_submitted[i] {
                continue;
            }

            let hash = self.result_hashes[i];
            let mut count: u8 = 0;

            for j in 0..self.results_count as usize {
                if self.result_submitted[j] && self.result_hashes[j] == hash {
                    count += 1;
                }
            }

            if count > best_count {
                best_count = count;
                best_hash = Some(hash);
            }
        }

        if best_count >= self.consensus_threshold {
            self.consensus_hash = best_hash;
            best_hash
        } else {
            None
        }
    }

    /// Check if all required provers have claimed
    pub fn is_fully_claimed(&self) -> bool {
        self.claimed_count >= self.required_provers
    }

    /// Check if consensus has been achieved
    pub fn has_consensus(&self) -> bool {
        self.consensus_hash.is_some()
    }

    /// Get provers who matched the consensus hash
    pub fn get_winning_provers(&self) -> Vec<Pubkey> {
        let Some(consensus) = self.consensus_hash else {
            return vec![];
        };

        let mut winners = Vec::new();
        for i in 0..self.claimed_count as usize {
            if self.result_submitted[i] && self.result_hashes[i] == consensus {
                winners.push(self.claimed_provers[i]);
            }
        }
        winners
    }

    /// Get provers who didn't match the consensus hash
    pub fn get_losing_provers(&self) -> Vec<Pubkey> {
        let Some(consensus) = self.consensus_hash else {
            return vec![];
        };

        let mut losers = Vec::new();
        for i in 0..self.claimed_count as usize {
            if self.result_submitted[i] && self.result_hashes[i] != consensus {
                losers.push(self.claimed_provers[i]);
            }
        }
        losers
    }
}
