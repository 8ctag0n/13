use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use solana_sdk::signature::{Keypair, Signer};
use std::path::PathBuf;

/// Terms and Conditions API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermsResponse {
    pub version: String,
    pub updated_at: String,
    pub terms: String,
    pub hash: String,
}

/// Local Terms acceptance record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermsAcceptance {
    pub version: String,
    pub accepted_at: String,
    pub terms_hash: String,
    pub signature: String,
    pub signer_pubkey: String,
}

/// Fetch latest Terms & Conditions from backend
pub async fn fetch_terms(backend_url: &str) -> Result<TermsResponse> {
    let url = format!("{}/api/v1/terms", backend_url);

    let response = reqwest::get(&url)
        .await
        .context("Failed to fetch terms from backend")?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to fetch terms: HTTP {}",
            response.status()
        ));
    }

    let terms = response
        .json::<TermsResponse>()
        .await
        .context("Failed to parse terms response")?;

    Ok(terms)
}

/// Sign Terms & Conditions hash with keypair
pub fn sign_terms(keypair: &Keypair, terms_hash: &str) -> Result<String> {
    let message = format!("ZyberLink Terms Acceptance\nHash: {}", terms_hash);
    let signature = keypair.sign_message(message.as_bytes());
    Ok(signature.to_string())
}

/// Save Terms acceptance record locally
pub async fn save_acceptance(acceptance: &TermsAcceptance) -> Result<PathBuf> {
    let config_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
        .join(".zyberlink");

    tokio::fs::create_dir_all(&config_dir)
        .await
        .context("Failed to create config directory")?;

    let path = config_dir.join("terms_acceptance.json");

    let json =
        serde_json::to_string_pretty(acceptance).context("Failed to serialize terms acceptance")?;

    tokio::fs::write(&path, json)
        .await
        .context("Failed to write terms acceptance file")?;

    Ok(path)
}

/// Load Terms acceptance record if exists
pub async fn load_acceptance() -> Result<Option<TermsAcceptance>> {
    let path = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
        .join(".zyberlink")
        .join("terms_acceptance.json");

    if !path.exists() {
        return Ok(None);
    }

    let json = tokio::fs::read_to_string(&path)
        .await
        .context("Failed to read terms acceptance file")?;

    let acceptance: TermsAcceptance =
        serde_json::from_str(&json).context("Failed to parse terms acceptance file")?;

    Ok(Some(acceptance))
}

/// Check if user needs to accept new terms
pub async fn needs_acceptance(backend_url: &str) -> Result<bool> {
    let latest_terms = fetch_terms(backend_url).await?;

    match load_acceptance().await? {
        Some(acceptance) => {
            // Check if version or hash changed
            Ok(acceptance.version != latest_terms.version
                || acceptance.terms_hash != latest_terms.hash)
        }
        None => Ok(true),
    }
}
