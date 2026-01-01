use anyhow::Result;
use std::process::Command;

#[derive(Debug)]
pub struct ValidationResult {
    pub name: String,
    pub passed: bool,
    pub details: Option<String>,
}

impl ValidationResult {
    pub fn success(name: impl Into<String>, details: Option<String>) -> Self {
        Self {
            name: name.into(),
            passed: true,
            details,
        }
    }

    pub fn failure(name: impl Into<String>, details: Option<String>) -> Self {
        Self {
            name: name.into(),
            passed: false,
            details,
        }
    }
}

/// Check if Solana CLI is installed
pub async fn check_solana_cli() -> Result<ValidationResult> {
    match Command::new("solana").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            Ok(ValidationResult::success(
                "Solana CLI installed",
                Some(version.trim().to_string()),
            ))
        }
        Ok(_) => Ok(ValidationResult::failure(
            "Solana CLI not working",
            Some("Install from https://docs.solana.com/cli/install-solana-cli-tools".to_string()),
        )),
        Err(_) => Ok(ValidationResult::failure(
            "Solana CLI not found",
            Some("Install from https://docs.solana.com/cli/install-solana-cli-tools".to_string()),
        )),
    }
}

/// Check internet connectivity
pub async fn check_internet_connectivity() -> Result<ValidationResult> {
    match reqwest::get("https://api.mainnet-beta.solana.com").await {
        Ok(_) => Ok(ValidationResult::success(
            "Internet connectivity confirmed",
            None,
        )),
        Err(e) => Ok(ValidationResult::failure(
            "No internet connectivity",
            Some(format!("Error: {}", e)),
        )),
    }
}

/// Check if we can write to config directory
pub async fn check_config_directory() -> Result<ValidationResult> {
    let config_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
        .join(".zyberlink");

    // Try to create directory
    match tokio::fs::create_dir_all(&config_dir).await {
        Ok(_) => Ok(ValidationResult::success(
            "Config directory writable",
            Some(format!("{}", config_dir.display())),
        )),
        Err(e) => Ok(ValidationResult::failure(
            "Cannot write to config directory",
            Some(format!("Error: {}", e)),
        )),
    }
}

/// Validate all system requirements
pub async fn validate_system() -> Result<Vec<ValidationResult>> {
    let mut results = Vec::new();

    results.push(check_solana_cli().await?);
    results.push(check_internet_connectivity().await?);
    results.push(check_config_directory().await?);

    Ok(results)
}

/// Check if all validations passed
pub fn all_passed(results: &[ValidationResult]) -> bool {
    results.iter().all(|r| r.passed)
}

/// Validate RPC URL format
pub fn validate_rpc_url(url: &str) -> Result<()> {
    // Check if URL is not empty
    if url.trim().is_empty() {
        anyhow::bail!("RPC URL cannot be empty");
    }

    // Check if it starts with http:// or https://
    if !url.starts_with("http://") && !url.starts_with("https://") {
        anyhow::bail!(
            "RPC URL must start with http:// or https://\nExample: https://api.devnet.solana.com"
        );
    }

    // Basic URL parsing check
    url.parse::<url::Url>().map_err(|e| {
        anyhow::anyhow!(
            "Invalid URL format: {}\nExample: https://api.devnet.solana.com",
            e
        )
    })?;

    Ok(())
}

/// Validate Solana public key string format
pub fn validate_pubkey_string(pubkey_str: &str) -> Result<()> {
    use solana_sdk::pubkey::Pubkey;

    // Check if not empty
    if pubkey_str.trim().is_empty() {
        anyhow::bail!("Public key cannot be empty");
    }

    // Try to parse
    pubkey_str.parse::<Pubkey>().map_err(|e| {
        anyhow::anyhow!(
            "Invalid public key format: {}\n\nExpected: Base58-encoded 32-byte public key (44 characters)\nExample: 11111111111111111111111111111111",
            e
        )
    })?;

    Ok(())
}

/// Validate file path exists
pub fn validate_file_path(path: &std::path::Path) -> Result<()> {
    if !path.exists() {
        anyhow::bail!(
            "File not found: {}\n\nPlease check the path and try again.",
            path.display()
        );
    }

    if !path.is_file() {
        anyhow::bail!(
            "Path is not a file: {}\n\nExpected a regular file, found a directory.",
            path.display()
        );
    }

    Ok(())
}
