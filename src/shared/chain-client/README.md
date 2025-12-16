# ZyberLink Chain Client - Multi-Chain Abstraction Layer

[![Status](https://img.shields.io/badge/status-production%20ready-brightgreen)]()
[![Interface](https://img.shields.io/badge/interface-LOCKED-red)]()

Multi-chain abstraction layer for ZyberLink infrastructure. Provides unified interface for Solana, Starknet, and Aptos blockchains.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Available Implementations](#available-implementations)
- [Usage Examples](#usage-examples)
- [For Verticales](#for-verticales)
- [Migration Guide](#migration-guide)
- [API Reference](#api-reference)

---

## Overview

### What is ChainClient?

`ChainClient` is a **LOCKED** trait that provides a unified interface for interacting with multiple blockchains. This allows:

- **Verticales** to develop using `MockChainClient` without blockchain dependencies
- **Infrastructure** to implement real chain clients (`SolanaClient`, `StarknetClient`, `AptosClient`)
- **Production** code to switch between chains with zero code changes

### Architecture

```
ChainClient Trait (LOCKED - no breaking changes)
      │
      ├── Account operations (balance, data, exists)
      ├── Transaction operations (send, confirm, status)
      ├── Token transfers
      ├── Smart contract calls
      └── Event subscriptions
      │
      ▼
┌──────────────┬──────────────┬──────────────┬──────────────┐
│ MockClient   │ SolanaClient │StarknetClient│ AptosClient  │
│ (Ready)      │ (Ready)      │ (Planned)    │ (Planned)    │
└──────────────┴──────────────┴──────────────┴──────────────┘
```

---

## Quick Start

### 1. Add Dependency

```toml
[dependencies]
zyberlink-chain-client = { workspace = true, features = ["solana"] }
```

### 2. Use MockChainClient (Development)

```rust
use zyberlink_chain_client::{ChainClient, MockChainClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = MockChainClient::new();

    // Mock client comes with pre-funded accounts
    let balance = client.get_balance("mock_user_1").await?;
    println!("Balance: {}", balance); // 1,000,000,000

    // Transfer tokens
    let sig = client.transfer("mock_user_1", "mock_user_2", 100_000).await?;
    println!("Transfer signature: {}", sig);

    Ok(())
}
```

### 3. Use SolanaClient (Production)

```rust
use zyberlink_chain_client::{ChainClient, SolanaClient};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Arc::new(SolanaClient::devnet()?);

    // Same API as MockChainClient!
    let balance = client.get_balance("your_address_here").await?;
    println!("Balance: {} lamports", balance);

    Ok(())
}
```

---

## Available Implementations

### MockChainClient (Ready for Development)

**Status:** Production Ready
**Purpose:** Testing and development
**Features:** In-memory state, pre-funded accounts, instant confirmations

```rust
let client = MockChainClient::new();

// Pre-funded accounts
client.get_balance("mock_user_1").await?; // 1B units
client.get_balance("mock_user_2").await?; // 500M units
client.get_balance("mock_program").await?; // Program account

// Add custom accounts
client.add_account("my_account", 1_000_000, vec![1, 2, 3]);

// Simulate block progression
client.advance_blocks(10);
```

### SolanaClient (Ready for Production)

**Status:** Production Ready
**Purpose:** Solana mainnet/devnet/testnet/localnet
**Features:** Full RPC integration, async operations

```rust
// Create client
let client = SolanaClient::devnet()?;
// or
let client = SolanaClient::mainnet()?;
// or
let client = SolanaClient::new("https://api.devnet.solana.com")?;

// Query blockchain
let balance = client.get_balance("address").await?;
let data = client.get_account_data("address").await?;
let block_height = client.get_block_height().await?;

// Transactions
let signature = client.send_transaction(&tx_bytes).await?;
let confirmed = client.confirm_transaction(&signature, 60).await?;
```

#### Solana-Specific Operations

For Solana-specific RPC methods, use `SolanaSpecificOps`:

```rust
use zyberlink_chain_client::{SolanaClient, SolanaSpecificOps};

let client = SolanaClient::devnet()?;

// Get all program accounts
let accounts = client.get_program_accounts(&program_id, None).await?;

// Get multiple accounts
let accounts = client.get_multiple_accounts(&[pubkey1, pubkey2]).await?;
```

### StarknetClient (Planned - Day 2-6)

**Status:** Placeholder
**Purpose:** Starknet testnet/mainnet

```rust
let client = StarknetClient::testnet()?;
// All ChainClient methods return NotImplemented for now
```

### AptosClient (Planned - Day 2-6)

**Status:** Placeholder
**Purpose:** Aptos devnet/testnet/mainnet

```rust
let client = AptosClient::devnet()?;
// All ChainClient methods return NotImplemented for now
```

---

## Usage Examples

### For Verticales: Start with MockChainClient

```rust
use zyberlink_chain_client::{ChainClient, MockChainClient};
use std::sync::Arc;

/// Your vertical service
pub struct MyVerticalService {
    chain_client: Arc<dyn ChainClient>,
}

impl MyVerticalService {
    /// For development/testing
    pub fn new_mock() -> Self {
        Self {
            chain_client: Arc::new(MockChainClient::new()),
        }
    }

    /// For production (when ready)
    pub fn new_solana(rpc_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            chain_client: Arc::new(SolanaClient::new(rpc_url)?),
        })
    }

    pub async fn check_balance(&self, address: &str) -> Result<u64, Box<dyn std::error::Error>> {
        Ok(self.chain_client.get_balance(address).await?)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Development: use mock
    let service = MyVerticalService::new_mock();
    let balance = service.check_balance("mock_user_1").await?;
    println!("Mock balance: {}", balance);

    // Production: switch to real chain (no code changes!)
    let service = MyVerticalService::new_solana("https://api.devnet.solana.com")?;
    let balance = service.check_balance("real_address").await?;
    println!("Real balance: {}", balance);

    Ok(())
}
```

### blink-server Example (Real Integration)

See `src/blink-server/src/chain_sync.rs` for production example:

```rust
use zyberlink_chain_client::{ChainClient, SolanaClient, SolanaSpecificOps};
use std::sync::Arc;

pub fn start_chain_sync(
    client: Arc<SolanaClient>,
    program_id: Pubkey,
    db_pool: PgPool,
) {
    tokio::spawn(async move {
        log::info!("Chain: {}", client.chain_id());
        log::info!("Network: {}", client.network());

        loop {
            // Use Solana-specific operations
            let accounts = client
                .get_program_accounts(&program_id, Some(config))
                .await?;

            // Process accounts...
            tokio::time::sleep(Duration::from_secs(8)).await;
        }
    });
}
```

---

## For Verticales

### Quick Start Guide

1. **Day 1: Start Development**
   ```bash
   # Add dependency
   zyberlink-chain-client = { workspace = true }
   ```

   ```rust
   use zyberlink_chain_client::{ChainClient, MockChainClient};

   let client = MockChainClient::new();
   // Start coding immediately!
   ```

2. **Day 2-6: Infrastructure Builds Real Chains**
   - Infrastructure team implements `SolanaClient`, `StarknetClient`, `AptosClient`
   - Verticales keep using `MockChainClient` - zero impact

3. **Day 7+: Switch to Real Chain**
   ```rust
   // Change 1 line:
   let client = Arc::new(SolanaClient::devnet()?);
   // Everything else works!
   ```

### Development Workflow

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use zyberlink_chain_client::MockChainClient;

    #[tokio::test]
    async fn test_my_feature() {
        let client = MockChainClient::new();
        client.add_account("test_account", 1_000_000, vec![]);

        let balance = client.get_balance("test_account").await.unwrap();
        assert_eq!(balance, 1_000_000);
    }
}
```

---

## Migration Guide

### From Direct RpcClient to ChainClient

**Before:**
```rust
use solana_client::rpc_client::RpcClient;

let rpc_client = RpcClient::new("https://api.devnet.solana.com");
let balance = rpc_client.get_balance(&pubkey)?;
```

**After:**
```rust
use zyberlink_chain_client::{ChainClient, SolanaClient};

let client = SolanaClient::new("https://api.devnet.solana.com")?;
let balance = client.get_balance(&pubkey_str).await?;
```

### Key Differences

1. **Async/Await**: All operations are async
2. **String Addresses**: Use string addresses instead of `Pubkey` (parsed internally)
3. **Error Handling**: Uses `ChainClientError` enum
4. **Arc Wrapping**: Clients are typically wrapped in `Arc` for sharing

---

## API Reference

### ChainClient Trait

All implementations must provide:

#### Network & Configuration
- `chain_id() -> &str` - Get chain identifier ("solana", "starknet", etc.)
- `network() -> &str` - Get network ("mainnet", "devnet", "testnet")
- `health_check() -> Result<bool>` - Check if client is healthy

#### Account Operations
- `get_balance(address: &str) -> Result<u64>` - Get native token balance
- `get_account_data(address: &str) -> Result<Vec<u8>>` - Get account data
- `account_exists(address: &str) -> Result<bool>` - Check if account exists

#### Transaction Operations
- `send_transaction(transaction: &[u8]) -> Result<String>` - Send signed transaction
- `confirm_transaction(signature: &str, timeout_secs: u64) -> Result<bool>` - Wait for confirmation
- `get_transaction_status(signature: &str) -> Result<TransactionStatus>` - Get tx status

#### Token Operations
- `transfer(from: &str, to: &str, amount: u64) -> Result<String>` - Transfer native tokens

#### Smart Contract Operations
- `call_contract(program: &str, method: &str, args: &[u8]) -> Result<Vec<u8>>` - Read-only call
- `execute_contract(program: &str, method: &str, args: &[u8], signer: &str) -> Result<String>` - Write operation

#### Event Subscriptions
- `subscribe_account(address: &str) -> Result<u64>` - Subscribe to account changes
- `unsubscribe(subscription_id: u64) -> Result<()>` - Unsubscribe

#### Utility Operations
- `get_block_height() -> Result<u64>` - Get current block height
- `get_recent_blockhash() -> Result<String>` - Get recent blockhash
- `estimate_fee(transaction: &[u8]) -> Result<u64>` - Estimate transaction fee

### Error Types

```rust
pub enum ChainClientError {
    Network(String),
    InvalidAddress(String),
    AccountNotFound(String),
    TransactionFailed(String),
    TransactionTimeout(u64),
    InsufficientBalance { required: u64, available: u64 },
    ContractCallFailed(String),
    Serialization(String),
    Deserialization(String),
    NotImplemented(String),
    // ... more variants
}
```

---

## Support & Contributing

### Questions?
- Check `src/shared/chain-client/src/lib.rs` for trait definition
- See `src/blink-server/src/chain_sync.rs` for real-world usage
- Review tests in `src/shared/chain-client/src/mock.rs`

### Contributing
1. **DO NOT** modify `ChainClient` trait (interface is LOCKED)
2. **DO** add Solana-specific operations to `SolanaSpecificOps` trait
3. **DO** add chain-specific traits for Starknet/Aptos features

### Status

- **ChainClient Trait**: LOCKED - No breaking changes allowed
- **MockChainClient**: Production ready
- **SolanaClient**: Production ready
- **SolanaSpecificOps**: Production ready
- **StarknetClient**: Planned (Day 2-6)
- **AptosClient**: Planned (Day 2-6)

---

**Last Updated:** 2025-12-15
**Maintained by:** ZyberLink Infrastructure Team
**Branch:** `infrastructure/multi-chain`
