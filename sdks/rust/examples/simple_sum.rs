//! Simple example demonstrating the high-level Zyber API
//!
//! This example shows how easy it is to use ZyberLink for
//! privacy-preserving computation.
//!
//! # Usage
//! ```bash
//! # Against localnet (default)
//! cargo run --example simple_sum
//!
//! # Against devnet
//! cargo run --example simple_sum -- devnet
//! ```

use anyhow::Result;
use zyberlink_sdk::Zyber;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    // Get network from args (default: localnet)
    let network = std::env::args().nth(1).unwrap_or_else(|| "localnet".to_string());

    println!("ZyberLink Simple Sum Example");
    println!("============================");
    println!();

    // Connect to network - this is all you need!
    println!("Connecting to {}...", network);
    let mut zyber = Zyber::connect(&network).await?;
    println!("Connected as: {}", zyber.pubkey());
    println!();

    // Example 1: Sum
    println!("Example 1: Computing sum of [10, 20, 30, 40, 50]");
    let values = [10u8, 20, 30, 40, 50];
    let sum = zyber.sum(&values).await?;
    println!("  Result: {} (expected: 150)", sum);
    println!();

    // Example 2: Average
    println!("Example 2: Computing average of [10, 20, 30]");
    let values2 = [10u8, 20, 30];
    let avg = zyber.average(&values2).await?;
    println!("  Result: {} (expected: 20)", avg);
    println!();

    // Example 3: Count If
    println!("Example 3: Counting values > 25 in [10, 20, 30, 40, 50]");
    let count = zyber.count_if(&values, ">", 25).await?;
    println!("  Result: {} (expected: 3)", count);
    println!();

    // Example 4: Threshold check
    println!("Example 4: Checking if any value >= 50");
    let has_fifty = zyber.threshold(&values, ">=", 50).await?;
    println!("  Result: {} (expected: true)", has_fifty);
    println!();

    println!("All examples completed successfully!");
    println!();
    println!("Notice how simple the API is:");
    println!("  let zyber = Zyber::connect(\"devnet\").await?;");
    println!("  let sum = zyber.sum(&values).await?;");
    println!();
    println!("That's it! All FHE encryption, witness upload, job creation,");
    println!("and result polling is handled automatically.");

    Ok(())
}
