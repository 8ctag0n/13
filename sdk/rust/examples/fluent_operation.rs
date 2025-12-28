//! Example: FluentOperation - Retry logic and error handling
//!
//! This example shows how to use the Layer 1 API to:
//! 1. Add retry logic to operations
//! 2. Configure custom error handlers
//! 3. Use exponential backoff

use anyhow::Result;
use std::time::Duration;
use zyberlink_sdk::{ErrorAction, RetryPolicy, Zyber};

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

    // Sample values
    let values = vec![10u8, 20, 30, 40, 50];

    // =========================================================================
    // Example 1: Simple retry
    // =========================================================================
    println!("\n--- Example 1: Simple Retry ---");
    println!("Creating operation with 3 retries...");

    let fluent = zyber.sum_fluent(&values).await?;

    // Inspect before executing
    println!("Job ID: {}", fluent.job_id());
    println!("Estimated cost: {:.6} SOL", fluent.estimate_cost().total_sol());

    // Configure and show (not executing for safety)
    let _configured = fluent
        .with_retry(3)
        .with_timeout(Duration::from_secs(30));

    println!("Configured: 3 retries, 30s timeout");
    println!("(Skipping execution in example)");

    // =========================================================================
    // Example 2: Custom error handler
    // =========================================================================
    println!("\n--- Example 2: Custom Error Handler ---");

    let fluent2 = zyber.sum_fluent(&values).await?;

    let _configured2 = fluent2
        .with_retry(5)
        .on_error(|e| {
            println!("Error occurred: {}", e.inner);
            println!("  Retry count: {}", e.retry_count);
            println!("  Elapsed: {:?}", e.elapsed);

            if e.is_timeout() {
                println!("  -> Timeout detected, will retry");
                ErrorAction::Retry
            } else if e.is_network() {
                println!("  -> Network issue, will retry");
                ErrorAction::Retry
            } else if e.is_rate_limited() {
                println!("  -> Rate limited, will retry");
                ErrorAction::Retry
            } else {
                println!("  -> Unrecoverable error, failing");
                ErrorAction::Fail
            }
        });

    println!("Custom error handler configured");

    // =========================================================================
    // Example 3: Exponential backoff
    // =========================================================================
    println!("\n--- Example 3: Exponential Backoff ---");

    let fluent3 = zyber.average_fluent(&values).await?;

    let policy = RetryPolicy {
        max_retries: 5,
        initial_delay: Duration::from_millis(500),
        backoff_multiplier: 2.0,
        max_delay: Duration::from_secs(30),
        jitter: true,
    };

    println!("Retry policy:");
    println!("  Max retries: {}", policy.max_retries);
    println!("  Initial delay: {:?}", policy.initial_delay);
    println!("  Backoff multiplier: {}", policy.backoff_multiplier);
    println!("  Max delay: {:?}", policy.max_delay);
    println!("  Jitter: {}", policy.jitter);

    // Show delay progression
    println!("\nDelay progression:");
    for i in 0..=5 {
        let delay = policy.delay_for_attempt(i);
        println!("  Attempt {}: {:?}", i, delay);
    }

    let _configured3 = fluent3
        .with_retry_policy(policy)
        .retry_transient(); // Only retry transient errors

    // =========================================================================
    // Example 4: Success callback
    // =========================================================================
    println!("\n--- Example 4: Success Callback ---");

    let fluent4 = zyber.count_if_fluent(&values, ">", 25).await?;

    let _configured4 = fluent4
        .with_retry(2)
        .on_success_callback(|result| {
            println!("Operation succeeded!");
            println!("Result: {} values matched the condition", result);
        });

    println!("Success callback configured");

    // =========================================================================
    // Example 5: Using RetryPolicy presets
    // =========================================================================
    println!("\n--- Example 5: Retry Policy Presets ---");

    println!("RetryPolicy::none():");
    let none = RetryPolicy::none();
    println!("  Max retries: {}", none.max_retries);

    println!("\nRetryPolicy::fixed(3, 1s):");
    let fixed = RetryPolicy::fixed(3, Duration::from_secs(1));
    println!("  Max retries: {}", fixed.max_retries);
    println!("  Delay: always {:?}", fixed.initial_delay);

    println!("\nRetryPolicy::exponential(5):");
    let exp = RetryPolicy::exponential(5);
    println!("  Max retries: {}", exp.max_retries);
    println!("  Initial: {:?}, multiplier: {}", exp.initial_delay, exp.backoff_multiplier);

    // =========================================================================
    // Summary
    // =========================================================================
    println!("\n--- API Summary ---");
    println!("
Layer 1 (Fluent) API methods:
  .with_retry(n)           - Set max retries
  .with_retry_policy(p)    - Use custom RetryPolicy
  .with_backoff(d, m)      - Configure backoff (initial delay, multiplier)
  .no_retry()              - Disable retries
  .with_timeout(d)         - Override timeout
  .on_error(f)             - Custom error handler
  .retry_transient()       - Only retry timeout/network/rate-limit
  .on_success_callback(f)  - Called on success
  .execute().await         - Run the operation
");

    Ok(())
}
