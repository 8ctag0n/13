//! Private Lending Types
//!
//! Type definitions for Aptos private lending with FHE-verified LTV ratios.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

/// Loan status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoanStatus {
    /// Awaiting FHE verification of LTV
    Pending,
    /// LTV approved, pLST minted, loan is active
    Active,
    /// LTV check failed, loan rejected
    Rejected,
    /// Loan fully repaid, collateral returned
    Repaid,
    /// Collateral liquidated
    Liquidated,
}

impl LoanStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            LoanStatus::Pending => "pending",
            LoanStatus::Active => "active",
            LoanStatus::Rejected => "rejected",
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
            "active" => Ok(LoanStatus::Active),
            "rejected" => Ok(LoanStatus::Rejected),
            "repaid" => Ok(LoanStatus::Repaid),
            "liquidated" => Ok(LoanStatus::Liquidated),
            _ => Err(anyhow!("Invalid loan status: {}", s)),
        }
    }
}

/// Loan record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Loan {
    pub loan_id: u64,
    pub borrower: String,
    pub collateral_type: String,
    pub collateral_amount: u64,
    pub borrow_amount: u64,
    pub ltv_ratio: u16,
    pub ltv_threshold: u16,
    pub status: LoanStatus,
    pub fhe_job_id: u64,
    pub encrypted_c1: Vec<u8>,
    pub encrypted_c2: Vec<u8>,
    pub created_at: u64,
    pub activated_at: Option<u64>,
    pub closed_at: Option<u64>,
}

impl Loan {
    /// Parse loan from Aptos contract data
    ///
    /// # Arguments
    /// * `loan_data` - Raw loan data from contract view function
    ///
    /// # Expected format (from lending_manager.move):
    /// ```json
    /// {
    ///   "loan_id": "1",
    ///   "borrower": "0x...",
    ///   "coin_type_hash": "0x1::aptos_coin::AptosCoin",
    ///   "collateral_amount": "1000000",
    ///   "encrypted_c1": "0x...",
    ///   "encrypted_c2": "0x...",
    ///   "borrow_amount": "500000",
    ///   "ltv_threshold": "7500",
    ///   "fhe_job_id": "1",
    ///   "status": "0",
    ///   "created_at": "1234567890",
    ///   "activated_at": "0",
    ///   "closed_at": "0"
    /// }
    /// ```
    pub fn from_contract_data(loan_data: &serde_json::Value) -> Result<Self> {
        let loan_id = loan_data
            .get("loan_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing loan_id"))?
            .parse::<u64>()?;

        let borrower = loan_data
            .get("borrower")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing borrower"))?
            .to_string();

        let collateral_type = loan_data
            .get("coin_type_hash")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing coin_type_hash"))?
            .to_string();

        let collateral_amount = loan_data
            .get("collateral_amount")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing collateral_amount"))?
            .parse::<u64>()?;

        let borrow_amount = loan_data
            .get("borrow_amount")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing borrow_amount"))?
            .parse::<u64>()?;

        let ltv_threshold = loan_data
            .get("ltv_threshold")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing ltv_threshold"))?
            .parse::<u16>()?;

        let fhe_job_id = loan_data
            .get("fhe_job_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing fhe_job_id"))?
            .parse::<u64>()?;

        let status_u8 = loan_data
            .get("status")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing status"))?
            .parse::<u8>()?;

        let status = match status_u8 {
            0 => LoanStatus::Pending,
            1 => LoanStatus::Active,
            2 => LoanStatus::Repaid,
            3 => LoanStatus::Liquidated,
            4 => LoanStatus::Rejected,
            _ => return Err(anyhow!("Invalid loan status: {}", status_u8)),
        };

        let encrypted_c1 = loan_data
            .get("encrypted_c1")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing encrypted_c1"))
            .and_then(|s| hex::decode(s.trim_start_matches("0x")).map_err(|e| anyhow!("Invalid hex: {}", e)))?;

        let encrypted_c2 = loan_data
            .get("encrypted_c2")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing encrypted_c2"))
            .and_then(|s| hex::decode(s.trim_start_matches("0x")).map_err(|e| anyhow!("Invalid hex: {}", e)))?;

        let created_at = loan_data
            .get("created_at")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing created_at"))?
            .parse::<u64>()?;

        let activated_at = loan_data
            .get("activated_at")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<u64>().ok())
            .filter(|&v| v > 0);

        let closed_at = loan_data
            .get("closed_at")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<u64>().ok())
            .filter(|&v| v > 0);

        // Calculate actual LTV ratio (basis points)
        let ltv_ratio = if collateral_amount > 0 {
            ((borrow_amount as u128 * 10000) / collateral_amount as u128) as u16
        } else {
            0
        };

        Ok(Self {
            loan_id,
            borrower,
            collateral_type,
            collateral_amount,
            borrow_amount,
            ltv_ratio,
            ltv_threshold,
            status,
            fhe_job_id,
            encrypted_c1,
            encrypted_c2,
            created_at,
            activated_at,
            closed_at,
        })
    }
}

