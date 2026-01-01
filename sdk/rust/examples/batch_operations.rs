//! Example: BatchBuilder - Parallel FHE Operations
//!
//! This example shows how to use the BatchBuilder to:
//! 1. Queue multiple FHE operations
//! 2. Execute them in parallel for reduced latency
//! 3. Handle mixed success/failure results

use anyhow::Result;
use std::time::Duration;
use zyberlink_sdk::Zyber;

#[tokio::main]
async fn main() -> Result<()> {
    // Connect to network
    let mut zyber = Zyber::builder()
        .network("localnet")
        .timeout(Duration::from_secs(60))
        .provers(2)
        .price_sol(0.01)
        .build()
        .await?;

    println!("Connected to network: {}", zyber.network());

    // Sample data sets
    let sales_data = vec![100u8, 150, 200, 175, 225];
    let temperature_readings = vec![22u8, 24, 23, 25, 21, 26];
    let age_data = vec![18u8, 25, 30, 17, 45, 16, 22];

    // =========================================================================
    // Example 1: Build a batch
    // =========================================================================
    println!("\n--- Example 1: Building a Batch ---");

    let batch = zyber.batch()
        .sum(&sales_data)           // Job 1: Total sales
        .average(&temperature_readings)  // Job 2: Average temperature
        .count_if(&age_data, ">=", 18);  // Job 3: Count adults

    println!("Batch created with {} operations:", batch.len());
    for (i, desc) in batch.describe().iter().enumerate() {
        println!("  [{}] {}", i, desc);
    }

    // =========================================================================
    // Example 2: Estimate costs before execution
    // =========================================================================
    println!("\n--- Example 2: Cost Estimation ---");

    let cost = batch.estimate_total_cost();
    println!("Estimated total cost:");
    println!("  Job creation: {} lamports", cost.job_creation_lamports);
    println!("  Prover payments: {} lamports", cost.prover_payment_lamports);
    println!("  TX fees: {} lamports", cost.tx_fee_lamports);
    println!("  TOTAL: {} lamports ({:.6} SOL)", cost.total_lamports, cost.total_sol());

    // =========================================================================
    // Example 3: Parallel vs Sequential
    // =========================================================================
    println!("\n--- Example 3: Execution Modes ---");
    println!("
Parallel execution (execute_parallel):
  - All jobs submitted simultaneously
  - Total time ≈ max(job times)
  - Best for independent operations

Sequential execution (execute_sequential):
  - Jobs run one after another
  - Total time = sum(job times)
  - Can stop on first error with .fail_fast()
");

    // =========================================================================
    // Example 4: Handling results
    // =========================================================================
    println!("--- Example 4: Result Handling ---");
    println!("
// Execute and get summary
let summary = zyber.batch()
    .sum(&data1)
    .average(&data2)
    .execute_parallel().await?;

// Check if all succeeded
if summary.all_ok() {{
    let values = summary.unwrap_all();
    println!(\"Results: {{:?}}\", values);
}}

// Or handle mixed results
for (idx, value) in summary.successes() {{
    println!(\"Op {{}} succeeded: {{}}\", idx, value);
}}
for (idx, error) in summary.failures() {{
    println!(\"Op {{}} failed: {{}}\", idx, error);
}}

// Individual access
let first_result = summary.results[0].get()?;
");

    // =========================================================================
    // Example 5: Fail-fast mode
    // =========================================================================
    println!("--- Example 5: Fail-Fast Mode ---");
    println!("
// Stop on first error (sequential only)
let summary = zyber.batch()
    .sum(&data1)
    .sum(&data2)
    .sum(&data3)
    .fail_fast()
    .execute_sequential().await?;

if !summary.all_ok() {{
    println!(\"Stopped at operation {{}}\", summary.results.len());
}}
");

    // =========================================================================
    // Example 6: Real-world use case
    // =========================================================================
    println!("--- Example 6: Real-World Use Case ---");
    println!("
// Analytics dashboard: compute multiple metrics in parallel
let metrics = zyber.batch()
    .sum(&daily_transactions)      // Total volume
    .average(&response_times)       // Avg latency
    .count_if(&error_codes, \">\", 0) // Error count
    .count_if(&user_ages, \">=\", 18) // Adult users
    .execute_parallel().await?;

let [volume, latency, errors, adults] = metrics.unwrap_all()[..] else {{
    panic!(\"Expected 4 results\");
}};

println!(\"Daily Report:\");
println!(\"  Volume: ${{}}\", volume);
println!(\"  Avg Latency: {{}}ms\", latency);
println!(\"  Errors: {{}}\", errors);
println!(\"  Adult Users: {{}}\", adults);
");

    // =========================================================================
    // Summary
    // =========================================================================
    println!("--- API Summary ---");
    println!("
BatchBuilder methods:
  .sum(values)              - Add sum operation
  .average(values)          - Add average operation
  .count_if(values, op, n)  - Add count_if operation

  .len()                    - Number of pending operations
  .is_empty()               - Check if empty
  .describe()               - Get operation descriptions
  .estimate_total_cost()    - Estimate total cost

  .continue_on_error(bool)  - Set error handling (default: true)
  .fail_fast()              - Stop on first error

  .execute_parallel()       - Run all in parallel
  .execute_sequential()     - Run one by one

BatchSummary methods:
  .all_ok()                 - Check if all succeeded
  .unwrap_all()             - Get all values (panics on error)
  .get_all()                - Get all values (returns Result)
  .successes()              - Iterate successful results
  .failures()               - Iterate failed results
  .results[i].get()         - Get individual result
");

    println!("\n(Skipping actual execution - uncomment to run on live network)");

    Ok(())
}
