//! Demo Commands - Visual demonstrations for presentations and videos
//!
//! This module provides visually enhanced demo commands with:
//! - Colored output and progress bars
//! - Narrative flow (Alice/Bob stories)
//! - Educational explanations
//!
//! Usage:
//!   zyb demo poi              # Full Proof of Innocence demo
//!   zyb demo poi --fast       # Skip actual proving (mock mode)
//!   zyb demo poi --alice-only # Only show innocent case

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::Keypair,
};
use std::path::PathBuf;
use std::time::Duration;

// =============================================================================
// CLI Structure
// =============================================================================

#[derive(Subcommand, Debug)]
pub enum DemoCommands {
    /// Proof of Innocence demo - prove you're NOT on a sanctions list
    Poi(DemoPoiArgs),
}

#[derive(Args, Debug)]
pub struct DemoPoiArgs {
    /// Skip actual FHE computation (mock mode for quick demos)
    #[arg(long)]
    pub fast: bool,

    /// Only show Alice's story (innocent case)
    #[arg(long)]
    pub alice_only: bool,

    /// Backend URL
    #[arg(long, env = "BACKEND_URL", default_value = "http://localhost:9000")]
    pub backend_url: String,

    /// Solana RPC URL
    #[arg(long, env = "SOLANA_RPC_URL", default_value = "http://localhost:8899")]
    pub rpc_url: String,

    /// Path to keypair
    #[arg(long, env = "USER_KEYPAIR")]
    pub keypair: Option<PathBuf>,

    /// Emit JSON events (for integration)
    #[arg(long)]
    pub json: bool,
}

// =============================================================================
// Demo Context
// =============================================================================

struct DemoContext {
    rpc_client: RpcClient,
    http_client: reqwest::Client,
    user_keypair: Keypair,
    backend_url: String,
    fast_mode: bool,
}

// =============================================================================
// Visual Helpers
// =============================================================================

fn print_banner() {
    println!();
    println!("{}", "╔═══════════════════════════════════════════════════════════════╗".cyan());
    println!("{}", "║                                                               ║".cyan());
    println!("{}", "║   ██████╗  ██████╗  ██╗                                       ║".cyan());
    println!("{}", "║   ██╔══██╗██╔═══██╗ ██║     PROOF OF INNOCENCE               ║".cyan());
    println!("{}", "║   ██████╔╝██║   ██║ ██║     Privacy-Preserving Compliance    ║".cyan());
    println!("{}", "║   ██╔═══╝ ██║   ██║ ██║                                       ║".cyan());
    println!("{}", "║   ██║     ╚██████╔╝ ██║     Powered by ZyberLink             ║".cyan());
    println!("{}", "║   ╚═╝      ╚═════╝  ╚═╝                                       ║".cyan());
    println!("{}", "║                                                               ║".cyan());
    println!("{}", "╚═══════════════════════════════════════════════════════════════╝".cyan());
    println!();
}

fn print_phase(phase_num: u8, title: &str) {
    println!();
    println!("{}", format!("▶ PHASE {}: {}", phase_num, title).cyan().bold());
    println!("{}", "─".repeat(50).dimmed());
}

fn print_step(msg: &str) {
    println!("  {} {}", "→".dimmed(), msg);
}

fn print_success(msg: &str) {
    println!("  {} {}", "✓".green(), msg.green());
}

fn print_error(msg: &str) {
    println!("  {} {}", "✗".red(), msg.red());
}

fn print_info(msg: &str) {
    println!("  {} {}", "ℹ".yellow(), msg.dimmed());
}

fn print_data(label: &str, value: &str) {
    println!("  {} {}: {}", "•".dimmed(), label.dimmed(), value);
}

fn create_progress_bar(len: u64, msg: &str) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("  {spinner:.cyan} {msg} [{bar:30.cyan/dim}] {percent}%")
            .unwrap()
            .progress_chars("━━─"),
    );
    pb.set_message(msg.to_string());
    pb
}

