//! Solana-specific extensions to ChainClient
//!
//! This module provides Solana-specific operations that are not part of the
//! core ChainClient trait (which must remain chain-agnostic).

use crate::{ChainClientError, Result};
use async_trait::async_trait;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_sdk::{account::Account, pubkey::Pubkey};

/// Solana-specific operations trait
///
/// This trait extends ChainClient with Solana-specific RPC methods that don't
/// have equivalents on other chains (Starknet, Aptos).
#[async_trait]
pub trait SolanaSpecificOps: Send + Sync {
    /// Get all accounts owned by a specific program
    ///
    /// This is used for syncing job accounts in blink-server.
    ///
    /// # Arguments
    /// * `program_id` - The program pubkey
    /// * `config` - Optional RPC configuration for filtering
    ///
    /// # Returns
    /// Vector of (Pubkey, Account) pairs
    async fn get_program_accounts(
        &self,
        program_id: &Pubkey,
        config: Option<RpcProgramAccountsConfig>,
    ) -> Result<Vec<(Pubkey, Account)>>;

    /// Get multiple accounts in a single RPC call
    ///
    /// # Arguments
    /// * `pubkeys` - Slice of pubkeys to fetch
    ///
    /// # Returns
    /// Vector of Option<Account> (None if account doesn't exist)
    async fn get_multiple_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> Result<Vec<Option<Account>>>;

    /// Get account info with custom config
    ///
    /// # Arguments
    /// * `pubkey` - The account pubkey
    /// * `config` - RPC account info configuration
    ///
    /// # Returns
    /// Account data
    async fn get_account_with_config(
        &self,
        pubkey: &Pubkey,
        config: RpcAccountInfoConfig,
    ) -> Result<Account>;
}
