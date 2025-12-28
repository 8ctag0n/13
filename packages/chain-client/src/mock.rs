//! Mock implementation of ChainClient for testing and development
//!
//! This mock client is used by verticals during development while real chain
//! implementations are being built. It simulates chain operations with in-memory state.

use crate::{ChainClient, ChainClientError, Result, TransactionStatus};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Mock chain client for testing and development
///
/// # Example
///
/// ```rust
/// use zyberlink_chain_client::{MockChainClient, ChainClient};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = MockChainClient::new();
///
///     // Mock client starts with default accounts
///     let balance = client.get_balance("mock_user_1").await?;
///     println!("Balance: {}", balance);
///
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct MockChainClient {
    state: Arc<Mutex<MockState>>,
    chain_id: String,
    network: String,
}

#[derive(Debug)]
struct MockState {
    accounts: HashMap<String, Account>,
    transactions: HashMap<String, Transaction>,
    subscriptions: HashMap<u64, String>,
    next_subscription_id: u64,
    block_height: u64,
    recent_blockhash: String,
}

#[derive(Debug, Clone)]
struct Account {
    balance: u64,
    data: Vec<u8>,
    owner: Option<String>,
}

#[derive(Debug, Clone)]
struct Transaction {
    signature: String,
    status: TransactionStatus,
    from: String,
    to: String,
    amount: u64,
    block_height: u64,
    fee: u64,
}

impl MockChainClient {
    /// Create a new mock client with default state
    pub fn new() -> Self {
        Self::with_network("mock", "devnet")
    }

    /// Create a new mock client with custom chain and network
    pub fn with_network(chain_id: &str, network: &str) -> Self {
        let mut accounts = HashMap::new();

        // Create some default mock accounts
        accounts.insert(
            "mock_user_1".to_string(),
            Account {
                balance: 1_000_000_000, // 1 billion base units
                data: vec![],
                owner: None,
            },
        );
        accounts.insert(
            "mock_user_2".to_string(),
            Account {
                balance: 500_000_000,
                data: vec![],
                owner: None,
            },
        );
        accounts.insert(
            "mock_program".to_string(),
            Account {
                balance: 0,
                data: vec![1, 2, 3, 4], // Some dummy program data
                owner: Some("system".to_string()),
            },
        );

        let state = MockState {
            accounts,
            transactions: HashMap::new(),
            subscriptions: HashMap::new(),
            next_subscription_id: 1,
            block_height: 1000,
            recent_blockhash: "mock_blockhash_12345".to_string(),
        };

        Self {
            state: Arc::new(Mutex::new(state)),
            chain_id: chain_id.to_string(),
            network: network.to_string(),
        }
    }

    /// Add a mock account for testing
    pub fn add_account(&self, address: &str, balance: u64, data: Vec<u8>) {
        let mut state = self.state.lock().unwrap();
        state.accounts.insert(
            address.to_string(),
            Account {
                balance,
                data,
                owner: None,
            },
        );
    }

    /// Set balance for an existing account
    pub fn set_balance(&self, address: &str, balance: u64) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        let account = state
            .accounts
            .get_mut(address)
            .ok_or_else(|| ChainClientError::AccountNotFound(address.to_string()))?;
        account.balance = balance;
        Ok(())
    }

    /// Get all transactions (for testing)
    pub fn get_all_transactions(&self) -> Vec<String> {
        let state = self.state.lock().unwrap();
        state.transactions.keys().cloned().collect()
    }

    /// Simulate block progression
    pub fn advance_blocks(&self, count: u64) {
        let mut state = self.state.lock().unwrap();
        state.block_height += count;
        state.recent_blockhash = format!("mock_blockhash_{}", state.block_height);
    }
}

impl Default for MockChainClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ChainClient for MockChainClient {
    fn chain_id(&self) -> &str {
        &self.chain_id
    }

    fn network(&self) -> &str {
        &self.network
    }

    async fn health_check(&self) -> Result<bool> {
        // Mock is always healthy
        Ok(true)
    }

    async fn get_balance(&self, address: &str) -> Result<u64> {
        let state = self.state.lock().unwrap();
        let account = state
            .accounts
            .get(address)
            .ok_or_else(|| ChainClientError::AccountNotFound(address.to_string()))?;
        Ok(account.balance)
    }

    async fn get_account_data(&self, address: &str) -> Result<Vec<u8>> {
        let state = self.state.lock().unwrap();
        let account = state
            .accounts
            .get(address)
            .ok_or_else(|| ChainClientError::AccountNotFound(address.to_string()))?;
        Ok(account.data.clone())
    }

    async fn account_exists(&self, address: &str) -> Result<bool> {
        let state = self.state.lock().unwrap();
        Ok(state.accounts.contains_key(address))
    }

    async fn send_transaction(&self, _transaction: &[u8]) -> Result<String> {
        // Generate a mock signature
        let signature = format!("mock_tx_{}", uuid::Uuid::new_v4());

        let mut state = self.state.lock().unwrap();
        let block_height = state.block_height;
        state.transactions.insert(
            signature.clone(),
            Transaction {
                signature: signature.clone(),
                status: TransactionStatus::Pending,
                from: "mock_sender".to_string(),
                to: "mock_receiver".to_string(),
                amount: 0,
                block_height,
                fee: 5000,
            },
        );

        Ok(signature)
    }

    async fn confirm_transaction(&self, signature: &str, timeout_secs: u64) -> Result<bool> {
        // Simulate confirmation delay
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let mut state = self.state.lock().unwrap();
        if let Some(tx) = state.transactions.get_mut(signature) {
            tx.status = TransactionStatus::Confirmed;
            Ok(true)
        } else {
            // If timeout_secs is 0, return false immediately
            if timeout_secs == 0 {
                Ok(false)
            } else {
                Err(ChainClientError::TransactionFailed(
                    "Transaction not found".to_string(),
                ))
            }
        }
    }

