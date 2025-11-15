use anyhow::{Context, Result};
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
