//! Unified Job Types - Cross-program job abstraction
//!
//! This module provides the `UnifiedJob` enum that can represent jobs from any generator:
//! - ZK-Generator (single prover, proof-based)
//! - FHE-Generator (multi-prover, consensus-based)
//! - Legacy Zyberlink (monolithic, deprecated)

use borsh::BorshDeserialize;
use solana_sdk::pubkey::Pubkey;
use zyberlink_jobs::JobCommon;
use zyberlink_types::JobStatus;

use crate::core::error::{Result, UnifiedError};

// Re-export from shared types
pub use zyberlink_types::CircuitType;

/// Maximum number of provers for FHE consensus
pub const MAX_FHE_PROVERS: usize = 5;

// ============================================================================
// Core Data Types
// ============================================================================

/// ZK-specific job data
#[derive(Debug, Clone)]
pub struct ZkJobData {
    /// On-chain address of this job account
    pub pubkey: Pubkey,

    /// Common job fields (200 bytes)
    pub common: JobCommon,

    /// ZK circuit type (0-3 legacy, 10-41 v2.0)
    pub circuit_type: u8,
}

/// FHE-specific job data
#[derive(Debug, Clone)]
pub struct FheJobData {
    /// On-chain address of this job account
    pub pubkey: Pubkey,

    /// Common job fields (200 bytes)
    pub common: JobCommon,

    /// FHE circuit type (4-11)
    pub circuit_type: u8,

    /// Bump for FheConsensusData PDA
    pub fhe_consensus_bump: u8,

    /// Associated consensus data (loaded separately, optional)
    pub consensus: Option<FheConsensusData>,
}

/// Legacy job data (from monolithic zyberlink program)
#[derive(Debug, Clone)]
pub struct LegacyJobData {
    /// On-chain address of this job account
    pub pubkey: Pubkey,

    /// Legacy JobAccount structure
    pub account: LegacyJobAccount,

    /// JobCommon view (converted from account)
    pub common: JobCommon,

    /// Associated FHE consensus data (if FHE job)
    pub fhe_consensus: Option<FheConsensusData>,
}

impl LegacyJobData {
    /// Create LegacyJobData from account
    pub fn from_account(pubkey: Pubkey, account: LegacyJobAccount, fhe_consensus: Option<FheConsensusData>) -> Self {
        let common = account.to_common();
        Self {
            pubkey,
            account,
            common,
            fhe_consensus,
        }
    }
}

// ============================================================================
// FHE Consensus Data
// ============================================================================

/// FHE Consensus Data - multi-prover consensus state
///
/// This is a separate on-chain account that tracks consensus for FHE jobs.
/// PDA: ["fhe_consensus", job_id.to_le_bytes()]
#[derive(Debug, Clone, BorshDeserialize)]
pub struct FheConsensusData {
    /// Job ID this consensus data belongs to
    pub job_id: u64,

    /// FHE operation type (matches circuit_type)
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

    /// Provers who have claimed this job
    pub claimed_provers: [Pubkey; MAX_FHE_PROVERS],

    /// Number of provers who have claimed
    pub claimed_count: u8,

    /// Result hashes submitted by provers
    pub result_hashes: [[u8; 32]; MAX_FHE_PROVERS],

    /// Which provers have submitted results
    pub result_submitted: [bool; MAX_FHE_PROVERS],

    /// Number of results submitted
    pub results_count: u8,

    /// Consensus hash (once achieved)
    pub consensus_hash: Option<[u8; 32]>,

    /// Bump seed for PDA
    pub bump: u8,
}

impl FheConsensusData {
    /// Check if all required provers have claimed
    pub fn is_fully_claimed(&self) -> bool {
        self.claimed_count >= self.required_provers
    }

    /// Check if consensus has been achieved
    pub fn has_consensus(&self) -> bool {
        self.consensus_hash.is_some()
    }

    /// Check if a specific prover has claimed this job
    pub fn has_prover_claimed(&self, prover: &Pubkey) -> bool {
        self.claimed_provers[..self.claimed_count as usize]
            .iter()
            .any(|p| p == prover)
    }

    /// Check if a specific prover has submitted a result
    pub fn has_prover_submitted(&self, prover: &Pubkey) -> bool {
        for i in 0..self.claimed_count as usize {
            if &self.claimed_provers[i] == prover {
                return self.result_submitted[i];
            }
        }
        false
    }

    /// Get the number of results still needed for consensus
    pub fn results_needed_for_consensus(&self) -> u8 {
        if self.results_count >= self.consensus_threshold {
            0
        } else {
            self.consensus_threshold - self.results_count
        }
    }
}

