use anyhow::Result;
use borsh::to_vec;
use cypherlink_types::CircuitType;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};

/// Client for interacting with the CypherLink marketplace program
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

        #[derive(borsh::BorshSerialize)]
        struct InitializeData {
            discriminator: u8,
            fee_basis_points: u16,
            min_stake_amount: u64,
            min_reputation_score: u32,
            default_job_timeout_seconds: i64,
        }

        let data = InitializeData {
            discriminator: 0, // Initialize = 0
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
            data: to_vec(&data)?,
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

        #[derive(borsh::BorshSerialize)]
        struct RegisterProverData {
            discriminator: u8,
            stake_amount: u64,
            encryption_pubkey: [u8; 32],
        }

        let data = RegisterProverData {
            discriminator: 1, // RegisterProver = 1
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
            data: to_vec(&data)?,
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
    ) -> Result<Instruction> {
        let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &self.program_id);
        let job_id_bytes = job_id.to_le_bytes();
        let (job_pda, _) = Pubkey::find_program_address(
            &[b"job", job_creator.as_ref(), &job_id_bytes],
            &self.program_id,
        );
        let (escrow_pda, _) = Pubkey::find_program_address(&[b"escrow", job_pda.as_ref()], &self.program_id);

        #[derive(borsh::BorshSerialize)]
        struct CreateJobData {
            discriminator: u8,
            circuit_type: CircuitType,
            witness_commitment: [u8; 32],
            witness_size: u32,
            price_lamports: u64,
            timeout_seconds: i64,
        }

        let data = CreateJobData {
            discriminator: 2, // CreateJob = 2
            circuit_type,
            witness_commitment,
            witness_size,
            price_lamports,
            timeout_seconds,
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
            data: to_vec(&data)?,
        })
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

        #[derive(borsh::BorshSerialize)]
        struct ClaimJobData {
            discriminator: u8,
        }

        let data = ClaimJobData {
            discriminator: 3, // ClaimJob = 3
        };

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(*prover_authority, true),
                AccountMeta::new(prover_pda, false),
                AccountMeta::new(*job_pda, false),
                AccountMeta::new_readonly(config_pda, false),
            ],
            data: to_vec(&data)?,
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
        let (escrow_pda, _) = Pubkey::find_program_address(&[b"escrow", job_pda.as_ref()], &self.program_id);

        #[derive(borsh::BorshSerialize)]
        struct SubmitProofData {
            discriminator: u8,
            proof_commitment: [u8; 32],
            proof_size: u32,
        }

        let data = SubmitProofData {
            discriminator: 4, // SubmitProof = 4
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
            data: to_vec(&data)?,
        })
    }

    /// Build CancelJob instruction
    pub fn cancel_job_instruction(
        &self,
        job_creator: &Pubkey,
        job_pda: &Pubkey,
    ) -> Result<Instruction> {
        let (escrow_pda, _) = Pubkey::find_program_address(&[b"escrow", job_pda.as_ref()], &self.program_id);

        #[derive(borsh::BorshSerialize)]
        struct CancelJobData {
            discriminator: u8,
        }

        let data = CancelJobData {
            discriminator: 5, // CancelJob = 5
        };

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(*job_creator, true),
                AccountMeta::new(*job_pda, false),
                AccountMeta::new(escrow_pda, false),
            ],
            data: to_vec(&data)?,
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
        let (escrow_pda, _) = Pubkey::find_program_address(&[b"escrow", job_pda.as_ref()], &self.program_id);

        #[derive(borsh::BorshSerialize)]
        struct SlashProverData {
            discriminator: u8,
        }

        let data = SlashProverData {
            discriminator: 6, // SlashProver = 6
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
            data: to_vec(&data)?,
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
        Self::new(
            "http://localhost:8899".to_string(),
            Pubkey::default(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = MarketplaceClient::new(
            "http://localhost:8899".to_string(),
            Pubkey::new_unique(),
        );
        assert_eq!(client.commitment, CommitmentConfig::confirmed());
    }

    #[test]
    fn test_pda_derivation() {
        let client = MarketplaceClient::new(
            "http://localhost:8899".to_string(),
            Pubkey::new_unique(),
        );

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
