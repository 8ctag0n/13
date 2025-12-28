//! Interactive wizard for ZyberLink CLI
//!
//! Provides a guided, user-friendly interface for creating ZK and FHE jobs.

use anyhow::{Context, Result};
use colored::Colorize;
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use indicatif::{ProgressBar, ProgressStyle};
use sha3::{Digest, Keccak256};
use std::fs;
use std::path::PathBuf;

use crate::client::{list_all_circuits, CreateJobRequest, ZkClient};

/// Main entry point for the interactive wizard
pub fn run_wizard() -> Result<()> {
    print_welcome();

    loop {
        match main_menu()? {
            MainMenuAction::CreateZkJob => {
                if let Err(e) = zk_job_wizard() {
                    eprintln!("{} {}", "Error:".red().bold(), e);
                    println!();
                }
            }
            MainMenuAction::CreateFheJob => {
                println!();
                println!("{}", "FHE job wizard coming soon!".yellow());
                println!("For now, use: zyb fhe encrypt --help");
                println!();
            }
            MainMenuAction::CheckStatus => {
                if let Err(e) = check_status_wizard() {
                    eprintln!("{} {}", "Error:".red().bold(), e);
                    println!();
                }
            }
            MainMenuAction::ListCircuits => {
                list_circuits()?;
            }
            MainMenuAction::Exit => {
                println!();
                println!("{}", "Goodbye!".cyan());
                break;
            }
        }
    }

    Ok(())
}

// =============================================================================
// Welcome Screen
// =============================================================================

fn print_welcome() {
    println!();
    println!("{}", style("╭─────────────────────────────────────╮").cyan());
    println!(
        "{}",
        style("│     Welcome to Zyberlink CLI        │").cyan()
    );
    println!(
        "{}",
        style("│     Privacy-Preserving Compute      │").cyan()
    );
    println!("{}", style("╰─────────────────────────────────────╯").cyan());
    println!();
}

// =============================================================================
// Main Menu
// =============================================================================

#[derive(Debug)]
enum MainMenuAction {
    CreateZkJob,
    CreateFheJob,
    CheckStatus,
    ListCircuits,
    Exit,
}