// ============================================================================
// Legacy JobAccount (from zyberlink program)
// ============================================================================

/// Legacy JobAccount structure from monolithic zyberlink program
///
/// NOTE: This is maintained for backwards compatibility with deployed legacy jobs.
/// New jobs should use ZK-Generator or FHE-Generator.
#[derive(Debug, Clone, BorshDeserialize)]
pub struct LegacyJobAccount {
    pub id: u64,
    pub creator: Pubkey,
    pub prover: Option<Pubkey>,
    pub status: JobStatus,
    pub circuit_type: u8,
    pub witness_hash: [u8; 32],
    pub witness_size: u32,
    pub proof_hash: Option<[u8; 32]>,
    pub price_lamports: u64,
    pub escrow_account: Pubkey,
    pub created_at: i64,
    pub timeout_at: i64,
    pub bump: u8,
    pub fhe_consensus_bump: Option<u8>,
}

impl LegacyJobAccount {
    /// Convert legacy JobAccount to JobCommon view
    pub fn to_common(&self) -> JobCommon {
        JobCommon {
            id: self.id,
            creator: self.creator,
            prover: self.prover,
            status: self.status,
            witness_hash: self.witness_hash,
            witness_size: self.witness_size,
            proof_hash: self.proof_hash,
            price_lamports: self.price_lamports,
            escrow_account: self.escrow_account,
            created_at: self.created_at,
            timeout_at: self.timeout_at,
            bump: self.bump,
        }
    }
}

// ============================================================================
// UnifiedJob Enum
// ============================================================================

/// Unified representation of a job from any generator program
///
/// This enum allows working with jobs from ZK-Generator, FHE-Generator, or legacy
/// Zyberlink program through a single consistent API.
///
/// # Design Decision
/// We use an enum (vs trait) for:
/// - Exhaustive pattern matching (compiler-enforced)
/// - Zero-cost abstraction (no vtable/dynamic dispatch)
/// - Easy serialization/deserialization
/// - Known, limited set of variants
#[derive(Debug, Clone)]
pub enum UnifiedJob {
    /// ZK-Generator job (single prover, proof-based)
    Zk(ZkJobData),

    /// FHE-Generator job (multi-prover, consensus-based)
    Fhe(FheJobData),

    /// Legacy Zyberlink job (deprecated, backwards compat)
    Legacy(LegacyJobData),
}

impl UnifiedJob {
    // === Access to Common Fields ===

    /// Get reference to common job fields
    pub fn common(&self) -> &JobCommon {
        match self {
            Self::Zk(data) => &data.common,
            Self::Fhe(data) => &data.common,
            Self::Legacy(data) => &data.common,
        }
    }

    /// Get job ID
    pub fn id(&self) -> u64 {
        match self {
            Self::Zk(data) => data.common.id,
            Self::Fhe(data) => data.common.id,
            Self::Legacy(data) => data.account.id,
        }
    }

    /// Get job status
    pub fn status(&self) -> JobStatus {
        match self {
            Self::Zk(data) => data.common.status,
            Self::Fhe(data) => data.common.status,
            Self::Legacy(data) => data.account.status,
        }
    }

    /// Get job creator pubkey
    pub fn creator(&self) -> &Pubkey {
        match self {
            Self::Zk(data) => &data.common.creator,
            Self::Fhe(data) => &data.common.creator,
            Self::Legacy(data) => &data.account.creator,
        }
    }

    /// Get first prover (or only prover for ZK)
    pub fn prover(&self) -> Option<&Pubkey> {
        match self {
            Self::Zk(data) => data.common.prover.as_ref(),
            Self::Fhe(data) => data.common.prover.as_ref(),
            Self::Legacy(data) => data.account.prover.as_ref(),
        }
    }

    /// Get job price in lamports
    pub fn price(&self) -> u64 {
        match self {
            Self::Zk(data) => data.common.price_lamports,
            Self::Fhe(data) => data.common.price_lamports,
            Self::Legacy(data) => data.account.price_lamports,
        }
    }

    /// Get job on-chain address
    pub fn address(&self) -> &Pubkey {
        match self {
            Self::Zk(data) => &data.pubkey,
            Self::Fhe(data) => &data.pubkey,
            Self::Legacy(data) => &data.pubkey,
        }
    }

