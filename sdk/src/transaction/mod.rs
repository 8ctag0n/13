/// Transaction building utilities for CypherLink
///
/// This module provides transaction composition tools that:
/// - Combine multiple instructions
/// - Set transaction parameters (fee payer, blockhash)
/// - Build unsigned transactions (for wallet signing)
/// - Build signed transactions (for CLI/testing)

mod builder;

pub use builder::TransactionBuilder;

// Re-export common types
pub use solana_sdk::{
    hash::Hash,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};
