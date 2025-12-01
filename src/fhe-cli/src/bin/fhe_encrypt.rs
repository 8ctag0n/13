/// FHE Encryption Tool for ZyberLink
///
/// Generates FHE keypair and encrypts user data locally.
/// Saves all files to an output directory for use with job creation.
use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use fhe_cli::*;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(name = "fhe-encrypt")]
#[command(about = "Encrypt data using FHE for ZyberLink jobs")]
struct Args {
    /// Path to save generated files
    #[arg(short, long, default_value = "./fhe-output")]
    path: PathBuf,

    /// Value to encrypt (0-255). If not provided, will prompt interactively.
    #[arg(short, long)]
    value: Option<u8>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    print_header();

    // Get value to encrypt
    let value = match args.value {
        Some(v) => {
            println!("  Value: {}", v.to_string().bright_yellow());
            v
        }
        None => get_value_from_user()?,
    };

    // Create output directory
    fs::create_dir_all(&args.path)?;
    println!(
        "\n{} Path: {}",
        ">>".cyan(),
        args.path.display().to_string().bright_blue()
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
    let client_key_path = args.path.join("client_key.bin");
    save_client_key(&client_key, &client_key_path)?;
    println!("    [+] {} (KEEP SECRET)", "client_key.bin".bright_red());

    // 2. Server key (for provers to compute)
    let server_key_path = args.path.join("server_key.bin");
    save_server_key(&server_key, &server_key_path)?;
    let server_key_size = fs::metadata(&server_key_path)?.len();
    println!(
        "    [+] {} ({:.1} MB)",
        "server_key.bin".bright_white(),
        server_key_size as f64 / 1_000_000.0
    );

    // 3. Encrypted data
    let encrypted_path = args.path.join("encrypted_data.bin");
    save_encrypted_data(&encrypted_data, &encrypted_path)?;
    println!(
        "    [+] {} ({} bytes)",
        "encrypted_data.bin".bright_white(),
        encrypted_data.len()
    );

    // 4. Witness file (combined for job creation)
    let witness_path = args.path.join("witness.bin");
    create_witness_file(&server_key, &encrypted_data, &witness_path)?;
    let witness_size = fs::metadata(&witness_path)?.len();
    println!(
        "    [+] {} ({:.1} MB) <- upload this",
        "witness.bin".bright_green(),
        witness_size as f64 / 1_000_000.0
    );

    // 5. Metadata
    let metadata_path = args.path.join("metadata.json");
    create_metadata_file(value, &metadata_path)?;
    println!("    [+] {}", "metadata.json".bright_white());

    // Print summary
    print_summary(&args.path);

    Ok(())
}

fn print_header() {
    println!(
        "\n{}",
        "=== ZyberLink FHE Encryption Tool ===".bold().cyan()
    );
}

fn get_value_from_user() -> Result<u8> {
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
    filepath: &Path,
) -> Result<()> {
    let encrypted_data_len = encrypted_data.len() as u32; // Use u32 for length

    let mut witness = Vec::new();
    witness.extend_from_slice(&encrypted_data_len.to_le_bytes()); // Write 4 bytes length
    witness.extend_from_slice(encrypted_data);
    witness.extend_from_slice(&bincode::serialize(server_key)?); // Append server key bytes

    fs::write(filepath, witness)?;
    Ok(())
}

fn create_metadata_file(original_value: u8, filepath: &Path) -> Result<()> {
    let metadata = serde_json::json!({
        "version": "1.0",
        "original_value": original_value,
        "created_at": chrono::Utc::now().to_rfc3339(),
        "files": {
            "client_key": "client_key.bin",
            "server_key": "server_key.bin",
            "encrypted_data": "encrypted_data.bin",
            "witness": "witness.bin"
        },
        "note": "Keep client_key.bin SECRET. Use witness.bin to create a job."
    });

    fs::write(filepath, serde_json::to_string_pretty(&metadata)?)?;
    Ok(())
}

fn print_summary(output_dir: &Path) {
    println!("\n{}", "=".repeat(50).bright_black());
    println!("{}", "NEXT STEPS:".bold().green());
    println!("{}", "=".repeat(50).bright_black());

    println!(
        "\n1. Keep {} safe (needed to decrypt)",
        "client_key.bin".bright_red()
    );

    println!(
        "\n2. Upload {} when creating a job:",
        "witness.bin".bright_green()
    );
    println!(
        "   job-creator --witness {}/witness.bin",
        output_dir.display()
    );

    println!("\n3. After job completes, decrypt with:");
    println!(
        "   fhe-decrypt --key {}/client_key.bin",
        output_dir.display()
    );

    println!("\n{}", "=".repeat(50).bright_black());
    println!();
}
