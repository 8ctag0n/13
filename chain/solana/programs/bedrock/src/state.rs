//! Bedrock state account definitions

pub mod config;
pub mod prover;
pub mod validator;

pub use config::{BedrockConfig, CONFIG_SEED};
pub use prover::{ProverAccount, PROVER_SEED};
pub use validator::{ValidatorAccount, ValidatorRegion, VALIDATOR_SEED};
