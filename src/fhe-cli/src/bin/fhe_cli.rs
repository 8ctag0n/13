/// FHE CLI Tool for ZyberLink
///
/// Unified tool for FHE encryption and decryption.
/// Users download this single binary to interact with ZyberLink jobs.
use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use fhe_cli::*;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "fhe-cli")]
#[command(about = "ZyberLink FHE encryption/decryption tool")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Encrypt data for a ZyberLink job
    Encrypt {
        /// Path to save generated files
        #[arg(short, long, default_value = "./fhe-output")]
        path: PathBuf,

        /// Value to encrypt (0-255). If not provided, will prompt interactively.
        #[arg(short, long)]
        value: Option<u8>,
    },

    /// Decrypt result from a completed job
    Decrypt {
        /// Path to the job folder (containing client_key.bin)
        #[arg(short, long)]
        path: PathBuf,

        /// Encrypted result (base64). If not provided, will prompt interactively.
        #[arg(short, long)]
        result: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Encrypt { path, value } => encrypt_command(path, value),
        Commands::Decrypt { path, result } => decrypt_command(path, result),
    }
}

// ============================================
// ENCRYPT COMMAND
// ============================================

fn encrypt_command(path: PathBuf, value: Option<u8>) -> Result<()> {
    println!("\n{}", "=== ZyberLink FHE Encrypt ===".bold().cyan());

    // Get value to encrypt
    let value = match value {
        Some(v) => {
            println!("  Value: {}", v.to_string().bright_yellow());
            v
        }
        None => get_value_interactive()?,
    };

    // Create output directory
    fs::create_dir_all(&path)?;
    println!(
        "\n{} Path: {}",
        ">>".cyan(),
        path.display().to_string().bright_blue()
    );

    // Generate keypair
    println!("\n[*] Generating FHE keypair...");
    println!("    (This may take 1-2 seconds)");

    let (client_key, server_key) = generate_fhe_keys()?;
    println!("[+] Keypair generated");

    // Encrypt the value
    println!("\n[*] Encrypting value: {}", value);
    let encrypted_data = encrypt_value(value, &client_key)?;
    println!("[+] Value encrypted successfully");

    // Save all files
    println!("\n[*] Saving files...");

    // 1. Client key (SECRET - for decryption)
    let client_key_path = path.join("client_key.bin");
    save_client_key(&client_key, &client_key_path)?;
    println!(
        "    [+] {} (KEEP SECRET)",
        "client_key.bin".bright_red()
    );

    // 2. Server key (for provers to compute)
    let server_key_path = path.join("server_key.bin");
    save_server_key(&server_key, &server_key_path)?;
    let server_key_size = fs::metadata(&server_key_path)?.len();
    println!(
        "    [+] {} ({:.1} MB)",
        "server_key.bin".bright_white(),
        server_key_size as f64 / 1_000_000.0
    );

    // 3. Encrypted data
    let encrypted_path = path.join("encrypted_data.bin");
    save_encrypted_data(&encrypted_data, &encrypted_path)?;
    println!(
        "    [+] {} ({} bytes)",
        "encrypted_data.bin".bright_white(),
        encrypted_data.len()
    );

    // 4. Witness file (combined for job creation)
    let witness_path = path.join("witness.bin");
    create_witness_file(&server_key, &encrypted_data, &witness_path)?;
    let witness_size = fs::metadata(&witness_path)?.len();
    println!(
        "    [+] {} ({:.1} MB) <- upload this",
        "witness.bin".bright_green(),
        witness_size as f64 / 1_000_000.0
    );

    // 5. Metadata
    let metadata_path = path.join("metadata.json");
    create_metadata_file(value, &metadata_path)?;
    println!("    [+] {}", "metadata.json".bright_white());

    // Print summary
    println!("\n{}", "=".repeat(50).bright_black());
    println!("{}", "NEXT STEPS:".bold().green());
    println!("{}", "=".repeat(50).bright_black());

    println!(
        "\n1. Upload {} when creating a job",
        "witness.bin".bright_green()
    );

    println!("\n2. After job completes, decrypt with:");
    println!(
        "   fhe-cli decrypt -p {}",
        path.display()
    );

    println!("\n{}", "=".repeat(50).bright_black());
    println!();

    Ok(())
}

fn get_value_interactive() -> Result<u8> {
    loop {
        let input = read_input("\nEnter value to encrypt (0-255): ")?;

        match parse_u8(&input) {
            Ok(value) => return Ok(value),
            Err(e) => {
                println!("[!] {}", e.to_string().red());
                continue;
            }
        }
    }
}

fn create_witness_file(
    server_key: &tfhe::ServerKey,
    encrypted_data: &[u8],
    filepath: &PathBuf,
) -> Result<()> {
    let server_key_bytes = bincode::serialize(server_key)?;
    let server_key_len = server_key_bytes.len() as u64;

    let mut witness = Vec::new();
    witness.extend_from_slice(&server_key_len.to_le_bytes());
    witness.extend_from_slice(&server_key_bytes);
    witness.extend_from_slice(encrypted_data);

    fs::write(filepath, witness)?;
    Ok(())
}

fn create_metadata_file(original_value: u8, filepath: &PathBuf) -> Result<()> {
    let metadata = serde_json::json!({
        "version": "1.0",
        "original_value": original_value,
        "created_at": chrono::Utc::now().to_rfc3339(),
        "files": {
            "client_key": "client_key.bin",
            "server_key": "server_key.bin",
            "encrypted_data": "encrypted_data.bin",
            "witness": "witness.bin"
        }
    });

    fs::write(filepath, serde_json::to_string_pretty(&metadata)?)?;
    Ok(())
}

// ============================================
// DECRYPT COMMAND
// ============================================

fn decrypt_command(path: PathBuf, result: Option<String>) -> Result<()> {
    println!("\n{}", "=== ZyberLink FHE Decrypt ===".bold().cyan());

    // Load client key from path
    let client_key_path = path.join("client_key.bin");

    if !client_key_path.exists() {
        anyhow::bail!(
            "Client key not found at: {}\nMake sure you're using the correct job path.",
            client_key_path.display()
        );
    }

    println!("[*] Loading client key from: {}", client_key_path.display());
    let client_key = load_client_key(&client_key_path)?;
    println!("[+] Client key loaded");

    // Get encrypted result
    let encrypted_result = match result {
        Some(r) => r,
        None => get_result_interactive()?,
    };

    // Decrypt
    println!("\n[*] Decrypting result...");
    let decrypted_bytes = deserialize_encrypted_base64(&encrypted_result)?;
    let decrypted_value = decrypt_value(&decrypted_bytes, &client_key)?;
    println!("[+] Decrypted successfully");

    // Print result
    println!("\n{}", "=".repeat(50).bright_black());
    println!(
        "  RESULT: {}",
        decrypted_value.to_string().bold().bright_yellow()
    );
    println!("{}", "=".repeat(50).bright_black());
    println!();

    Ok(())
}

fn get_result_interactive() -> Result<String> {
    println!("\nPaste the encrypted result (base64) from the completed job:");
    println!("(Press Enter twice when done)");

    let mut lines = Vec::new();

    loop {
        let input = read_input("> ")?;

        if input.is_empty() {
            break;
        }

        lines.push(input);
    }

    let result = lines.join("")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>();

    if result.is_empty() {
        anyhow::bail!("No result provided");
    }

    Ok(result)
}
