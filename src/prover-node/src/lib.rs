#![allow(
    dead_code,
    unused_imports,
    unused_variables,
    clippy::too_many_arguments,
    clippy::manual_range_contains,
    clippy::manual_contains,
    clippy::unnecessary_cast,
    clippy::needless_borrows_for_generic_args,
    deprecated
)]

// === NEW MODULAR STRUCTURE ===
pub mod cli;
pub mod core;
pub mod engines;
pub mod gateway;
pub mod marketplace;
pub mod services;

// === EXISTING MODULES (legacy) ===
pub mod circuits;
pub mod config;
pub mod halo2_prover;
pub mod roi_calculator;
pub mod tui;
pub mod wizard;
pub mod witness_encryption;
pub mod witness_fetcher;

// Re-export commonly used types from new modules
pub use core::{CircuitCategory, CircuitRegistry, JobProcessor};
pub use engines::{ArkworksProver, ProofType, ZkEngine, ZkProofResult, ZkProver};
pub use gateway::{GatewayAuthHeaders, GatewayClient};
pub use marketplace::{
    ChainType, FheConsensusConfig, JobData, MarketplaceConfig, MarketplaceError,
    MarketplaceFactory, MarketplaceOperations, MarketplaceOperationsExt, ProverData,
    SolanaMarketplace, TransactionResult,
};
pub use services::{DiscoveredJob, JobPoller, WitnessService};

// Re-export legacy types for backwards compatibility
pub use circuits::{CensusCircuit, DemographicsCircuit, PassportCircuit, VotingCircuit};
pub use config::{ProverConfig, ProverConfiguration};
pub use halo2_prover::{Halo2Prover, OrchardWitness};
pub use roi_calculator::ROICalculator;
pub use witness_encryption::{EncryptedWitness, WitnessEncryption};
pub use witness_fetcher::WitnessFetcher;

// Re-export FHE engine from shared crate
pub use zyberlink_fhe::{generate_keys as generate_fhe_keys, FheEngine};
