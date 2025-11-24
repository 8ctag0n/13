// Public modules for testing and external use
pub mod circuits;
pub mod fhe_engine;
pub mod halo2_prover;
pub mod witness_encryption;
pub mod witness_fetcher;

// Re-export commonly used types
pub use circuits::{CensusCircuit, DemographicsCircuit, PassportCircuit, VotingCircuit};
pub use fhe_engine::{generate_fhe_keys, FheEngine};
pub use halo2_prover::{Halo2Prover, OrchardWitness};
pub use witness_encryption::{EncryptedWitness, WitnessEncryption};
pub use witness_fetcher::WitnessFetcher;
