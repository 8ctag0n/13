//! Starknet-specific operations extension trait
//!
//! This module provides additional Starknet-specific functionality
//! beyond the generic ChainClient trait, similar to SolanaSpecificOps.

use crate::{Result, StarknetClient, ChainClientError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Starknet event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarknetEvent {
    /// Contract address that emitted the event
    pub from_address: String,

    /// Event keys (first key is usually the event selector/name hash)
    pub keys: Vec<String>,

    /// Event data fields
    pub data: Vec<String>,

    /// Block number where event was emitted
    pub block_number: u64,

    /// Transaction hash that emitted the event
    pub transaction_hash: String,
}

/// Filter for querying Starknet events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFilter {
    /// Optional contract address to filter by
    pub from_address: Option<String>,

    /// Optional event keys to filter by (array of arrays for AND/OR logic)
    pub keys: Option<Vec<Vec<String>>>,

    /// Starting block number (inclusive)
    pub from_block: u64,

    /// Ending block number (inclusive)
    pub to_block: u64,
}

/// Starknet-specific operations trait
///
/// Extends StarknetClient with Starknet-specific RPC methods
/// that don't fit into the generic ChainClient interface.
#[async_trait]
pub trait StarknetSpecificOps {
    /// Get events emitted by contracts
    ///
    /// # Arguments
    /// * `from_block` - Starting block number (inclusive)
    /// * `to_block` - Ending block number (inclusive)
    /// * `address` - Optional contract address to filter by
    /// * `keys` - Optional event keys to filter by
    ///
    /// # Returns
    /// Vector of StarknetEvent structs
    async fn get_events(
        &self,
        from_block: u64,
        to_block: u64,
        address: Option<&str>,
        keys: Option<Vec<Vec<String>>>,
    ) -> Result<Vec<StarknetEvent>>;
}

#[async_trait]
impl StarknetSpecificOps for StarknetClient {
    async fn get_events(
        &self,
        from_block: u64,
        to_block: u64,
        address: Option<&str>,
        keys: Option<Vec<Vec<String>>>,
    ) -> Result<Vec<StarknetEvent>> {
        #[derive(Deserialize)]
        struct EventsResponse {
            events: Vec<RawEvent>,
            continuation_token: Option<String>,
        }

        #[derive(Deserialize)]
        struct RawEvent {
            from_address: String,
            keys: Vec<String>,
            data: Vec<String>,
            block_number: Option<u64>,
            transaction_hash: String,
        }

        // Build filter parameters
        let mut filter = serde_json::json!({
            "from_block": {
                "block_number": from_block
            },
            "to_block": {
                "block_number": to_block
            },
            "chunk_size": 1000
        });

        // Add optional address filter
        if let Some(addr) = address {
            filter["from_address"] = serde_json::json!(addr);
        }

        // Add optional keys filter
        if let Some(k) = keys {
            filter["keys"] = serde_json::json!(k);
        }

        let params = serde_json::json!({
            "filter": filter
        });

        // Make RPC call
        let response: EventsResponse = self.rpc_call("starknet_getEvents", params).await?;

        // Convert raw events to our StarknetEvent type
        let events: Vec<StarknetEvent> = response.events
            .into_iter()
            .map(|e| StarknetEvent {
                from_address: e.from_address,
                keys: e.keys,
                data: e.data,
                block_number: e.block_number.unwrap_or(0),
                transaction_hash: e.transaction_hash,
            })
            .collect();

        Ok(events)
    }
}

/// Helper to parse felt252 from hex string
pub fn parse_felt252(hex_str: &str) -> Result<String> {
    let hex = hex_str.trim_start_matches("0x");

    // Validate hex string
    if hex.len() > 64 {
        return Err(ChainClientError::InvalidParameter(
            format!("felt252 too long: {} chars (max 64)", hex.len())
        ));
    }

    // Pad with zeros if needed
    let padded = format!("{:0>64}", hex);
    Ok(format!("0x{}", padded))
}

/// Helper to format address with 0x prefix
pub fn format_address(addr: &str) -> String {
    if addr.starts_with("0x") {
        addr.to_string()
    } else {
        format!("0x{}", addr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_felt252() {
        assert_eq!(
            parse_felt252("0x1").unwrap(),
            "0x0000000000000000000000000000000000000000000000000000000000000001"
        );

        assert_eq!(
            parse_felt252("0xabc").unwrap(),
            "0x0000000000000000000000000000000000000000000000000000000000000abc"
        );

        assert_eq!(
            parse_felt252("abc").unwrap(),
            "0x0000000000000000000000000000000000000000000000000000000000000abc"
        );
    }

    #[test]
    fn test_format_address() {
        assert_eq!(format_address("0x123"), "0x123");
        assert_eq!(format_address("123"), "0x123");
    }
}
