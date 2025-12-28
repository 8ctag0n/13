use anyhow::{Context, Result};
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum ProfileCommands {
    /// Set the active profile in dev-job config
    Use {
        /// Profile name
        name: String,

        /// Config file path (default: dev-job.toml in current directory)
        #[arg(long)]
        config: Option<PathBuf>,
    },

    /// List available profiles from dev-job config
    List {
        /// Config file path (default: dev-job.toml in current directory)
        #[arg(long)]
        config: Option<PathBuf>,
    },

    /// Show the active profile from dev-job config
    Show {
        /// Config file path (default: dev-job.toml in current directory)
        #[arg(long)]
        config: Option<PathBuf>,
    },
}

pub fn handle_profile_command(command: ProfileCommands) -> Result<()> {
    match command {
        ProfileCommands::Use { name, config } => {
            let path = config.unwrap_or_else(|| PathBuf::from("dev-job.toml"));
            let mut value = load_or_default(&path)?;

            let root = value
                .as_table_mut()
                .context("Config root must be a TOML table")?;
            let common = root
                .entry("common")
                .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
            let common_table = common
                .as_table_mut()
                .context("common must be a TOML table")?;
            common_table.insert("profile".to_string(), toml::Value::String(name.clone()));

            write_config(&path, &value)?;
            println!("Active profile set to '{}' in {}", name, path.display());
            Ok(())
        }
        ProfileCommands::List { config } => {
            let path = config.unwrap_or_else(|| PathBuf::from("dev-job.toml"));
            let value = load_or_default(&path)?;
            let root = value
                .as_table()
                .context("Config root must be a TOML table")?;

            let profiles = root
                .get("profiles")
                .and_then(|v| v.as_table())
                .map(|table| table.keys().cloned().collect::<Vec<_>>())
                .unwrap_or_default();

            if profiles.is_empty() {
                println!("No profiles found in {}", path.display());
            } else {
                println!("Profiles in {}:", path.display());
                for name in profiles {
                    println!("  - {}", name);
                }
            }
            Ok(())
        }
        ProfileCommands::Show { config } => {
            let path = config.unwrap_or_else(|| PathBuf::from("dev-job.toml"));
            let value = load_or_default(&path)?;
            let root = value
                .as_table()
                .context("Config root must be a TOML table")?;

            let profile = root
                .get("common")
                .and_then(|v| v.as_table())
                .and_then(|table| table.get("profile"))
                .and_then(|v| v.as_str());

            match profile {
                Some(name) => println!("Active profile: {}", name),
                None => println!("No active profile set in {}", path.display()),
            }
            Ok(())
        }
    }
}

fn load_or_default(path: &PathBuf) -> Result<toml::Value> {
    if !path.exists() {
        return Ok(toml::Value::Table(toml::map::Map::new()));
    }
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file {}", path.display()))?;
    let value = toml::from_str(&contents).context("Failed to parse dev-job.toml")?;
    Ok(value)
}

fn write_config(path: &PathBuf, value: &toml::Value) -> Result<()> {
    let contents = toml::to_string_pretty(value).context("Failed to serialize dev-job.toml")?;
    std::fs::write(path, contents)
        .with_context(|| format!("Failed to write config file {}", path.display()))
}
