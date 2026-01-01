/// ZyberLink CLI utilities
///
/// Provides FHE and ZK utilities for client-side operations.

pub mod chains;
pub mod commands;
pub mod client;
pub mod config;
pub mod solana;
pub mod ui;

// Re-export common utilities
use anyhow::{Context, Result};
use base64::Engine;
use std::fs;
use std::path::PathBuf;
use tfhe::{generate_keys, ClientKey, ConfigBuilder, FheUint8, ServerKey};

// ============================================
// FHE KEY MANAGEMENT
// ============================================

/// Get the directory for storing client keys
pub fn get_keys_directory() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not find home directory")?;
    let keys_dir = home.join(".zyberlink").join("client_keys");

    // Create directory if it doesn't exist
    fs::create_dir_all(&keys_dir).context("Failed to create keys directory")?;

    Ok(keys_dir)
}

/// Generate a timestamped filename for storing client keys
pub fn generate_key_filename() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    format!("key_{}.txt", timestamp)
}

/// Generate FHE keypair
pub fn generate_fhe_keys() -> Result<(ClientKey, ServerKey)> {
    let config = ConfigBuilder::default().build();
    Ok(generate_keys(config))
}

// ============================================
// FHE ENCRYPTION/DECRYPTION
// ============================================

/// Encrypt a u8 value with client key
pub fn encrypt_value(value: u8, client_key: &ClientKey) -> Result<Vec<u8>> {
    use tfhe::prelude::*;
    let encrypted = FheUint8::try_encrypt(value, client_key)?;
    Ok(bincode::serialize(&encrypted)?)
}

/// Decrypt encrypted data with client key
pub fn decrypt_value(encrypted_bytes: &[u8], client_key: &ClientKey) -> Result<u8> {
    use tfhe::prelude::*;
    let encrypted: FheUint8 = bincode::deserialize(encrypted_bytes)?;
    Ok(encrypted.decrypt(client_key))
}

// ============================================
// SERIALIZATION UTILITIES
// ============================================

/// Serialize server key to base64
pub fn serialize_server_key_base64(server_key: &ServerKey) -> Result<String> {
    let bytes = bincode::serialize(server_key)?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
}

/// Serialize client key to base64
pub fn serialize_client_key_base64(client_key: &ClientKey) -> Result<String> {
    let bytes = bincode::serialize(client_key)?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
}

/// Deserialize client key from base64
pub fn deserialize_client_key_base64(base64_str: &str) -> Result<ClientKey> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(base64_str)
        .context("Invalid base64 encoding")?;
    Ok(bincode::deserialize(&bytes)?)
}

/// Serialize encrypted data to base64
pub fn serialize_encrypted_base64(encrypted: &[u8]) -> Result<String> {
    Ok(base64::engine::general_purpose::STANDARD.encode(encrypted))
}

/// Deserialize encrypted data from base64
pub fn deserialize_encrypted_base64(base64_str: &str) -> Result<Vec<u8>> {
    base64::engine::general_purpose::STANDARD
        .decode(base64_str)
        .context("Invalid base64 encoding")
}

// ============================================
// FILE I/O
// ============================================

/// Save client key to file
pub fn save_client_key(client_key: &ClientKey, filepath: &PathBuf) -> Result<()> {
    let base64_key = serialize_client_key_base64(client_key)?;
    fs::write(filepath, base64_key).context("Failed to write client key to file")?;
    Ok(())
}

/// Save server key to file (binary format for efficiency)
pub fn save_server_key(server_key: &ServerKey, filepath: &PathBuf) -> Result<()> {
    let bytes = bincode::serialize(server_key)?;
    fs::write(filepath, bytes).context("Failed to write server key to file")?;
    Ok(())
}

/// Save encrypted data to file (binary format)
pub fn save_encrypted_data(encrypted: &[u8], filepath: &PathBuf) -> Result<()> {
    fs::write(filepath, encrypted).context("Failed to write encrypted data to file")?;
    Ok(())
}

/// Load server key from file
pub fn load_server_key(filepath: &PathBuf) -> Result<ServerKey> {
    let bytes = fs::read(filepath).context("Failed to read server key file")?;
    Ok(bincode::deserialize(&bytes)?)
}

/// Load encrypted data from file
pub fn load_encrypted_data(filepath: &PathBuf) -> Result<Vec<u8>> {
    fs::read(filepath).context("Failed to read encrypted data file")
}

/// Load client key from file
pub fn load_client_key(filepath: &PathBuf) -> Result<ClientKey> {
    let base64_key = fs::read_to_string(filepath).context("Failed to read client key file")?;
    deserialize_client_key_base64(base64_key.trim())
}

// ============================================
// CLI INPUT UTILITIES
// ============================================

/// Read user input from stdin, skipping empty lines in interactive mode
pub fn read_input(prompt: &str) -> Result<String> {
    use std::io::{self, Write};

    loop {
        // Only print prompt if stdin is a terminal
        if atty::is(atty::Stream::Stdin) {
            print!("{}", prompt);
            io::stdout().flush()?;
        }

        let mut input = String::new();
        let bytes_read = io::stdin().read_line(&mut input)?;

        // Check for EOF
        if bytes_read == 0 {
            return Ok(String::new());
        }

        let trimmed = input.trim();

        // Skip empty lines only in interactive mode
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }

        // If not a terminal (piped), skip empty lines too
        if !atty::is(atty::Stream::Stdin) {
            continue;
        }
    }
}

/// Parse u8 value from string
pub fn parse_u8(s: &str) -> Result<u8> {
    s.parse::<u8>()
        .context("Invalid number. Please enter a value between 0 and 255")
}
