//! FHE Circuit implementations for advanced privacy-preserving operations
//!
//! This module contains specialized circuits for different use cases:
//! - Census: Population counting and aggregation
//! - Passport: Identity verification and age checks
//! - Demographics: Statistical analysis
//! - Voting: Private voting and tallying

pub mod census;
pub mod demographics;
pub mod passport;
pub mod voting;

pub use census::CensusCircuit;
pub use demographics::DemographicsCircuit;
pub use passport::PassportCircuit;
pub use voting::VotingCircuit;
