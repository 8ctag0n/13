/// Global configuration system for ZyberLink CLI
///
/// This module provides a unified TOML-based configuration system that eliminates
/// the need to pass multiple environment variables for each command.
///
/// ## Configuration Priority (Cascade)
/// 1. CLI arguments (highest priority)
/// 2. Profile configuration from [profiles.X]
/// 3. Common configuration from [common]
/// 4. Environment variables
/// 5. Hardcoded defaults (lowest priority)
///
/// ## Configuration File Search
/// The config loader searches for `zyb.toml` in the following order:
/// 1. Path specified via `--config` flag
/// 2. `./zyb.toml` (current directory)
/// 3. `~/.config/zyb/zyb.toml` (user home directory)

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

/// Global configuration structure loaded from zyb.toml
#[derive(Debug, Deserialize, Default)]
pub struct GlobalConfig {
    /// Common settings shared across all commands
    #[serde(default)]
    pub common: CommonConfig,

    /// Named profiles for different environments (local, devnet, mainnet, etc.)
    #[serde(default)]
    pub profiles: HashMap<String, ProfileConfig>,

    /// Dev-job specific configuration
    #[serde(rename = "dev-job", default)]
    pub dev_job: Option<DevJobGlobalConfig>,
}

/// Common settings that apply to all commands
#[derive(Debug, Deserialize, Default)]
pub struct CommonConfig {
    /// Default profile to use when not specified
    pub profile: Option<String>,

    /// Default chain (solana, starknet)
    pub chain: Option<String>,

    /// Enable JSON output by default
    pub json: Option<bool>,

    /// Disable auto-airdrop on low balance
    pub no_airdrop: Option<bool>,
}

/// Profile configuration for a specific environment
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProfileConfig {
    // Solana configuration
    pub rpc_url: Option<String>,
    pub backend_url: Option<String>,
    pub witness_url: Option<String>,
    pub keypair: Option<PathBuf>,

    // Program IDs
    pub bedrock_program_id: Option<String>,
    pub fhe_generator_program_id: Option<String>,
    pub zk_generator_program_id: Option<String>,
    pub futarchy_program_id: Option<String>,

    // Multi-chain (future)
    pub starknet_rpc_url: Option<String>,
    pub starknet_account: Option<String>,
    pub starknet_private_key: Option<String>,

    // Profile-specific settings
    pub no_airdrop: Option<bool>,
    pub json: Option<bool>,
}

/// Dev-job specific global configuration
#[derive(Debug, Clone, Deserialize, Default)]
pub struct DevJobGlobalConfig {
    pub interval_secs: Option<u64>,
    pub once: Option<bool>,
    pub shuffle: Option<bool>,
}

/// Load global configuration from a TOML file
///
/// ## Arguments
/// * `path` - Optional path to config file. If None, searches default locations.
///
/// ## Returns
/// * `Ok(Some(GlobalConfig))` if config file found and loaded
/// * `Ok(None)` if no config file found (not an error)
/// * `Err(...)` if config file exists but has parse errors
pub fn load_global_config(path: Option<PathBuf>) -> Result<Option<GlobalConfig>> {
    let config_path = if let Some(p) = path {
        // If explicit path provided, it must exist
        if !p.exists() {
            anyhow::bail!("Config file not found: {}", p.display());
        }
        p
    } else {
        // Search for config in default locations
        find_config_file()?
            .ok_or_else(|| anyhow::anyhow!("No config file found"))?
    };

    log::debug!("Loading config from: {}", config_path.display());

    let contents = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

    let config: GlobalConfig = toml::from_str(&contents)
        .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?;

    Ok(Some(config))
}

/// Find config file in default locations
///
/// Searches:
/// 1. ./zyb.toml (current directory)
/// 2. ~/.config/zyb/zyb.toml (user config directory)
///
/// Returns the first file found, or None if no config exists.
fn find_config_file() -> Result<Option<PathBuf>> {
    // Try current directory first
    let current_dir_config = PathBuf::from("zyb.toml");
    if current_dir_config.exists() {
        return Ok(Some(current_dir_config));
    }

    // Try user config directory
    if let Some(home_dir) = dirs::home_dir() {
        let user_config = home_dir.join(".config/zyb/zyb.toml");
        if user_config.exists() {
            return Ok(Some(user_config));
        }
    }

    // No config found (not an error)
    Ok(None)
}

