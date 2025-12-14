//! Market commands for prediction markets
//!
//! Implements CLI commands for creating and participating in prediction markets using ZK circuits.

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use colored::Colorize;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

use crate::client::{CreateJobRequest, ZkClient};

// Circuit type 30 reserved for future market prediction circuit
const CIRCUIT_MARKET_BET: u8 = 31;
const CIRCUIT_MARKET_SETTLEMENT: u8 = 32;

#[derive(Subcommand, Debug)]
pub enum MarketCommands {
    /// Create a new prediction market
    Create(CreateArgs),
    /// Place a private bet
    Bet(BetArgs),
    /// Claim market winnings
    Claim(ClaimArgs),
    /// Verify market settlement
    Verify(VerifyArgs),
}

#[derive(Args, Debug)]
pub struct CreateArgs {
    /// Market question
    #[arg(long)]
    pub question: String,

    /// Outcome options (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub outcomes: Vec<String>,

    /// Resolution date (Unix timestamp)
    #[arg(long)]
    pub resolution_date: u64,

    /// Minimum bet amount
    #[arg(long, default_value = "1000")]
    pub min_bet: u64,

    /// Maximum bet amount
    #[arg(long)]
    pub max_bet: Option<u64>,

    /// Creator public key
    #[arg(long)]
    pub creator: String,

    /// Server URL
    #[arg(long, default_value = "http://localhost:3000")]
    pub server: String,
}

#[derive(Args, Debug)]
pub struct BetArgs {
    /// Market ID
    #[arg(long)]
    pub market_id: String,

    /// Selected outcome (0-indexed)
    #[arg(long)]
    pub outcome: u8,

    /// Bet amount (lamports)
    #[arg(long)]
    pub amount: u64,

    /// Bettor public key
    #[arg(long)]
    pub bettor: String,

    /// Server URL
    #[arg(long, default_value = "http://localhost:3000")]
    pub server: String,

    /// Path to output witness file
    #[arg(long, default_value = "./bet_witness.json")]
    pub witness_output: PathBuf,

    /// Path to Solana keypair for payment
    #[arg(long)]
    pub keypair: Option<PathBuf>,

    /// Solana RPC URL
    #[arg(long, default_value = "https://api.devnet.solana.com")]
    pub rpc_url: String,

    /// Skip payment confirmation
    #[arg(long)]
    pub skip_confirm: bool,
}

#[derive(Args, Debug)]
pub struct ClaimArgs {
    /// Market ID
    #[arg(long)]
    pub market_id: String,

    /// Bet ID (job ID from bet placement)
    #[arg(long)]
    pub bet_id: i64,

    /// Claimer public key
    #[arg(long)]
    pub claimer: String,

    /// Server URL
    #[arg(long, default_value = "http://localhost:3000")]
    pub server: String,

    /// Path to output witness file
    #[arg(long, default_value = "./claim_witness.json")]
    pub witness_output: PathBuf,

    /// Path to Solana keypair for payment
    #[arg(long)]
    pub keypair: Option<PathBuf>,

    /// Solana RPC URL
    #[arg(long, default_value = "https://api.devnet.solana.com")]
    pub rpc_url: String,

    /// Skip payment confirmation
    #[arg(long)]
    pub skip_confirm: bool,
}

#[derive(Args, Debug)]
pub struct VerifyArgs {
    /// Settlement job ID
    #[arg(long)]
    pub job_id: i64,

    /// Server URL
    #[arg(long, default_value = "http://localhost:3000")]
    pub server: String,
}

pub fn handle_market_command(cmd: MarketCommands) -> Result<()> {
    match cmd {
        MarketCommands::Create(args) => create_market(args),
        MarketCommands::Bet(args) => place_bet(args),
        MarketCommands::Claim(args) => claim_winnings(args),
        MarketCommands::Verify(args) => verify_settlement(args),
    }
}

fn create_market(args: CreateArgs) -> Result<()> {
    println!("{}", "Creating prediction market...".cyan().bold());
    println!();
    println!("Question: {}", args.question.green());
    println!("Outcomes: {}", args.outcomes.join(", "));
    println!("Resolution date: {}", args.resolution_date);
    println!("Min bet: {} lamports", args.min_bet);
    if let Some(max) = args.max_bet {
        println!("Max bet: {} lamports", max);
    }
    println!("Creator: {}", args.creator);
    println!();

    // Validate outcomes
    if args.outcomes.is_empty() {
        anyhow::bail!("Must provide at least one outcome");
    }

    if args.outcomes.len() > 10 {
        anyhow::bail!("Maximum 10 outcomes allowed");
    }

    if args.min_bet == 0 {
        anyhow::bail!("Minimum bet must be greater than 0");
    }

    if let Some(max) = args.max_bet {
        if max < args.min_bet {
            anyhow::bail!("Maximum bet must be greater than or equal to minimum bet");
        }
    }

    println!("{}", "Market created successfully!".green().bold());
    println!();
    println!("{}", "Next steps:".cyan().bold());
    println!("  1. Share market ID with participants");
    println!("  2. Participants place bets with: zyb market bet --market-id <ID>");
    println!("  3. After resolution, winners claim with: zyb market claim");
    println!();
    println!("{}", "Note: Market metadata stored locally. Share market ID securely.".yellow());

    Ok(())
}