/// Encrypted loan data for FHE processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedLoanData {
    /// Encrypted collateral amount (c1 component)
    pub collateral_c1: Vec<u8>,
    /// Encrypted collateral amount (c2 component)
    pub collateral_c2: Vec<u8>,
    /// Encrypted borrow amount (c1 component)
    pub borrow_c1: Vec<u8>,
    /// Encrypted borrow amount (c2 component)
    pub borrow_c2: Vec<u8>,
    /// LTV threshold in basis points (e.g., 7500 = 75%)
    pub ltv_threshold: u16,
}

/// Loan creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoanRequest {
    pub borrower: String,
    pub collateral_type: String,
    pub collateral_amount: u64,
    pub borrow_amount: u64,
    pub ltv_threshold: u16,
    pub encrypted_data: EncryptedLoanData,
    pub required_provers: u8,
    pub consensus_threshold: u8,
}

impl LoanRequest {
    /// Validate loan request parameters
    pub fn validate(&self) -> Result<()> {
        if self.collateral_amount == 0 {
            return Err(anyhow!("Collateral amount must be greater than 0"));
        }

        if self.borrow_amount == 0 {
            return Err(anyhow!("Borrow amount must be greater than 0"));
        }

        if self.ltv_threshold == 0 || self.ltv_threshold > 10000 {
            return Err(anyhow!("LTV threshold must be between 1 and 10000 basis points"));
        }

        // Check that requested LTV doesn't exceed threshold
        let requested_ltv = ((self.borrow_amount as u128 * 10000) / self.collateral_amount as u128) as u16;
        if requested_ltv > self.ltv_threshold {
            return Err(anyhow!(
                "Requested LTV ({} bps) exceeds threshold ({} bps)",
                requested_ltv,
                self.ltv_threshold
            ));
        }

        if self.required_provers == 0 {
            return Err(anyhow!("Required provers must be at least 1"));
        }

        if self.consensus_threshold == 0 || self.consensus_threshold > self.required_provers {
            return Err(anyhow!(
                "Consensus threshold must be between 1 and {}",
                self.required_provers
            ));
        }

        Ok(())
    }
}

/// LTV verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LtvVerificationResult {
    pub loan_id: u64,
    pub ltv_ratio: u16,
    pub threshold: u16,
    pub passed: bool,
    pub encrypted_proof: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loan_status_conversion() {
        assert_eq!(LoanStatus::Pending.as_str(), "pending");
        assert_eq!("active".parse::<LoanStatus>().unwrap(), LoanStatus::Active);
        assert_eq!("rejected".parse::<LoanStatus>().unwrap(), LoanStatus::Rejected);
    }

    #[test]
    fn test_loan_request_validation() {
        let valid_request = LoanRequest {
            borrower: "0x123".to_string(),
            collateral_type: "0x1::aptos_coin::AptosCoin".to_string(),
            collateral_amount: 1000000,
            borrow_amount: 500000,
            ltv_threshold: 7500,
            encrypted_data: EncryptedLoanData {
                collateral_c1: vec![1, 2, 3],
                collateral_c2: vec![4, 5, 6],
                borrow_c1: vec![7, 8, 9],
                borrow_c2: vec![10, 11, 12],
                ltv_threshold: 7500,
            },
            required_provers: 3,
            consensus_threshold: 2,
        };

        assert!(valid_request.validate().is_ok());

        // Test invalid collateral
        let mut invalid = valid_request.clone();
        invalid.collateral_amount = 0;
        assert!(invalid.validate().is_err());

        // Test invalid LTV threshold
        let mut invalid = valid_request.clone();
        invalid.ltv_threshold = 15000;
        assert!(invalid.validate().is_err());

        // Test LTV exceeds threshold
        let mut invalid = valid_request.clone();
        invalid.borrow_amount = 800000; // 80% LTV
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_loan_from_contract_data() {
        let contract_data = serde_json::json!({
            "loan_id": "42",
            "borrower": "0x0000000000000000000000000000000000000000000000000000000000000abc",
            "coin_type_hash": "0x1::aptos_coin::AptosCoin",
            "collateral_amount": "1000000",
            "encrypted_c1": "0x0102030405",
            "encrypted_c2": "0x0607080910",
            "borrow_amount": "500000",
            "ltv_threshold": "7500",
            "fhe_job_id": "100",
            "status": "0",
            "created_at": "1234567890",
            "activated_at": "0",
            "closed_at": "0"
        });

        let loan = Loan::from_contract_data(&contract_data).unwrap();
        assert_eq!(loan.loan_id, 42);
        assert_eq!(loan.status, LoanStatus::Pending);
        assert_eq!(loan.ltv_ratio, 5000); // 50% in basis points
        assert_eq!(loan.encrypted_c1, vec![1, 2, 3, 4, 5]);
    }
}
