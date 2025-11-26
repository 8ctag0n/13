use borsh::{BorshDeserialize, BorshSerialize};
use zyberlink_types::JobStatus;
use solana_program::pubkey::Pubkey;

/// Maximum number of provers for FHE consensus
pub const MAX_FHE_PROVERS: usize = 5;

/// On-chain job account - OPTIMIZED (tamaño fijo)
/// Para jobs ZK: solo usa los campos base
/// Para jobs FHE: usa esta cuenta + FheConsensusData separada
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct JobAccount {
    /// Unique identifier for this job
    pub id: u64,                              // 8 bytes

    /// Public key of the job creator (wallet)
    pub creator: Pubkey,                      // 32 bytes

    /// Public key of the prover (for ZK jobs, or first prover for FHE)
    pub prover: Option<Pubkey>,               // 1 + 32 = 33 bytes

    /// Current status of the job
    pub status: JobStatus,                    // 1 byte

    /// Type of circuit (0-3 = ZK, 4-11 = FHE)
    pub circuit_type: u8,                     // 1 byte

    /// Hash of encrypted witness data
    pub witness_hash: [u8; 32],               // 32 bytes

    /// Size of the original witness (for validation)
    pub witness_size: u32,                    // 4 bytes

    /// Hash of encrypted proof (once submitted)
    pub proof_hash: Option<[u8; 32]>,         // 1 + 32 = 33 bytes

    /// Price offered for completing this job (in lamports)
    pub price_lamports: u64,                  // 8 bytes

    /// Public key of the escrow account holding the payment
    pub escrow_account: Pubkey,               // 32 bytes

    /// Timestamp when the job was created (Unix timestamp)
    pub created_at: i64,                      // 8 bytes

    /// Timestamp when the job will timeout (Unix timestamp)
    pub timeout_at: i64,                      // 8 bytes

    /// Bump seed for PDA derivation
    pub bump: u8,                             // 1 byte

    /// If FHE job, bump of the associated FheConsensusData PDA
    /// None for ZK jobs
    pub fhe_consensus_bump: Option<u8>,       // 1 + 1 = 2 bytes
}
// TOTAL: 8 + 32 + 33 + 1 + 1 + 32 + 4 + 33 + 8 + 32 + 8 + 8 + 1 + 2 = 203 bytes

impl JobAccount {
    /// Fixed size for this account (tamaño constante)
    pub const LEN: usize = 203;

    /// Circuit type IDs for ZK circuits
    pub const CIRCUIT_ZCASH_ORCHARD: u8 = 0;
    pub const CIRCUIT_ZCASH_SAPLING: u8 = 1;
    pub const CIRCUIT_ANONYMOUS_VOTE: u8 = 2;
    pub const CIRCUIT_CREDENTIAL: u8 = 3;

    /// Circuit type IDs for FHE operations
    pub const CIRCUIT_FHE_ADD: u8 = 4;
    pub const CIRCUIT_FHE_MULTIPLY: u8 = 5;
    pub const CIRCUIT_FHE_SUM: u8 = 6;
    pub const CIRCUIT_FHE_THRESHOLD: u8 = 7;
    pub const CIRCUIT_FHE_RANGE_CHECK: u8 = 8;
    pub const CIRCUIT_FHE_AVERAGE: u8 = 9;
    pub const CIRCUIT_FHE_COUNT_IF: u8 = 10;
    pub const CIRCUIT_FHE_HISTOGRAM: u8 = 11;

    /// Check if this is an FHE job (requires consensus)
    pub fn is_fhe(&self) -> bool {
        self.circuit_type >= 4 && self.circuit_type <= 11
    }

    /// Check if this is a ZK job
    pub fn is_zk(&self) -> bool {
        self.circuit_type <= 3
    }