    async fn get_transaction_status(&self, signature: &str) -> Result<TransactionStatus> {
        let state = self.state.lock().unwrap();
        state
            .transactions
            .get(signature)
            .map(|tx| tx.status)
            .ok_or_else(|| ChainClientError::TransactionFailed("Not found".to_string()))
    }

    async fn transfer(&self, from: &str, to: &str, amount: u64) -> Result<String> {
        // Check balance
        let mut state = self.state.lock().unwrap();

        let from_account = state
            .accounts
            .get(from)
            .ok_or_else(|| ChainClientError::AccountNotFound(from.to_string()))?;

        if from_account.balance < amount {
            return Err(ChainClientError::InsufficientBalance {
                required: amount,
                available: from_account.balance,
            });
        }

        // Ensure recipient exists
        if !state.accounts.contains_key(to) {
            state.accounts.insert(
                to.to_string(),
                Account {
                    balance: 0,
                    data: vec![],
                    owner: None,
                },
            );
        }

        // Execute transfer
        let from_account = state.accounts.get_mut(from).unwrap();
        from_account.balance -= amount;

        let to_account = state.accounts.get_mut(to).unwrap();
        to_account.balance += amount;

        // Create transaction record
        let signature = format!("mock_transfer_{}", uuid::Uuid::new_v4());
        let block_height = state.block_height;
        state.transactions.insert(
            signature.clone(),
            Transaction {
                signature: signature.clone(),
                status: TransactionStatus::Confirmed,
                from: from.to_string(),
                to: to.to_string(),
                amount,
                block_height,
                fee: 5000,
            },
        );

        Ok(signature)
    }

    async fn call_contract(
        &self,
        program_address: &str,
        _method: &str,
        _args: &[u8],
    ) -> Result<Vec<u8>> {
        // Return the program's data as mock result
        let state = self.state.lock().unwrap();
        let account = state
            .accounts
            .get(program_address)
            .ok_or_else(|| ChainClientError::AccountNotFound(program_address.to_string()))?;
        Ok(account.data.clone())
    }

    async fn execute_contract(
        &self,
        program_address: &str,
        method: &str,
        _args: &[u8],
        _signer: &str,
    ) -> Result<String> {
        // Verify program exists
        let state = self.state.lock().unwrap();
        if !state.accounts.contains_key(program_address) {
            return Err(ChainClientError::AccountNotFound(program_address.to_string()));
        }
        drop(state);

        // Create transaction
        let signature = format!("mock_contract_{}_{}", method, uuid::Uuid::new_v4());

        let mut state = self.state.lock().unwrap();
        let block_height = state.block_height;
        state.transactions.insert(
            signature.clone(),
            Transaction {
                signature: signature.clone(),
                status: TransactionStatus::Confirmed,
                from: program_address.to_string(),
                to: program_address.to_string(),
                amount: 0,
                block_height,
                fee: 10000,
            },
        );

        Ok(signature)
    }

    async fn subscribe_account(&self, address: &str) -> Result<u64> {
        let mut state = self.state.lock().unwrap();
        let id = state.next_subscription_id;
        state.next_subscription_id += 1;
        state.subscriptions.insert(id, address.to_string());
        Ok(id)
    }

    async fn unsubscribe(&self, subscription_id: u64) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        state.subscriptions.remove(&subscription_id);
        Ok(())
    }

    async fn get_block_height(&self) -> Result<u64> {
        let state = self.state.lock().unwrap();
        Ok(state.block_height)
    }

    async fn get_recent_blockhash(&self) -> Result<String> {
        let state = self.state.lock().unwrap();
        Ok(state.recent_blockhash.clone())
    }

    async fn estimate_fee(&self, _transaction: &[u8]) -> Result<u64> {
        // Return a mock fee
        Ok(5000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_client_creation() {
        let client = MockChainClient::new();
        assert_eq!(client.chain_id(), "mock");
        assert_eq!(client.network(), "devnet");
    }

    #[tokio::test]
    async fn test_health_check() {
        let client = MockChainClient::new();
        let result = client.health_check().await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_get_balance() {
        let client = MockChainClient::new();
        let balance = client.get_balance("mock_user_1").await.unwrap();
        assert_eq!(balance, 1_000_000_000);
    }

    #[tokio::test]
    async fn test_transfer() {
        let client = MockChainClient::new();

        let initial_balance = client.get_balance("mock_user_1").await.unwrap();
        let signature = client
            .transfer("mock_user_1", "mock_user_2", 100_000)
            .await
            .unwrap();

        assert!(!signature.is_empty());

        let new_balance = client.get_balance("mock_user_1").await.unwrap();
        assert_eq!(new_balance, initial_balance - 100_000);
    }

    #[tokio::test]
    async fn test_insufficient_balance() {
        let client = MockChainClient::new();

        let result = client
            .transfer("mock_user_1", "mock_user_2", 10_000_000_000)
            .await;

        assert!(matches!(
            result,
            Err(ChainClientError::InsufficientBalance { .. })
        ));
    }

    #[tokio::test]
    async fn test_account_exists() {
        let client = MockChainClient::new();

        let exists = client.account_exists("mock_user_1").await.unwrap();
        assert!(exists);

        let not_exists = client.account_exists("nonexistent").await.unwrap();
        assert!(!not_exists);
    }

    #[tokio::test]
    async fn test_block_advancement() {
        let client = MockChainClient::new();

        let initial_height = client.get_block_height().await.unwrap();
        client.advance_blocks(10);
        let new_height = client.get_block_height().await.unwrap();

        assert_eq!(new_height, initial_height + 10);
    }
}