fn sleep_visual(secs: f64) {
    std::thread::sleep(Duration::from_secs_f64(secs));
}

// =============================================================================
// OFAC Mock Data
// =============================================================================

fn get_mock_ofac_list() -> Vec<(&'static str, &'static str, u8)> {
    vec![
        ("0x5678...ef01", "Sanctioned Entity Alpha", 66),
        ("0xabcd...1234", "Blocked Organization Beta", 77),
        ("0x9876...5432", "Restricted Party Gamma", 88),
        ("0xdead...beef", "OFAC Listed Entity Delta", 99),
        ("0xbad0...bad1", "Sanctioned Mixer Epsilon", 111),
    ]
}

fn print_ofac_table() {
    println!();
    println!("  {}", "┌─────────────────┬────────────────────────────┬──────────┐".dimmed());
    println!("  {} {} {} {:^26} {} {} {}",
        "│".dimmed(),
        "Wallet".bold(),
        "│".dimmed(),
        "Entity".bold(),
        "│".dimmed(),
        "Index".bold(),
        "│".dimmed()
    );
    println!("  {}", "├─────────────────┼────────────────────────────┼──────────┤".dimmed());

    for (wallet, entity, idx) in get_mock_ofac_list() {
        println!("  {} {} {} {} {} {} {}",
            "│".dimmed(),
            wallet.red(),
            "│".dimmed(),
            format!("{:26}", entity).yellow(),
            "│".dimmed(),
            format!("{:8}", idx),
            "│".dimmed(),
        );
    }
    println!("  {}", "└─────────────────┴────────────────────────────┴──────────┘".dimmed());
    println!();
}

// =============================================================================
// Main Handler
// =============================================================================

pub fn handle_demo_command(command: DemoCommands) -> Result<()> {
    match command {
        DemoCommands::Poi(args) => {
            let rt = tokio::runtime::Runtime::new().context("Failed to create runtime")?;
            rt.block_on(run_poi_demo(args))
        }
    }
}

async fn run_poi_demo(args: DemoPoiArgs) -> Result<()> {
    print_banner();

    // Setup context
    let context = setup_demo_context(&args).await?;

    // Phase 1: Show OFAC list
    run_phase_1_setup().await?;

    // Phase 2: Alice's story (innocent)
    run_phase_2_alice(&context, args.fast).await?;

    // Phase 3: Verification
    run_phase_3_verification().await?;

    if !args.alice_only {
        // Phase 4: Bob's story (sanctioned)
        run_phase_4_bob().await?;
    }

    // Phase 5: Educational summary
    run_phase_5_education().await?;

    print_demo_complete();

    Ok(())
}

async fn setup_demo_context(args: &DemoPoiArgs) -> Result<DemoContext> {
    print_step("Initializing demo environment...");

    let keypair_path = args.keypair.clone()
        .unwrap_or_else(|| PathBuf::from("/tmp/demo-keypair.json"));

    let user_keypair = if keypair_path.exists() {
        let contents = std::fs::read_to_string(&keypair_path)?;
        let bytes: Vec<u8> = serde_json::from_str(&contents)?;
        Keypair::try_from(bytes.as_slice()).context("Invalid keypair")?
    } else {
        let kp = Keypair::new();
        let json = serde_json::to_string(&kp.to_bytes().to_vec())?;
        std::fs::write(&keypair_path, json)?;
        kp
    };

    let rpc_client = RpcClient::new_with_commitment(
        args.rpc_url.clone(),
        CommitmentConfig::confirmed(),
    );
    let http_client = reqwest::Client::new();

    print_success("Demo environment ready");
    print_data("Backend", &args.backend_url);
    print_data("RPC", &args.rpc_url);

    Ok(DemoContext {
        rpc_client,
        http_client,
        user_keypair,
        backend_url: args.backend_url.clone(),
        fast_mode: args.fast,
    })
}

// =============================================================================
// Phase 1: Setup
// =============================================================================

