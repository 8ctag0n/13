//! Threshold - ZyberLink Threshold Encryption Coordinator
//!
//! This program coordinates the 3-of-5 threshold encryption protocol for witness data protection.
//! It manages the on-chain protocol where:
//! 1. Provers request key shares for encrypted witness data
//! 2. Validators (registered in Bedrock) submit their encrypted key shares
//! 3. Once 3+ validators respond, the prover can reconstruct the decryption key off-chain
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────┐
//! │                    THRESHOLD (this)                       │
//! │        (Key Share Request/Response Coordination)          │
//! └──────────────────────────────────────────────────────────┘
//!                          ↓ reads                ↑ reads
//! ┌──────────────────┐                    ┌─────────────────┐
//! │   ZK-GENERATOR   │                    │     BEDROCK     │
//! │  (ZK Job state)  │                    │  (Validators)   │
//! └──────────────────┘                    └─────────────────┘
//! ```
//!
//! # Protocol Flow
//!
//! 1. **Request Phase**: Prover with a claimed ZK job calls `RequestKeyShare` with the
//!    encrypted witness CID. Creates a `KeyShareRequest` PDA.
//!
//! 2. **Response Phase**: Validators monitor requests and call `SubmitKeyShare` with their
//!    encrypted key share. Each validator can respond once per request.
//!
//! 3. **Threshold Met**: Once 3+ validators respond, request status becomes `Ready`.
//!    Prover retrieves all responses off-chain and reconstructs the key.
//!
//! # Security
//!
//! - Only active validators (staked in Bedrock) can submit shares
//! - Each validator can only respond once per request
//! - Requests expire after 5 minutes
//! - Shares are encrypted for the prover's public key (off-chain)
//!
//! # Instructions
//!
//! - `RequestKeyShare` - Prover requests key shares for a job
//! - `SubmitKeyShare` - Validator submits encrypted key share

pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

pub use error::*;
pub use instruction::*;
pub use state::*;

// Temporary program ID - replace with actual keypair-generated ID in production
solana_program::declare_id!("Thre5Z4kbkc6tEWEVMSdT6QsMUgSaVKEyFRwxrEsFfE");
