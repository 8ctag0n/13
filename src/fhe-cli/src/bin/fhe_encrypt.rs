/// FHE Encryption Tool for ZyberLink
///
/// Generates FHE keypair and encrypts user data locally.
/// Output can be pasted into the ZyberLink webapp.
use anyhow::Result;
use colored::Colorize;
use fhe_cli::*;

fn main() -> Result<()> {
    print_header();

    // Get value to encrypt
    let value = get_value_from_user()?;

    // Generate keypair
    println!("\n{} Generating FHE keypair...", "⏳".yellow());
    println!("   (This may take 1-2 seconds)");

    let (client_key, server_key) = generate_fhe_keys()?;

    println!("{} Keypair generated", "✅".green());

    // Encrypt the value
    println!("\n{} Encrypting value: {}", "🔒".cyan(), value);

    let encrypted_data = encrypt_value(value, &client_key)?;

    println!("{} Value encrypted successfully", "✅".green());

    // Serialize to base64
    let encrypted_base64 = serialize_encrypted_base64(&encrypted_data)?;
    let server_key_base64 = serialize_server_key_base64(&server_key)?;
    let client_key_base64 = serialize_client_key_base64(&client_key)?;

    // Save client key
    let keys_dir = get_keys_directory()?;
    let key_filename = generate_key_filename();
    let key_filepath = keys_dir.join(&key_filename);

    save_client_key(&client_key, &key_filepath)?;

    // Print output
    print_output(
        &encrypted_base64,
        &server_key_base64,
        &client_key_base64,
        &key_filepath,
    )?;

    Ok(())
}

fn print_header() {
    println!("\n{}", "🔐 ZyberLink FHE Encryption Tool".bold().cyan());
    println!("{}", "================================".cyan());
}

fn get_value_from_user() -> Result<u8> {
    loop {
        let input = read_input("\nEnter value to encrypt (0-255): ")?;

        match parse_u8(&input) {
            Ok(value) => return Ok(value),
            Err(e) => {
                println!("{} {}", "❌".red(), e.to_string().red());
                continue;
            }
        }
    }
}

fn print_output(
    encrypted_base64: &str,
    server_key_base64: &str,
    client_key_base64: &str,
    key_filepath: &std::path::PathBuf,
) -> Result<()> {
    println!("\n{}", "━".repeat(80).bright_black());
    println!("{}", "📋 COPY THESE TO THE WEBAPP:".bold().green());
    println!("{}", "━".repeat(80).bright_black());

    // Print encrypted data
    println!("\n{}", "Encrypted Data (base64):".bold());
    print_wrapped_base64(encrypted_base64)?;

    // Print server key
    println!("\n{}", "Server Key (base64):".bold());
    print_wrapped_base64(server_key_base64)?;

    // Print client key warning
    println!(
        "\n{} {}",
        "⚠️".yellow(),
        "KEEP THIS SECRET - Client Key (base64):".yellow().bold()
    );
    print_wrapped_base64_dimmed(client_key_base64)?;

    println!("\n{}", "━".repeat(80).bright_black());

    // Print save location
    println!(
        "\n{} Client key saved to: {}",
        "💾".cyan(),
        key_filepath.display().to_string().bright_blue()
    );

    println!(
        "\n{} Keep this file safe! You'll need it to decrypt the result.",
        "💡".yellow()
    );
    println!();

    Ok(())
}

fn print_wrapped_base64(text: &str) -> Result<()> {
    const LINE_WIDTH: usize = 76;

    for chunk in text.as_bytes().chunks(LINE_WIDTH) {
        let line = std::str::from_utf8(chunk)?;
        println!("{}", line.bright_white());
    }

    Ok(())
}

fn print_wrapped_base64_dimmed(text: &str) -> Result<()> {
    const LINE_WIDTH: usize = 76;

    for chunk in text.as_bytes().chunks(LINE_WIDTH) {
        let line = std::str::from_utf8(chunk)?;
        println!("{}", line.dimmed());
    }

    Ok(())
}
