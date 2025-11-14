use anyhow::Result;
use cypherlink_types::{CircuitType, FheConsensusConfig};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program, sysvar,
};

use crate::instruction::MarketplaceInstruction;

/// Pure instruction builder for CypherLink marketplace
///
/// This builder creates wallet-compatible instructions without requiring keypairs.
/// All methods accept Pubkey instead of &Keypair, making them safe for wallet integration.
///
/// # Example
/// ```no_run
/// use cypherlink_sdk::instructions::InstructionBuilder;
/// use solana_sdk::pubkey::Pubkey;
///
/// let program_id = Pubkey::new_unique();
/// let builder = InstructionBuilder::new(program_id);
///
/// // Create instruction without signing
/// let creator_pubkey = Pubkey::new_unique();
/// let ix = builder.create_job(
///     creator_pubkey,
///     1,
///     CircuitType::ZcashOrchard,
///     [0u8; 32],
///     1024,
///     1_000_000_000,
///     600,
///     None,
/// )?;
///
/// // Instruction can be signed by wallet
/// ```
pub struct InstructionBuilder {
    program_id: Pubkey,
}

impl InstructionBuilder {
    /// Create a new instruction builder
    pub fn new(program_id: Pubkey) -> Self {
        Self { program_id }
    }

    // ============================================================================
    // PDA Helpers
    // ============================================================================

