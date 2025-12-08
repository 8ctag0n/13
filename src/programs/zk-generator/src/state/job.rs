//! ZK Job state - uses JobCommon from zyberlink-jobs

use borsh::{BorshDeserialize, BorshSerialize};
use zyberlink_jobs::JobCommon;

use crate::circuits::{is_valid_circuit_type, CircuitType};

/// Seeds for ZK job PDA
pub const ZK_JOB_SEED: &[u8] = b"zk_job";

// Legacy constants for backwards compatibility
pub const CIRCUIT_ZCASH_ORCHARD: u8 = CircuitType::ZcashOrchard as u8;
pub const CIRCUIT_ZCASH_SAPLING: u8 = CircuitType::ZcashSapling as u8;
pub const CIRCUIT_ANONYMOUS_VOTE: u8 = CircuitType::AnonymousVote as u8;
pub const CIRCUIT_CREDENTIAL: u8 = CircuitType::Credential as u8;

// v2.0 circuit constants
pub const CIRCUIT_PROOF_OF_INNOCENCE: u8 = CircuitType::ProofOfInnocence as u8;
pub const CIRCUIT_PRIVATE_VOTE: u8 = CircuitType::PrivateVote as u8;
pub const CIRCUIT_PRIVATE_VOTE_POI: u8 = CircuitType::PrivateVoteWithPoI as u8;
pub const CIRCUIT_MARKET_BET: u8 = CircuitType::MarketBet as u8;
pub const CIRCUIT_MARKET_BET_POI: u8 = CircuitType::MarketBetWithPoI as u8;
pub const CIRCUIT_MARKET_CLAIM: u8 = CircuitType::MarketClaim as u8;
pub const CIRCUIT_PORTFOLIO_COMPLIANCE: u8 = CircuitType::PortfolioCompliance as u8;
pub const CIRCUIT_PORTFOLIO_NET_WORTH: u8 = CircuitType::PortfolioNetWorth as u8;

/// ZK Job Account
///
/// Composes JobCommon with ZK-specific fields.
///
/// PDA: ["zk_job", creator, job_id.to_le_bytes()]
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct ZkJob {
    /// Common job fields (200 bytes)
    pub common: JobCommon,

    /// Circuit type (see CircuitType enum)
    pub circuit_type: u8,
}

impl ZkJob {
    /// Size: JobCommon::SIZE (200) + 1 = 201 bytes
    pub const SIZE: usize = JobCommon::SIZE + 1;

    /// Check if circuit type is valid for ZK
    pub fn is_valid_circuit(circuit_type: u8) -> bool {
        is_valid_circuit_type(circuit_type)
    }

    /// Get the CircuitType enum for this job
    pub fn get_circuit_type(&self) -> Option<CircuitType> {
        CircuitType::from_u8(self.circuit_type)
    }

    /// Check if this is a v2.0 circuit
    pub fn is_v2_circuit(&self) -> bool {
        self.get_circuit_type()
            .map(|ct| ct.is_v2())
            .unwrap_or(false)
    }

    /// Check if this circuit requires PoI verification
    pub fn requires_poi(&self) -> bool {
        self.get_circuit_type()
            .map(|ct| ct.requires_poi())
            .unwrap_or(false)
    }
}
