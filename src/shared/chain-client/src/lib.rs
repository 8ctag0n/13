//! ZyberLink Chain Client - Multi-chain abstraction layer
//!
//! This library provides a unified interface for interacting with multiple blockchains
//! (Solana, Starknet, Aptos) to enable cross-chain proof generation and verification.
//!
//! # Architecture
//!
//! ```text
//! ChainClient Trait (this crate)
//!       │
//!       ├── Transaction operations
//!       ├── Account queries
//!       ├── Token transfers
//!       └── Event subscriptions
//!       │
//!       ▼
//! ┌──────────┬──────────┬──────────┐
//! │  Solana  │ Starknet │  Aptos   │
//! │  Client  │  Client  │  Client  │
//! └──────────┴──────────┴──────────┘
//! ```
//!
//! # Interface Stability
//!
//! ⚠️ **INTERFACE LOCKED** - This trait is now FINAL (as of Day 1)
//! Breaking changes after today will break all verticals.
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use zyberlink_chain_client::{ChainClient, MockChainClient};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = MockChainClient::new();
//!
//!     let balance = client.get_balance("user_address").await?;
//!     println!("Balance: {}", balance);
//!
//!     Ok(())
//! }
//! ```

pub mod types;
pub mod error;
pub mod mock;
pub mod signature;

#[cfg(feature = "solana")]
pub mod solana;

#[cfg(feature = "solana")]
pub mod solana_extensions;

#[cfg(feature = "starknet")]
pub mod starknet;

#[cfg(feature = "starknet")]
pub mod starknet_extensions;

#[cfg(feature = "aptos")]
pub mod aptos;

#[cfg(feature = "aptos")]
pub mod aptos_primitives;

pub use error::{ChainClientError, Result};
pub use mock::MockChainClient;
pub use types::*;
pub use signature::SignatureVerifier;

#[cfg(feature = "solana")]
pub use solana::SolanaClient;

#[cfg(feature = "solana")]
pub use solana_extensions::SolanaSpecificOps;

#[cfg(feature = "starknet")]
pub use starknet::StarknetClient;

#[cfg(feature = "starknet")]
pub use starknet_extensions::{StarknetEvent, StarknetSpecificOps};

#[cfg(feature = "aptos")]
pub use aptos::AptosClient;

#[cfg(feature = "aptos")]
pub use aptos_primitives::{
    AccountAddress as AptosAccountAddress,
    AptosAccount,
    RawTransaction as AptosRawTransaction,
    SignedTransaction as AptosSignedTransaction,
    EntryFunction as AptosEntryFunction,
    TypeTag as AptosTypeTag,
};

use async_trait::async_trait;

/// Core trait for multi-chain client operations
///
/// This trait provides a unified interface for interacting with different blockchains.
/// All implementations must be async and thread-safe.
///
/// # Interface Stability
///
/// This interface is **LOCKED** as of Day 1. No breaking changes allowed after today.
/// Verticals depend on this interface being stable.
#[async_trait]
pub trait ChainClient: Send + Sync {
    // ==================== Network & Configuration ====================

    /// Get the chain identifier (e.g., "solana", "starknet", "aptos")
    fn chain_id(&self) -> &str;

    /// Get the network name (e.g., "mainnet", "devnet", "testnet")
    fn network(&self) -> &str;

    /// Check if the client is connected and healthy
    async fn health_check(&self) -> Result<bool>;

    // ==================== Account Operations ====================

    /// Get native token balance for an address
    ///
    /// # Arguments
    /// * `address` - The account address as a string
    ///
    /// # Returns
    /// Balance in smallest unit (lamports for Solana, wei equivalent for others)
    async fn get_balance(&self, address: &str) -> Result<u64>;

    /// Get account data/state
    ///
    /// # Arguments
    /// * `address` - The account address
    ///
    /// # Returns
    /// Raw account data as bytes
    async fn get_account_data(&self, address: &str) -> Result<Vec<u8>>;

    /// Check if an account exists on-chain
    async fn account_exists(&self, address: &str) -> Result<bool>;

    // ==================== Transaction Operations ====================

    /// Send a signed transaction to the network
    ///
    /// # Arguments
    /// * `transaction` - The signed transaction data
    ///
    /// # Returns
    /// Transaction signature/hash as string
    async fn send_transaction(&self, transaction: &[u8]) -> Result<String>;

    /// Wait for transaction confirmation
    ///
    /// # Arguments
    /// * `signature` - Transaction signature/hash
    /// * `timeout_secs` - Maximum time to wait in seconds
    ///
    /// # Returns
    /// true if confirmed, false if timeout
    async fn confirm_transaction(
        &self,
        signature: &str,
        timeout_secs: u64
    ) -> Result<bool>;

    /// Get transaction status
    ///
    /// # Arguments
    /// * `signature` - Transaction signature/hash
    ///
    /// # Returns
    /// Transaction status (pending, confirmed, failed)
    async fn get_transaction_status(&self, signature: &str) -> Result<TransactionStatus>;

    // ==================== Token Operations ====================

    /// Transfer native tokens from one address to another
    ///
    /// # Arguments
    /// * `from` - Sender address (must have signing capability)
    /// * `to` - Recipient address
    /// * `amount` - Amount in smallest unit
    ///
    /// # Returns
    /// Transaction signature
    async fn transfer(
        &self,
        from: &str,
        to: &str,
        amount: u64,
    ) -> Result<String>;

    // ==================== Smart Contract / Program Calls ====================

    /// Call a smart contract/program (read-only)
    ///
    /// # Arguments
    /// * `program_address` - The contract/program address
    /// * `method` - Method/instruction name
    /// * `args` - Serialized arguments
    ///
    /// # Returns
    /// Serialized return data
    async fn call_contract(
        &self,
        program_address: &str,
        method: &str,
        args: &[u8],
    ) -> Result<Vec<u8>>;

    /// Execute a smart contract/program (write operation)
    ///
    /// # Arguments
    /// * `program_address` - The contract/program address
    /// * `method` - Method/instruction name
    /// * `args` - Serialized arguments
    /// * `signer` - Address that signs the transaction
    ///
    /// # Returns
    /// Transaction signature
    async fn execute_contract(
        &self,
        program_address: &str,
        method: &str,
        args: &[u8],
        signer: &str,
    ) -> Result<String>;

    // ==================== Event Subscriptions ====================

    /// Subscribe to account changes
    ///
    /// # Arguments
    /// * `address` - Account to watch
    /// * `callback` - Function to call on updates
    ///
    /// # Returns
    /// Subscription ID for later unsubscription
    async fn subscribe_account(
        &self,
        address: &str,
    ) -> Result<u64>;

    /// Unsubscribe from account changes
    async fn unsubscribe(&self, subscription_id: u64) -> Result<()>;

    // ==================== Utility Operations ====================

    /// Get the current block height/slot
    async fn get_block_height(&self) -> Result<u64>;

    /// Get recent block hash (for transaction construction)
    async fn get_recent_blockhash(&self) -> Result<String>;

    /// Estimate transaction fee
    ///
    /// # Arguments
    /// * `transaction` - The transaction to estimate
    ///
    /// # Returns
    /// Estimated fee in smallest unit
    async fn estimate_fee(&self, transaction: &[u8]) -> Result<u64>;
}
