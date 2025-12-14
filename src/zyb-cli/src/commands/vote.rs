//! Vote commands for private voting
//!
//! Implements CLI commands for creating and managing private votes using ZK circuits.

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use colored::Colorize;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

use crate::client::{CreateJobRequest, ZkClient};

const CIRCUIT_PRIVATE_VOTE: u8 = 20;
const CIRCUIT_PRIVATE_VOTE_WITH_POI: u8 = 21;

#[derive(Subcommand, Debug)]
pub enum VoteCommands {
    /// Create a new voting poll
    Create(CreateArgs),
    /// Cast a private vote
    Cast(CastArgs),
    /// Tally votes
    Tally(TallyArgs),
    /// Verify voting results
    Verify(VerifyArgs),
}

#[derive(Args, Debug)]
pub struct CreateArgs {
    /// Poll question
    #[arg(long)]
    pub question: String,

    /// Poll options (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub options: Vec<String>,

    /// Duration in seconds
    #[arg(long, default_value = "86400")]
    pub duration: u64,

    /// Creator public key
    #[arg(long)]
    pub creator: String,

    /// Server URL
    #[arg(long, default_value = "http://localhost:3000")]
    pub server: String,
}

#[derive(Args, Debug)]
pub struct CastArgs {
    /// Poll ID
    #[arg(long)]
    pub poll_id: String,

    /// Selected option (0-indexed)
    #[arg(long)]
    pub option: u8,

    /// Use PoI (Proof of Innocence) circuit
    #[arg(long)]
    pub use_poi: bool,

    /// Merkle root (required if use_poi is true)
    #[arg(long)]
    pub merkle_root: Option<String>,

    /// Merkle path (JSON file with path siblings)
    #[arg(long)]
    pub merkle_path: Option<PathBuf>,

    /// Voter public key
    #[arg(long)]
    pub voter: String,

    /// Server URL
    #[arg(long, default_value = "http://localhost:3000")]
    pub server: String,

    /// Path to output witness file
    #[arg(long, default_value = "./vote_witness.json")]
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
pub struct TallyArgs {
    /// Poll ID
    #[arg(long)]
    pub poll_id: String,

    /// Server URL
    #[arg(long, default_value = "http://localhost:3000")]
    pub server: String,
}

#[derive(Args, Debug)]
pub struct VerifyArgs {
    /// Job ID of voting proof
    #[arg(long)]
    pub job_id: i64,