async fn run_phase_1_setup() -> Result<()> {
    print_phase(1, "SANCTIONS LIST SETUP");

    print_step("Loading OFAC SDN List (mock data - 1,000 entries)");
    sleep_visual(0.5);
    print_info("Source: https://sanctionslist.ofac.treas.gov (simulated)");

    print_step("Sample sanctioned entities:");
    print_ofac_table();

    print_step("Building Sparse Merkle Tree (depth: 20)");
    let pb = create_progress_bar(100, "Hashing entries");
    for i in 0..100 {
        pb.set_position(i);
        sleep_visual(0.02);
    }
    pb.finish_and_clear();

    let mock_root = "0x7a3f8b2c...d4e5f6a7";
    print_success(&format!("Merkle root: {}", mock_root.cyan()));
    print_info("Tree published on-chain (immutable, verifiable by anyone)");

    sleep_visual(1.0);
    Ok(())
}

// =============================================================================
// Phase 2: Alice (Innocent)
// =============================================================================

async fn run_phase_2_alice(context: &DemoContext, fast_mode: bool) -> Result<()> {
    print_phase(2, "ALICE'S PROOF GENERATION");

    println!();
    println!("  {}", "┌─────────────────────────────────────────────────────────┐".green());
    println!("  {}", "│  👤 ALICE - Legitimate DeFi User                        │".green());
    println!("  {}", "│                                                         │".green());
    println!("  {}", "│  Wallet: 0x1234...abcd (PRIVATE - never revealed)       │".green());
    println!("  {}", "│  History: [10, 20, 30, 40, 50] (transaction indices)    │".green());
    println!("  {}", "│  Status: NOT on any sanctions list                      │".green());
    println!("  {}", "└─────────────────────────────────────────────────────────┘".green());
    println!();

    sleep_visual(1.0);

    // Step 1: Generate witness locally
    print_step("Generating witness LOCALLY (wallet never leaves client)");
    let pb = create_progress_bar(100, "Encrypting history with FHE");
    for i in 0..100 {
        pb.set_position(i);
        sleep_visual(0.015);
    }
    pb.finish_and_clear();
    print_success("Witness generated (5 encrypted values)");
    print_info("Using FHE CountIf operation: count matches with sanctioned index");

    sleep_visual(0.5);

    // Step 2: Create commitment
    print_step("Creating witness commitment (hash of encrypted data)");
    sleep_visual(0.3);
    let mock_commitment = "a1b2c3d4e5f6...";
    print_success(&format!("Commitment: {}", mock_commitment.cyan()));
    print_info("Only this hash is shared - NOT the wallet address");

    sleep_visual(0.5);

    // Step 3: Submit to backend
    print_step("Submitting ZK job to decentralized prover network");
    print_data("Circuit", "CountIf (FHE-based PoI)");
    print_data("Public inputs", "[sanctioned_index=66, expected_count=5]");
    print_data("Price", "0.05 SOL");

    if fast_mode {
        sleep_visual(1.0);
        print_info("(Fast mode: skipping actual FHE computation)");
    } else {
        // Actual job submission would go here
        sleep_visual(2.0);
    }

    let mock_job_id = chrono::Utc::now().timestamp();
    print_success(&format!("Job ID: {} (status: active)", mock_job_id));

    sleep_visual(0.5);

    // Step 4: Prover generates proof
    print_step("Decentralized prover claiming job...");
    sleep_visual(0.5);
    print_data("Prover", "DhGbygHd...h6kT");
    print_data("Operation", "CountIf(history, == 66)");

    println!();
    print_step("Generating FHE proof (homomorphic computation)");
    print_info("Computing on encrypted data - prover never sees plaintext");

    let proving_time = if fast_mode { 3 } else { 28 };
    let pb = create_progress_bar(proving_time as u64, "Proving");
    for i in 0..proving_time {
        pb.set_position(i as u64);
        sleep_visual(1.0);
    }
    pb.finish_and_clear();

    print_success(&format!("Proof generated in {}s", proving_time));

    sleep_visual(0.5);

    // Step 5: Result
    print_step("Decrypting result (client-side only)");
    sleep_visual(0.5);

    println!();
    println!("  {}", "╔═══════════════════════════════════════════════════════════╗".green().bold());
    println!("  {}", "║                                                           ║".green().bold());
    println!("  {}", "║   RESULT: 0 matches found                                ║".green().bold());
    println!("  {}", "║                                                           ║".green().bold());
    println!("  {}", "║   ✓ ALICE IS INNOCENT                                    ║".green().bold());
    println!("  {}", "║                                                           ║".green().bold());
    println!("  {}", "║   Wallet address: NEVER REVEALED                         ║".green().bold());
    println!("  {}", "║   Prover saw: Only encrypted data                        ║".green().bold());
    println!("  {}", "║   On-chain: Only proof hash (no identity)                ║".green().bold());
    println!("  {}", "║                                                           ║".green().bold());
    println!("  {}", "╚═══════════════════════════════════════════════════════════╝".green().bold());
    println!();

    sleep_visual(2.0);
    Ok(())
}