    /// Create a new ZK job account
    #[allow(clippy::too_many_arguments)]
    pub fn new_zk(
        id: u64,
        creator: Pubkey,
        circuit_type: u8,
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
            circuit_type,
            witness_hash,
            witness_size,
            proof_hash: None,
            price_lamports,
            escrow_account,
            created_at,
            timeout_at: created_at + timeout_seconds,
            bump,
            fhe_consensus_bump: None,
        }
    }

    /// Create a new FHE job account (requires associated FheConsensusData)
    #[allow(clippy::too_many_arguments)]
    pub fn new_fhe(
        id: u64,
        creator: Pubkey,
        circuit_type: u8,
        witness_hash: [u8; 32],
        witness_size: u32,
        price_lamports: u64,
        escrow_account: Pubkey,
        created_at: i64,
        timeout_seconds: i64,
        bump: u8,
        fhe_consensus_bump: u8,
    ) -> Self {
        Self {
            id,
            creator,
            prover: None,
            status: JobStatus::Pending,
            circuit_type,
            witness_hash,
            witness_size,
            proof_hash: None,
            price_lamports,
            escrow_account,
            created_at,
            timeout_at: created_at + timeout_seconds,
            bump,
            fhe_consensus_bump: Some(fhe_consensus_bump),
        }
    }

    /// Check if the job has timed out
    pub fn is_timed_out(&self, current_time: i64) -> bool {
        current_time >= self.timeout_at && self.status == JobStatus::Claimed
    }

    /// Claim this job for a prover (ZK jobs only)
    pub fn claim(&mut self, prover: Pubkey, _current_time: i64) {
        self.prover = Some(prover);
        self.status = JobStatus::Claimed;
    }

    /// Mark job as completed with proof hash
    pub fn complete(&mut self, proof_hash: [u8; 32], _current_time: i64) {
        self.status = JobStatus::Completed;
        self.proof_hash = Some(proof_hash);
    }

    /// Mark job as failed
    pub fn fail(&mut self) {
        self.status = JobStatus::Failed;
    }

    /// Cancel this job
    pub fn cancel(&mut self) {
        self.status = JobStatus::Cancelled;
    }
}

/// FHE Consensus Data - cuenta separada para jobs FHE
/// Contiene la configuración de consenso y los resultados de los provers
/// PDA: ["fhe_consensus", job_id.to_le_bytes()]
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct FheConsensusData {
    /// Job ID this consensus data belongs to
    pub job_id: u64,                          // 8 bytes

    /// FHE operation type (matches circuit_type in JobAccount)
    pub operation_type: u8,                   // 1 byte

    /// Packed operation parameters
    pub operation_param1: u16,                // 2 bytes (threshold, operand, count, etc.)
    pub operation_param2: u8,                 // 1 byte (min, flags, etc.)
    pub operation_param3: u8,                 // 1 byte (max, etc.)

    /// Number of provers required for this job
    pub required_provers: u8,                 // 1 byte

    /// Minimum matching results for consensus
    pub consensus_threshold: u8,              // 1 byte

    /// Timeout for submissions (Unix timestamp)
    pub submission_timeout: i64,              // 8 bytes

    /// Provers who have claimed this job (fixed array)
    pub claimed_provers: [Pubkey; MAX_FHE_PROVERS], // 5 * 32 = 160 bytes

    /// Number of provers who have claimed
    pub claimed_count: u8,                    // 1 byte

    /// Result hashes submitted by provers (fixed array)
    pub result_hashes: [[u8; 32]; MAX_FHE_PROVERS], // 5 * 32 = 160 bytes

    /// Which provers have submitted results
    pub result_submitted: [bool; MAX_FHE_PROVERS], // 5 bytes

    /// Number of results submitted
    pub results_count: u8,                    // 1 byte

    /// Consensus hash (once achieved)
    pub consensus_hash: Option<[u8; 32]>,     // 1 + 32 = 33 bytes

    /// Bump seed for PDA derivation
    pub bump: u8,                             // 1 byte
}
// Size with Option<[u8; 32]> = 1 (tag) + 32 (data) = 33 bytes when Some
// TOTAL: 8 + 1 + 2 + 1 + 1 + 1 + 1 + 8 + 160 + 1 + 160 + 5 + 1 + 33 + 1 = 384 bytes
// NOTE: We allocate 384 bytes to account for the maximum size when consensus_hash is Some

impl FheConsensusData {
    /// Fixed size for this account (maximum size when consensus_hash = Some)
    pub const LEN: usize = 384;

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
    pub fn submit_result(&mut self, prover: &Pubkey, result_hash: [u8; 32]) -> Result<(), &'static str> {
        // Find prover index
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

