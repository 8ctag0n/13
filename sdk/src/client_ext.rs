/// Extended client methods for wallet integration and high-level operations
///
/// This module extends MarketplaceClient with:
/// - Wallet-compatible methods (no auto-signing)
/// - High-level helper methods
/// - Transaction simulation support
/// - Fluent API for common workflows

use anyhow::Result;
use cypherlink_types::{CircuitType, FheConsensusConfig, FheOperation};
use solana_sdk::{
    hash::Hash,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};

use crate::{
    instructions::InstructionBuilder,
    transaction::TransactionBuilder,
    MarketplaceClient,
};

/// Extension trait for wallet-friendly operations
impl MarketplaceClient {
    // ============================================================================
    // Instruction Builders (Wallet-Compatible)
    // ============================================================================

    /// Get instruction builder for this client
    ///
    /// Returns a pure instruction builder that doesn't require keypairs.
    /// Perfect for wallet integration.
    ///
    /// # Example
    /// ```no_run
    /// let builder = client.instructions();
    /// let ix = builder.create_job(
    ///     creator_pubkey,
    ///     job_id,
    ///     circuit_type,
    ///     commitment,
    ///     size,
    ///     price,
    ///     timeout,
    ///     None,
    /// )?;
    /// // Wallet will sign this instruction
    /// ```
    pub fn instructions(&self) -> InstructionBuilder {
        InstructionBuilder::new(self.program_id)
    }

    // ============================================================================
    // High-Level Wallet Methods
    // ============================================================================

    /// Create a ZK job instruction (wallet-compatible)
    ///
    /// Simplified method for creating ZK proof jobs.
    ///
    /// # Arguments
    /// * `creator` - Job creator pubkey
    /// * `circuit` - Circuit type
    /// * `witness_commitment` - Hash of witness data
    /// * `witness_size` - Size of witness
    /// * `price_lamports` - Payment for completion
    ///
    /// # Returns
    /// Unsigned instruction for wallet to sign
    pub fn create_zk_job_ix(
        &self,
        creator: Pubkey,
        circuit: CircuitType,
        witness_commitment: [u8; 32],
        witness_size: u32,
        price_lamports: u64,
    ) -> Result<Instruction> {
        let job_id = self.fetch_next_job_id()?;

        self.instructions().create_job(
            creator,
            job_id,
            circuit,
            witness_commitment,
            witness_size,
            price_lamports,
            600, // Default 10min timeout
            None,
        )
    }

    /// Create an FHE job instruction (wallet-compatible)
    ///
    /// Simplified method for creating FHE computation jobs with multi-prover consensus.
    ///
    /// # Arguments
    /// * `creator` - Job creator pubkey
    /// * `operation` - FHE operation to perform
    /// * `encrypted_input_commitment` - Hash of encrypted input
    /// * `encrypted_input_size` - Size of encrypted input
    /// * `required_provers` - Number of provers needed (default: 3)
    /// * `consensus_threshold` - Minimum matching results (default: 2)
    ///
    /// # Returns
    /// Unsigned instruction for wallet to sign
    pub fn create_fhe_job_ix(
        &self,
        creator: Pubkey,
        operation: FheOperation,
        encrypted_input_commitment: [u8; 32],
        encrypted_input_size: u32,
        required_provers: u8,
        consensus_threshold: u8,
    ) -> Result<Instruction> {
        let job_id = self.fetch_next_job_id()?;

        let fhe_config = FheConsensusConfig {
            required_provers,
            consensus_threshold,
            submission_timeout_secs: 300,
            operation: operation.clone(),
        };

        let price = (required_provers as u64) * 1_000_000; // 0.001 SOL per prover

        self.instructions().create_job(
            creator,
            job_id,
            CircuitType::FheComputation(operation),
            encrypted_input_commitment,
            encrypted_input_size,
            price,
            600,
            Some(fhe_config),
        )
    }

    /// Register prover instruction (wallet-compatible)
    ///
    /// # Arguments
    /// * `prover` - Prover authority pubkey
    /// * `stake_amount` - Amount to stake
    /// * `encryption_key` - Public encryption key
    pub fn register_prover_ix(
        &self,
        prover: Pubkey,
        stake_amount: u64,
        encryption_key: [u8; 32],
    ) -> Result<Instruction> {
        self.instructions().register_prover(prover, stake_amount, encryption_key)
    }

    /// Claim job instruction (wallet-compatible)
    pub fn claim_job_ix(&self, prover: Pubkey, job_pda: Pubkey) -> Result<Instruction> {
        self.instructions().claim_job(prover, job_pda)
    }

    /// Submit FHE result instruction (wallet-compatible)
    pub fn submit_fhe_result_ix(
        &self,
        prover: Pubkey,
        job_pda: Pubkey,
        result_hash: [u8; 32],
    ) -> Result<Instruction> {
        self.instructions().submit_fhe_result(prover, job_pda, result_hash)
    }