    /// Get witness hash
    pub fn witness_hash(&self) -> &[u8; 32] {
        match self {
            Self::Zk(data) => &data.common.witness_hash,
            Self::Fhe(data) => &data.common.witness_hash,
            Self::Legacy(data) => &data.account.witness_hash,
        }
    }

    /// Get witness size
    pub fn witness_size(&self) -> u32 {
        match self {
            Self::Zk(data) => data.common.witness_size,
            Self::Fhe(data) => data.common.witness_size,
            Self::Legacy(data) => data.account.witness_size,
        }
    }

    // === Circuit Type ===

    /// Get circuit type
    pub fn circuit_type(&self) -> u8 {
        match self {
            Self::Zk(data) => data.circuit_type,
            Self::Fhe(data) => data.circuit_type,
            Self::Legacy(data) => data.account.circuit_type,
        }
    }

    // === Type Checks ===

    /// Check if this is a ZK job
    pub fn is_zk(&self) -> bool {
        matches!(self, Self::Zk(_)) ||
        (matches!(self, Self::Legacy(_)) && self.circuit_type() <= 3)
    }

    /// Check if this is an FHE job (requires consensus)
    pub fn is_fhe(&self) -> bool {
        matches!(self, Self::Fhe(_)) ||
        (matches!(self, Self::Legacy(_)) && self.circuit_type() >= 4)
    }

    /// Check if this is a legacy job
    pub fn is_legacy(&self) -> bool {
        matches!(self, Self::Legacy(_))
    }

    // === State Checks ===

    /// Check if job can be claimed
    pub fn can_claim(&self) -> bool {
        match self {
            Self::Zk(data) => data.common.can_claim(),
            Self::Fhe(data) => {
                // FHE jobs can be claimed by multiple provers
                if let Some(consensus) = &data.consensus {
                    // Check if there's still room for more provers
                    data.common.status == JobStatus::Pending ||
                    (data.common.status == JobStatus::Claimed &&
                     consensus.claimed_count < consensus.required_provers)
                } else {
                    data.common.can_claim()
                }
            }
            Self::Legacy(data) => {
                // Legacy FHE jobs
                if data.account.circuit_type >= 4 {
                    if let Some(consensus) = &data.fhe_consensus {
                        data.account.status == JobStatus::Pending ||
                        (data.account.status == JobStatus::Claimed &&
                         consensus.claimed_count < consensus.required_provers)
                    } else {
                        data.account.status == JobStatus::Pending
                    }
                } else {
                    // Legacy ZK jobs
                    data.account.status == JobStatus::Pending
                }
            }
        }
    }

    /// Check if a specific prover can claim this job
    pub fn can_claim_by(&self, prover: &Pubkey) -> bool {
        if !self.can_claim() {
            return false;
        }

        // For FHE jobs, check if prover already claimed
        match self {
            Self::Fhe(data) => {
                if let Some(consensus) = &data.consensus {
                    !consensus.has_prover_claimed(prover)
                } else {
                    true
                }
            }
            Self::Legacy(data) if data.account.circuit_type >= 4 => {
                if let Some(consensus) = &data.fhe_consensus {
                    !consensus.has_prover_claimed(prover)
                } else {
                    true
                }
            }
            _ => {
                // ZK jobs: check if not already claimed by someone
                self.prover().is_none()
            }
        }
    }

    /// Check if job is expired
    pub fn is_expired(&self, current_time: i64) -> bool {
        match self {
            Self::Zk(data) => data.common.is_expired(current_time),
            Self::Fhe(data) => data.common.is_expired(current_time),
            Self::Legacy(data) => {
                current_time >= data.account.timeout_at &&
                data.account.status == JobStatus::Claimed
            }
        }
    }

    /// Check if job is in a terminal state (Completed/Failed/Cancelled)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status(),
            JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled
        )
    }

    // === Timing ===

    /// Get time remaining until timeout (in seconds)
    pub fn time_remaining(&self, current_time: i64) -> i64 {
        match self {
            Self::Zk(data) => data.common.time_remaining(current_time),
            Self::Fhe(data) => data.common.time_remaining(current_time),
            Self::Legacy(data) => {
                (data.account.timeout_at - current_time).max(0)
            }
        }
    }

    /// Get elapsed time since creation (in seconds)
    pub fn elapsed(&self, current_time: i64) -> i64 {
        match self {
            Self::Zk(data) => data.common.elapsed(current_time),
            Self::Fhe(data) => data.common.elapsed(current_time),
            Self::Legacy(data) => {
                current_time - data.account.created_at
            }
        }
    }

    // === FHE Specific ===

    /// Get FHE consensus data (if FHE job)
    pub fn fhe_consensus(&self) -> Option<&FheConsensusData> {
        match self {
            Self::Fhe(data) => data.consensus.as_ref(),
            Self::Legacy(data) => data.fhe_consensus.as_ref(),
            _ => None,
        }
    }

    /// Check if this FHE job is fully claimed (all required provers claimed)
    pub fn is_fhe_fully_claimed(&self) -> bool {
        if !self.is_fhe() {
            return false;
        }

        self.fhe_consensus()
            .map(|c| c.is_fully_claimed())
            .unwrap_or(false)
    }

    /// Check if this FHE job has reached consensus
    pub fn has_fhe_consensus(&self) -> bool {
        if !self.is_fhe() {
            return false;
        }

        self.fhe_consensus()
            .map(|c| c.has_consensus())
            .unwrap_or(false)
    }
}