        // Count matching hashes
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_account_size() {
        // Test minimum size (all Options are None)
        let job_min = JobAccount::new_zk(
            1,
            Pubkey::new_unique(),
            JobAccount::CIRCUIT_ZCASH_ORCHARD,
            [0u8; 32],
            2048,
            1_000_000,
            Pubkey::new_unique(),
            1000,
            600,
            255,
        );

        let serialized_min = borsh::to_vec(&job_min).unwrap();
        // Min size: prover=None(1), proof_hash=None(1), fhe_consensus_bump=None(1)
        let min_size = 138;
        assert_eq!(serialized_min.len(), min_size,
            "JobAccount min size mismatch: expected {}, got {}",
            min_size, serialized_min.len());

        // Test maximum size (all Options are Some)
        let mut job_max = job_min.clone();
        job_max.prover = Some(Pubkey::new_unique());
        job_max.proof_hash = Some([1u8; 32]);
        job_max.fhe_consensus_bump = Some(254);

        let serialized_max = borsh::to_vec(&job_max).unwrap();
        assert_eq!(serialized_max.len(), JobAccount::LEN,
            "JobAccount max size mismatch: expected {}, got {}",
            JobAccount::LEN, serialized_max.len());
    }

    #[test]
    fn test_fhe_consensus_data_size() {
        // Test minimum size (when consensus_hash = None)
        let fhe_none = FheConsensusData::new(
            1,
            JobAccount::CIRCUIT_FHE_ADD,
            42, // operand
            0,
            0,
            3,  // required_provers
            2,  // consensus_threshold
            2000,
            255,
        );

        let serialized_none = borsh::to_vec(&fhe_none).unwrap();
        // Minimum size when Option is None: 352 bytes
        let min_size = 352;
        assert_eq!(serialized_none.len(), min_size,
            "FheConsensusData min size mismatch: expected {}, got {}",
            min_size, serialized_none.len());

        // Test maximum size (when consensus_hash = Some)
        let mut fhe_some = fhe_none.clone();
        fhe_some.consensus_hash = Some([42u8; 32]);

        let serialized_some = borsh::to_vec(&fhe_some).unwrap();
        assert_eq!(serialized_some.len(), FheConsensusData::LEN,
            "FheConsensusData max size mismatch: expected {}, got {}",
            FheConsensusData::LEN, serialized_some.len());
    }

    #[test]
    fn test_job_account_zk() {
        let job = JobAccount::new_zk(
            1,
            Pubkey::new_unique(),
            JobAccount::CIRCUIT_ZCASH_ORCHARD,
            [0u8; 32],
            2048,
            1_000_000,
            Pubkey::new_unique(),
            1000,
            600,
            255,
        );

        assert!(job.is_zk());
        assert!(!job.is_fhe());
        assert!(job.fhe_consensus_bump.is_none());
        assert_eq!(job.status, JobStatus::Pending);
    }

    #[test]
    fn test_job_account_fhe() {
        let job = JobAccount::new_fhe(
            1,
            Pubkey::new_unique(),
            JobAccount::CIRCUIT_FHE_THRESHOLD,
            [0u8; 32],
            65536,
            5_000_000,
            Pubkey::new_unique(),
            1000,
            300,
            255,
            254, // fhe_consensus_bump
        );

        assert!(!job.is_zk());
        assert!(job.is_fhe());
        assert_eq!(job.fhe_consensus_bump, Some(254));
    }

