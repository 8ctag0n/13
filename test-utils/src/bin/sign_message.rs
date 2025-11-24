use anyhow::{Context, Result};
use base64::Engine;
use clap::Parser;
use solana_sdk::signature::{Keypair, Signer};
use std::fs;
use std::path::PathBuf;

/// Sign a message with Solana keypair and output base64 signature
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to Solana keypair JSON file
    #[arg(short, long)]
    keypair: PathBuf,

    /// Message to sign
    #[arg(short, long)]
    message: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Load keypair
    let keypair_data = fs::read_to_string(&args.keypair).context("Failed to read keypair file")?;
    let keypair_bytes: Vec<u8> =
        serde_json::from_str(&keypair_data).context("Failed to parse keypair JSON")?;
    let keypair =
        Keypair::from_bytes(&keypair_bytes).context("Failed to create keypair from bytes")?;

    // Sign message
    let message_bytes = args.message.as_bytes();
    let signature = keypair.sign_message(message_bytes);

    // Convert to base64
    let signature_bytes = signature.as_ref();
    let signature_b64 = base64::engine::general_purpose::STANDARD.encode(signature_bytes);

    // Output just the signature (for script usage)
    println!("{}", signature_b64);

    Ok(())
}
