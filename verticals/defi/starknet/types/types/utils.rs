//! Utility functions for pBTCFi type conversions
//!
//! Helper functions for converting between Cairo types (felt252, u256, ContractAddress)
//! and Rust/PostgreSQL types (String, i64, etc.)

use anyhow::{anyhow, Result};

/// Normalize a felt252 value to 64-character hex string with 0x prefix
///
/// # Arguments
/// * `felt` - Hex string (with or without 0x prefix)
///
/// # Returns
/// Normalized string: "0x" + 64 hex characters (zero-padded)
///
/// # Example
/// ```
/// let normalized = normalize_felt252("0x1")?;
/// assert_eq!(normalized, "0x0000000000000000000000000000000000000000000000000000000000000001");
/// ```
pub fn normalize_felt252(felt: &str) -> Result<String> {
    // Remove 0x prefix if present
    let hex = felt.trim_start_matches("0x");

    // Validate hex string
    if hex.is_empty() {
        return Err(anyhow!("Empty felt252 value"));
    }

    if hex.len() > 64 {
        return Err(anyhow!("felt252 too long: {} chars (max 64)", hex.len()));
    }

    // Validate all characters are hex
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(anyhow!("Invalid hex characters in felt252: {}", felt));
    }

    // Pad with zeros to 64 characters
    let padded = format!("{:0>64}", hex);

    Ok(format!("0x{}", padded))
}

/// Normalize a Starknet ContractAddress to standard format
///
/// # Arguments
/// * `addr` - Address string (with or without 0x prefix)
///
/// # Returns
/// Normalized string: "0x" + 64 hex characters
///
/// # Example
/// ```
/// let addr = normalize_address("0xabc")?;
/// assert_eq!(addr.len(), 66); // 0x + 64 hex chars
/// ```
pub fn normalize_address(addr: &str) -> Result<String> {
    // Starknet addresses are felt252 values
    normalize_felt252(addr)
}

/// Parse u64 from felt252 hex string
///
/// # Arguments
/// * `felt` - Hex string representing a u64 value
///
/// # Returns
/// u64 value
///
/// # Example
/// ```
/// let value = parse_u64_from_felt("0x4d2")?;  // 1234 in hex
/// assert_eq!(value, 1234);
/// ```
pub fn parse_u64_from_felt(felt: &str) -> Result<u64> {
    let hex = felt.trim_start_matches("0x");

    u64::from_str_radix(hex, 16)
        .map_err(|e| anyhow!("Failed to parse u64 from felt252 '{}': {}", felt, e))
}

/// Parse i64 from felt252 (for database storage)
pub fn parse_i64_from_felt(felt: &str) -> Result<i64> {
    let value = parse_u64_from_felt(felt)?;

    // Check if value fits in i64
    if value > i64::MAX as u64 {
        return Err(anyhow!("Value too large for i64: {}", value));
    }

    Ok(value as i64)
}

/// Convert u64 timestamp to felt252 hex string
pub fn u64_to_felt(value: u64) -> String {
    format!("0x{:x}", value)
}

/// Format hex string with or without 0x prefix
pub fn ensure_0x_prefix(hex: &str) -> String {
    if hex.starts_with("0x") {
        hex.to_string()
    } else {
        format!("0x{}", hex)
    }
}

/// Remove 0x prefix from hex string
pub fn remove_0x_prefix(hex: &str) -> String {
    hex.trim_start_matches("0x").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_felt252() {
        // Short hex
        assert_eq!(
            normalize_felt252("0x1").unwrap(),
            "0x0000000000000000000000000000000000000000000000000000000000000001"
        );

        // Medium hex
        assert_eq!(
            normalize_felt252("0xabc123").unwrap(),
            "0x0000000000000000000000000000000000000000000000000000000000abc123"
        );

        // Without 0x prefix
        assert_eq!(
            normalize_felt252("abc").unwrap(),
            "0x0000000000000000000000000000000000000000000000000000000000000abc"
        );

        // Full length (64 chars)
        let full = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        assert_eq!(
            normalize_felt252(&format!("0x{}", full)).unwrap(),
            format!("0x{}", full)
        );
    }

    #[test]
    fn test_normalize_felt252_errors() {
        // Too long
        assert!(normalize_felt252("0x12345678901234567890123456789012345678901234567890123456789012345").is_err());

        // Invalid characters
        assert!(normalize_felt252("0xzzzz").is_err());

        // Empty
        assert!(normalize_felt252("0x").is_err());
        assert!(normalize_felt252("").is_err());
    }

    #[test]
    fn test_parse_u64_from_felt() {
        assert_eq!(parse_u64_from_felt("0x0").unwrap(), 0);
        assert_eq!(parse_u64_from_felt("0x1").unwrap(), 1);
        assert_eq!(parse_u64_from_felt("0xff").unwrap(), 255);
        assert_eq!(parse_u64_from_felt("0x4d2").unwrap(), 1234);
        assert_eq!(parse_u64_from_felt("0xffffffffffffffff").unwrap(), u64::MAX);
    }

    #[test]
    fn test_u64_to_felt() {
        assert_eq!(u64_to_felt(0), "0x0");
        assert_eq!(u64_to_felt(1), "0x1");
        assert_eq!(u64_to_felt(255), "0xff");
        assert_eq!(u64_to_felt(1234), "0x4d2");
    }

    #[test]
    fn test_ensure_0x_prefix() {
        assert_eq!(ensure_0x_prefix("abc"), "0xabc");
        assert_eq!(ensure_0x_prefix("0xabc"), "0xabc");
    }

    #[test]
    fn test_remove_0x_prefix() {
        assert_eq!(remove_0x_prefix("0xabc"), "abc");
        assert_eq!(remove_0x_prefix("abc"), "abc");
    }
}
