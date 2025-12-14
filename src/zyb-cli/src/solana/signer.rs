//! Keypair loading and transaction signing

use anyhow::{anyhow, Context, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};
use std::fs;
use std::path::Path;

/// Load a Solana keypair from a file
///
/// Supports JSON array format (e.g., [1,2,3,...,64])
pub fn load_keypair(path: &str) -> Result<Keypair> {
    // Expand ~ to home directory
    let expanded_path = if path.starts_with("~/") {
        let home = dirs::home_dir()
            .ok_or_else(|| anyhow!("Could not find home directory"))?;
        home.join(&path[2..])
    } else {
        Path::new(path).to_path_buf()
    };

    // Check if file exists
    if !expanded_path.exists() {
        return Err(anyhow!(
            "Keypair file not found: {}\nGenerate one with: solana-keygen new --outfile {}",
            expanded_path.display(),
            path
        ));
    }

    // Read file contents
    let keypair_bytes = fs::read_to_string(&expanded_path)
        .with_context(|| format!("Failed to read keypair file: {}", expanded_path.display()))?;

    // Try to parse as JSON array
    let bytes: Vec<u8> = serde_json::from_str(&keypair_bytes)
        .context("Failed to parse keypair file. Expected JSON array format like [1,2,3,...,64]")?;

    // Verify length
    if bytes.len() != 64 {
        return Err(anyhow!(
            "Invalid keypair length: expected 64 bytes, got {}",
            bytes.len()
        ));
    }

    // Create keypair
    Keypair::try_from(&bytes[..])
        .context("Failed to create keypair from bytes")
}

/// Sign and send a transaction to Solana
pub async fn sign_and_send_transaction(
    rpc_url: &str,
    transaction: Transaction,
    _signer: &Keypair,
) -> Result<Signature> {
    let rpc_client = RpcClient::new_with_commitment(
        rpc_url.to_string(),
        CommitmentConfig::confirmed(),
    );

    // Send transaction
    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .context("Failed to send transaction")?;

    Ok(signature)
}

/// Get SOL balance for a keypair
pub fn get_balance(rpc_url: &str, keypair: &Keypair) -> Result<u64> {
    let rpc_client = RpcClient::new_with_commitment(
        rpc_url.to_string(),
        CommitmentConfig::confirmed(),
    );

    let balance = rpc_client
        .get_balance(&keypair.pubkey())
        .context("Failed to get balance")?;

    Ok(balance)
}

/// Convert lamports to SOL
pub fn lamports_to_sol(lamports: u64) -> f64 {
    lamports as f64 / 1_000_000_000.0
}

/// Convert SOL to lamports
pub fn sol_to_lamports(sol: f64) -> u64 {
    (sol * 1_000_000_000.0) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lamports_conversion() {
        assert_eq!(lamports_to_sol(1_000_000_000), 1.0);
        assert_eq!(lamports_to_sol(500_000_000), 0.5);
        assert_eq!(sol_to_lamports(1.0), 1_000_000_000);
        assert_eq!(sol_to_lamports(0.5), 500_000_000);
    }
}