    // ============================================================================
    // Transaction Building (Unsigned for Wallets)
    // ============================================================================

    /// Build an unsigned transaction from instructions
    ///
    /// Creates a transaction ready for wallet signing.
    ///
    /// # Arguments
    /// * `instructions` - List of instructions to include
    /// * `payer` - Fee payer pubkey
    ///
    /// # Returns
    /// Unsigned transaction that wallet can sign
    pub fn build_transaction(
        &self,
        instructions: Vec<Instruction>,
        payer: Pubkey,
    ) -> Result<Transaction> {
        let blockhash = self.rpc_client.get_latest_blockhash()?;

        TransactionBuilder::new()
            .add_instructions(instructions)
            .payer(payer)
            .blockhash(blockhash)
            .build_unsigned()
    }

    /// Build unsigned transaction with custom blockhash
    ///
    /// Useful when you want to control the blockhash (e.g., durable nonces)
    pub fn build_transaction_with_blockhash(
        &self,
        instructions: Vec<Instruction>,
        payer: Pubkey,
        blockhash: Hash,
    ) -> Result<Transaction> {
        TransactionBuilder::new()
            .add_instructions(instructions)
            .payer(payer)
            .blockhash(blockhash)
            .build_unsigned()
    }

    // ============================================================================
    // Execution Mode (CLI/Testing - Auto-signing)
    // ============================================================================

    /// Execute a transaction with auto-signing (CLI mode)
    ///
    /// This is a convenience method for CLI tools and testing.
    /// For wallet integration, use `build_transaction()` instead.
    ///
    /// # Arguments
    /// * `instructions` - Instructions to execute
    /// * `signers` - Keypairs to sign with
    pub fn execute_transaction(
        &self,
        instructions: Vec<Instruction>,
        signers: &[&Keypair],
    ) -> Result<Signature> {
        if signers.is_empty() {
            anyhow::bail!("No signers provided");
        }

        let blockhash = self.rpc_client.get_latest_blockhash()?;

        let tx = TransactionBuilder::new()
            .add_instructions(instructions)
            .payer(signers[0].pubkey())
            .blockhash(blockhash)
            .build_and_sign(signers)?;

        let sig = self.rpc_client.send_and_confirm_transaction(&tx)?;
        Ok(sig)
    }

    /// Create and execute ZK job (CLI mode with auto-signing)
    pub fn create_zk_job(
        &self,
        creator: &Keypair,
        circuit: CircuitType,
        witness_commitment: [u8; 32],
        witness_size: u32,
        price_lamports: u64,
    ) -> Result<Signature> {
        let ix = self.create_zk_job_ix(
            creator.pubkey(),
            circuit,
            witness_commitment,
            witness_size,
            price_lamports,
        )?;

        self.execute_transaction(vec![ix], &[creator])
    }

    /// Create and execute FHE job (CLI mode with auto-signing)
    pub fn create_fhe_job(
        &self,
        creator: &Keypair,
        operation: FheOperation,
        encrypted_input_commitment: [u8; 32],
        encrypted_input_size: u32,
        required_provers: u8,
        consensus_threshold: u8,
    ) -> Result<Signature> {
        let ix = self.create_fhe_job_ix(
            creator.pubkey(),
            operation,
            encrypted_input_commitment,
            encrypted_input_size,
            required_provers,
            consensus_threshold,
        )?;

        self.execute_transaction(vec![ix], &[creator])
    }

    // ============================================================================
    // Simulation Support
    // ============================================================================

    /// Simulate a transaction to estimate compute units and check for errors
    ///
    /// Perfect for UI previews and validation before wallet signing.
    ///
    /// # Returns
    /// Tuple of (compute_units_used, logs, error_if_any)
    pub fn simulate_transaction(
        &self,
        instructions: Vec<Instruction>,
        payer: Pubkey,
    ) -> Result<(Option<u64>, Option<Vec<String>>, Option<String>)> {
        let tx = self.build_transaction(instructions, payer)?;

        let result = self.rpc_client.simulate_transaction(&tx)?;

        Ok((
            result.value.units_consumed,
            result.value.logs,
            result.value.err.map(|e| e.to_string()),
        ))
    }

    // ============================================================================
    // Helper Methods
    // ============================================================================

    /// Fetch the next available job ID from chain
    ///
    /// Queries the marketplace config to get the next job ID.
    pub fn fetch_next_job_id(&self) -> Result<u64> {
        let (config_pda, _) = self.get_config_pda();
        let account = self.rpc_client.get_account(&config_pda)?;

        // Parse config to get next_job_id
        use borsh::BorshDeserialize;
        use crate::helpers::MarketplaceConfig;

        let config = MarketplaceConfig::try_from_slice(&account.data)?;
        Ok(config.next_job_id)
    }

    /// Get recent blockhash for transaction building
    pub fn get_blockhash(&self) -> Result<Hash> {
        Ok(self.rpc_client.get_latest_blockhash()?)
    }
}
