use super::{terms, ui, validation};
use crate::marketplace::ChainType;
use anyhow::{Context, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{read_keypair_file, write_keypair_file, Keypair, Signer},
};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum Network {
    Localnet,
    Devnet,
    Testnet,
    MainnetBeta,
}

impl Network {
    pub fn name(&self) -> &'static str {
        match self {
            Network::Localnet => "Localnet",
            Network::Devnet => "Devnet",
            Network::Testnet => "Testnet",
            Network::MainnetBeta => "Mainnet-Beta",
        }
    }

    pub fn default_rpc_url(&self) -> &'static str {
        match self {
            Network::Localnet => "http://localhost:8899",
            Network::Devnet => "https://api.devnet.solana.com",
            Network::Testnet => "https://api.testnet.solana.com",
            Network::MainnetBeta => "https://api.mainnet-beta.solana.com",
        }
    }

    pub fn supports_airdrop(&self) -> bool {
        matches!(self, Network::Localnet | Network::Devnet | Network::Testnet)
    }

    pub fn from_rpc_url(url: &str) -> Self {
        if url.contains("localhost") || url.contains("127.0.0.1") {
            Network::Localnet
        } else if url.contains("devnet") {
            Network::Devnet
        } else if url.contains("testnet") {
            Network::Testnet
        } else {
            Network::MainnetBeta
        }
    }
}

/// Step 1: Validate system requirements
pub async fn step_system_validation() -> Result<()> {
    ui::print_step_header(1, 7, "System Requirements Check");

    let spinner = ui::Spinner::new("Checking system requirements...");
    let results = validation::validate_system().await?;
    spinner.success("System checks complete");

    println!();
    for result in &results {
        ui::print_check_result(&result.name, result.passed, result.details.as_deref());
    }

    if !validation::all_passed(&results) {
        ui::print_error("\nSome system requirements are not met!");
        ui::print_info("Please fix the issues above and try again.");
        return Err(anyhow::anyhow!("System validation failed"));
    }

    ui::print_success("\nAll system requirements met!");
    ui::wait_for_enter()?;

    Ok(())
}

/// Step 2: Setup Solana keypair
pub async fn step_keypair_setup() -> Result<(Keypair, PathBuf)> {
    ui::print_step_header(2, 7, "Keypair Setup");

    let choice = ui::select(
        "Do you have an existing Solana keypair?",
        &["Use existing keypair", "Generate new keypair"],
    )?;

    let (keypair, path) = match choice {
        0 => load_existing_keypair().await?,
        1 => generate_new_keypair().await?,
        _ => unreachable!(),
    };

    println!();
    ui::print_success("Keypair loaded successfully");
    println!("  Public Key: {}", keypair.pubkey());

    ui::wait_for_enter()?;

    Ok((keypair, path))
}

async fn load_existing_keypair() -> Result<(Keypair, PathBuf)> {
    let default_path = shellexpand::tilde("~/.config/solana/id.json").to_string();

    let path_str = ui::input_with_validation(
        "Enter path to keypair file",
        Some(&default_path),
        |input| {
            let expanded = shellexpand::tilde(input);
            let path = PathBuf::from(expanded.as_ref());
            validation::validate_file_path(&path)
        },
        3, // max retries
    )?;

    let expanded = shellexpand::tilde(&path_str);
    let path = PathBuf::from(expanded.as_ref());

    let keypair = read_keypair_file(&path).map_err(|e| {
        anyhow::anyhow!(
            "Failed to read keypair file: {}\n\nMake sure the file is a valid Solana keypair JSON.",
            e
        )
    })?;

    Ok((keypair, path))
}

async fn generate_new_keypair() -> Result<(Keypair, PathBuf)> {
    ui::print_warning("Generating a new keypair...");

    let keypair = Keypair::new();

    let default_path = shellexpand::tilde("~/.config/solana/id.json").to_string();
    let path_str = ui::input("Save keypair to", Some(&default_path))?;

    let expanded = shellexpand::tilde(&path_str);
    let path = PathBuf::from(expanded.as_ref());

    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("Failed to create keypair directory")?;
    }

    write_keypair_file(&keypair, &path)
        .map_err(|e| anyhow::anyhow!("Failed to write keypair file: {}", e))?;

    ui::print_success(&format!("Keypair saved to: {}", path.display()));

    Ok((keypair, path))
}

