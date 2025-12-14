//! ZK Generator SDK - Client library for ZyberLink zk-generator program
//!
//! This SDK provides:
//! - PDA derivation functions
//! - Instruction builders
//! - Account helpers
//! - Type re-exports from zk-generator program
//!
//! # Example
//!
//! ```no_run
//! use zk_generator_sdk::{derive_job_pda, derive_escrow_pda, instructions};
//! use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
//!
//! let program_id = Pubkey::default();
//! let creator = Keypair::new();
//! let job_id = 12345u64;
//!
//! let (job_pda, _) = derive_job_pda(&program_id, &creator.pubkey(), job_id);
//! let (escrow_pda, _) = derive_escrow_pda(&program_id, &job_pda);
//!
//! let ix = instructions::create_job(
//!     &program_id,
//!     &creator.pubkey(),
//!     &job_pda,
//!     &escrow_pda,
//!     10, // CircuitType::ProofOfInnocence
//!     [0u8; 32], // witness_hash
//!     1024, // witness_size
//!     100_000_000, // price_lamports (0.1 SOL)
//!     3600, // timeout_seconds (1 hour)
//! );
//! ```

pub mod accounts;
pub mod instructions;
pub mod pda;

// Re-export core types from zk-generator
pub use zk_generator::{
    circuits::CircuitType,
    ZkGeneratorInstruction, ZkJob, ZK_JOB_SEED,
};

// Re-export PDA derivation functions
pub use pda::{derive_escrow_pda, derive_job_pda};

// Re-export instruction builders
pub use instructions::*;

// Re-export escrow seed constant
pub const ESCROW_SEED: &[u8] = b"zk_escrow";