    /// Server URL
    #[arg(long, default_value = "http://localhost:3000")]
    pub server: String,
}

pub fn handle_vote_command(cmd: VoteCommands) -> Result<()> {
    match cmd {
        VoteCommands::Create(args) => create_poll(args),
        VoteCommands::Cast(args) => cast_vote(args),
        VoteCommands::Tally(args) => tally_votes(args),
        VoteCommands::Verify(args) => verify_result(args),
    }
}

fn create_poll(args: CreateArgs) -> Result<()> {
    println!("{}", "Creating voting poll...".cyan().bold());
    println!();
    println!("Question: {}", args.question.green());
    println!("Options: {}", args.options.join(", "));
    println!("Duration: {} seconds", args.duration);
    println!("Creator: {}", args.creator);
    println!();

    // Validate options
    if args.options.is_empty() {
        anyhow::bail!("Must provide at least one option");
    }

    if args.options.len() > 10 {
        anyhow::bail!("Maximum 10 options allowed");
    }

    println!("{}", "Poll created successfully!".green().bold());
    println!();
    println!("{}", "Next steps:".cyan().bold());
    println!("  1. Share poll ID with voters");
    println!("  2. Voters cast votes with: zyb vote cast --poll-id <ID>");
    println!("  3. Tally results with: zyb vote tally --poll-id <ID>");
    println!();
    println!("{}", "Note: Poll metadata stored locally. Share poll ID securely.".yellow());

    Ok(())
}

#[tokio::main]
async fn cast_vote(args: CastArgs) -> Result<()> {
    println!("{}", "Casting private vote...".cyan().bold());
    println!();
    println!("Poll ID: {}", args.poll_id);
    println!("Selected option: {}", args.option);
    println!(
        "Circuit: {}",
        if args.use_poi {
            "PrivateVoteWithPoI (21)".green()
        } else {
            "PrivateVote (20)".green()
        }
    );
    println!();

    let circuit_type = if args.use_poi {
        if args.merkle_root.is_none() || args.merkle_path.is_none() {
            anyhow::bail!("--merkle-root and --merkle-path required when using --use-poi");
        }
        CIRCUIT_PRIVATE_VOTE_WITH_POI
    } else {
        CIRCUIT_PRIVATE_VOTE
    };

    // Build witness
    println!("{}", "Building witness...".cyan());

    let witness = if args.use_poi {
        let merkle_root = args.merkle_root.as_ref().unwrap();
        let merkle_path_file = args.merkle_path.as_ref().unwrap();

        let path_data = fs::read_to_string(merkle_path_file)
            .with_context(|| format!("Failed to read merkle path file: {:?}", merkle_path_file))?;

        let path_json: serde_json::Value = serde_json::from_str(&path_data)?;

        json!({
            "pollId": args.poll_id,
            "vote": args.option,
            "voterPubkey": args.voter,
            "merkleRoot": merkle_root,
            "merklePath": path_json.get("siblings").unwrap_or(&json!([])),
            "publicInputs": [merkle_root]
        })
    } else {
        json!({
            "pollId": args.poll_id,
            "vote": args.option,
            "voterPubkey": args.voter,
            "publicInputs": []
        })
    };

    // Save witness to file
    fs::write(&args.witness_output, serde_json::to_string_pretty(&witness)?)?;

    println!("{}", "Witness saved to:".green());
    println!("  {:?}", args.witness_output);
    println!();

    // Create ZK job
    println!("{}", "Submitting vote as ZK job...".cyan());

    let witness_str = serde_json::to_string(&witness)?;
    let witness_commitment = {
        use sha3::{Digest, Keccak256};
        let mut hasher = Keccak256::new();
        hasher.update(witness_str.as_bytes());
        hex::encode(hasher.finalize())
    };

    let public_inputs = if args.use_poi {
        vec![args.merkle_root.unwrap()]
    } else {
        vec![]
    };

    let client = ZkClient::with_url(&args.server);

    let request = CreateJobRequest {
        circuit_type,
        witness_commitment,
        public_inputs,
        creator: args.voter.clone(),
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
            circuit_type,
            &args.voter,
            args.skip_confirm,
        )
        .await?
    } else {
        client.create_job(request).await?
    };

    println!("{}", "Vote cast successfully!".green().bold());
    println!();
    println!("Job ID: {}", response.job_id.to_string().yellow().bold());
    println!("Status: {}", response.status);
    println!();
    println!("{}", "Your vote is private and will be proven using ZK.".cyan());
    println!("Check status with: zyb zk status {}", response.job_id);

    Ok(())
}

fn tally_votes(args: TallyArgs) -> Result<()> {
    println!("{}", "Tallying votes...".cyan().bold());
    println!();
    println!("Poll ID: {}", args.poll_id);
    println!("Server: {}", args.server);
    println!();

    println!("{}", "Vote tally results:".green().bold());
    println!();
    println!("{}", "Note: Full tally implementation requires aggregating all vote proofs.".yellow());
    println!("This is a placeholder showing the command structure.");
    println!();
    println!("{}", "Next steps:".cyan().bold());
    println!("  1. Query server for all vote job IDs for this poll");
    println!("  2. Verify each vote proof");
    println!("  3. Aggregate results while maintaining privacy");

    Ok(())
}

#[tokio::main]
async fn verify_result(args: VerifyArgs) -> Result<()> {
    println!("{}", "Verifying vote result...".cyan().bold());
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
        println!("{}", "Vote proof verified successfully!".green().bold());
        println!();
        if let Some(proof_hash) = status.proof_hash {
            println!("Proof hash: {}", proof_hash);
        }
    } else {
        println!("{}", format!("Vote is still {}", status.status).yellow());
    }

    Ok(())
}
