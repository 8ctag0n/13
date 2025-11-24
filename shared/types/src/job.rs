use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;

use crate::circuit::CircuitType;

/// Status of a proving job in the marketplace
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    Default,
)]
pub enum JobStatus {
    /// Job has been created and is waiting for a prover to claim it
    #[default]
    Pending,
    /// Job has been claimed by a prover and is being processed
    Claimed,
    /// Proof has been submitted and verified successfully
    Completed,
    /// Job failed (timeout, invalid proof, or prover issues)
    Failed,
    /// Job was cancelled by the creator before completion
    Cancelled,
}

/// Represents a proving job in the marketplace
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct Job {
    /// Unique identifier for this job
    pub id: u64,

    /// Public key of the job creator (wallet)
    pub creator: Pubkey,

    /// Public key of the prover (if claimed)
    pub prover: Option<Pubkey>,

    /// Current status of the job
    pub status: JobStatus,

    /// Type of circuit/proof being requested
    pub circuit_type: CircuitType,

    /// Encrypted witness data (compressed via Light Protocol)
    /// Contains the private inputs needed to generate the proof
    pub encrypted_witness: Vec<u8>,

    /// Encrypted proof data (submitted by prover)
    pub encrypted_proof: Option<Vec<u8>>,

    /// Price offered for completing this job (in lamports)
    pub price_lamports: u64,

    /// Public key of the escrow account holding the payment
    pub escrow_account: Pubkey,

    /// Timestamp when the job was created
    pub created_at: i64,

    /// Timestamp when the job was claimed (if claimed)
    pub claimed_at: Option<i64>,

    /// Timestamp when the job was completed (if completed)
    pub completed_at: Option<i64>,

    /// Timestamp when the job will timeout
    pub timeout_at: i64,
}

impl Job {
    /// Check if the job has timed out
    pub fn is_timed_out(&self, current_time: i64) -> bool {
        current_time >= self.timeout_at && self.status == JobStatus::Claimed
    }

    /// Check if the job is in a terminal state (completed, failed, or cancelled)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled
        )
    }

    /// Calculate the duration from creation to completion (if completed)
    pub fn completion_duration_secs(&self) -> Option<i64> {
        match (self.created_at, self.completed_at) {
            (created, Some(completed)) => Some(completed - created),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_status_default() {
        assert_eq!(JobStatus::default(), JobStatus::Pending);
    }

    #[test]
    fn test_job_serialization() {
        let job = Job {
            id: 1,
            creator: Pubkey::new_unique(),
            prover: None,
            status: JobStatus::Pending,
            circuit_type: CircuitType::ZcashOrchard,
            encrypted_witness: vec![1, 2, 3],
            encrypted_proof: None,
            price_lamports: 1_000_000,
            escrow_account: Pubkey::new_unique(),
            created_at: 1000,
            claimed_at: None,
            completed_at: None,
            timeout_at: 2000,
        };

        let serialized = borsh::to_vec(&job).unwrap();
        let deserialized: Job = borsh::from_slice(&serialized).unwrap();

        assert_eq!(job.id, deserialized.id);
        assert_eq!(job.status, deserialized.status);
    }

    #[test]
    fn test_job_timeout() {
        let mut job = Job {
            id: 1,
            creator: Pubkey::new_unique(),
            prover: Some(Pubkey::new_unique()),
            status: JobStatus::Claimed,
            circuit_type: CircuitType::ZcashOrchard,
            encrypted_witness: vec![],
            encrypted_proof: None,
            price_lamports: 1_000_000,
            escrow_account: Pubkey::new_unique(),
            created_at: 1000,
            claimed_at: Some(1100),
            completed_at: None,
            timeout_at: 2000,
        };

        assert!(!job.is_timed_out(1500));
        assert!(job.is_timed_out(2000));
        assert!(job.is_timed_out(2500));

        // Timeout only applies to Claimed status
        job.status = JobStatus::Pending;
        assert!(!job.is_timed_out(2500));
    }

    #[test]
    fn test_job_terminal_state() {
        let mut job = Job {
            id: 1,
            creator: Pubkey::new_unique(),
            prover: None,
            status: JobStatus::Pending,
            circuit_type: CircuitType::ZcashOrchard,
            encrypted_witness: vec![],
            encrypted_proof: None,
            price_lamports: 1_000_000,
            escrow_account: Pubkey::new_unique(),
            created_at: 1000,
            claimed_at: None,
            completed_at: None,
            timeout_at: 2000,
        };

        assert!(!job.is_terminal());

        job.status = JobStatus::Claimed;
        assert!(!job.is_terminal());

        job.status = JobStatus::Completed;
        assert!(job.is_terminal());

        job.status = JobStatus::Failed;
        assert!(job.is_terminal());

        job.status = JobStatus::Cancelled;
        assert!(job.is_terminal());
    }
}
