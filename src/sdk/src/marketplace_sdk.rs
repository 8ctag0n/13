use anyhow::{Context, Result};
use cypherlink_types::fhe::FheConsensusConfig;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

use crate::instructions::InstructionBuilder;

/// High-level SDK for marketplace operations
///
/// This is a facade over InstructionBuilder that provides a simpler API
/// for common marketplace operations. It wraps instruction creation without
/// handling transaction signing or RPC communication.
pub struct MarketplaceSDK {
    instruction_builder: InstructionBuilder,
}

impl MarketplaceSDK {
    /// Create a new MarketplaceSDK instance
    ///
    /// # Arguments
    /// * `program_id` - The CypherLink program ID
    pub fn new(program_id: Pubkey) -> Self {
        Self {
            instruction_builder: InstructionBuilder::new(program_id),
        }
    }

    /// Initialize the marketplace
    ///
    /// # Arguments
    /// * `authority` - Public key of the marketplace authority (must sign)
    /// * `fee_basis_points` - Fee in basis points (100 = 1%)
    /// * `min_stake_amount` - Minimum stake amount for provers
    /// * `min_reputation_score` - Minimum reputation score to claim jobs
    /// * `default_job_timeout_seconds` - Default timeout for jobs
    ///
    /// # Returns
    /// Instruction to initialize the marketplace
    pub fn initialize(
        &self,
        authority: Pubkey,
        fee_basis_points: u16,
        min_stake_amount: u64,
        min_reputation_score: u32,
        default_job_timeout_seconds: i64,
    ) -> Result<Instruction> {
        self.instruction_builder.initialize(
            authority,
            fee_basis_points,
            min_stake_amount,
            min_reputation_score,
            default_job_timeout_seconds,
        )
    }

    /// Get the next job ID from the marketplace config
    ///
    /// # Arguments
    /// * `rpc_client` - RPC client to fetch on-chain data
    ///
    /// # Returns
    /// The next available job ID
    pub fn get_next_job_id(&self, rpc_client: &RpcClient) -> Result<u64> {
        let (config_pda, _) = self.instruction_builder.config_pda();

        let account = rpc_client
            .get_account(&config_pda)
            .context("Failed to fetch marketplace config")?;

        // Layout: authority(32) + fee(2) + min_stake(8) + min_rep(4) + timeout(8) + protocol_fee_recipient(32) + next_job_id(8)
        // Offset: 32 + 2 + 8 + 4 + 8 + 32 = 86 bytes
        let next_job_id = u64::from_le_bytes(
            account.data[86..94]
                .try_into()
                .context("Invalid config data")?,
        );

        Ok(next_job_id)
    }

    /// Create an FHE computation job
    ///
    /// # Arguments
    /// * `creator` - Public key of the job creator
    /// * `job_id` - Unique job identifier (get from get_next_job_id)
    /// * `encrypted_input` - Encrypted FHE input data
    /// * `fhe_config` - FHE consensus configuration
    /// * `price_lamports` - Job reward in lamports
    /// * `timeout_seconds` - Job timeout in seconds
    ///
    /// # Returns
    /// Instruction to create the FHE job
    pub fn create_fhe_job(
        &self,
        creator: Pubkey,
        job_id: u64,
        encrypted_input: &[u8],
        fhe_config: FheConsensusConfig,
        price_lamports: u64,
        timeout_seconds: i64,
    ) -> Result<Instruction> {
        self.instruction_builder.create_fhe_job(
            creator,
            job_id,
            encrypted_input,
            fhe_config,
            price_lamports,
            timeout_seconds,
        )
    }
}
