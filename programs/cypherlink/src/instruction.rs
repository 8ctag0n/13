use borsh::{BorshDeserialize, BorshSerialize};
use cypherlink_types::{CircuitType, FheConsensusConfig};

/// Instructions supported by the CypherLink marketplace program
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum MarketplaceInstruction {
    /// Initialize the marketplace
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` Marketplace authority
    /// 1. `[writable]` MarketplaceConfig account (PDA)
    /// 2. `[]` System program
    Initialize {
        /// Platform fee in basis points (e.g., 1000 = 10%)
        fee_basis_points: u16,
        /// Minimum stake amount required for provers (in lamports)
        min_stake_amount: u64,
        /// Minimum reputation score required to claim jobs (0-1000)
        min_reputation_score: u32,
        /// Default job timeout in seconds
        default_job_timeout_seconds: i64,
    },

    /// Register a new prover
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` Prover authority
    /// 1. `[writable]` Prover account (PDA)
    /// 2. `[]` MarketplaceConfig account
    /// 3. `[]` System program
    RegisterProver {
        /// Amount of SOL to stake (must meet minimum)
        stake_amount: u64,
        /// X25519 public key for encrypting witness data
        encryption_pubkey: [u8; 32],
    },

    /// Create a new proving job
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` Job creator (wallet)
    /// 1. `[writable]` Job account (PDA)
    /// 2. `[writable]` MarketplaceConfig account
    /// 3. `[writable]` Escrow account (PDA)
    /// 4. `[]` System program
    /// 5. `[]` Light Protocol program
    CreateJob {
        /// Type of circuit/proof being requested
        circuit_type: CircuitType,
        /// Light Protocol commitment for encrypted witness
        witness_commitment: [u8; 32],
        /// Size of the witness data
        witness_size: u32,
        /// Price offered for completing this job (in lamports)
        price_lamports: u64,
        /// Job timeout in seconds (0 = use default)
        timeout_seconds: i64,
        /// FHE consensus config (only for FHE jobs)
        fhe_config: Option<FheConsensusConfig>,
    },

    /// Claim a pending job
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` Prover authority
    /// 1. `[writable]` Prover account (PDA)
    /// 2. `[writable]` Job account (PDA)
    /// 3. `[]` MarketplaceConfig account
    /// 4. `[]` Clock sysvar
    ClaimJob,

    /// Submit proof for a claimed job
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` Prover authority
    /// 1. `[writable]` Prover account (PDA)
    /// 2. `[writable]` Job account (PDA)
    /// 3. `[writable]` Escrow account (PDA)
    /// 4. `[writable]` Job creator account
    /// 5. `[writable]` Protocol fee recipient
    /// 6. `[]` MarketplaceConfig account
    /// 7. `[]` System program
    /// 8. `[]` Light Protocol program
    /// 9. `[]` Clock sysvar
    SubmitProof {
        /// Light Protocol commitment for encrypted proof
        proof_commitment: [u8; 32],
        /// Size of the proof data
        proof_size: u32,
    },

    /// Cancel a pending job (only by creator)
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` Job creator
    /// 1. `[writable]` Job account (PDA)
    /// 2. `[writable]` Escrow account (PDA)
    /// 3. `[]` System program
    CancelJob,

    /// Slash a prover for misbehavior
    ///
    /// Accounts expected:
    /// 0. `[signer]` Marketplace authority
    /// 1. `[writable]` Prover account (PDA)
    /// 2. `[writable]` Job account (PDA - evidence)
    /// 3. `[writable]` Protocol fee recipient
    /// 4. `[]` MarketplaceConfig account
    /// 5. `[]` System program
    SlashProver {
        /// Amount to slash (in lamports)
        slash_amount: u64,
    },

    /// Submit FHE computation result (prover → chain)
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` Prover authority
    /// 1. `[writable]` Job account (PDA)
    /// 2. `[]` Clock sysvar
    SubmitFheResult {
        /// Hash of encrypted result
        result_hash: [u8; 32],
    },

    /// Finalize FHE job after consensus reached
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` Finalizer (can be anyone)
    /// 1. `[writable]` Job account (PDA)
    /// 2. `[writable]` Escrow account (PDA)
    /// 3. `[writable]` Job creator account
    /// 4. `[writable]` Protocol fee recipient
    /// 5. `[]` MarketplaceConfig account
    /// 6. `[]` System program
    /// 7. `[]` Clock sysvar
    /// 8..N. `[writable]` Prover accounts (PDA) - dynamic list based on fhe_results
    FinalizeFheJob,
}

impl MarketplaceInstruction {
    /// Deserialize instruction from instruction data
    pub fn unpack(input: &[u8]) -> Result<Self, std::io::Error> {
        borsh::from_slice(input)
    }

    /// Serialize instruction to instruction data
    pub fn pack(&self) -> Result<Vec<u8>, std::io::Error> {
        borsh::to_vec(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize_serialization() {
        let instruction = MarketplaceInstruction::Initialize {
            fee_basis_points: 1000,
            min_stake_amount: 5_000_000_000,
            min_reputation_score: 500,
            default_job_timeout_seconds: 600,
        };

        let packed = instruction.pack().unwrap();
        let unpacked = MarketplaceInstruction::unpack(&packed).unwrap();

        assert_eq!(instruction, unpacked);
    }

    #[test]
    fn test_register_prover_serialization() {
        let instruction = MarketplaceInstruction::RegisterProver {
            stake_amount: 10_000_000_000,
            encryption_pubkey: [1u8; 32],
        };

        let packed = instruction.pack().unwrap();
        let unpacked = MarketplaceInstruction::unpack(&packed).unwrap();

        assert_eq!(instruction, unpacked);
    }

    #[test]
    fn test_create_job_serialization() {
        let instruction = MarketplaceInstruction::CreateJob {
            circuit_type: CircuitType::ZcashOrchard,
            witness_commitment: [1u8; 32],
            witness_size: 2048,
            price_lamports: 1_000_000,
            timeout_seconds: 600,
        };

        let packed = instruction.pack().unwrap();
        let unpacked = MarketplaceInstruction::unpack(&packed).unwrap();

        assert_eq!(instruction, unpacked);
    }

    #[test]
    fn test_submit_proof_serialization() {
        let instruction = MarketplaceInstruction::SubmitProof {
            proof_commitment: [2u8; 32],
            proof_size: 1536,
        };

        let packed = instruction.pack().unwrap();
        let unpacked = MarketplaceInstruction::unpack(&packed).unwrap();

        assert_eq!(instruction, unpacked);
    }

    #[test]
    fn test_claim_job_serialization() {
        let instruction = MarketplaceInstruction::ClaimJob;

        let packed = instruction.pack().unwrap();
        let unpacked = MarketplaceInstruction::unpack(&packed).unwrap();

        assert_eq!(instruction, unpacked);
    }
}