    #[test]
    fn test_job_claim() {
        let mut job = JobAccount::new_zk(
            1,
            Pubkey::new_unique(),
            JobAccount::CIRCUIT_ZCASH_ORCHARD,
            [0u8; 32],
            2048,
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
    }

    #[test]
    fn test_job_complete() {
        let mut job = JobAccount::new_zk(
            1,
            Pubkey::new_unique(),
            JobAccount::CIRCUIT_ZCASH_ORCHARD,
            [0u8; 32],
            2048,
            1_000_000,
            Pubkey::new_unique(),
            1000,
            600,
            255,
        );

        job.claim(Pubkey::new_unique(), 1100);
        job.complete([1u8; 32], 1115);

        assert_eq!(job.status, JobStatus::Completed);
        assert_eq!(job.proof_hash, Some([1u8; 32]));
    }

    #[test]
    fn test_job_timeout() {
        let mut job = JobAccount::new_zk(
            1,
            Pubkey::new_unique(),
            JobAccount::CIRCUIT_ZCASH_ORCHARD,
            [0u8; 32],
            2048,
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
    fn test_fhe_add_provers() {
        let mut fhe = FheConsensusData::new(
            1,
            JobAccount::CIRCUIT_FHE_ADD,
            42,
            0,
            0,
            3,
            2,
            2000,
            255,
        );

        let p1 = Pubkey::new_unique();
        let p2 = Pubkey::new_unique();
        let p3 = Pubkey::new_unique();

        assert_eq!(fhe.add_prover(p1).unwrap(), 0);
        assert_eq!(fhe.add_prover(p2).unwrap(), 1);
        assert_eq!(fhe.add_prover(p3).unwrap(), 2);

        assert_eq!(fhe.claimed_count, 3);
        assert!(fhe.is_fully_claimed());

        // Can't add more
        let p4 = Pubkey::new_unique();
        assert!(fhe.add_prover(p4).is_err());

        // Can't add duplicate
        assert!(fhe.add_prover(p1).is_err());
    }

    #[test]
    fn test_fhe_submit_results() {
        let mut fhe = FheConsensusData::new(
            1,
            JobAccount::CIRCUIT_FHE_ADD,
            42,
            0,
            0,
            3,
            2,
            2000,
            255,
        );

        let p1 = Pubkey::new_unique();
        let p2 = Pubkey::new_unique();
        let p3 = Pubkey::new_unique();

        fhe.add_prover(p1).unwrap();
        fhe.add_prover(p2).unwrap();
        fhe.add_prover(p3).unwrap();

        let hash_a = [1u8; 32];
        let hash_b = [2u8; 32];

        fhe.submit_result(&p1, hash_a).unwrap();
        fhe.submit_result(&p2, hash_a).unwrap(); // Same as p1
        fhe.submit_result(&p3, hash_b).unwrap(); // Different

        assert_eq!(fhe.results_count, 3);

        // Can't submit twice
        assert!(fhe.submit_result(&p1, hash_a).is_err());
    }

    #[test]
    fn test_fhe_consensus_achieved() {
        let mut fhe = FheConsensusData::new(
            1,
            JobAccount::CIRCUIT_FHE_ADD,
            42,
            0,
            0,
            3,
            2, // 2 of 3 for consensus
            2000,
            255,
        );

        let p1 = Pubkey::new_unique();
        let p2 = Pubkey::new_unique();
        let p3 = Pubkey::new_unique();

        fhe.add_prover(p1).unwrap();
        fhe.add_prover(p2).unwrap();
        fhe.add_prover(p3).unwrap();

        let hash_consensus = [42u8; 32];
        let hash_different = [99u8; 32];

        fhe.submit_result(&p1, hash_consensus).unwrap();
        assert!(fhe.check_consensus().is_none()); // Only 1 result

        fhe.submit_result(&p2, hash_consensus).unwrap();
        let consensus = fhe.check_consensus();
        assert!(consensus.is_some()); // 2 matching = consensus
        assert_eq!(consensus.unwrap(), hash_consensus);

        fhe.submit_result(&p3, hash_different).unwrap();
        // Consensus already achieved
        assert!(fhe.has_consensus());
        assert_eq!(fhe.consensus_hash.unwrap(), hash_consensus);
    }

    #[test]
    fn test_fhe_no_consensus() {
        let mut fhe = FheConsensusData::new(
            1,
            JobAccount::CIRCUIT_FHE_ADD,
            42,
            0,
            0,
            3,
            2,
            2000,
            255,
        );

        let p1 = Pubkey::new_unique();
        let p2 = Pubkey::new_unique();
        let p3 = Pubkey::new_unique();

        fhe.add_prover(p1).unwrap();
        fhe.add_prover(p2).unwrap();
        fhe.add_prover(p3).unwrap();

        // All different hashes
        fhe.submit_result(&p1, [1u8; 32]).unwrap();
        fhe.submit_result(&p2, [2u8; 32]).unwrap();
        fhe.submit_result(&p3, [3u8; 32]).unwrap();

        assert!(fhe.check_consensus().is_none());
        assert!(!fhe.has_consensus());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let job = JobAccount::new_fhe(
            123,
            Pubkey::new_unique(),
            JobAccount::CIRCUIT_FHE_HISTOGRAM,
            [5u8; 32],
            65536,
            100_000_000,
            Pubkey::new_unique(),
            1704067200,
            3600,
            250,
            251,
        );

        let serialized = borsh::to_vec(&job).unwrap();
        let deserialized: JobAccount = borsh::from_slice(&serialized).unwrap();

        assert_eq!(job.id, deserialized.id);
        assert_eq!(job.circuit_type, deserialized.circuit_type);
        assert_eq!(job.fhe_consensus_bump, deserialized.fhe_consensus_bump);
    }
}
