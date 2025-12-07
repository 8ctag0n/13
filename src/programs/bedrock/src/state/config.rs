//! Bedrock configuration state

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Seeds for config PDA
pub const CONFIG_SEED: &[u8] = b"config";

/// Bedrock global configuration
///
/// PDA: ["config"]
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct BedrockConfig {
    /// Admin authority (can update config)
    pub admin: Pubkey,

    /// ZK Generator program ID
    pub zk_generator_program: Pubkey,

    /// FHE Generator program ID
    pub fhe_generator_program: Pubkey,

    /// Next job ID counter (shared across all generators)
    pub next_job_id: u64,

    /// Total registered provers
    pub total_provers: u64,

    /// Total completed jobs
    pub total_jobs_completed: u64,

    /// Is initialized flag
    pub is_initialized: bool,

    /// Bump seed for PDA
    pub bump: u8,
}

impl BedrockConfig {
    /// Size: 32 + 32 + 32 + 8 + 8 + 8 + 1 + 1 = 122 bytes
    pub const SIZE: usize = 122;

    pub fn new(
        admin: Pubkey,
        zk_generator_program: Pubkey,
        fhe_generator_program: Pubkey,
        bump: u8,
    ) -> Self {
        Self {
            admin,
            zk_generator_program,
            fhe_generator_program,
            next_job_id: 1,
            total_provers: 0,
            total_jobs_completed: 0,
            is_initialized: true,
            bump,
        }
    }

    /// Get and increment the next job ID
    pub fn get_next_job_id(&mut self) -> u64 {
        let id = self.next_job_id;
        self.next_job_id += 1;
        id
    }

    /// Check if a program is a registered generator
    pub fn is_registered_generator(&self, program_id: &Pubkey) -> bool {
        program_id == &self.zk_generator_program || program_id == &self.fhe_generator_program
    }
}
