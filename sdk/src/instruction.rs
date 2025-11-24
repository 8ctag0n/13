use borsh::{BorshDeserialize, BorshSerialize};
use cypherlink_types::{CircuitType, FheConsensusConfig};

/// Instructions supported by the CypherLink marketplace program
///
/// NOTE: This is a duplicate of the enum in programs/cypherlink/src/instruction.rs
/// We duplicate it here because the SDK cannot depend on solana_program crate.
/// Any changes to the program's instruction enum MUST be replicated here.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum MarketplaceInstruction {
    /// Initialize the marketplace
    Initialize {
        fee_basis_points: u16,
        min_stake_amount: u64,
        min_reputation_score: u32,
        default_job_timeout_seconds: i64,
    },

    /// Register a new prover
    RegisterProver {
        stake_amount: u64,
        encryption_pubkey: [u8; 32],
    },

    /// Create a new proving job
    CreateJob {
        circuit_type: CircuitType,
        witness_commitment: [u8; 32],
        witness_size: u32,
        price_lamports: u64,
        timeout_seconds: i64,
        fhe_config: Option<FheConsensusConfig>,
    },

    /// Create a new proving job with SPL token payment
    CreateJobWithToken {
        circuit_type: CircuitType,
        witness_commitment: [u8; 32],
        witness_size: u32,
        price_token_amount: u64,
        timeout_seconds: i64,
        fhe_config: Option<FheConsensusConfig>,
    },

    /// Claim a pending job
    ClaimJob,

    /// Submit proof for a claimed job
    SubmitProof {
        proof_commitment: [u8; 32],
        proof_size: u32,
    },

    /// Cancel a pending job
    CancelJob,

    /// Slash a prover for misbehavior
    SlashProver { slash_amount: u64 },

    /// Submit FHE computation result
    SubmitFheResult { result_hash: [u8; 32] },

    /// Finalize FHE job after consensus reached
    FinalizeFheJob,
}

impl MarketplaceInstruction {
    /// Serialize instruction to bytes
    pub fn pack(&self) -> Result<Vec<u8>, std::io::Error> {
        borsh::to_vec(self)
    }

    /// Deserialize instruction from bytes
    pub fn unpack(input: &[u8]) -> Result<Self, std::io::Error> {
        borsh::from_slice(input)
    }
}
