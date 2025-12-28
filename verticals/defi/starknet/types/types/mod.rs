//! pBTCFi Types - Event structures and database models
//!
//! This module defines typed structures for pBTCFi Cairo contract events
//! and database models for loan state persistence.

pub mod utils;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

// Event key constants (computed from Cairo event names)
// TODO: Calculate actual keccak256 hashes of event names
pub const EVENT_KEY_LOAN_CREATED: &str = "0x00000000000000000000000000000000000000000000000000004c6f616e437265617465645f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f";
pub const EVENT_KEY_COLLATERAL_REGISTERED: &str = "0x436f6c6c61746572616c52656769737465726564";
pub const EVENT_KEY_LOAN_ACTIVATED: &str = "0x4c6f616e41637469766174656400000000000000";
pub const EVENT_KEY_LOAN_REPAID: &str = "0x4c6f616e5265706169640000000000000000000000000000";
pub const EVENT_KEY_LOAN_LIQUIDATED: &str = "0x4c6f616e4c6971756964617465640000000000000000";

// =============================================================================
// Cairo Event Types
// =============================================================================

/// LoanCreated event from pbtcfi_core.cairo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoanCreatedEvent {
    pub loan_id: String,                // u256 as hex string
    pub borrower: String,               // ContractAddress as hex
    pub btc_commitment: String,         // felt252 as hex
    pub btc_encrypted: (String, String), // ElGamal tuple (c1, c2)
    pub timestamp: u64,                 // Unix timestamp
}

impl LoanCreatedEvent {
    /// Parse from Starknet event data array
    pub fn from_event_data(data: &[String]) -> Result<Self> {
        if data.len() < 6 {
            return Err(anyhow!("LoanCreated event requires 6 data fields, got {}", data.len()));
        }

        Ok(Self {
            loan_id: utils::normalize_felt252(&data[0])?,
            borrower: utils::normalize_address(&data[1])?,
            btc_commitment: utils::normalize_felt252(&data[2])?,
            btc_encrypted: (
                utils::normalize_felt252(&data[3])?,
                utils::normalize_felt252(&data[4])?,
            ),
            timestamp: utils::parse_u64_from_felt(&data[5])?,
        })
    }
}

/// CollateralRegistered event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollateralRegisteredEvent {
    pub loan_id: String,
    pub btc_encrypted: (String, String),
    pub proof_hash: String,
    pub timestamp: u64,
}

impl CollateralRegisteredEvent {
    pub fn from_event_data(data: &[String]) -> Result<Self> {
        if data.len() < 5 {
            return Err(anyhow!("CollateralRegistered event requires 5 data fields"));
        }

        Ok(Self {
            loan_id: utils::normalize_felt252(&data[0])?,
            btc_encrypted: (
                utils::normalize_felt252(&data[1])?,
                utils::normalize_felt252(&data[2])?,
            ),
            proof_hash: utils::normalize_felt252(&data[3])?,
            timestamp: utils::parse_u64_from_felt(&data[4])?,
        })
    }
}

/// LoanActivated event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoanActivatedEvent {
    pub loan_id: String,
    pub plst_encrypted: (String, String),
    pub timestamp: u64,
}

impl LoanActivatedEvent {
    pub fn from_event_data(data: &[String]) -> Result<Self> {
        if data.len() < 4 {
            return Err(anyhow!("LoanActivated event requires 4 data fields"));
        }

        Ok(Self {
            loan_id: utils::normalize_felt252(&data[0])?,
            plst_encrypted: (
                utils::normalize_felt252(&data[1])?,
                utils::normalize_felt252(&data[2])?,
            ),
            timestamp: utils::parse_u64_from_felt(&data[3])?,
        })
    }
}

/// LoanRepaid event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoanRepaidEvent {
    pub loan_id: String,
    pub timestamp: u64,
}

