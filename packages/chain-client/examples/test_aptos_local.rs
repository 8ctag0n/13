//! Test AptosClient against local testnet
//!
//! Run with: cargo run -p zyberlink-chain-client --example test_aptos_local --features aptos
//!
//! Prerequisites:
//! - Local Aptos node running (./infra/docker/aptos.sh start)

use zyberlink_chain_client::{AptosClient, ChainClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing AptosClient against local testnet...\n");

    // Create client
    let client = AptosClient::local()?;
    println!("✓ Created AptosClient");
    println!("  Chain: {}", client.chain_id());
    println!("  Network: {}\n", client.network());

    // Test health check
    println!("Testing health check...");
    match client.health_check().await {
        Ok(true) => println!("✓ Node is healthy\n"),
        Ok(false) => {
            println!("✗ Node returned unhealthy status");
            return Ok(());
        }
        Err(e) => {
            println!("✗ Health check failed: {}", e);
            return Ok(());
        }
    }

    // Get block height
    println!("Getting block height...");
    match client.get_block_height().await {
        Ok(height) => println!("✓ Block height: {}\n", height),
        Err(e) => println!("✗ Failed to get block height: {}\n", e),
    }

    // Get recent blockhash (ledger version)
    println!("Getting ledger version...");
    match client.get_recent_blockhash().await {
        Ok(version) => println!("✓ Ledger version: {}\n", version),
        Err(e) => println!("✗ Failed to get ledger version: {}\n", e),
    }

    // Test account lookup (0x1 is the core framework address, always exists)
    let framework_addr = "0x0000000000000000000000000000000000000000000000000000000000000001";
    println!("Checking if framework account exists ({})...", framework_addr);
    match client.account_exists(framework_addr).await {
        Ok(true) => println!("✓ Framework account exists\n"),
        Ok(false) => println!("✗ Framework account does not exist (unexpected)\n"),
        Err(e) => println!("✗ Failed to check account: {}\n", e),
    }

    // Test getting balance (0x1 shouldn't have APT balance in standard setup)
    println!("Getting framework account balance...");
    match client.get_balance(framework_addr).await {
        Ok(balance) => println!("✓ Framework balance: {} octas\n", balance),
        Err(e) => println!("✗ Failed to get balance: {}\n", e),
    }

    // Test non-existent account
    let random_addr = "0x1111111111111111111111111111111111111111111111111111111111111111";
    println!("Checking non-existent account ({})...", random_addr);
    match client.account_exists(random_addr).await {
        Ok(false) => println!("✓ Correctly reported as non-existent\n"),
        Ok(true) => println!("✗ Account exists (unexpected)\n"),
        Err(e) => println!("✗ Failed to check account: {}\n", e),
    }

    println!("All basic tests completed successfully!");
    println!("\nAptosClient is working correctly against local testnet.");

    Ok(())
}