fn main_menu() -> Result<MainMenuAction> {
    let options = vec![
        "Create a ZK proof job",
        "Create an FHE computation job",
        "Check job status",
        "List available circuits",
        "Exit",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("What would you like to do?")
        .items(&options)
        .default(0)
        .interact()?;

    Ok(match selection {
        0 => MainMenuAction::CreateZkJob,
        1 => MainMenuAction::CreateFheJob,
        2 => MainMenuAction::CheckStatus,
        3 => MainMenuAction::ListCircuits,
        4 => MainMenuAction::Exit,
        _ => unreachable!(),
    })
}

// =============================================================================
// ZK Job Wizard
// =============================================================================

#[tokio::main]
async fn zk_job_wizard() -> Result<()> {
    println!();
    println!("{}", "Creating a ZK Proof Job".green().bold());
    println!();

    // Step 1: Select circuit
    let circuit = select_circuit()?;
    println!();
    println!("{} Selected: {} ({})", "✓".green(), circuit.name, circuit.id);
    println!("  Category: {}", circuit.category);
    println!("  Description: {}", circuit.description);
    println!();

    // Step 2: Get witness file path
    let witness_path = get_witness_path()?;

    // Step 3: Load and validate witness
    let (witness_commitment, public_inputs, witness_size) = load_witness(&witness_path)?;
    println!();
    println!("{} Witness file loaded ({:.2} KB)", "✓".green(), witness_size);
    println!("  Commitment: {}", witness_commitment.chars().take(16).collect::<String>() + "...");
    println!("  Public inputs: {} values", public_inputs.len());
    println!();

    // Step 4: Get keypair for payment
    let keypair_path = get_keypair_path()?;
    println!("{} Keypair path: {}", "✓".green(), keypair_path.display());
    println!();

    // Step 5: Server configuration
    let server_url = get_server_url()?;
    let rpc_url = get_rpc_url()?;

    // Step 6: Get creator pubkey (derive from keypair)
    let creator = get_creator_pubkey(&keypair_path)?;
    println!();
    println!("{} Creator pubkey: {}...{}",
        "✓".green(),
        &creator.chars().take(4).collect::<String>(),
        &creator.chars().skip(creator.len() - 4).collect::<String>()
    );
    println!();

    // Step 7: Get price quote
    let client = ZkClient::with_url(&server_url);

    println!("{}", "Fetching price quote...".cyan());
    let quote = client
        .get_quote(circuit.id, &creator)
        .await
        .context("Failed to get price quote")?;

    // Step 8: Display payment info
    print_payment_info(&quote, &circuit);

    // Step 9: Confirm payment
    if !confirm_payment()? {
        println!("{}", "Operation cancelled.".yellow());
        return Ok(());
    }

    // Step 10: Execute payment and create job
    let request = CreateJobRequest {
        circuit_type: circuit.id,
        witness_commitment: witness_commitment.clone(),
        public_inputs,
        creator: creator.clone(),
        timeout_seconds: Some(3600), // 1 hour default
    };

    println!();
    println!("{}", "Processing payment...".cyan());

    let signature = execute_payment_transaction(&keypair_path, &rpc_url, &quote).await?;

    println!("{} Payment successful!", "✓".green().bold());
    println!("  Transaction: {}", signature.to_string().yellow());
    println!();

    // Step 11: Create job
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
        .context("Failed to create ZK job")?;

    spinner.finish_and_clear();

    // Step 12: Display success
    print_job_success(&response, &circuit);

    // Step 13: What's next?
    next_action_menu(response.job_id, &server_url)?;

    Ok(())
}

// =============================================================================
// Circuit Selection
// =============================================================================

fn select_circuit() -> Result<crate::client::CircuitInfo> {
    let circuits = list_all_circuits();

    let items: Vec<String> = circuits
        .iter()
        .map(|c| format!("{} ({}) - {}", c.name, c.id, c.description))
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select circuit type")
        .items(&items)
        .default(0)
        .interact()?;

    Ok(circuits[selection].clone())
}

// =============================================================================
// Input Helpers
// =============================================================================

fn get_witness_path() -> Result<PathBuf> {
    let path: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter path to witness file")
        .default("./witness.json".to_string())
        .interact_text()?;

    let path_buf = PathBuf::from(path);

    if !path_buf.exists() {
        anyhow::bail!("Witness file not found: {:?}", path_buf);
    }

    Ok(path_buf)
}

fn get_keypair_path() -> Result<PathBuf> {
    let default_path = dirs::home_dir()
        .map(|h| h.join(".config/solana/id.json"))
        .and_then(|p| p.to_str().map(String::from))
        .unwrap_or_else(|| "~/.config/solana/id.json".to_string());

    let path: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter Solana keypair path")
        .default(default_path)
        .interact_text()?;

    let path_buf = PathBuf::from(shellexpand::tilde(&path).to_string());

    if !path_buf.exists() {
        anyhow::bail!("Keypair file not found: {:?}", path_buf);
    }

    Ok(path_buf)
}

fn get_server_url() -> Result<String> {
    let url: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Server URL")
        .default("http://localhost:3000".to_string())
        .interact_text()?;

    Ok(url)
}

fn get_rpc_url() -> Result<String> {
    let url: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Solana RPC URL")
        .default("https://api.devnet.solana.com".to_string())
        .interact_text()?;

    Ok(url)
}

fn confirm_payment() -> Result<bool> {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Proceed with payment?")
        .default(false)
        .interact()
        .context("Failed to get confirmation")
}

// =============================================================================
// Witness Processing
// =============================================================================

fn load_witness(path: &PathBuf) -> Result<(String, Vec<String>, f64)> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message("Reading witness file...");

    let witness_content =
        fs::read_to_string(path).with_context(|| format!("Failed to read witness file: {:?}", path))?;

    let file_size = witness_content.len() as f64 / 1024.0; // KB

    // Parse witness JSON to extract public inputs
    let witness_json: serde_json::Value =
        serde_json::from_str(&witness_content).context("Failed to parse witness JSON")?;

    // Calculate witness commitment (SHA3-256 hash)
    let mut hasher = Keccak256::new();
    hasher.update(witness_content.as_bytes());
    let witness_commitment = hex::encode(hasher.finalize());

    spinner.set_message("Extracting public inputs...");

    // Extract public inputs from witness
    let public_inputs = if let Some(public) = witness_json.get("publicInputs") {
        extract_public_inputs(public)?
    } else if let Some(public) = witness_json.get("public") {
        extract_public_inputs(public)?
    } else {
        vec![]
    };

    spinner.finish_and_clear();

    Ok((witness_commitment, public_inputs, file_size))
}

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

// =============================================================================
// Solana Helpers
// =============================================================================

fn get_creator_pubkey(keypair_path: &PathBuf) -> Result<String> {
    use solana_sdk::signature::{read_keypair_file, Signer};

    let keypair = read_keypair_file(keypair_path)
        .map_err(|e| anyhow::anyhow!("Failed to read keypair from {:?}: {}", keypair_path, e))?;

    Ok(keypair.pubkey().to_string())
}

