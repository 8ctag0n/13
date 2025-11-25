/// FHE Decryption Tool for ZyberLink
///
/// Decrypts FHE computation results using the user's private client key.
use anyhow::{Context, Result};
use colored::Colorize;
use fhe_cli::*;
use std::path::PathBuf;

fn main() -> Result<()> {
    print_header();

    // Get encrypted result
    let encrypted_result = get_encrypted_result()?;

    // Get client key
    let client_key = get_client_key()?;

    // Decrypt
    println!("\n{} Decrypting result...", "🔓".cyan());

    let decrypted_bytes = deserialize_encrypted_base64(&encrypted_result)?;
    let result = decrypt_value(&decrypted_bytes, &client_key)?;

    println!("{} Result decrypted successfully", "✅".green());

    // Print result
    print_result(result);

    Ok(())
}

fn print_header() {
    println!("\n{}", "🔓 ZyberLink FHE Decryption Tool".bold().cyan());
    println!("{}", "=================================".cyan());
}

fn get_encrypted_result() -> Result<String> {
    if atty::is(atty::Stream::Stdin) {
        println!("\n{}", "Enter encrypted result (base64):".bold());
        println!(
            "{}",
            "(Paste the encrypted result from the webapp)".dimmed()
        );
    }

    // If piped input, just read one line directly
    if !atty::is(atty::Stream::Stdin) {
        let input = read_input("")?;
        if input.is_empty() {
            anyhow::bail!("Encrypted result cannot be empty");
        }
        return Ok(input);
    }

    // Interactive mode: allow multiline
    let input = read_multiline_input(">")?;

    if input.is_empty() {
        anyhow::bail!("Encrypted result cannot be empty");
    }

    Ok(input)
}

fn get_client_key() -> Result<tfhe::ClientKey> {
    if atty::is(atty::Stream::Stdin) {
        println!(
            "\n{}",
            "Enter client key (base64) or path to key file:".bold()
        );
        println!("{}", "(You can paste the key or provide a file path like ~/.zyberlink/client_keys/key_1234567890.txt)".dimmed());
    }

    let input = read_input("> ")?;

    if input.is_empty() {
        anyhow::bail!("Client key cannot be empty");
    }

    // Try to interpret as file path first
    if input.contains('/') || input.starts_with('~') || input.starts_with('.') {
        // Expand home directory
        let path = if input.starts_with('~') {
            let home = dirs::home_dir().context("Could not find home directory")?;
            let rest = input.trim_start_matches('~').trim_start_matches('/');
            home.join(rest)
        } else {
            PathBuf::from(&input)
        };

        println!("\n{} Loading client key from file...", "⏳".yellow());

        match load_client_key(&path) {
            Ok(key) => {
                println!("{} Client key loaded from file", "✅".green());
                return Ok(key);
            }
            Err(e) => {
                println!(
                    "{} Failed to load from file: {}",
                    "❌".red(),
                    e.to_string().red()
                );
                println!("{} Trying to parse as base64...", "⏳".yellow());
            }
        }
    }

    // Try to parse as base64
    println!("\n{} Loading client key...", "⏳".yellow());

    // Remove any whitespace or newlines
    let cleaned = input
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>();

    let client_key = deserialize_client_key_base64(&cleaned).context(
        "Failed to deserialize client key. Make sure it's valid base64 or a valid file path.",
    )?;

    println!("{} Client key loaded", "✅".green());

    Ok(client_key)
}

fn print_result(result: u8) {
    println!("\n{}", "━".repeat(80).bright_black());
    println!(
        "{} {}",
        "🎯 RESULT:".bold().green(),
        result.to_string().bold().bright_yellow()
    );
    println!("{}", "━".repeat(80).bright_black());
    println!();
}

fn read_multiline_input(prompt: &str) -> Result<String> {
    use std::io::{self, Write};

    // Only print prompt if interactive
    if atty::is(atty::Stream::Stdin) {
        print!("{} ", prompt);
        io::stdout().flush()?;
    }

    let mut lines = Vec::new();
    let stdin = io::stdin();

    loop {
        let mut line = String::new();
        stdin.read_line(&mut line)?;

        let trimmed = line.trim();

        // Empty line signals end of input
        if trimmed.is_empty() {
            break;
        }

        lines.push(trimmed.to_string());

        // If it looks like a single-line base64, accept it immediately
        if lines.len() == 1 && trimmed.len() > 20 && !trimmed.contains(' ') {
            break;
        }
    }

    // Join all lines and remove any whitespace
    let result = lines
        .join("")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();

    Ok(result)
}
