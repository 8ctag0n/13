//! Bedrock state account definitions

pub mod config;
pub mod prover;

pub use config::{BedrockConfig, CONFIG_SEED};
pub use prover::{ProverAccount, PROVER_SEED};