async fn execute_payment_transaction(
    keypair_path: &PathBuf,
    rpc_url: &str,
    quote: &crate::client::QuoteResponse,
) -> Result<solana_sdk::signature::Signature> {
    use crate::solana::execute_payment;

    let keypair_path_str = keypair_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid keypair path"))?;

    let signature = execute_payment(
        rpc_url,
        keypair_path_str,
        &quote.payment_recipient,
        quote.price_lamports,
    )
    .await
    .context("Failed to execute payment")?;

    Ok(signature)
}

// =============================================================================
// Display Helpers
// =============================================================================

fn print_payment_info(quote: &crate::client::QuoteResponse, circuit: &crate::client::CircuitInfo) {
    println!();
    println!("{}", style("╭────────────────────────────────────╮").cyan());
    println!(
        "{}",
        style("│ Payment Required                    │").cyan()
    );
    println!(
        "│ Circuit: {:<28}│",
        format!("{}", circuit.name)
    );
    println!(
        "│ Price: {:<30}│",
        format!("{:.4} SOL", quote.price_sol)
    );
    println!(
        "│ Recipient: {}...{}  │",
        &quote.payment_recipient.chars().take(8).collect::<String>(),
        &quote.payment_recipient.chars().skip(quote.payment_recipient.len() - 6).collect::<String>()
    );
    println!("{}", style("╰────────────────────────────────────╯").cyan());
    println!();
}

fn print_job_success(
    response: &crate::client::CreateJobResponse,
    circuit: &crate::client::CircuitInfo,
) {
    println!();
    println!("{} Job created successfully!", "✓".green().bold());
    println!();
    println!("{}", style("╭────────────────────────────────────╮").cyan());
    println!(
        "{}",
        style("│ Job Details                         │").cyan()
    );
    println!(
        "│ ID: {:<33}│",
        format!("{}", response.job_id)
    );
    println!(
        "│ Status: {:<30}│",
        format!("{}", response.status)
    );
    println!(
        "│ Circuit: {:<29}│",
        format!("{} ({})", circuit.name, circuit.id)
    );
    println!("{}", style("╰────────────────────────────────────╯").cyan());
    println!();
    println!(
        "Check status: {}",
        format!("zyb zk status {}", response.job_id).yellow()
    );
    println!();
}

// =============================================================================
// Check Status Wizard
// =============================================================================

#[tokio::main]
async fn check_status_wizard() -> Result<()> {
    println!();

    let job_id: i64 = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter job ID")
        .interact_text()?;

    let server_url = get_server_url()?;

    println!();
    println!("{}", format!("Fetching status for job {}...", job_id).cyan());

    let client = ZkClient::with_url(&server_url);
    let status = client
        .get_job_status(job_id)
        .await
        .context("Failed to fetch job status")?;

    println!();
    println!("{}", "Job Status".green().bold());
    println!("  ID: {}", status.job_id);
    println!("  Status: {}", colorize_status(&status.status));
    println!("  Circuit Type: {}", status.circuit_type);
    println!("  Created: {}", status.created_at);
    println!("  Updated: {}", status.updated_at);

    if let Some(prover) = status.prover_pubkey {
        println!("  Prover: {}", prover);
    }

    if let Some(proof_hash) = status.proof_hash {
        println!("  Proof Hash: {}", proof_hash);
    }

    println!();

    Ok(())
}

// =============================================================================
// List Circuits
// =============================================================================

fn list_circuits() -> Result<()> {
    use comfy_table::{presets::UTF8_FULL, Table};

    println!();
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

    Ok(())
}

// =============================================================================
// Next Action Menu
// =============================================================================

fn next_action_menu(job_id: i64, server_url: &str) -> Result<()> {
    let options = vec!["Create another job", "Check this job's status", "Exit"];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("What would you like to do next?")
        .items(&options)
        .default(0)
        .interact()?;

    match selection {
        0 => {
            // Will return to main menu
        }
        1 => {
            // Call async function to check status
            check_job_status_sync(job_id, server_url)?;
        }
        2 => {
            // Exit handled by caller
        }
        _ => unreachable!(),
    }

    Ok(())
}

#[tokio::main]
async fn check_job_status_sync(job_id: i64, server_url: &str) -> Result<()> {
    println!();
    println!("{}", format!("Fetching status for job {}...", job_id).cyan());

    let client = ZkClient::with_url(server_url);
    let status = client
        .get_job_status(job_id)
        .await
        .context("Failed to fetch job status")?;

    println!();
    println!("{}", "Job Status".green().bold());
    println!("  ID: {}", status.job_id);
    println!("  Status: {}", colorize_status(&status.status));
    println!("  Circuit Type: {}", status.circuit_type);

    if let Some(prover) = status.prover_pubkey {
        println!("  Prover: {}", prover);
    }

    if let Some(proof_hash) = status.proof_hash {
        println!("  Proof Hash: {}", proof_hash);
    }

    println!();
    Ok(())
}

// =============================================================================
// Utility Functions
// =============================================================================

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
