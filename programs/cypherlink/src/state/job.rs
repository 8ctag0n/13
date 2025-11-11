use borsh::{BorshDeserialize, BorshSerialize};
use cypherlink_types::{CircuitType, JobStatus};
use solana_program::pubkey::Pubkey;

/// On-chain job account
/// Stores job state and data (witness will be compressed via Light Protocol later)
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct JobAccount {
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

    /// Encrypted witness data
    /// For MVP: stored directly (will use Light Protocol compression in Day 4)
    /// Contains the private inputs needed to generate the proof
    pub encrypted_witness_hash: [u8; 32], // Hash of witness (actual data off-chain or compressed)

    /// Whether proof has been submitted
    pub proof_submitted: bool,

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

    /// Bump seed for PDA derivation
    pub bump: u8,
}

impl JobAccount {
    // Base size without dynamic fields
    pub const BASE_LEN: usize = 8       // id
        + 32                             // creator
        + 1 + 32                         // prover (Option<Pubkey>)
        + 1                              // status (enum, 1 byte)
        + 1 + 4                          // circuit_type (enum + potential String len, conservative)
        + 32                             // encrypted_witness_hash
        + 1                              // proof_submitted
        + 8                              // price_lamports
        + 32                             // escrow_account
        + 8                              // created_at
        + 1 + 8                          // claimed_at (Option<i64>)
        + 1 + 8                          // completed_at (Option<i64>)
        + 8                              // timeout_at
        + 1;                             // bump

    // Total with some padding for circuit_type variants
    pub const LEN: usize = Self::BASE_LEN + 64; // Extra space for Custom circuit names

    /// Create a new job account
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: u64,
        creator: Pubkey,
        circuit_type: CircuitType,
        encrypted_witness_hash: [u8; 32],
        price_lamports: u64,
        escrow_account: Pubkey,
        created_at: i64,
        timeout_seconds: i64,
        bump: u8,
    ) -> Self {
        Self {
            id,
            creator,
            prover: None,
            status: JobStatus::Pending,
            circuit_type,
            encrypted_witness_hash,
            proof_submitted: false,
            price_lamports,
            escrow_account,
            created_at,
            claimed_at: None,
            completed_at: None,
            timeout_at: created_at + timeout_seconds,
            bump,
        }
    }

    /// Check if the job has timed out
    pub fn is_timed_out(&self, current_time: i64) -> bool {
        current_time >= self.timeout_at && self.status == JobStatus::Claimed
    }

    /// Claim this job for a prover
    pub fn claim(&mut self, prover: Pubkey, current_time: i64) {
        self.prover = Some(prover);
        self.status = JobStatus::Claimed;
        self.claimed_at = Some(current_time);
    }

    /// Mark job as completed
    pub fn complete(&mut self, current_time: i64) {
        self.status = JobStatus::Completed;
        self.completed_at = Some(current_time);
        self.proof_submitted = true;
    }

    /// Mark job as failed
    pub fn fail(&mut self) {
        self.status = JobStatus::Failed;
    }

    /// Cancel this job
    pub fn cancel(&mut self) {
        self.status = JobStatus::Cancelled;
    }

    /// Get the completion duration in seconds (if completed)
    pub fn completion_duration_secs(&self) -> Option<i64> {
        match (self.claimed_at, self.completed_at) {
            (Some(claimed), Some(completed)) => Some(completed - claimed),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_account_new() {
        let job = JobAccount::new(
            1,
            Pubkey::new_unique(),
            CircuitType::ZcashOrchard,
            [0u8; 32],
            1_000_000,
            Pubkey::new_unique(),
            1000,
            600,
            255,
        );

        assert_eq!(job.id, 1);
        assert_eq!(job.status, JobStatus::Pending);
        assert_eq!(job.timeout_at, 1600);
    }

    #[test]
    fn test_job_claim() {
        let mut job = JobAccount::new(
            1,
            Pubkey::new_unique(),
            CircuitType::ZcashOrchard,
            [0u8; 32],
            1_000_000,
            Pubkey::new_unique(),
            1000,
            600,
            255,
        );

        let prover = Pubkey::new_unique();
        job.claim(prover, 1100);

        assert_eq!(job.prover, Some(prover));
        assert_eq!(job.status, JobStatus::Claimed);
        assert_eq!(job.claimed_at, Some(1100));
    }

    #[test]
    fn test_job_complete() {
        let mut job = JobAccount::new(
            1,
            Pubkey::new_unique(),
            CircuitType::ZcashOrchard,
            [0u8; 32],
            1_000_000,
            Pubkey::new_unique(),
            1000,
            600,
            255,
        );

        job.claim(Pubkey::new_unique(), 1100);
        job.complete(1115);

        assert_eq!(job.status, JobStatus::Completed);
        assert_eq!(job.completed_at, Some(1115));
        assert_eq!(job.completion_duration_secs(), Some(15));
        assert!(job.proof_submitted);
    }

    #[test]
    fn test_job_timeout() {
        let mut job = JobAccount::new(
            1,
            Pubkey::new_unique(),
            CircuitType::ZcashOrchard,
            [0u8; 32],
            1_000_000,
            Pubkey::new_unique(),
            1000,
            600,
            255,
        );

        job.claim(Pubkey::new_unique(), 1100);

        assert!(!job.is_timed_out(1500));
        assert!(job.is_timed_out(1600));
        assert!(job.is_timed_out(2000));
    }

    #[test]
    fn test_job_serialization() {
        let job = JobAccount::new(
            1,
            Pubkey::new_unique(),
            CircuitType::ZcashOrchard,
            [0u8; 32],
            1_000_000,
            Pubkey::new_unique(),
            1000,
            600,
            255,
        );

        let serialized = borsh::to_vec(&job).unwrap();
        let deserialized: JobAccount = borsh::from_slice(&serialized).unwrap();

        assert_eq!(job.id, deserialized.id);
        assert_eq!(job.status, deserialized.status);
        assert!(serialized.len() <= JobAccount::LEN);
    }
}