/// Get a profile from the global config
///
/// ## Arguments
/// * `config` - The global configuration
/// * `profile_name` - Name of the profile to retrieve. If None, uses common.profile
///
/// ## Returns
/// * `Some(ProfileConfig)` if profile found
/// * `None` if profile not found or no config provided
pub fn get_profile(
    config: &Option<GlobalConfig>,
    profile_name: &Option<String>,
) -> Option<ProfileConfig> {
    let config = config.as_ref()?;

    // Determine which profile to use
    let name = profile_name
        .as_ref()
        .or_else(|| config.common.profile.as_ref())?;

    // Look up the profile
    config.profiles.get(name).cloned()
}

/// Expand tilde (~) in path to user's home directory
pub fn expand_tilde(path: &PathBuf) -> PathBuf {
    shellexpand::tilde(path.to_string_lossy().as_ref())
        .to_string()
        .into()
}

/// Helper macro for cascade resolution
///
/// Resolves a value using the priority cascade:
/// CLI arg > Profile config > Common config > Env var > Default
#[macro_export]
macro_rules! resolve_config {
    ($cli:expr, $profile:expr, $common:expr, $env:expr, $default:expr) => {
        $cli.clone()
            .or_else(|| $profile.as_ref().and_then(|p| p.clone()))
            .or_else(|| $common.and_then(|c| c.clone()))
            .or_else(|| std::env::var($env).ok())
            .unwrap_or_else(|| $default.to_string())
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_config() {
        let toml = r#"
            [common]
            profile = "local"

            [profiles.local]
            rpc_url = "http://localhost:8899"
        "#;

        let config: GlobalConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.common.profile, Some("local".to_string()));
        assert!(config.profiles.contains_key("local"));
    }

    #[test]
    fn test_parse_full_config() {
        let toml = r#"
            [common]
            profile = "devnet"
            chain = "solana"
            json = false

            [profiles.local]
            rpc_url = "http://localhost:8899"
            backend_url = "http://localhost:9000"
            keypair = "/tmp/demo-keypair.json"
            fhe_generator_program_id = "C8PpHFCKZ4F2Szbir2EMS4S4H3mwQqHNWUXK1N21nfAB"

            [profiles.devnet]
            rpc_url = "https://api.devnet.solana.com"
            backend_url = "https://api.zyberlink.dev"
            keypair = "~/.config/solana/id.json"

            [dev-job]
            interval_secs = 10
            shuffle = false
        "#;

        let config: GlobalConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.common.profile, Some("devnet".to_string()));
        assert_eq!(config.profiles.len(), 2);
        assert!(config.dev_job.is_some());
        assert_eq!(config.dev_job.as_ref().unwrap().interval_secs, Some(10));
    }

    #[test]
    fn test_get_profile() {
        let toml = r#"
            [common]
            profile = "local"

            [profiles.local]
            rpc_url = "http://localhost:8899"

            [profiles.devnet]
            rpc_url = "https://api.devnet.solana.com"
        "#;

        let config: GlobalConfig = toml::from_str(toml).unwrap();
        let global_config = Some(config);

        // Test explicit profile name
        let profile = get_profile(&global_config, &Some("devnet".to_string()));
        assert!(profile.is_some());
        assert_eq!(
            profile.unwrap().rpc_url,
            Some("https://api.devnet.solana.com".to_string())
        );

        // Test default profile from common
        let profile = get_profile(&global_config, &None);
        assert!(profile.is_some());
        assert_eq!(
            profile.unwrap().rpc_url,
            Some("http://localhost:8899".to_string())
        );
    }

    #[test]
    fn test_expand_tilde() {
        let path = PathBuf::from("~/test/path");
        let expanded = expand_tilde(&path);
        assert!(!expanded.to_string_lossy().contains('~'));
    }
}
