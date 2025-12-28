/// Pure instruction builders for ZyberLink marketplace
///
/// This module provides wallet-compatible instruction builders that:
/// - Accept Pubkey instead of Keypair (no signing)
/// - Return pure Instruction objects
/// - Can be used with any Solana wallet
/// - Support transaction simulation
mod marketplace;

pub use marketplace::InstructionBuilder;

// Re-export common types
#[allow(deprecated)]
pub use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
};
