//! Common job structure shared by all generators
//!
//! `JobCommon` contains all fields that are shared between ZK and FHE jobs.
//! Each generator composes this struct with their specific fields.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;
use zyberlink_types::JobStatus;

/// Common job fields shared by all generator types (ZK, FHE, etc.)
///
/// This struct is embedded in each generator's job account:
/// - ZkJob { common: JobCommon, circuit_type: u8 }
/// - FheJob { common: JobCommon, circuit_type: u8, fhe_consensus_bump: u8 }
///
/// # Size Calculation
/// ```text
/// id:              8 bytes
/// creator:        32 bytes
/// prover:         33 bytes (Option<Pubkey>: 1 + 32)
/// status:          1 byte
/// witness_hash:   32 bytes
/// witness_size:    4 bytes
/// proof_hash:     33 bytes (Option<[u8; 32]>: 1 + 32)
/// price_lamports:  8 bytes
/// escrow_account: 32 bytes
/// created_at:      8 bytes
/// timeout_at:      8 bytes
/// bump:            1 byte
/// ─────────────────────────
/// TOTAL:         200 bytes
/// ```
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct JobCommon {
    // === Identification ===
    /// Unique identifier for this job
    pub id: u64,

    /// Public key of the job creator (wallet)
    pub creator: Pubkey,

    /// Public key of the prover (first prover for FHE, only prover for ZK)
    pub prover: Option<Pubkey>,

    // === State ===
    /// Current status of the job
    pub status: JobStatus,

    // === Witness Data ===
    /// Hash of encrypted witness data (commitment)
    pub witness_hash: [u8; 32],

    /// Size of the original witness (for validation)
    pub witness_size: u32,

    // === Result ===
    /// Hash of encrypted proof (once submitted)
    pub proof_hash: Option<[u8; 32]>,

    // === Payment ===
    /// Price offered for completing this job (in lamports)
    pub price_lamports: u64,

    /// Public key of the escrow account holding the payment
    pub escrow_account: Pubkey,

    // === Timing ===
    /// Timestamp when the job was created (Unix timestamp)
    pub created_at: i64,

    /// Timestamp when the job will timeout (Unix timestamp)
    pub timeout_at: i64,

    // === PDA ===
    /// Bump seed for PDA derivation
    pub bump: u8,
}

impl JobCommon {
    /// Fixed size for JobCommon (maximum size when all Options are Some)
    /// This matches the original JobAccount size minus type-specific fields
    pub const SIZE: usize = 200;

    /// Minimum size when all Options are None
    /// id(8) + creator(32) + prover_tag(1) + status(1) + witness_hash(32) +
    /// witness_size(4) + proof_hash_tag(1) + price(8) + escrow(32) +
    /// created_at(8) + timeout_at(8) + bump(1) = 136 bytes
    pub const MIN_SIZE: usize = 136;

