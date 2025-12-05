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

// Public modules for testing and external use
pub mod circuits;
pub mod halo2_prover;
pub mod witness_encryption;
pub mod witness_fetcher;

// Re-export commonly used types
pub use circuits::{CensusCircuit, DemographicsCircuit, PassportCircuit, VotingCircuit};
pub use halo2_prover::{Halo2Prover, OrchardWitness};
pub use witness_encryption::{EncryptedWitness, WitnessEncryption};
pub use witness_fetcher::WitnessFetcher;

// Re-export FHE engine from shared crate
pub use zyberlink_fhe::{FheEngine, generate_keys as generate_fhe_keys};