    /// Get marketplace config PDA
    pub fn config_pda(&self) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"config"], &self.program_id)
    }

    /// Get prover PDA
    pub fn prover_pda(&self, authority: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"prover", authority.as_ref()], &self.program_id)
    }

    /// Get job PDA
    pub fn job_pda(&self, creator: &Pubkey, job_id: u64) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[b"job", creator.as_ref(), &job_id.to_le_bytes()],
            &self.program_id,
        )
    }

    /// Get escrow PDA
    pub fn escrow_pda(&self, job_pda: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"escrow", job_pda.as_ref()], &self.program_id)
    }

    // ============================================================================
    // Instruction Builders
    // ============================================================================

    /// Build Initialize instruction
    ///
    /// Creates the marketplace configuration account.
    ///
    /// # Arguments
    /// * `authority` - Marketplace authority pubkey (will sign)
    /// * `fee_basis_points` - Protocol fee in basis points
    /// * `min_stake_amount` - Minimum stake for provers
    /// * `min_reputation_score` - Minimum reputation to claim jobs
    /// * `default_job_timeout_seconds` - Default timeout for jobs
    pub fn initialize(
        &self,
        authority: Pubkey,
        fee_basis_points: u16,
        min_stake_amount: u64,
        min_reputation_score: u32,
        default_job_timeout_seconds: i64,
    ) -> Result<Instruction> {
        let (config_pda, _) = self.config_pda();

        let instruction_data = MarketplaceInstruction::Initialize {
            fee_basis_points,
            min_stake_amount,
            min_reputation_score,
            default_job_timeout_seconds,
        };

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(authority, true),
                AccountMeta::new(config_pda, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data: instruction_data.pack()?,
        })
    }

    /// Build RegisterProver instruction
    ///
    /// Registers a new prover in the marketplace.
    ///
    /// # Arguments
    /// * `prover_authority` - Prover's authority pubkey (will sign)
    /// * `stake_amount` - Amount to stake
    /// * `encryption_pubkey` - Public encryption key for witness encryption
    pub fn register_prover(
        &self,
        prover_authority: Pubkey,
        stake_amount: u64,
        encryption_pubkey: [u8; 32],
    ) -> Result<Instruction> {
        let (config_pda, _) = self.config_pda();
        let (prover_pda, _) = self.prover_pda(&prover_authority);

        let instruction_data = MarketplaceInstruction::RegisterProver {
            stake_amount,
            encryption_pubkey,
        };

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(prover_authority, true),
                AccountMeta::new(prover_pda, false),
                AccountMeta::new_readonly(config_pda, false),
                AccountMeta::new_readonly(system_program::id(), false),
                AccountMeta::new_readonly(sysvar::clock::id(), false),
            ],
            data: instruction_data.pack()?,
        })
    }

    /// Build CreateJob instruction
    ///
    /// Creates a new compute job in the marketplace.
    /// Supports both ZK and FHE jobs.
    ///
    /// # Arguments
    /// * `creator` - Job creator pubkey (will sign and pay)
    /// * `job_id` - Unique job ID (used for PDA derivation)
    /// * `circuit_type` - Type of circuit (ZK or FHE)
    /// * `witness_commitment` - Hash commitment of witness data
    /// * `witness_size` - Size of witness data in bytes
    /// * `price_lamports` - Payment for job completion
    /// * `timeout_seconds` - Job timeout in seconds
    /// * `fhe_config` - Optional FHE consensus configuration
    pub fn create_job(
        &self,
        creator: Pubkey,
        job_id: u64,
        circuit_type: CircuitType,
        witness_commitment: [u8; 32],
        witness_size: u32,
        price_lamports: u64,
        timeout_seconds: i64,
        fhe_config: Option<FheConsensusConfig>,
    ) -> Result<Instruction> {
        let (config_pda, _) = self.config_pda();
        let (job_pda, _) = self.job_pda(&creator, job_id);
        let (escrow_pda, _) = self.escrow_pda(&job_pda);

        let instruction_data = MarketplaceInstruction::CreateJob {
            circuit_type,
            witness_commitment,
            witness_size,
            price_lamports,
            timeout_seconds,
            fhe_config,
        };

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(creator, true),
                AccountMeta::new(job_pda, false),
                AccountMeta::new(escrow_pda, false),
                AccountMeta::new_readonly(config_pda, false),
                AccountMeta::new_readonly(system_program::id(), false),
                AccountMeta::new_readonly(sysvar::clock::id(), false),
            ],
            data: instruction_data.pack()?,
        })
    }

    /// Build ClaimJob instruction
    ///
    /// Allows a prover to claim a pending job.
    ///
    /// # Arguments
    /// * `prover` - Prover pubkey (will sign)
    /// * `job_pda` - Job account PDA
    pub fn claim_job(&self, prover: Pubkey, job_pda: Pubkey) -> Result<Instruction> {
        let (prover_pda, _) = self.prover_pda(&prover);
        let (config_pda, _) = self.config_pda();

        let instruction_data = MarketplaceInstruction::ClaimJob;

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(prover, true),
                AccountMeta::new(job_pda, false),
                AccountMeta::new_readonly(prover_pda, false),
                AccountMeta::new_readonly(config_pda, false),
                AccountMeta::new_readonly(sysvar::clock::id(), false),
            ],
            data: instruction_data.pack()?,
        })
    }

    /// Build SubmitProof instruction (for ZK jobs)
    ///
    /// Submits a ZK proof for verification.
    ///
    /// # Arguments
    /// * `prover` - Prover pubkey (will sign)
    /// * `job_pda` - Job account PDA
    /// * `proof_commitment` - Hash commitment of proof
    /// * `proof_size` - Size of proof in bytes
    pub fn submit_proof(
        &self,
        prover: Pubkey,
        job_pda: Pubkey,
        proof_commitment: [u8; 32],
        proof_size: u32,
    ) -> Result<Instruction> {
        let (prover_pda, _) = self.prover_pda(&prover);

        let instruction_data = MarketplaceInstruction::SubmitProof {
            proof_commitment,
            proof_size,
        };

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(prover, true),
                AccountMeta::new(job_pda, false),
                AccountMeta::new(prover_pda, false),
                AccountMeta::new_readonly(sysvar::clock::id(), false),
            ],
            data: instruction_data.pack()?,
        })
    }

    /// Build SubmitFheResult instruction (for FHE jobs)
    ///
    /// Submits an FHE computation result hash for consensus.
    ///
    /// # Arguments
    /// * `prover` - Prover pubkey (will sign)
    /// * `job_pda` - Job account PDA
    /// * `result_hash` - Hash of FHE computation result
    pub fn submit_fhe_result(
        &self,
        prover: Pubkey,
        job_pda: Pubkey,
        result_hash: [u8; 32],
    ) -> Result<Instruction> {
        let instruction_data = MarketplaceInstruction::SubmitFheResult { result_hash };

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(prover, true),
                AccountMeta::new(job_pda, false),
            ],
            data: instruction_data.pack()?,
        })
    }

    /// Build FinalizeFheJob instruction
    ///
    /// Finalizes an FHE job after consensus is reached.
    /// Distributes payments to matching provers.
    ///
    /// # Arguments
    /// * `finalizer` - Anyone can finalize (will sign for tx fee)
    /// * `job_pda` - Job account PDA
    /// * `job_creator` - Original job creator
    /// * `protocol_fee_recipient` - Protocol fee recipient
    /// * `matching_provers` - List of provers who submitted matching results
    pub fn finalize_fhe_job(
        &self,
        finalizer: Pubkey,
        job_pda: Pubkey,
        job_creator: Pubkey,
        protocol_fee_recipient: Pubkey,
        matching_provers: &[Pubkey],
    ) -> Result<Instruction> {
        let (escrow_pda, _) = self.escrow_pda(&job_pda);
        let (config_pda, _) = self.config_pda();

        let instruction_data = MarketplaceInstruction::FinalizeFheJob;

        let mut accounts = vec![
            AccountMeta::new(finalizer, true),
            AccountMeta::new(job_pda, false),
            AccountMeta::new(escrow_pda, false),
            AccountMeta::new(job_creator, false),
            AccountMeta::new(protocol_fee_recipient, false),
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ];

        // Add matching provers as writable accounts
        for prover in matching_provers {
            accounts.push(AccountMeta::new(*prover, false));
        }

        Ok(Instruction {
            program_id: self.program_id,
            accounts,
            data: instruction_data.pack()?,
        })
    }

    /// Build CancelJob instruction
    ///
    /// Cancels a job and refunds the creator.
    ///
    /// # Arguments
    /// * `creator` - Job creator pubkey (will sign)
    /// * `job_pda` - Job account PDA
    pub fn cancel_job(&self, creator: Pubkey, job_pda: Pubkey) -> Result<Instruction> {
        let (escrow_pda, _) = self.escrow_pda(&job_pda);

        let instruction_data = MarketplaceInstruction::CancelJob;

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(creator, true),
                AccountMeta::new(job_pda, false),
                AccountMeta::new(escrow_pda, false),
                AccountMeta::new_readonly(system_program::id(), false),
                AccountMeta::new_readonly(sysvar::clock::id(), false),
            ],
            data: instruction_data.pack()?,
        })
    }
}