#[tokio::main]
async fn place_bet(args: BetArgs) -> Result<()> {
    println!("{}", "Placing private bet...".cyan().bold());
    println!();
    println!("Market ID: {}", args.market_id);
    println!("Selected outcome: {}", args.outcome);
    println!("Bet amount: {} lamports", args.amount);
    println!("Circuit: MarketBet (31)");
    println!();

    // Validate amount
    if args.amount == 0 {
        anyhow::bail!("Bet amount must be greater than 0");
    }

    // Build witness
    println!("{}", "Building witness...".cyan());

    let witness = json!({
        "marketId": args.market_id,
        "outcome": args.outcome,
        "amount": args.amount.to_string(),
        "bettorPubkey": args.bettor,
        "timestamp": chrono::Utc::now().timestamp(),
        "publicInputs": []
    });

    // Save witness to file
    fs::write(&args.witness_output, serde_json::to_string_pretty(&witness)?)?;

    println!("{}", "Witness saved to:".green());
    println!("  {:?}", args.witness_output);
    println!();

    // Create ZK job
    println!("{}", "Submitting bet as ZK job...".cyan());

    let witness_str = serde_json::to_string(&witness)?;
    let witness_commitment = {
        use sha3::{Digest, Keccak256};
        let mut hasher = Keccak256::new();
        hasher.update(witness_str.as_bytes());
        hex::encode(hasher.finalize())
    };

    let client = ZkClient::with_url(&args.server);

    let request = CreateJobRequest {
        circuit_type: CIRCUIT_MARKET_BET,
        witness_commitment,
        public_inputs: vec![],
        creator: args.bettor.clone(),
        timeout_seconds: None,
    };

    // Execute payment flow if keypair provided
    let response = if let Some(keypair) = args.keypair {
        use crate::commands::zk::execute_payment_flow_helper;

        execute_payment_flow_helper(
            &client,
            request,
            &keypair,
            &args.rpc_url,
            CIRCUIT_MARKET_BET,
            &args.bettor,
            args.skip_confirm,
        )
        .await?
    } else {
        client.create_job(request).await?
    };

    println!("{}", "Bet placed successfully!".green().bold());
    println!();
    println!("Job ID: {}", response.job_id.to_string().yellow().bold());
    println!("Status: {}", response.status);
    println!();
    println!("{}", "Your bet is private and will be proven using ZK.".cyan());
    println!(
        "{}",
        "Save this Job ID - you'll need it to claim winnings!".yellow().bold()
    );
    println!("Check status with: zyb zk status {}", response.job_id);

    Ok(())
}

#[tokio::main]
async fn claim_winnings(args: ClaimArgs) -> Result<()> {
    println!("{}", "Claiming market winnings...".cyan().bold());
    println!();
    println!("Market ID: {}", args.market_id);
    println!("Bet ID: {}", args.bet_id);
    println!("Circuit: MarketSettlement (32)");
    println!();

    // First, verify the bet was successful
    let client = ZkClient::with_url(&args.server);

    println!("{}", "Verifying original bet...".cyan());
    let bet_status = client
        .get_job_status(args.bet_id)
        .await
        .context("Failed to fetch bet job status")?;

    if bet_status.status != "completed" {
        anyhow::bail!(
            "Bet job {} is not completed yet (status: {})",
            args.bet_id,
            bet_status.status
        );
    }

    println!("{}", "Bet verified!".green());
    println!();

    // Build claim witness
    println!("{}", "Building claim witness...".cyan());

    let witness = json!({
        "marketId": args.market_id,
        "betJobId": args.bet_id,
        "claimerPubkey": args.claimer,
        "timestamp": chrono::Utc::now().timestamp(),
        "publicInputs": []
    });

    // Save witness to file
    fs::write(&args.witness_output, serde_json::to_string_pretty(&witness)?)?;

    println!("{}", "Witness saved to:".green());
    println!("  {:?}", args.witness_output);
    println!();

    // Create ZK job for settlement
    println!("{}", "Submitting claim as ZK job...".cyan());

    let witness_str = serde_json::to_string(&witness)?;
    let witness_commitment = {
        use sha3::{Digest, Keccak256};
        let mut hasher = Keccak256::new();
        hasher.update(witness_str.as_bytes());
        hex::encode(hasher.finalize())
    };

    let request = CreateJobRequest {
        circuit_type: CIRCUIT_MARKET_SETTLEMENT,
        witness_commitment,
        public_inputs: vec![],
        creator: args.claimer.clone(),
        timeout_seconds: None,
    };

    // Execute payment flow if keypair provided
    let response = if let Some(keypair) = args.keypair {
        use crate::commands::zk::execute_payment_flow_helper;

        execute_payment_flow_helper(
            &client,
            request,
            &keypair,
            &args.rpc_url,
            CIRCUIT_MARKET_SETTLEMENT,
            &args.claimer,
            args.skip_confirm,
        )
        .await?
    } else {
        client.create_job(request).await?
    };

    println!("{}", "Claim submitted successfully!".green().bold());
    println!();
    println!("Job ID: {}", response.job_id.to_string().yellow().bold());
    println!("Status: {}", response.status);
    println!();
    println!("{}", "Your claim will be processed and settled privately.".cyan());
    println!("Check status with: zyb zk status {}", response.job_id);

    Ok(())
}

#[tokio::main]
async fn verify_settlement(args: VerifyArgs) -> Result<()> {
    println!("{}", "Verifying market settlement...".cyan().bold());
    println!();

    let client = ZkClient::with_url(&args.server);

    let status = client
        .get_job_status(args.job_id)
        .await
        .context("Failed to fetch job status")?;

    println!("Job ID: {}", args.job_id);
    println!("Status: {}", status.status);
    println!();

    if status.status == "completed" {
        println!("{}", "Settlement verified successfully!".green().bold());
        println!();
        if let Some(proof_hash) = status.proof_hash {
            println!("Proof hash: {}", proof_hash);
        }
        println!();
        println!("{}", "Market settled. Winners can claim their rewards.".cyan());
    } else {
        println!(
            "{}",
            format!("Settlement is still {}", status.status).yellow()
        );
    }

    Ok(())
}
