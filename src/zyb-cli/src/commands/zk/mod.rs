//! ZK proof commands
//!
//! Implements CLI commands for Zero-Knowledge proof operations.

use anyhow::{Context, Result};
use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Table};
use indicatif::{ProgressBar, ProgressStyle};
use sha3::{Digest, Keccak256};
use std::fs;
use std::path::PathBuf;

use crate::client::{get_circuit_info, list_all_circuits, CreateJobRequest, ZkClient};

/// Create a new ZK proof job
#[tokio::main]
pub async fn create_command(
    circuit_type: u8,
    witness_path: PathBuf,
    creator: String,
    server_url: String,
    timeout: Option<i32>,
    keypair_path: Option<PathBuf>,
    rpc_url: String,
    skip_confirm: bool,
) -> Result<()> {
    // Validate circuit type
    if circuit_type < 10 || circuit_type > 49 {
        anyhow::bail!(
            "Invalid circuit type: {}. Must be between 10-49.\nUse 'zyb zk circuits' to see available circuits.",
            circuit_type
        );
    }

    // Get circuit info
    let circuit_info = get_circuit_info(circuit_type).ok_or_else(|| {
        anyhow::anyhow!(
            "Circuit type {} not found. Use 'zyb zk circuits' to see available circuits.",
            circuit_type
        )
    })?;

    println!("{}", "Creating ZK proof job...".cyan().bold());
    println!("Circuit: {} ({})", circuit_info.name.green(), circuit_info.id);
    println!("Category: {}", circuit_info.category);
    println!();

    // Read witness file
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message("Reading witness file...");

    let witness_content = fs::read_to_string(&witness_path)
        .with_context(|| format!("Failed to read witness file: {:?}", witness_path))?;

    // Parse witness JSON to extract public inputs
    let witness_json: serde_json::Value = serde_json::from_str(&witness_content)
        .context("Failed to parse witness JSON")?;

    // Calculate witness commitment (SHA3-256 hash)
    let mut hasher = Keccak256::new();
    hasher.update(witness_content.as_bytes());
    let witness_commitment = hex::encode(hasher.finalize());

    spinner.set_message("Extracting public inputs...");

    // Extract public inputs from witness
    // Assuming witness has a "publicInputs" or "public" field
    let public_inputs = if let Some(public) = witness_json.get("publicInputs") {
        extract_public_inputs(public)?
    } else if let Some(public) = witness_json.get("public") {
        extract_public_inputs(public)?
    } else {
        // If no explicit public inputs, use empty array
        vec![]
    };

    spinner.finish_and_clear();

    println!("{}", "Witness file processed:".green());
    println!("  Commitment: {}", witness_commitment);
    println!("  Public inputs: {} values", public_inputs.len());
    println!();

    // Create ZK client
    let client = ZkClient::with_url(&server_url);

    // Prepare request
    let request = CreateJobRequest {
        circuit_type,
        witness_commitment: witness_commitment.clone(),
        public_inputs,
        creator: creator.clone(),
        timeout_seconds: timeout,
    };

    // Check if payment flow is enabled
    let response = if let Some(keypair_path_buf) = keypair_path {
        // New payment flow with Solana
        execute_payment_flow_helper(
            &client,
            request,
            &keypair_path_buf,
            &rpc_url,
            circuit_type,
            &creator,
            skip_confirm,
        )
        .await?
    } else {
        // Legacy flow (uses x402 gateway)
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        spinner.set_message("Submitting job to server...");

        let resp = client
            .create_job(request)
            .await
            .context("Failed to create ZK job")?;

        spinner.finish_and_clear();
        resp
    };

    // Display results
    println!("{}", "ZK job created successfully!".green().bold());
    println!();
    println!("Job ID: {}", response.job_id.to_string().yellow().bold());
    println!("Status: {}", colorize_status(&response.status));
    println!("Circuit: {} (type {})", circuit_info.name, circuit_type);
    println!("Price: {} lamports", response.price_lamports);
    println!();
    println!("{}", "Next steps:".cyan().bold());
    println!("  1. Check job status: zyb zk status {}", response.job_id);
    println!("  2. Wait for a prover to claim and complete the job");
    println!();
    println!(
        "{}",
        "Note: Save your witness file securely! You may need it for verification.".yellow()
    );

    Ok(())
}

