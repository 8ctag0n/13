//! pBTCFi Prover Binary
//!
//! Standalone prover for pBTCFi FHE jobs.
//! Polls blink-server for pending loans and processes FHE computations.
//!
//! Usage:
//!   pbtcfi-prover --server http://localhost:8080 --prover-id my-prover
//!   pbtcfi-prover --config /path/to/config.json

use anyhow::{Context, Result};
use clap::Parser;
use std::time::Duration;

use prover_node::services::{PbtcfiProcessor, PbtcfiLoanParams};

/// pBTCFi FHE Prover
#[derive(Parser, Debug)]
#[command(name = "pbtcfi-prover")]
#[command(about = "FHE prover for pBTCFi loans")]
#[command(version)]
struct Args {
    /// Blink server URL
    #[arg(short, long, env = "BLINK_SERVER_URL", default_value = "http://localhost:8080")]
    server: String,

    /// Prover identifier (unique per prover instance)
    #[arg(short, long, env = "PROVER_ID")]
    prover_id: Option<String>,

    /// Poll interval in seconds
    #[arg(long, default_value = "10")]
    poll_interval: u64,

    /// Maximum concurrent jobs
    #[arg(long, default_value = "3")]
    max_concurrent: usize,

    /// BTC price in USD (for collateral calculation)
    #[arg(long, env = "BTC_PRICE_USD", default_value = "45000")]
    btc_price: u64,

    /// Loan-to-value ratio percentage
    #[arg(long, env = "LTV_PERCENT", default_value = "70")]
    ltv: u8,

    /// pLST conversion factor
    #[arg(long, default_value = "1")]
    plst_factor: u8,

    /// Run in dry-run mode (don't submit results)
    #[arg(long)]
    dry_run: bool,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.verbose { "debug" } else { "info" };
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(log_level)
    ).init();

    log::info!("===========================================");
    log::info!("       pBTCFi FHE Prover v0.1.0");
    log::info!("===========================================");

    // Generate prover ID if not provided
    let prover_id = args.prover_id.unwrap_or_else(|| {
        let id = format!("pbtcfi-prover-{}", &uuid_simple()[..8]);
        log::info!("Generated prover ID: {}", id);
        id
    });

    // Display configuration
    log::info!("Configuration:");
    log::info!("  Server URL: {}", args.server);
    log::info!("  Prover ID: {}", prover_id);
    log::info!("  Poll interval: {}s", args.poll_interval);
    log::info!("  Max concurrent: {}", args.max_concurrent);
    log::info!("  BTC Price: ${}", args.btc_price);
    log::info!("  LTV: {}%", args.ltv);
    log::info!("  Dry run: {}", args.dry_run);
    log::info!("-------------------------------------------");

    if args.dry_run {
        log::warn!("DRY RUN MODE - results will NOT be submitted");
    }

    // Create loan parameters
    let params = PbtcfiLoanParams {
        btc_price_usd: args.btc_price,
        ltv_percent: args.ltv,
        plst_factor: args.plst_factor,
    };

    // Create processor
    let processor = PbtcfiProcessor::new(&args.server, &prover_id)
        .with_params(params)
        .with_poll_interval(Duration::from_secs(args.poll_interval))
        .with_max_concurrent(args.max_concurrent);

    // Run the processor
    log::info!("Starting pBTCFi job processor...");
    processor.run().await.context("Processor failed")?;

    Ok(())
}

/// Generate a simple UUID-like string
fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    format!("{:016x}", timestamp)
}
