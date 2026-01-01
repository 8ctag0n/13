use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;

use crate::*;

pub fn run(path: PathBuf, result: Option<String>) -> Result<()> {
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

    let result = lines
        .join("")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>();

    if result.is_empty() {
        anyhow::bail!("No result provided");
    }

    Ok(result)
}
