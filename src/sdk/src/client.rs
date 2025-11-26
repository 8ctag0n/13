use anyhow::Result;
use blake2::{Blake2s256, Digest};
use zyberlink_types::{CircuitType, FheConsensusConfig};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};

use crate::instruction::MarketplaceInstruction;

/// Client for interacting with the ZyberLink marketplace program
pub struct MarketplaceClient {
    /// RPC client for Solana
    pub rpc_client: RpcClient,
    /// Program ID of the marketplace
    pub program_id: Pubkey,
    /// Commitment level for transactions
    pub commitment: CommitmentConfig,
}

impl MarketplaceClient {
    /// Create a new marketplace client
    pub fn new(rpc_url: String, program_id: Pubkey) -> Self {
        Self {
            rpc_client: RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed()),
            program_id,
            commitment: CommitmentConfig::confirmed(),
        }
    }

    /// Create client with custom commitment level
    pub fn new_with_commitment(
        rpc_url: String,
        program_id: Pubkey,
        commitment: CommitmentConfig,
    ) -> Self {
        Self {
            rpc_client: RpcClient::new_with_commitment(rpc_url.clone(), commitment),
            program_id,
            commitment,
        }
    }

    // ============================================================================
    // Instruction Builders
    // ============================================================================

    /// Build Initialize instruction
    pub fn initialize_instruction(
        &self,
        authority: &Pubkey,
        fee_basis_points: u16,
        min_stake_amount: u64,
        min_reputation_score: u32,
        default_job_timeout_seconds: i64,
    ) -> Result<Instruction> {
        let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &self.program_id);

        let instruction_data = MarketplaceInstruction::Initialize {
            fee_basis_points,
            min_stake_amount,
            min_reputation_score,
            default_job_timeout_seconds,
        };

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(*authority, true),
                AccountMeta::new(config_pda, false),
                AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            ],
            data: instruction_data.pack()?,
        })
    }

    /// Build RegisterProver instruction
    pub fn register_prover_instruction(
        &self,
        prover_authority: &Pubkey,
        stake_amount: u64,
        encryption_pubkey: [u8; 32],
    ) -> Result<Instruction> {
        let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &self.program_id);
        let (prover_pda, _) =
            Pubkey::find_program_address(&[b"prover", prover_authority.as_ref()], &self.program_id);

        let instruction_data = MarketplaceInstruction::RegisterProver {
            stake_amount,
            encryption_pubkey,
        };

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(*prover_authority, true),
                AccountMeta::new(prover_pda, false),
                AccountMeta::new(config_pda, false),
                AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            ],
            data: instruction_data.pack()?,
        })
    }

    /// Build CreateJob instruction
    pub fn create_job_instruction(
        &self,
        job_creator: &Pubkey,
        job_id: u64,
        circuit_type: CircuitType,
        witness_commitment: [u8; 32],
        witness_size: u32,
        price_lamports: u64,
        timeout_seconds: i64,
        fhe_config: Option<FheConsensusConfig>,
    ) -> Result<Instruction> {
        let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &self.program_id);
        let job_id_bytes = job_id.to_le_bytes();
        let (job_pda, _) = Pubkey::find_program_address(
            &[b"job", job_creator.as_ref(), &job_id_bytes],
            &self.program_id,
        );
        let (escrow_pda, _) =
            Pubkey::find_program_address(&[b"escrow", job_pda.as_ref()], &self.program_id);

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
                AccountMeta::new(*job_creator, true),
                AccountMeta::new(job_pda, false),
                AccountMeta::new(config_pda, false),
                AccountMeta::new(escrow_pda, false),
                AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
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
    pub fn create_fhe_job_instruction(
        &self,
        creator: &Pubkey,
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

        self.create_job_instruction(
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

    /// Build ClaimJob instruction
    pub fn claim_job_instruction(
        &self,
        prover_authority: &Pubkey,
        job_pda: &Pubkey,
    ) -> Result<Instruction> {
        let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &self.program_id);
        let (prover_pda, _) =
            Pubkey::find_program_address(&[b"prover", prover_authority.as_ref()], &self.program_id);

        let instruction_data = MarketplaceInstruction::ClaimJob;

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(*prover_authority, true),
                AccountMeta::new(prover_pda, false),
                AccountMeta::new(*job_pda, false),
                AccountMeta::new_readonly(config_pda, false),
            ],
            data: instruction_data.pack()?,
        })
    }

    /// Build SubmitProof instruction with protocol fee recipient lookup
    pub fn submit_proof_instruction(
        &self,
        prover_authority: &Pubkey,
        job_pda: &Pubkey,
        job_creator: &Pubkey,
        proof_commitment: [u8; 32],
        proof_size: u32,
    ) -> Result<Instruction> {
        // Get protocol fee recipient from config via RPC
        let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &self.program_id);
        let config_account = self.rpc_client.get_account(&config_pda)?;
        // Offset: authority(32) + fee_basis_points(2) + min_stake(8) + min_rep(4) + timeout(8) = 54
        let protocol_fee_recipient = if config_account.data.len() >= 86 {
            Pubkey::try_from(&config_account.data[54..86])?
        } else {
            return Err(anyhow::anyhow!("Invalid config account"));
        };

        self.submit_proof_instruction_with_recipient(
            prover_authority,
            job_pda,
            job_creator,
            &protocol_fee_recipient,
            proof_commitment,
            proof_size,
        )
    }

    /// Build SubmitProof instruction with provided protocol fee recipient
    /// Use this version when you already know the protocol fee recipient
    /// (e.g., in tests or when you have the config cached)
    pub fn submit_proof_instruction_with_recipient(
        &self,
        prover_authority: &Pubkey,
        job_pda: &Pubkey,
        job_creator: &Pubkey,
        protocol_fee_recipient: &Pubkey,
        proof_commitment: [u8; 32],
        proof_size: u32,
    ) -> Result<Instruction> {
        let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &self.program_id);
        let (prover_pda, _) =
            Pubkey::find_program_address(&[b"prover", prover_authority.as_ref()], &self.program_id);
        let (escrow_pda, _) =
            Pubkey::find_program_address(&[b"escrow", job_pda.as_ref()], &self.program_id);

        let instruction_data = MarketplaceInstruction::SubmitProof {
            proof_commitment,
            proof_size,
        };

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(*prover_authority, true),
                AccountMeta::new(prover_pda, false),
                AccountMeta::new(*job_pda, false),
                AccountMeta::new(escrow_pda, false),
                AccountMeta::new(*job_creator, false),
                AccountMeta::new(*protocol_fee_recipient, false),
                AccountMeta::new(config_pda, false),
                AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            ],
            data: instruction_data.pack()?,
        })
    }

    /// Build CancelJob instruction
    pub fn cancel_job_instruction(
        &self,
        job_creator: &Pubkey,
        job_pda: &Pubkey,
    ) -> Result<Instruction> {
        let (escrow_pda, _) =
            Pubkey::find_program_address(&[b"escrow", job_pda.as_ref()], &self.program_id);

        let instruction_data = MarketplaceInstruction::CancelJob;

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(*job_creator, true),
                AccountMeta::new(*job_pda, false),
                AccountMeta::new(escrow_pda, false),
            ],
            data: instruction_data.pack()?,
        })
    }

    /// Build SlashProver instruction
    pub fn slash_prover_instruction(
        &self,
        authority: &Pubkey,
        job_pda: &Pubkey,
        prover_authority: &Pubkey,
    ) -> Result<Instruction> {
        let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &self.program_id);
        let (prover_pda, _) =
            Pubkey::find_program_address(&[b"prover", prover_authority.as_ref()], &self.program_id);
        let (escrow_pda, _) =
            Pubkey::find_program_address(&[b"escrow", job_pda.as_ref()], &self.program_id);

        let instruction_data = MarketplaceInstruction::SlashProver {
            slash_amount: 0, // TODO: Make this a parameter
        };

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(*authority, true),
                AccountMeta::new(*job_pda, false),
                AccountMeta::new(prover_pda, false),
                AccountMeta::new(escrow_pda, false),
                AccountMeta::new_readonly(config_pda, false),
            ],
            data: instruction_data.pack()?,
        })
    }

    /// Build SubmitFheResult instruction
    ///
    /// Accounts:
    /// 0. [writable, signer] Prover authority
    /// 1. [writable] Job account (PDA)
    pub fn submit_fhe_result_instruction(
        &self,
        prover_authority: &Pubkey,
        job_pda: &Pubkey,
        result_hash: [u8; 32],
    ) -> Result<Instruction> {
        let instruction_data = MarketplaceInstruction::SubmitFheResult { result_hash };

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(*prover_authority, true),
                AccountMeta::new(*job_pda, false),
            ],
            data: instruction_data.pack()?,
        })
    }

    /// Build FinalizeFheJob instruction
    ///
    /// Accounts:
    /// 0. [signer] Finalizer (can be anyone)
    /// 1. [writable] Job account (PDA)
    /// 2. [writable] Escrow account (PDA)
    /// 3. [writable] Job creator account
    /// 4. [writable] Protocol fee recipient
    /// 5. [] MarketplaceConfig account
    /// 6. [] System program
    /// 7. [] Clock sysvar
    /// 8..N. [writable] Prover accounts (matching provers)
    pub fn finalize_fhe_job_instruction(
        &self,
        finalizer: &Pubkey,
        job_pda: &Pubkey,
        job_creator: &Pubkey,
        prover_accounts: &[Pubkey],
    ) -> Result<Instruction> {
        // Get protocol fee recipient from config via RPC
        let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &self.program_id);
        let config_account = self.rpc_client.get_account(&config_pda)?;
        // Offset: authority(32) + fee_basis_points(2) + min_stake(8) + min_rep(4) + timeout(8) = 54
        let protocol_fee_recipient = if config_account.data.len() >= 86 {
            Pubkey::try_from(&config_account.data[54..86])?
        } else {
            return Err(anyhow::anyhow!("Invalid config account"));
        };

        self.finalize_fhe_job_instruction_with_recipient(
            finalizer,
            job_pda,
            job_creator,
            &protocol_fee_recipient,
            prover_accounts,
        )
    }

    /// Build FinalizeFheJob instruction with provided protocol fee recipient
    /// Use this version when you already know the protocol fee recipient
    pub fn finalize_fhe_job_instruction_with_recipient(
        &self,
        finalizer: &Pubkey,
        job_pda: &Pubkey,
        job_creator: &Pubkey,
        protocol_fee_recipient: &Pubkey,
        prover_accounts: &[Pubkey],
    ) -> Result<Instruction> {
        let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &self.program_id);
        let (escrow_pda, _) =
            Pubkey::find_program_address(&[b"escrow", job_pda.as_ref()], &self.program_id);

        let instruction_data = MarketplaceInstruction::FinalizeFheJob;

        let mut accounts = vec![
            AccountMeta::new(*finalizer, true),
            AccountMeta::new(*job_pda, false),
            AccountMeta::new(escrow_pda, false),
            AccountMeta::new(*job_creator, false),
            AccountMeta::new(*protocol_fee_recipient, false),
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            AccountMeta::new_readonly(solana_sdk::sysvar::clock::id(), false),
        ];

        // Add prover accounts (dynamic) - need [authority, pda] per prover
        for prover_authority in prover_accounts {
            let (prover_pda, _) = Pubkey::find_program_address(
                &[b"prover", prover_authority.as_ref()],
                &self.program_id,
            );
            accounts.push(AccountMeta::new(*prover_authority, false));
            accounts.push(AccountMeta::new(prover_pda, false));
        }

        Ok(Instruction {
            program_id: self.program_id,
            accounts,
            data: instruction_data.pack()?,
        })
    }

    // ============================================================================
    // Transaction Helpers
    // ============================================================================

    /// Send and confirm a transaction
    pub fn send_and_confirm_transaction(
        &self,
        instructions: &[Instruction],
        signers: &[&Keypair],
    ) -> Result<Signature> {
        let recent_blockhash = self.rpc_client.get_latest_blockhash()?;
        let mut transaction = Transaction::new_with_payer(instructions, Some(&signers[0].pubkey()));
        transaction.sign(signers, recent_blockhash);

        let signature = self
            .rpc_client
            .send_and_confirm_transaction_with_spinner(&transaction)?;

        Ok(signature)
    }

    // ============================================================================
    // High-Level FHE Methods
    // ============================================================================

    /// Submit FHE computation result
    pub fn submit_fhe_result(
        &self,
        prover: &Keypair,
        job_pda: &Pubkey,
        result_hash: [u8; 32],
    ) -> Result<Signature> {
        let ix = self.submit_fhe_result_instruction(&prover.pubkey(), job_pda, result_hash)?;

        self.send_and_confirm_transaction(&[ix], &[prover])
    }

    /// Finalize FHE job and distribute payments
    pub fn finalize_fhe_job(
        &self,
        finalizer: &Keypair,
        job_pda: &Pubkey,
        job_creator: &Pubkey,
        matching_prover_pubkeys: &[Pubkey],
    ) -> Result<Signature> {
        let ix = self.finalize_fhe_job_instruction(
            &finalizer.pubkey(),
            job_pda,
            job_creator,
            matching_prover_pubkeys,
        )?;

        self.send_and_confirm_transaction(&[ix], &[finalizer])
    }

    /// Finalize FHE job with provided protocol fee recipient
    pub fn finalize_fhe_job_with_recipient(
        &self,
        finalizer: &Keypair,
        job_pda: &Pubkey,
        job_creator: &Pubkey,
        protocol_fee_recipient: &Pubkey,
        matching_prover_pubkeys: &[Pubkey],
    ) -> Result<Signature> {
        let ix = self.finalize_fhe_job_instruction_with_recipient(
            &finalizer.pubkey(),
            job_pda,
            job_creator,
            protocol_fee_recipient,
            matching_prover_pubkeys,
        )?;

        self.send_and_confirm_transaction(&[ix], &[finalizer])
    }

    // ============================================================================
    // PDA Helpers
    // ============================================================================

    /// Get config PDA
    pub fn get_config_pda(&self) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"config"], &self.program_id)
    }

    /// Get prover PDA for an authority
    pub fn get_prover_pda(&self, prover_authority: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"prover", prover_authority.as_ref()], &self.program_id)
    }

    /// Get job PDA
    pub fn get_job_pda(&self, job_creator: &Pubkey, job_id: u64) -> (Pubkey, u8) {
        let job_id_bytes = job_id.to_le_bytes();
        Pubkey::find_program_address(
            &[b"job", job_creator.as_ref(), &job_id_bytes],
            &self.program_id,
        )
    }

    /// Get escrow PDA for a job
    pub fn get_escrow_pda(&self, job_pda: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"escrow", job_pda.as_ref()], &self.program_id)
    }
}

impl Default for MarketplaceClient {
    fn default() -> Self {
        Self::new("http://localhost:8899".to_string(), Pubkey::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client =
            MarketplaceClient::new("http://localhost:8899".to_string(), Pubkey::new_unique());
        assert_eq!(client.commitment, CommitmentConfig::confirmed());
    }

    #[test]
    fn test_pda_derivation() {
        let client =
            MarketplaceClient::new("http://localhost:8899".to_string(), Pubkey::new_unique());

        let (config_pda, _) = client.get_config_pda();
        assert_ne!(config_pda, Pubkey::default());

        let prover_authority = Pubkey::new_unique();
        let (prover_pda, _) = client.get_prover_pda(&prover_authority);
        assert_ne!(prover_pda, Pubkey::default());

        let job_creator = Pubkey::new_unique();
        let (job_pda, _) = client.get_job_pda(&job_creator, 0);
        assert_ne!(job_pda, Pubkey::default());

        let (escrow_pda, _) = client.get_escrow_pda(&job_pda);
        assert_ne!(escrow_pda, Pubkey::default());
    }
}
