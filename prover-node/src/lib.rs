// Public modules for testing and external use
pub mod fhe_engine;
pub mod halo2_prover;
pub mod witness_encryption;
pub mod witness_fetcher;

// Re-export commonly used types
pub use fhe_engine::{FheEngine, generate_fhe_keys};
pub use halo2_prover::{Halo2Prover, OrchardWitness};
pub use witness_encryption::{WitnessEncryption, EncryptedWitness};
pub use witness_fetcher::WitnessFetcher;
