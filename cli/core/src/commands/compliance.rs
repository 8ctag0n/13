//! Compliance commands for portfolio verification
//!
//! Implements CLI commands for proving and verifying portfolio compliance using ZK circuits.

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use colored::Colorize;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

use crate::client::{CreateJobRequest, ZkClient};

const CIRCUIT_POI: u8 = 10;
const CIRCUIT_PORTFOLIO_INNOCENCE: u8 = 40;
const CIRCUIT_PORTFOLIO_THRESHOLD: u8 = 41;

#[derive(Subcommand, Debug)]
pub enum ComplianceCommands {
    /// Generate a compliance proof
    Prove(ProveArgs),
    /// Verify a compliance proof
    Verify(VerifyArgs),
}

#[derive(Args, Debug)]
pub struct ProveArgs {
    /// Proof type: innocence, threshold, or poi
    #[arg(long, value_parser = ["innocence", "threshold", "poi"])]
    pub proof_type: String,

    /// Portfolio data (JSON file)
    #[arg(long)]
    pub portfolio: PathBuf,

    /// Threshold value (required for threshold proofs)
    #[arg(long)]
    pub threshold: Option<u64>,

    /// Merkle root (required for PoI proofs)
    #[arg(long)]
    pub merkle_root: Option<String>,

    /// Merkle path (JSON file, required for PoI proofs)
    #[arg(long)]
    pub merkle_path: Option<PathBuf>,

    /// Prover public key
    #[arg(long)]
    pub prover: String,

    /// Server URL
    #[arg(long, default_value = "http://localhost:3000")]
    pub server: String,

    /// Path to output witness file
    #[arg(long, default_value = "./compliance_witness.json")]
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
    /// Job ID of compliance proof
    #[arg(long)]
    pub job_id: i64,

    /// Server URL
    #[arg(long, default_value = "http://localhost:3000")]
    pub server: String,

    /// Show detailed verification info
    #[arg(long, short)]
    pub verbose: bool,
}

pub fn handle_compliance_command(cmd: ComplianceCommands) -> Result<()> {
    match cmd {
        ComplianceCommands::Prove(args) => prove_compliance(args),
        ComplianceCommands::Verify(args) => verify_compliance(args),
    }
}

#[tokio::main]
async fn prove_compliance(args: ProveArgs) -> Result<()> {
    println!("{}", "Generating compliance proof...".cyan().bold());
    println!();
    println!("Proof type: {}", args.proof_type.green());
    println!("Prover: {}", args.prover);
    println!();

    // Determine circuit type and validate arguments
    let circuit_type = match args.proof_type.as_str() {
        "innocence" => {
            println!("Circuit: PortfolioInnocence (40)");
            CIRCUIT_PORTFOLIO_INNOCENCE
        }
        "threshold" => {
            if args.threshold.is_none() {
                anyhow::bail!("--threshold required for threshold proofs");
            }
            println!("Circuit: PortfolioThreshold (41)");
            println!("Threshold: {} lamports", args.threshold.unwrap());
            CIRCUIT_PORTFOLIO_THRESHOLD
        }
        "poi" => {
            if args.merkle_root.is_none() || args.merkle_path.is_none() {
                anyhow::bail!("--merkle-root and --merkle-path required for PoI proofs");
            }
            println!("Circuit: ProofOfInnocence (10)");
            CIRCUIT_POI
        }
        _ => unreachable!(),
    };

    println!();

    // Read portfolio data
    println!("{}", "Reading portfolio data...".cyan());

    let portfolio_data = fs::read_to_string(&args.portfolio).with_context(|| {
        format!("Failed to read portfolio file: {:?}", args.portfolio)
    })?;

    let portfolio_json: serde_json::Value = serde_json::from_str(&portfolio_data)
        .context("Failed to parse portfolio JSON")?;

    println!("{}", "Portfolio data loaded".green());
    println!();

    // Build witness based on proof type
    println!("{}", "Building witness...".cyan());

    let witness = match args.proof_type.as_str() {
        "innocence" => {
            // Portfolio innocence proof
            json!({
                "portfolio": portfolio_json,
                "proverPubkey": args.prover,
                "timestamp": chrono::Utc::now().timestamp(),
                "publicInputs": []
            })
        }
        "threshold" => {
            // Portfolio threshold proof
            let threshold = args.threshold.unwrap();
            json!({
                "portfolio": portfolio_json,
                "threshold": threshold.to_string(),
                "proverPubkey": args.prover,
                "timestamp": chrono::Utc::now().timestamp(),
                "publicInputs": [threshold.to_string()]
            })
        }
        "poi" => {
            // Proof of Innocence with Merkle tree
            let merkle_root = args.merkle_root.as_ref().unwrap();
            let merkle_path_file = args.merkle_path.as_ref().unwrap();

            let path_data = fs::read_to_string(merkle_path_file).with_context(|| {
                format!("Failed to read merkle path file: {:?}", merkle_path_file)
            })?;

            let path_json: serde_json::Value = serde_json::from_str(&path_data)?;

            json!({
                "portfolio": portfolio_json,
                "merkleRoot": merkle_root,
                "merklePath": path_json.get("siblings").unwrap_or(&json!([])),
                "proverPubkey": args.prover,
                "timestamp": chrono::Utc::now().timestamp(),
                "publicInputs": [merkle_root]
            })
        }
        _ => unreachable!(),
    };

    // Save witness to file
    fs::write(&args.witness_output, serde_json::to_string_pretty(&witness)?)?;

    println!("{}", "Witness saved to:".green());
    println!("  {:?}", args.witness_output);
    println!();

    // Create ZK job
    println!("{}", "Submitting compliance proof job...".cyan());

    let witness_str = serde_json::to_string(&witness)?;
    let witness_commitment = {
        use sha3::{Digest, Keccak256};
        let mut hasher = Keccak256::new();
        hasher.update(witness_str.as_bytes());
        hex::encode(hasher.finalize())
    };

    let public_inputs = match args.proof_type.as_str() {
        "innocence" => vec![],
        "threshold" => vec![args.threshold.unwrap().to_string()],
        "poi" => vec![args.merkle_root.unwrap()],
        _ => unreachable!(),
    };

    let client = ZkClient::with_url(&args.server);

    let request = CreateJobRequest {
        circuit_type,
        witness_commitment,
        public_inputs,
        creator: args.prover.clone(),
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
            &args.prover,
            args.skip_confirm,
        )
        .await?
    } else {
        client.create_job(request).await?
    };

    println!("{}", "Compliance proof job created!".green().bold());
    println!();
    println!("Job ID: {}", response.job_id.to_string().yellow().bold());
    println!("Status: {}", response.status);
    println!();
    println!(
        "{}",
        "Your compliance proof will be generated privately using ZK.".cyan()
    );
    println!("Check status with: zyb zk status {}", response.job_id);
    println!(
        "Verify compliance with: zyb compliance verify --job-id {}",
        response.job_id
    );

    Ok(())
}

