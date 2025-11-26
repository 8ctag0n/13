use borsh::{BorshDeserialize, BorshSerialize};
use zyberlink_types::{CircuitType, FheConsensusConfig, FheJobResult, JobStatus};
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

    /// Light Protocol commitment for encrypted witness
    /// Actual witness data is stored in Light Protocol compressed state
    /// Provers retrieve it from Light indexer using this commitment
    pub witness_commitment: [u8; 32],

    /// Size of the original witness (for validation)
    pub witness_size: u32,

    /// Light Protocol commitment for encrypted proof (once submitted)
    pub proof_commitment: Option<[u8; 32]>,

    /// Size of the proof (for validation)
    pub proof_size: Option<u32>,

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

    // ========== FHE-SPECIFIC FIELDS ==========
    /// FHE consensus configuration (only for FHE jobs)
    /// For ZK jobs, this is None
    pub fhe_config: Option<FheConsensusConfig>,

    /// Provers who have claimed this FHE job
    /// Empty for ZK jobs
    pub claimed_provers: Vec<Pubkey>,

    /// Results submitted by provers (only for FHE jobs)
    /// Empty for ZK jobs
    pub fhe_results: Vec<FheJobResult>,

    /// Consensus hash (once consensus achieved)
    /// None if no consensus yet or if ZK job
    pub fhe_consensus_hash: Option<[u8; 32]>,
}

impl JobAccount {
    // Base size without dynamic fields
    pub const BASE_LEN: usize = 8       // id
        + 32                             // creator
        + 1 + 32                         // prover (Option<Pubkey>)
        + 1                              // status (enum, 1 byte)
        + 1 + 4                          // circuit_type (enum + potential String len, conservative)
        + 32                             // witness_commitment
        + 4                              // witness_size
        + 1 + 32                         // proof_commitment (Option<[u8; 32]>)
        + 1 + 4                          // proof_size (Option<u32>)
        + 8                              // price_lamports
        + 32                             // escrow_account
        + 8                              // created_at
        + 1 + 8                          // claimed_at (Option<i64>)
        + 1 + 8                          // completed_at (Option<i64>)
        + 8                              // timeout_at
        + 1                              // bump
        + 1 + 20                         // fhe_config (Option<FheConsensusConfig>)
        + 4                              // claimed_provers (Vec len)
        + 4                              // fhe_results (Vec len)
        + 1 + 32; // fhe_consensus_hash (Option<[u8; 32]>)

    // Total with padding for dynamic fields
    // Allow for up to 5 provers: 5 * (32 + 73) = 525 bytes
    pub const LEN: usize = Self::BASE_LEN + 64 + 525;

    /// Create a new job account with Light Protocol compression
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: u64,
        creator: Pubkey,
        circuit_type: CircuitType,
        witness_commitment: [u8; 32],
        witness_size: u32,
        price_lamports: u64,
        escrow_account: Pubkey,
        created_at: i64,
        timeout_seconds: i64,
        bump: u8,
        fhe_config: Option<FheConsensusConfig>,
    ) -> Self {
        Self {
            id,
            creator,
            prover: None,
            status: JobStatus::Pending,
            circuit_type,
            witness_commitment,
            witness_size,
            proof_commitment: None,
            proof_size: None,
            price_lamports,
            escrow_account,
            created_at,
            claimed_at: None,
            completed_at: None,
            timeout_at: created_at + timeout_seconds,
            bump,
            fhe_config,
            claimed_provers: Vec::new(),
            fhe_results: Vec::new(),
            fhe_consensus_hash: None,
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

    /// Mark job as completed with proof commitment
    pub fn complete(&mut self, proof_commitment: [u8; 32], proof_size: u32, current_time: i64) {
        self.status = JobStatus::Completed;
        self.completed_at = Some(current_time);
        self.proof_commitment = Some(proof_commitment);
        self.proof_size = Some(proof_size);
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
            2048, // witness_size
            1_000_000,
            Pubkey::new_unique(),
            1000,
            600,
            255,
            None, // No FHE config for ZK jobs
        );

        assert_eq!(job.id, 1);
        assert_eq!(job.status, JobStatus::Pending);
        assert_eq!(job.timeout_at, 1600);
        assert_eq!(job.witness_size, 2048);
        assert!(job.proof_commitment.is_none());
    }

    #[test]
    fn test_job_claim() {
        let mut job = JobAccount::new(
            1,
            Pubkey::new_unique(),
            CircuitType::ZcashOrchard,
            [0u8; 32],
            2048,
            1_000_000,
            Pubkey::new_unique(),
            1000,
            600,
            255,
            None, // No FHE config for ZK jobs
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
            2048,
            1_000_000,
            Pubkey::new_unique(),
            1000,
            600,
            255,
            None, // No FHE config for ZK jobs
        );

        job.claim(Pubkey::new_unique(), 1100);

        let proof_commitment = [1u8; 32];
        job.complete(proof_commitment, 1536, 1115);

        assert_eq!(job.status, JobStatus::Completed);
        assert_eq!(job.completed_at, Some(1115));
        assert_eq!(job.completion_duration_secs(), Some(15));
        assert_eq!(job.proof_commitment, Some(proof_commitment));
        assert_eq!(job.proof_size, Some(1536));
    }

    #[test]
    fn test_job_timeout() {
        let mut job = JobAccount::new(
            1,
            Pubkey::new_unique(),
            CircuitType::ZcashOrchard,
            [0u8; 32],
            2048,
            1_000_000,
            Pubkey::new_unique(),
            1000,
            600,
            255,
            None, // No FHE config for ZK jobs
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
            2048,
            1_000_000,
            Pubkey::new_unique(),
            1000,
            600,
            255,
            None, // No FHE config for ZK jobs
        );

        let serialized = borsh::to_vec(&job).unwrap();
        let deserialized: JobAccount = borsh::from_slice(&serialized).unwrap();

        assert_eq!(job.id, deserialized.id);
        assert_eq!(job.status, deserialized.status);
        assert!(serialized.len() <= JobAccount::LEN);
    }
}