// ============================================================================
// From Implementations (Conversions)
// ============================================================================

impl From<ZkJobData> for UnifiedJob {
    fn from(data: ZkJobData) -> Self {
        Self::Zk(data)
    }
}

impl From<FheJobData> for UnifiedJob {
    fn from(data: FheJobData) -> Self {
        Self::Fhe(data)
    }
}

impl From<LegacyJobData> for UnifiedJob {
    fn from(data: LegacyJobData) -> Self {
        Self::Legacy(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_common() -> JobCommon {
        JobCommon::new(
            1,
            Pubkey::new_unique(),
            [0u8; 32],
            2048,
            1_000_000,
            Pubkey::new_unique(),
            1000,
            600,
            255,
        )
    }

    #[test]
    fn test_zk_job_basic() {
        let common = create_test_common();
        let job = UnifiedJob::Zk(ZkJobData {
            pubkey: Pubkey::new_unique(),
            common: common.clone(),
            circuit_type: 10, // ProofOfInnocence
        });

        assert_eq!(job.id(), 1);
        assert_eq!(job.circuit_type(), 10);
        assert!(job.is_zk());
        assert!(!job.is_fhe());
        assert!(!job.is_legacy());
        assert!(job.can_claim());
    }

    #[test]
    fn test_fhe_job_basic() {
        let common = create_test_common();
        let job = UnifiedJob::Fhe(FheJobData {
            pubkey: Pubkey::new_unique(),
            common: common.clone(),
            circuit_type: 4, // FHE_ADD
            fhe_consensus_bump: 250,
            consensus: None,
        });

        assert_eq!(job.id(), 1);
        assert_eq!(job.circuit_type(), 4);
        assert!(!job.is_zk());
        assert!(job.is_fhe());
        assert!(!job.is_legacy());
        assert!(job.can_claim());
    }

    #[test]
    fn test_legacy_job_zk() {
        let legacy = LegacyJobAccount {
            id: 1,
            creator: Pubkey::new_unique(),
            prover: None,
            status: JobStatus::Pending,
            circuit_type: 0, // ZcashOrchard (ZK)
            witness_hash: [0u8; 32],
            witness_size: 2048,
            proof_hash: None,
            price_lamports: 1_000_000,
            escrow_account: Pubkey::new_unique(),
            created_at: 1000,
            timeout_at: 1600,
            bump: 255,
            fhe_consensus_bump: None,
        };

        let job = UnifiedJob::Legacy(LegacyJobData {
            pubkey: Pubkey::new_unique(),
            account: legacy,
            fhe_consensus: None,
        });

        assert_eq!(job.id(), 1);
        assert!(job.is_zk());
        assert!(!job.is_fhe());
        assert!(job.is_legacy());
    }

    #[test]
    fn test_legacy_job_fhe() {
        let legacy = LegacyJobAccount {
            id: 2,
            creator: Pubkey::new_unique(),
            prover: None,
            status: JobStatus::Pending,
            circuit_type: 4, // FHE_ADD
            witness_hash: [0u8; 32],
            witness_size: 2048,
            proof_hash: None,
            price_lamports: 2_000_000,
            escrow_account: Pubkey::new_unique(),
            created_at: 1000,
            timeout_at: 1600,
            bump: 255,
            fhe_consensus_bump: Some(250),
        };

        let job = UnifiedJob::Legacy(LegacyJobData {
            pubkey: Pubkey::new_unique(),
            account: legacy,
            fhe_consensus: None,
        });

        assert_eq!(job.id(), 2);
        assert!(!job.is_zk());
        assert!(job.is_fhe());
        assert!(job.is_legacy());
    }
}