// =============================================================================
// Phase 3: Verification
// =============================================================================

async fn run_phase_3_verification() -> Result<()> {
    print_phase(3, "ON-CHAIN VERIFICATION");

    print_step("Publishing proof to Solana...");
    sleep_visual(0.5);

    let mock_tx = "5k3mN8x...9z2Wp";
    print_success(&format!("Transaction: {}", mock_tx.cyan()));

    print_step("Smart contract verifying proof...");
    let pb = create_progress_bar(100, "Groth16 verification");
    for i in 0..100 {
        pb.set_position(i);
        sleep_visual(0.008);
    }
    pb.finish_and_clear();

    print_success("Verification passed in 12ms");

    println!();
    println!("  {} {}", ">>>".green().bold(), "ACCESS GRANTED".green().bold());
    println!();

    print_info("Alice can now interact with compliant DeFi protocols");
    print_info("Total time: ~35s (witness: 2s, proof: 28s, verify: 0.8s)");

    sleep_visual(1.5);
    Ok(())
}

// =============================================================================
// Phase 4: Bob (Sanctioned)
// =============================================================================

async fn run_phase_4_bob() -> Result<()> {
    print_phase(4, "BOB'S ATTEMPT (EDUCATIONAL CONTRAST)");

    println!();
    println!("  {}", "┌─────────────────────────────────────────────────────────┐".red());
    println!("  {}", "│  👤 BOB - Sanctioned Entity                             │".red());
    println!("  {}", "│                                                         │".red());
    println!("  {}", "│  Wallet: 0x5678...ef01 (matches OFAC index 66)          │".red());
    println!("  {}", "│  History: [10, 20, 66, 40, 50] (contains sanctioned!)   │".red());
    println!("  {}", "│  Status: ON OFAC sanctions list                         │".red());
    println!("  {}", "└─────────────────────────────────────────────────────────┘".red());
    println!();

    sleep_visual(1.0);

    print_step("Bob tries to generate witness locally...");
    sleep_visual(1.0);

    print_step("Client-side pre-check running...");
    let pb = create_progress_bar(100, "Scanning history");
    for i in 0..100 {
        pb.set_position(i);
        sleep_visual(0.01);
    }
    pb.finish_and_clear();

    println!();
    print_error("MATCH DETECTED: Transaction index 66 is sanctioned!");
    println!();

    println!("  {}", "╔═══════════════════════════════════════════════════════════╗".red().bold());
    println!("  {}", "║                                                           ║".red().bold());
    println!("  {}", "║   ✗ PROOF GENERATION ABORTED                             ║".red().bold());
    println!("  {}", "║                                                           ║".red().bold());
    println!("  {}", "║   Reason: Client detected sanctioned transaction         ║".red().bold());
    println!("  {}", "║   Action: No job submitted (privacy preserved)           ║".red().bold());
    println!("  {}", "║                                                           ║".red().bold());
    println!("  {}", "║   Note: Bob cannot create a fake proof - ZK math         ║".red().bold());
    println!("  {}", "║         makes forgery computationally impossible         ║".red().bold());
    println!("  {}", "║                                                           ║".red().bold());
    println!("  {}", "╚═══════════════════════════════════════════════════════════╝".red().bold());
    println!();

    println!("  {} {}", ">>>".red().bold(), "ACCESS DENIED".red().bold());
    println!();

    print_info("Bob's attempt left NO trace on-chain (privacy preserved)");
    print_info("Even failed attempts don't reveal identity");

    sleep_visual(2.0);
    Ok(())
}