/// Step 3: Network configuration
pub async fn step_network_setup() -> Result<(Network, String, RpcClient)> {
    ui::print_step_header(3, 7, "Network Configuration");

    let choice = ui::select(
        "Select Solana network:",
        &[
            "Devnet (recommended for testing)",
            "Testnet (public testing)",
            "Localnet (local validator)",
            "Mainnet-Beta (production)",
        ],
    )?;

    let network = match choice {
        0 => Network::Devnet,
        1 => Network::Testnet,
        2 => Network::Localnet,
        3 => Network::MainnetBeta,
        _ => unreachable!(),
    };

    let default_url = network.default_rpc_url();

    // Get RPC URL with validation
    let rpc_url = ui::input_with_validation(
        "RPC URL",
        Some(default_url),
        validation::validate_rpc_url,
        3, // max retries
    )?;

    // Test connection
    let spinner = ui::Spinner::new("Testing connection...");
    let client = RpcClient::new_with_commitment(rpc_url.clone(), CommitmentConfig::confirmed());

    match client.get_version() {
        Ok(version) => {
            spinner.success(&format!("Connected! (version: {})", version.solana_core));
        }
        Err(e) => {
            spinner.error("Connection failed");
            ui::print_error(&format!(
                "Failed to connect to RPC: {}\n\nTips:\n  - Check if the URL is correct\n  - Verify internet connection\n  - Try a different RPC endpoint",
                e
            ));
            return Err(anyhow::anyhow!("RPC connection failed"));
        }
    }

    println!();
    ui::print_info(&format!("Network: {}", network.name()));
    ui::print_info(&format!("RPC URL: {}", rpc_url));

    ui::wait_for_enter()?;

    Ok((network, rpc_url, client))
}

/// Step 4: Balance check and funding
pub async fn step_balance_check(
    client: &RpcClient,
    keypair: &Keypair,
    network: &Network,
    required_lamports: u64,
) -> Result<()> {
    ui::print_step_header(4, 7, "Balance Check & Funding");

    let balance = client
        .get_balance(&keypair.pubkey())
        .context("Failed to fetch balance")?;

    let required_sol = required_lamports as f64 / 1_000_000_000.0;
    let balance_sol = balance as f64 / 1_000_000_000.0;

    println!("\n{}", console::style("Balance Requirements:").bold());
    println!("  • Prover stake:        {:.4} SOL", required_sol);
    println!("  • Transaction fees:    ~0.0015 SOL");
    println!(
        "  {} Total required:      {:.4} SOL",
        console::style("").dim(),
        required_sol + 0.0015
    );
    println!();
    println!("  Current balance:       {:.4} SOL", balance_sol);

    if balance >= required_lamports {
        ui::print_success("✓ Sufficient balance for registration!");
        ui::wait_for_enter()?;
        return Ok(());
    }

    // Insufficient balance - show Solana Pay funding
    let shortfall = (required_lamports - balance) as f64 / 1_000_000_000.0;
    ui::print_warning(&format!(
        "⚠ Insufficient balance! Need {:.4} more SOL",
        shortfall
    ));

    // Generate Solana Pay URL
    let solana_pay_url = ui::generate_solana_pay_url(
        &keypair.pubkey().to_string(),
        required_lamports - balance,
        "ZyberLink Prover Stake",
        "Fund your prover node to start earning",
    );

    // Generate Solana Blink URL
    let blink_url =
        ui::generate_blink_url(&keypair.pubkey().to_string(), required_lamports - balance);

    // Display QR code and funding instructions
    ui::print_solana_pay_qr(&solana_pay_url)?;
    ui::print_funding_instructions(
        &solana_pay_url,
        &blink_url,
        &keypair.pubkey().to_string(),
        shortfall,
    );

    // For devnet/testnet, still offer airdrop as fallback
    if network.supports_airdrop() {
        println!("{}", console::style("Devnet/Testnet Options:").bold());
        let choice = ui::select(
            "How would you like to proceed?",
            &[
                "Wait for Solana Pay transfer",
                &format!("Request airdrop ({} only)", network.name()),
                "Skip funding (exit wizard)",
            ],
        )?;

        match choice {
            0 => wait_for_manual_funding(client, keypair, required_lamports).await?,
            1 => request_airdrop(client, keypair, required_lamports - balance).await?,
            2 => return Err(anyhow::anyhow!("User canceled funding")),
            _ => unreachable!(),
        }
    } else {
        // Mainnet - only wait for transfer
        wait_for_manual_funding(client, keypair, required_lamports).await?;
    }

    Ok(())
}