/// Check status of a ZK job
#[tokio::main]
pub async fn status_command(job_id: i64, server_url: String, verbose: bool) -> Result<()> {
    let client = ZkClient::with_url(&server_url);

    println!("{}", format!("Fetching status for job {}...", job_id).cyan());
    println!();

    if verbose {
        // Get full details
        let details = client
            .get_job_details(job_id)
            .await
            .context("Failed to fetch job details")?;

        // Display detailed information
        println!("{}", "Job Details:".green().bold());
        println!();

        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        table.set_header(vec!["Field", "Value"]);

        table.add_row(vec!["Job ID", &details.job_id.to_string()]);
        table.add_row(vec!["Status", &colorize_status(&details.status)]);
        table.add_row(vec!["Creator", &details.creator_pubkey]);
        table.add_row(vec![
            "Circuit",
            &format!("{} (type {})", details.circuit_category, details.circuit_type),
        ]);
        table.add_row(vec!["Witness Commitment", &details.witness_commitment]);
        table.add_row(vec![
            "Price",
            &format!("{} lamports", details.price_lamports),
        ]);
        table.add_row(vec![
            "Timeout",
            &format!("{} seconds", details.timeout_seconds),
        ]);

        if let Some(prover) = &details.prover_pubkey {
            table.add_row(vec!["Prover", prover]);
        }

        if let Some(claimed_at) = &details.claimed_at {
            table.add_row(vec!["Claimed At", claimed_at]);
        }

        if let Some(proof_hash) = &details.proof_hash {
            table.add_row(vec!["Proof Hash", proof_hash]);
        }

        if let Some(completed_at) = &details.completed_at {
            table.add_row(vec!["Completed At", completed_at]);
        }

        table.add_row(vec!["Created At", &details.created_at]);
        table.add_row(vec!["Updated At", &details.updated_at]);

        println!("{}", table);
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
    } else {
        // Get simple status
        let status = client
            .get_job_status(job_id)
            .await
            .context("Failed to fetch job status")?;

        // Display status information
        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        table.set_header(vec!["Field", "Value"]);

        table.add_row(vec!["Job ID", &status.job_id.to_string()]);
        table.add_row(vec!["Status", &colorize_status(&status.status)]);
        table.add_row(vec!["Circuit Type", &status.circuit_type.to_string()]);

        if let Some(prover) = &status.prover_pubkey {
            table.add_row(vec!["Prover", prover]);
        }

        if let Some(proof_hash) = &status.proof_hash {
            table.add_row(vec!["Proof Hash", proof_hash]);
        }

        table.add_row(vec!["Created", &status.created_at]);
        table.add_row(vec!["Updated", &status.updated_at]);

        println!("{}", table);
        println!();

        if status.status == "completed" {
            println!(
                "{}",
                "Job completed successfully! Proof is ready.".green().bold()
            );
        } else if status.status == "proving" {
            println!("{}", "Job is being processed by a prover...".yellow());
        } else if status.status == "active" {
            println!("{}", "Job is waiting for a prover to claim it...".yellow());
        } else if status.status == "pending_tx" {
            println!("{}", "Job is pending transaction confirmation...".yellow());
        } else if status.status == "failed" {
            println!("{}", "Job failed. Please check the logs.".red().bold());
        }

        println!();
        println!("Use {} for full details", "--verbose".cyan());
    }

    Ok(())
}

/// List available ZK circuits
pub fn circuits_command() -> Result<()> {
    println!("{}", "Available ZK Circuits".cyan().bold());
    println!();

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["ID", "Name", "Category", "Description"]);

    let circuits = list_all_circuits();
    for circuit in circuits {
        table.add_row(vec![
            &circuit.id.to_string(),
            circuit.name,
            circuit.category,
            circuit.description,
        ]);
    }

    println!("{}", table);
    println!();

    println!("{}", "Usage:".green().bold());
    println!(
        "  zyb zk create --circuit-type <ID> --witness <path> --creator <pubkey>"
    );
    println!();

    println!("{}", "Circuit Categories:".cyan().bold());
    println!("  10-19: {} - Core primitives (PoI, PoR)", "Core".yellow());
    println!("  20-29: {} - Private voting circuits", "Voting".yellow());
    println!("  30-39: {} - Market data circuits", "Market".yellow());
    println!(
        "  40-49: {} - Portfolio compliance circuits",
        "Portfolio".yellow()
    );

    Ok(())
}

