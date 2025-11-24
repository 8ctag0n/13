use anyhow::Result;
use blake2::{Blake2s256, Digest};
use cypherlink_types::{CircuitType, FheConsensusConfig};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program, sysvar,
};
use std::str::FromStr;

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

    /// Get token escrow PDA (for SPL token payments)
    pub fn token_escrow_pda(&self, job_pda: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"token_escrow", job_pda.as_ref()], &self.program_id)
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
                AccountMeta::new(config_pda, false), // Writable - program increments next_job_id
                AccountMeta::new(escrow_pda, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data: instruction_data.pack()?,
        })
    }

    /// Build CreateFheJob instruction (convenience wrapper for FHE jobs)
    ///
    /// Creates an FHE computation job from encrypted input data.
    /// Automatically computes the witness commitment from the encrypted input using Blake2s-256.
    ///
    /// # Arguments
    /// * `creator` - Job creator pubkey (will sign and pay)
    /// * `job_id` - Unique job ID (used for PDA derivation)
    /// * `encrypted_input` - Encrypted input data for FHE computation
    /// * `fhe_config` - FHE consensus configuration
    /// * `price_lamports` - Payment for job completion
    /// * `timeout_seconds` - Job timeout in seconds
    pub fn create_fhe_job(
        &self,
        creator: Pubkey,
        job_id: u64,
        encrypted_input: &[u8],
        fhe_config: FheConsensusConfig,
        price_lamports: u64,
        timeout_seconds: i64,
    ) -> Result<Instruction> {
        // Create witness commitment from encrypted input using Blake2s-256
        let mut hasher = Blake2s256::new();
        hasher.update(encrypted_input);
        let witness_commitment: [u8; 32] = hasher.finalize().into();

        self.create_job(
            creator,
            job_id,
            CircuitType::FheComputation(fhe_config.operation.clone()),
            witness_commitment,
            encrypted_input.len() as u32,
            price_lamports,
            timeout_seconds,
            Some(fhe_config),
        )
    }

    /// Build CreateJobWithToken instruction for SPL token payments (e.g. wZEC)
    ///
    /// Creates an FHE computation job with SPL token payment.
    /// Automatically computes the witness commitment from the encrypted input using Blake2s-256.
    ///
    /// # Arguments
    /// * `creator` - Job creator pubkey (will sign and pay)
    /// * `job_id` - Unique job ID (used for PDA derivation)
    /// * `encrypted_input` - Encrypted input data for FHE computation
    /// * `fhe_config` - FHE consensus configuration
    /// * `price_token_amount` - Payment in token base units (e.g. zatoshis for wZEC)
    /// * `timeout_seconds` - Job timeout in seconds
    /// * `token_mint` - SPL token mint address (e.g. wZEC mint)
    /// * `creator_token_account` - Creator's associated token account
    pub fn create_fhe_job_with_token(
        &self,
        creator: Pubkey,
        job_id: u64,
        encrypted_input: &[u8],
        fhe_config: FheConsensusConfig,
        price_token_amount: u64,
        timeout_seconds: i64,
        token_mint: Pubkey,
        creator_token_account: Pubkey,
    ) -> Result<Instruction> {
        // Create witness commitment from encrypted input using Blake2s-256
        let mut hasher = Blake2s256::new();
        hasher.update(encrypted_input);
        let witness_commitment: [u8; 32] = hasher.finalize().into();

        let (config_pda, _) = self.config_pda();
        let (job_pda, _) = self.job_pda(&creator, job_id);
        let (token_escrow_pda, _) = self.token_escrow_pda(&job_pda);

        let instruction_data = MarketplaceInstruction::CreateJobWithToken {
            circuit_type: CircuitType::FheComputation(fhe_config.operation.clone()),
            witness_commitment,
            witness_size: encrypted_input.len() as u32,
            price_token_amount,
            timeout_seconds,
            fhe_config: Some(fhe_config),
        };

        // SPL Token program ID
        let token_program_id = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
            .expect("Valid SPL Token program ID");

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(creator, true),                // 0. creator (signer)
                AccountMeta::new(job_pda, false),               // 1. job_pda
                AccountMeta::new(config_pda, false),            // 2. config_pda
                AccountMeta::new(token_escrow_pda, false),      // 3. token_escrow (PDA)
                AccountMeta::new(creator_token_account, false), // 4. creator's token account
                AccountMeta::new_readonly(token_mint, false),   // 5. token mint
                AccountMeta::new_readonly(system_program::id(), false), // 6. system program
                AccountMeta::new_readonly(token_program_id, false), // 7. token program
                AccountMeta::new_readonly(sysvar::rent::id(), false), // 8. rent sysvar
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
                AccountMeta::new_readonly(prover_pda, false),
                AccountMeta::new(job_pda, false),
                AccountMeta::new_readonly(config_pda, false),
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
    /// * `job_creator` - Job creator pubkey
    /// * `protocol_fee_recipient` - Protocol fee recipient pubkey
    /// * `proof_commitment` - Hash commitment of proof
    /// * `proof_size` - Size of proof in bytes
    pub fn submit_proof(
        &self,
        prover: Pubkey,
        job_pda: Pubkey,
        job_creator: Pubkey,
        protocol_fee_recipient: Pubkey,
        proof_commitment: [u8; 32],
        proof_size: u32,
    ) -> Result<Instruction> {
        let (prover_pda, _) = self.prover_pda(&prover);
        let (escrow_pda, _) = self.escrow_pda(&job_pda);
        let (config_pda, _) = self.config_pda();

        let instruction_data = MarketplaceInstruction::SubmitProof {
            proof_commitment,
            proof_size,
        };

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(prover, true),
                AccountMeta::new(prover_pda, false),
                AccountMeta::new(job_pda, false),
                AccountMeta::new(escrow_pda, false),
                AccountMeta::new(job_creator, false),
                AccountMeta::new(protocol_fee_recipient, false),
                AccountMeta::new_readonly(config_pda, false),
                AccountMeta::new_readonly(system_program::id(), false),
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

    /// Build SlashProver instruction
    ///
    /// Slashes a misbehaving prover's stake.
    /// Only the marketplace authority can call this.
    ///
    /// # Arguments
    /// * `authority` - Marketplace authority pubkey (will sign)
    /// * `prover_authority` - Prover to slash
    /// * `job_pda` - Evidence job PDA
    /// * `protocol_fee_recipient` - Where slashed funds go
    /// * `slash_amount` - Amount to slash in lamports
    pub fn slash_prover(
        &self,
        authority: Pubkey,
        prover_authority: Pubkey,
        job_pda: Pubkey,
        protocol_fee_recipient: Pubkey,
        slash_amount: u64,
    ) -> Result<Instruction> {
        let (prover_pda, _) = self.prover_pda(&prover_authority);
        let (config_pda, _) = self.config_pda();

        let instruction_data = MarketplaceInstruction::SlashProver { slash_amount };

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new_readonly(authority, true),
                AccountMeta::new(prover_pda, false),
                AccountMeta::new(job_pda, false),
                AccountMeta::new(protocol_fee_recipient, false),
                AccountMeta::new_readonly(config_pda, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data: instruction_data.pack()?,
        })
    }
}
