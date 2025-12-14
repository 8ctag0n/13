use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::PathBuf;

use crate::*;

pub fn run(path: PathBuf, value: Option<u8>, values: Option<Vec<u8>>) -> Result<()> {
    println!("\n{}", "=== ZyberLink FHE Encrypt ===".bold().cyan());

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

    // Encrypt the value(s)
    let (encrypted_data, original_values) = if let Some(vals) = values {
        println!("\n[*] Encrypting values for aggregation: {:?}", vals);
        let mut encrypted_values = Vec::new();
        for v in &vals {
            let encrypted = encrypt_value(*v, &client_key)?;
            encrypted_values.push(encrypted);
        }
        (bincode::serialize(&encrypted_values)?, vals)
    } else {
        let val = match value {
            Some(v) => {
                println!("\n[*] Encrypting value: {}", v.to_string().bright_yellow());
                v
            }
            None => get_value_interactive()?,
        };
        (encrypt_value(val, &client_key)?, vec![val])
    };
    println!("[+] Encryption successful");

    // Save all files
    println!("\n[*] Saving files...");

    // 1. Client key (SECRET - for decryption)
    let client_key_path = path.join("client_key.bin");
    save_client_key(&client_key, &client_key_path)?;
    println!("    [+] {} (KEEP SECRET)", "client_key.bin".bright_red());

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
    create_metadata_file(&original_values, &metadata_path)?;
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
    println!("   zyb fhe decrypt -p {}", path.display());

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
    // Serialize server_key first to get its length
    let server_key_bytes = bincode::serialize(server_key)?;
    let server_key_len = server_key_bytes.len() as u64;

    // Format: [8 bytes: server_key_len (u64 LE)] [server_key] [encrypted_data]
    let mut witness = Vec::new();
    witness.extend_from_slice(&server_key_len.to_le_bytes()); // Write 8 bytes length
    witness.extend_from_slice(&server_key_bytes);
    witness.extend_from_slice(encrypted_data);

    fs::write(filepath, witness)?;
    Ok(())
}

fn create_metadata_file(original_values: &[u8], filepath: &PathBuf) -> Result<()> {
    let metadata = serde_json::json!({
        "version": "1.0",
        "original_values": original_values,
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