async fn request_airdrop(client: &RpcClient, keypair: &Keypair, amount: u64) -> Result<()> {
    ui::print_info("Requesting airdrop...");

    match client.request_airdrop(&keypair.pubkey(), amount) {
        Ok(sig) => {
            let spinner = ui::Spinner::new("Confirming airdrop...");
            std::thread::sleep(std::time::Duration::from_secs(2));

            match client.confirm_transaction(&sig) {
                Ok(_) => spinner.success("Airdrop confirmed!"),
                Err(e) => {
                    spinner.error("Airdrop failed");
                    return Err(anyhow::anyhow!("Failed to confirm airdrop: {}", e));
                }
            }
        }
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to request airdrop: {}", e));
        }
    }

    Ok(())
}

async fn wait_for_manual_funding(
    client: &RpcClient,
    keypair: &Keypair,
    required: u64,
) -> Result<()> {
    use std::time::Duration;

    ui::print_info("Waiting for SOL transfer...");
    ui::print_info("(Checking balance every 5 seconds, press Ctrl+C to cancel)");

    let required_sol = required as f64 / 1_000_000_000.0;

    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;

        match client.get_balance(&keypair.pubkey()) {
            Ok(balance) => {
                let balance_sol = balance as f64 / 1_000_000_000.0;
                println!(
                    "  Current: {:.4} SOL / {:.4} SOL required",
                    balance_sol, required_sol
                );

                if balance >= required {
                    ui::print_success("✓ Received SOL!");
                    break;
                }
            }
            Err(e) => {
                ui::print_warning(&format!("Failed to check balance: {}", e));
            }
        }
    }

    Ok(())
}

/// Step 6: Register prover on-chain
pub async fn step_register_prover(
    client: &RpcClient,
    keypair: &Keypair,
    program_id: &Pubkey,
    stake_amount: u64,
    witness_encryption: &crate::witness_encryption::WitnessEncryption,
) -> Result<Pubkey> {
    ui::print_step_header(6, 7, "Prover Registration");

    let encryption_pubkey = witness_encryption.public_key();

    println!("\n{}", console::style("Registration Details:").bold());
    println!("  Authority:       {}", keypair.pubkey());
    println!("  Program ID:      {}", program_id);
    println!(
        "  Stake Amount:    {:.4} SOL",
        stake_amount as f64 / 1_000_000_000.0
    );
    println!(
        "  Encryption Key:  {}...",
        hex::encode(&encryption_pubkey[..8])
    );

    println!();
    if !ui::confirm("Proceed with registration?")? {
        return Err(anyhow::anyhow!("User canceled registration"));
    }

    // Create marketplace client
    let marketplace_client = zyberlink_sdk::MarketplaceClient::new_with_commitment(
        client.url(),
        *program_id,
        CommitmentConfig::confirmed(),
    );

    // Create register instruction
    let spinner = ui::Spinner::new("Creating registration transaction...");
    let ix = marketplace_client
        .register_prover_instruction(&keypair.pubkey(), stake_amount, encryption_pubkey)
        .context("Failed to create register instruction")?;

    spinner.update("Sending transaction...".to_string());

    // Send and confirm
    let sig = marketplace_client
        .send_and_confirm_transaction(&[ix], &[keypair])
        .context("Failed to send registration transaction")?;

    spinner.success("Registration confirmed!");

    // Get prover PDA
    let (prover_pda, _) = marketplace_client.get_prover_pda(&keypair.pubkey());

    println!();
    ui::print_success("✓ Prover registered successfully!");
    println!("  Signature:   {}", sig);
    println!("  Prover PDA:  {}", prover_pda);

    ui::wait_for_enter()?;

    Ok(prover_pda)
}