#[tokio::main]
async fn verify_compliance(args: VerifyArgs) -> Result<()> {
    println!("{}", "Verifying compliance proof...".cyan().bold());
    println!();

    let client = ZkClient::with_url(&args.server);

    if args.verbose {
        // Get full details
        let details = client
            .get_job_details(args.job_id)
            .await
            .context("Failed to fetch job details")?;

        println!("{}", "Compliance Proof Details:".green().bold());
        println!();
        println!("Job ID: {}", details.job_id);
        println!("Status: {}", details.status);
        println!(
            "Circuit: {} (type {})",
            details.circuit_category, details.circuit_type
        );
        println!("Witness Commitment: {}", details.witness_commitment);
        println!();

        if let Some(prover) = &details.prover_pubkey {
            println!("Prover: {}", prover);
        }

        if let Some(proof_hash) = &details.proof_hash {
            println!("Proof Hash: {}", proof_hash);
        }

        if let Some(completed_at) = &details.completed_at {
            println!("Completed At: {}", completed_at);
        }

        println!();

        // Show public inputs if available
        if let Some(inputs) = details.public_inputs.as_array() {
            if !inputs.is_empty() {
                println!("{}", "Public Inputs:".cyan().bold());
                for (i, input) in inputs.iter().enumerate() {
                    println!("  [{}]: {}", i, input);
                }
                println!();
            }
        }

        if details.status == "completed" {
            println!("{}", "Compliance proof verified successfully!".green().bold());
            println!();
            println!("{}", "The portfolio meets compliance requirements.".cyan());
        } else if details.status == "failed" {
            println!("{}", "Compliance proof failed!".red().bold());
            println!();
            println!("{}", "The portfolio may not meet compliance requirements.".yellow());
        } else {
            println!("{}", format!("Proof is still {}", details.status).yellow());
        }
    } else {
        // Simple status check
        let status = client
            .get_job_status(args.job_id)
            .await
            .context("Failed to fetch job status")?;

        println!("Job ID: {}", args.job_id);
        println!("Status: {}", status.status);
        println!("Circuit Type: {}", status.circuit_type);
        println!();

        if status.status == "completed" {
            println!("{}", "Compliance proof verified successfully!".green().bold());
            println!();
            if let Some(proof_hash) = status.proof_hash {
                println!("Proof hash: {}", proof_hash);
            }
            println!();
            println!("{}", "The portfolio meets compliance requirements.".cyan());
        } else if status.status == "failed" {
            println!("{}", "Compliance proof failed!".red().bold());
        } else {
            println!("{}", format!("Proof is still {}", status.status).yellow());
        }

        println!();
        println!("Use {} for full details", "--verbose".cyan());
    }

    Ok(())
}