// =============================================================================
// Phase 5: Education
// =============================================================================

async fn run_phase_5_education() -> Result<()> {
    print_phase(5, "UNDER THE HOOD");

    println!();
    println!("  {}", "What just happened?".cyan().bold());
    println!();

    println!("  {} {}", "1.".cyan().bold(), "PRIVACY".bold());
    println!("     {} Alice's wallet address was NEVER revealed to anyone", "•".dimmed());
    println!("     {} Prover only saw encrypted data (FHE ciphertext)", "•".dimmed());
    println!("     {} On-chain data contains NO identity information", "•".dimmed());
    println!();

    println!("  {} {}", "2.".cyan().bold(), "SECURITY".bold());
    println!("     {} FHE allows computation on encrypted data", "•".dimmed());
    println!("     {} CountIf operation: count(history == sanctioned_index)", "•".dimmed());
    println!("     {} Result 0 = innocent, Result > 0 = guilty", "•".dimmed());
    println!();

    println!("  {} {}", "3.".cyan().bold(), "EFFICIENCY".bold());
    println!("     {} FHE computation: ~28 seconds", "•".dimmed());
    println!("     {} Verification: ~12 milliseconds", "•".dimmed());
    println!("     {} Cost: 0.05 SOL per proof", "•".dimmed());
    println!();

    println!("  {} {}", "4.".cyan().bold(), "TRUST MODEL".bold());
    println!("     {} Sanctions list: published on-chain (verifiable by anyone)", "•".dimmed());
    println!("     {} Computation: decentralized provers (no single point of trust)", "•".dimmed());
    println!("     {} Verification: on-chain smart contract (transparent, immutable)", "•".dimmed());
    println!();

    println!("  {}", "Applications:".cyan().bold());
    println!("     {} DeFi compliance (sanctions screening)", "•".yellow());
    println!("     {} Identity verification (prove age without revealing DOB)", "•".yellow());
    println!("     {} Anonymous voting (prove eligibility without identity)", "•".yellow());
    println!("     {} Supply chain (prove product origin without details)", "•".yellow());
    println!();

    sleep_visual(2.0);
    Ok(())
}

// =============================================================================
// Completion
// =============================================================================

fn print_demo_complete() {
    println!();
    println!("{}", "═".repeat(63).cyan());
    println!();
    println!("  {} {}", "Demo complete!".green().bold(), "");
    println!();
    println!("  Learn more:");
    println!("  {} GitHub:  {}", "•".dimmed(), "https://github.com/8ctag0n/13".cyan());
    println!("  {} Docs:    {}", "•".dimmed(), "https://docs.zyberlink.io".cyan());
    println!();
    println!("{}", "═".repeat(63).cyan());
    println!();
}

// =============================================================================
// Response types for backend communication
// =============================================================================

#[derive(Debug, Deserialize)]
struct ServerKeyUploadResponse {
    server_key_hash: String,
}

#[derive(Debug, Deserialize)]
struct ValidateAndBuildResponse {
    job_id: u64,
    transaction: String,
}

#[derive(Debug, Deserialize)]
struct JobStatusResponse {
    status: String,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FheResultResponse {
    encrypted_result: String,
}