    /// Create a new pending job
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: u64,
        creator: Pubkey,
        witness_hash: [u8; 32],
        witness_size: u32,
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
            witness_hash,
            witness_size,
            proof_hash: None,
            price_lamports,
            escrow_account,
            created_at,
            timeout_at: created_at + timeout_seconds,
            bump,
        }
    }

    // === Status Checks ===

    /// Check if the job has timed out (only applies to Claimed jobs)
    pub fn is_expired(&self, current_time: i64) -> bool {
        current_time >= self.timeout_at && self.status == JobStatus::Claimed
    }

    /// Check if the job can be cancelled (only Pending jobs)
    pub fn can_cancel(&self) -> bool {
        self.status == JobStatus::Pending
    }

    /// Check if the job can be claimed (only Pending jobs)
    pub fn can_claim(&self) -> bool {
        self.status == JobStatus::Pending
    }

    /// Check if the job is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled
        )
    }

    // === State Transitions ===

    /// Claim this job for a prover
    ///
    /// # Panics
    /// Panics if job is not in Pending status
    pub fn claim(&mut self, prover: Pubkey) {
        debug_assert!(self.can_claim(), "Job must be Pending to claim");
        self.prover = Some(prover);
        self.status = JobStatus::Claimed;
    }

    /// Mark job as completed with proof hash
    ///
    /// # Panics
    /// Panics if job is not in Claimed status
    pub fn complete(&mut self, proof_hash: [u8; 32]) {
        debug_assert!(
            self.status == JobStatus::Claimed,
            "Job must be Claimed to complete"
        );
        self.status = JobStatus::Completed;
        self.proof_hash = Some(proof_hash);
    }

    /// Mark job as failed
    pub fn fail(&mut self) {
        self.status = JobStatus::Failed;
    }

    /// Cancel this job
    ///
    /// # Panics
    /// Panics if job is not in Pending status
    pub fn cancel(&mut self) {
        debug_assert!(self.can_cancel(), "Job must be Pending to cancel");
        self.status = JobStatus::Cancelled;
    }

    // === Helpers ===

    /// Time remaining until timeout (in seconds)
    /// Returns 0 if already timed out
    pub fn time_remaining(&self, current_time: i64) -> i64 {
        (self.timeout_at - current_time).max(0)
    }

    /// Duration from creation to now (or timeout)
    pub fn elapsed(&self, current_time: i64) -> i64 {
        current_time - self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_job() -> JobCommon {
        JobCommon::new(
            1,
            Pubkey::new_unique(),
            [0u8; 32],
            2048,
            1_000_000,
            Pubkey::new_unique(),
            1000,
            600, // 600 second timeout
            255,
        )
    }

    #[test]
    fn test_job_common_size() {
        let job = create_test_job();
        let serialized = borsh::to_vec(&job).unwrap();

        // Minimum size (all Options None)
        assert_eq!(
            serialized.len(),
            JobCommon::MIN_SIZE,
            "JobCommon min size mismatch"
        );

        // Maximum size (all Options Some)
        let mut job_max = job.clone();
        job_max.prover = Some(Pubkey::new_unique());
        job_max.proof_hash = Some([1u8; 32]);

        let serialized_max = borsh::to_vec(&job_max).unwrap();
        assert_eq!(
            serialized_max.len(),
            JobCommon::SIZE,
            "JobCommon max size mismatch"
        );
    }

    #[test]
    fn test_new_job() {
        let job = create_test_job();

        assert_eq!(job.id, 1);
        assert_eq!(job.status, JobStatus::Pending);
        assert!(job.prover.is_none());
        assert!(job.proof_hash.is_none());
        assert_eq!(job.timeout_at, 1600); // 1000 + 600
    }

    #[test]
    fn test_can_claim() {
        let mut job = create_test_job();

        assert!(job.can_claim());

        job.status = JobStatus::Claimed;
        assert!(!job.can_claim());

        job.status = JobStatus::Completed;
        assert!(!job.can_claim());
    }

    #[test]
    fn test_can_cancel() {
        let mut job = create_test_job();

        assert!(job.can_cancel());

        job.status = JobStatus::Claimed;
        assert!(!job.can_cancel());

        job.status = JobStatus::Completed;
        assert!(!job.can_cancel());
    }

    #[test]
    fn test_claim() {
        let mut job = create_test_job();
        let prover = Pubkey::new_unique();

        job.claim(prover);

        assert_eq!(job.prover, Some(prover));
        assert_eq!(job.status, JobStatus::Claimed);
    }

    #[test]
    fn test_complete() {
        let mut job = create_test_job();
        let prover = Pubkey::new_unique();
        let proof_hash = [42u8; 32];

        job.claim(prover);
        job.complete(proof_hash);

        assert_eq!(job.status, JobStatus::Completed);
        assert_eq!(job.proof_hash, Some(proof_hash));
    }

    #[test]
    fn test_cancel() {
        let mut job = create_test_job();

        job.cancel();

        assert_eq!(job.status, JobStatus::Cancelled);
    }

    #[test]
    fn test_is_expired() {
        let mut job = create_test_job();
        let prover = Pubkey::new_unique();

        // Pending jobs don't expire
        assert!(!job.is_expired(2000));

        // Claim the job
        job.claim(prover);

        // Not expired before timeout
        assert!(!job.is_expired(1500));

        // Expired at timeout
        assert!(job.is_expired(1600));

        // Expired after timeout
        assert!(job.is_expired(2000));

        // Completed jobs don't expire
        job.complete([0u8; 32]);
        assert!(!job.is_expired(2000));
    }

    #[test]
    fn test_is_terminal() {
        let mut job = create_test_job();

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

    #[test]
    fn test_time_remaining() {
        let job = create_test_job();

        assert_eq!(job.time_remaining(1000), 600);
        assert_eq!(job.time_remaining(1300), 300);
        assert_eq!(job.time_remaining(1600), 0);
        assert_eq!(job.time_remaining(2000), 0); // Never negative
    }

    #[test]
    fn test_elapsed() {
        let job = create_test_job();

        assert_eq!(job.elapsed(1000), 0);
        assert_eq!(job.elapsed(1300), 300);
        assert_eq!(job.elapsed(1600), 600);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut job = create_test_job();
        job.prover = Some(Pubkey::new_unique());
        job.proof_hash = Some([5u8; 32]);
        job.status = JobStatus::Completed;

        let serialized = borsh::to_vec(&job).unwrap();
        let deserialized: JobCommon = borsh::from_slice(&serialized).unwrap();

        assert_eq!(job.id, deserialized.id);
        assert_eq!(job.creator, deserialized.creator);
        assert_eq!(job.prover, deserialized.prover);
        assert_eq!(job.status, deserialized.status);
        assert_eq!(job.witness_hash, deserialized.witness_hash);
        assert_eq!(job.proof_hash, deserialized.proof_hash);
        assert_eq!(job.price_lamports, deserialized.price_lamports);
        assert_eq!(job.bump, deserialized.bump);
    }
}