impl LoanRepaidEvent {
    pub fn from_event_data(data: &[String]) -> Result<Self> {
        if data.len() < 2 {
            return Err(anyhow!("LoanRepaid event requires 2 data fields"));
        }

        Ok(Self {
            loan_id: utils::normalize_felt252(&data[0])?,
            timestamp: utils::parse_u64_from_felt(&data[1])?,
        })
    }
}

/// LoanLiquidated event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoanLiquidatedEvent {
    pub loan_id: String,
    pub liquidator: String,
    pub timestamp: u64,
}

impl LoanLiquidatedEvent {
    pub fn from_event_data(data: &[String]) -> Result<Self> {
        if data.len() < 3 {
            return Err(anyhow!("LoanLiquidated event requires 3 data fields"));
        }

        Ok(Self {
            loan_id: utils::normalize_felt252(&data[0])?,
            liquidator: utils::normalize_address(&data[1])?,
            timestamp: utils::parse_u64_from_felt(&data[2])?,
        })
    }
}

// =============================================================================
// Database Models
// =============================================================================

/// Database model for pbtcfi_loans table
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct PbtcfiLoan {
    pub loan_id: String,
    pub borrower: String,
    pub status: String,
    pub btc_commitment: String,
    pub plst_commitment: Option<String>,
    pub ltv_commitment: Option<String>,
    pub btc_encrypted_c1: String,
    pub btc_encrypted_c2: String,
    pub plst_encrypted_c1: Option<String>,
    pub plst_encrypted_c2: Option<String>,
    pub collateral_hash: Option<String>,
    pub created_at: i64,
    pub activated_at: Option<i64>,
    pub repaid_at: Option<i64>,
    pub liquidated_at: Option<i64>,
}

impl From<LoanCreatedEvent> for PbtcfiLoan {
    fn from(event: LoanCreatedEvent) -> Self {
        Self {
            loan_id: event.loan_id,
            borrower: event.borrower,
            status: "pending".to_string(),
            btc_commitment: event.btc_commitment,
            plst_commitment: None,
            ltv_commitment: None,
            btc_encrypted_c1: event.btc_encrypted.0,
            btc_encrypted_c2: event.btc_encrypted.1,
            plst_encrypted_c1: None,
            plst_encrypted_c2: None,
            collateral_hash: None,
            created_at: event.timestamp as i64,
            activated_at: None,
            repaid_at: None,
            liquidated_at: None,
        }
    }
}

/// Loan status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoanStatus {
    Pending,
    CollateralRegistered,
    Active,
    Repaid,
    Liquidated,
}

impl LoanStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            LoanStatus::Pending => "pending",
            LoanStatus::CollateralRegistered => "collateral_registered",
            LoanStatus::Active => "active",
            LoanStatus::Repaid => "repaid",
            LoanStatus::Liquidated => "liquidated",
        }
    }
}

impl std::str::FromStr for LoanStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "pending" => Ok(LoanStatus::Pending),
            "collateral_registered" => Ok(LoanStatus::CollateralRegistered),
            "active" => Ok(LoanStatus::Active),
            "repaid" => Ok(LoanStatus::Repaid),
            "liquidated" => Ok(LoanStatus::Liquidated),
            _ => Err(anyhow!("Invalid loan status: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loan_created_event_parse() {
        let data = vec![
            "0x1".to_string(),                  // loan_id
            "0xabc".to_string(),                // borrower
            "0x123".to_string(),                // btc_commitment
            "0x456".to_string(),                // btc_encrypted.0
            "0x789".to_string(),                // btc_encrypted.1
            "1234567890".to_string(),           // timestamp
        ];

        let event = LoanCreatedEvent::from_event_data(&data).unwrap();

        assert_eq!(event.loan_id, "0x0000000000000000000000000000000000000000000000000000000000000001");
        assert!(event.borrower.starts_with("0x"));
        assert_eq!(event.timestamp, 1234567890);
    }

    #[test]
    fn test_loan_status_conversion() {
        assert_eq!(LoanStatus::Pending.as_str(), "pending");
        assert_eq!("active".parse::<LoanStatus>().unwrap(), LoanStatus::Active);
    }
}