/// Verify a ZK proof (placeholder for future implementation)
pub fn verify_command(_proof: PathBuf) -> Result<()> {
    println!("{}", "ZK proof verification".cyan().bold());
    println!();
    println!(
        "{}",
        "Local verification coming in future release!".yellow()
    );
    println!();
    println!("For now, proofs are verified on the server using Groth16.");
    println!("You can download and verify proofs using:");
    println!("  - snarkjs (for circom circuits)");
    println!("  - arkworks (Rust library)");
    println!();
    println!("{}", "Example with snarkjs:".green());
    println!("  snarkjs groth16 verify vkey.json public.json proof.json");

    Ok(())
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Extract public inputs from witness JSON
fn extract_public_inputs(public: &serde_json::Value) -> Result<Vec<String>> {
    if let Some(arr) = public.as_array() {
        Ok(arr
            .iter()
            .map(|v| v.to_string().trim_matches('"').to_string())
            .collect())
    } else if let Some(s) = public.as_str() {
        Ok(vec![s.to_string()])
    } else if let Some(n) = public.as_i64() {
        Ok(vec![n.to_string()])
    } else {
        Ok(vec![public.to_string()])
    }
}

/// Colorize status string
fn colorize_status(status: &str) -> String {
    match status {
        "completed" => status.green().bold().to_string(),
        "proving" => status.yellow().bold().to_string(),
        "active" => status.cyan().bold().to_string(),
        "pending_tx" => status.blue().to_string(),
        "failed" => status.red().bold().to_string(),
        _ => status.to_string(),
    }
}

/// Execute payment flow with Solana
pub async fn execute_payment_flow_helper(
    client: &ZkClient,
    request: CreateJobRequest,
    keypair_path: &PathBuf,
    rpc_url: &str,
    circuit_type: u8,
    payer: &str,
    skip_confirm: bool,
) -> Result<crate::client::CreateJobResponse> {
    use crate::solana::{execute_payment, signer::lamports_to_sol};

    // Step 1: Get price quote
    println!("{}", "Getting price quote...".cyan());

    let quote = client
        .get_quote(circuit_type, payer)
        .await
        .context("Failed to get price quote")?;

    println!();
    println!("{}", "Payment Information:".green().bold());
    println!("  Circuit Type: {}", circuit_type);
    println!(
        "  Price: {} SOL ({} lamports)",
        lamports_to_sol(quote.price_lamports),
        quote.price_lamports
    );
    println!("  Recipient: {}", quote.payment_recipient);
    println!();

    // Step 2: Confirm payment
    if !skip_confirm {
        use std::io::{self, Write};

        print!("{}", "Proceed with payment? [y/N]: ".yellow());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let confirmed = input.trim().to_lowercase();
        if confirmed != "y" && confirmed != "yes" {
            println!("{}", "Payment cancelled.".red());
            anyhow::bail!("Payment cancelled by user");
        }
    }

    // Step 3: Execute payment
    println!();
    println!("{}", "Processing payment...".cyan());

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );

    spinner.set_message("Loading keypair...");

    let keypair_path_str = keypair_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid keypair path"))?;

    spinner.set_message("Building transaction...");

    let signature = execute_payment(
        rpc_url,
        keypair_path_str,
        &quote.payment_recipient,
        quote.price_lamports,
    )
    .await
    .context("Failed to execute payment")?;

    spinner.finish_and_clear();

    println!("{}", "Payment successful!".green().bold());
    println!("  Transaction: {}", signature.to_string().yellow());
    println!();

    // Step 4: Create job with payment signature
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message("Creating ZK job...");

    let response = client
        .create_job_with_payment(request, &signature.to_string())
        .await
        .context("Failed to create ZK job with payment")?;

    spinner.finish_and_clear();

    Ok(response)
}