/// Step 5: Accept Terms & Conditions
pub async fn step_terms_acceptance(keypair: &Keypair, backend_url: &str) -> Result<()> {
    ui::print_step_header(5, 7, "Terms & Conditions");

    // Fetch latest terms
    let spinner = ui::Spinner::new("Fetching latest Terms & Conditions...");
    let terms_response = terms::fetch_terms(backend_url).await?;
    spinner.success("Terms loaded");

    println!();
    println!("{}", console::style("━".repeat(60)).cyan());
    println!(
        "{}",
        console::style("ZYBERLINK PROVER TERMS & CONDITIONS")
            .cyan()
            .bold()
    );
    println!("{}", console::style("━".repeat(60)).cyan());
    println!();
    println!(
        "{}",
        console::style(format!("Version: {}", terms_response.version)).dim()
    );
    println!(
        "{}",
        console::style(format!("Updated: {}", terms_response.updated_at)).dim()
    );
    println!();
    println!("{}", console::style("━".repeat(60)).cyan());
    println!();

    // Display terms (first 500 chars with scroll option)
    let preview = if terms_response.terms.len() > 500 {
        format!(
            "{}...\n\n(Full terms available at https://zyberlink.io/terms)",
            &terms_response.terms[..500]
        )
    } else {
        terms_response.terms.clone()
    };

    println!("{}", preview);
    println!();
    println!("{}", console::style("━".repeat(60)).cyan());
    println!();

    // Ask for acceptance
    if !ui::confirm("Do you accept these Terms & Conditions?")? {
        return Err(anyhow::anyhow!("User declined Terms & Conditions"));
    }

    // Sign terms
    let spinner = ui::Spinner::new("Signing Terms & Conditions...");
    let signature = terms::sign_terms(keypair, &terms_response.hash)?;
    spinner.success("Terms signed");

    // Save acceptance
    let acceptance = terms::TermsAcceptance {
        version: terms_response.version.clone(),
        accepted_at: chrono::Utc::now().to_rfc3339(),
        terms_hash: terms_response.hash,
        signature,
        signer_pubkey: keypair.pubkey().to_string(),
    };

    let path = terms::save_acceptance(&acceptance).await?;

    println!();
    ui::print_success("✓ Terms & Conditions accepted and signed!");
    println!("  Acceptance saved to: {}", path.display());

    ui::wait_for_enter()?;

    Ok(())
}

/// Step 0: Chain selection (before all other steps)
///
/// Allows the user to select which blockchain to use for the prover node.
/// Currently only Solana is fully supported, with Aptos and Starknet coming soon.
pub async fn step_chain_selection() -> Result<ChainType> {
    ui::print_step_header(0, 7, "Blockchain Selection");

    println!("\n{}", console::style("Select your target blockchain:").bold());
    println!();
    println!(
        "  {}",
        console::style("The prover node can operate on multiple blockchains.").dim()
    );
    println!(
        "  {}",
        console::style("Each chain has its own marketplace contract and token economics.").dim()
    );
    println!();

    let choice = ui::select(
        "Choose blockchain:",
        &[
            "Solana (SOL) - Production Ready",
            "Aptos (APT) - Coming Soon",
            "Starknet (STRK) - Coming Soon",
        ],
    )?;

    let chain = match choice {
        0 => {
            ui::print_success("Selected: Solana");
            ChainType::Solana
        }
        1 => {
            ui::print_warning("Aptos support is coming soon!");
            ui::print_info("For now, please use Solana. Aptos integration is in development.");
            println!();
            ui::print_info("Defaulting to Solana...");
            ChainType::Solana
        }
        2 => {
            ui::print_warning("Starknet support is coming soon!");
            ui::print_info("For now, please use Solana. Starknet integration is in development.");
            println!();
            ui::print_info("Defaulting to Solana...");
            ChainType::Solana
        }
        _ => unreachable!(),
    };

    println!();
    ui::print_info(&format!("Chain: {}", chain.as_str()));

    ui::wait_for_enter()?;

    Ok(chain)
}
