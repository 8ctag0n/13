//! Private Lending Vertical for Aptos
//!
//! This vertical provides privacy-preserving lending with FHE-verified LTV ratios.
//!
//! # Architecture
//!
//! ```text
//! Borrower                    Prover Nodes                 Aptos Blockchain
//! --------                    ------------                 ----------------
//! 1. Encrypt collateral  -->  2. Fetch encrypted data  <-- 3. Store in contract
//!    and borrow amounts   -->  4. Verify LTV with FHE   --> 5. Submit verification
//!                         <--  6. Consensus reached     --> 7. Activate loan
//! 8. Receive pLST tokens <--                            <-- 9. Mint pLST
//! ```
//!
//! # Components
//!
//! - `types`: Loan, LoanStatus, LoanRequest, EncryptedLoanData
//! - `fhe_ltv`: FHE computation for LTV verification
//! - `PrivateLendingClient`: Interface to Aptos lending contracts
//!
//! # Example
//!
//! ```rust,ignore
//! use private_lending::{PrivateLendingClient, LoanRequest};
//!
//! // Create client
//! let client = PrivateLendingClient::new(aptos_client, registry_addr)?;
//!
//! // Create loan request
//! let request = LoanRequest {
//!     borrower: "0x123...".to_string(),
//!     collateral_type: "0x1::aptos_coin::AptosCoin".to_string(),
//!     collateral_amount: 1_000_000,
//!     borrow_amount: 500_000,
//!     ltv_threshold: 7500, // 75%
//!     encrypted_data: encrypted_data,
//!     required_provers: 3,
//!     consensus_threshold: 2,
//! };
//!
//! // Submit loan
//! let loan_id = client.create_loan_request(request).await?;
//!
//! // Query loan status
//! let loan = client.get_loan(loan_id).await?;
//! ```

pub mod types;
pub mod fhe_ltv;

use anyhow::{anyhow, Context, Result};
use zyberlink_chain_client::aptos::AptosClient;
use log::{debug, info, warn};
use serde_json::json;

pub use types::{Loan, LoanRequest, LoanStatus, EncryptedLoanData, LtvVerificationResult};
pub use fhe_ltv::{FheLtvComputer, create_ltv_computer};

/// Private Lending Client
///
/// Provides interface to Aptos private lending contracts with FHE verification.
pub struct PrivateLendingClient {
    aptos_client: AptosClient,
    registry_address: String,
    module_name: String,
}

impl PrivateLendingClient {
    /// Create new private lending client
    ///
    /// # Arguments
    /// * `aptos_client` - Aptos blockchain client
    /// * `registry_address` - Address where LendingRegistry is deployed
    pub fn new(aptos_client: AptosClient, registry_address: String) -> Result<Self> {
        Ok(Self {
            aptos_client,
            registry_address,
            module_name: "lending_manager".to_string(),
        })
    }

    /// Create loan request
    ///
    /// # Arguments
    /// * `request` - Loan request with encrypted data
    ///
    /// # Returns
    /// * Loan ID for tracking
    ///
    /// # Note
    /// This submits a transaction to the Aptos blockchain.
    /// The loan will be in PENDING status until FHE verification completes.
    pub async fn create_loan_request(&self, request: LoanRequest) -> Result<u64> {
        // Validate request
        request.validate()?;

        info!(
            "Creating loan request: collateral={}, borrow={}, ltv_threshold={}bps",
            request.collateral_amount, request.borrow_amount, request.ltv_threshold
        );

        // Note: In production, this would require a signer with private key
        // For now, return a mock loan ID
        // TODO: Implement proper transaction signing and submission
        // Prepare function arguments (BCS-encoded):
        // - registry_addr, collateral_amount, encrypted_c1, encrypted_c2
        // - borrow_amount, ltv_threshold, required_provers, consensus_threshold

        warn!("create_loan_request not fully implemented - requires transaction signing");

        // Return mock loan ID
        Ok(1)
    }

    /// Get loan by ID
    ///
    /// # Arguments
    /// * `loan_id` - Loan ID to query
    ///
    /// # Returns
    /// * Loan details
    pub async fn get_loan(&self, loan_id: u64) -> Result<Loan> {
        debug!("Fetching loan {}", loan_id);

        // Call view function: get_loan(registry_addr, loan_id)
        let result = self
            .aptos_client
            .call_view_function(
                &self.registry_address,
                &self.module_name,
                "get_loan",
                vec![], // No type args
                vec![
                    json!(self.registry_address),
                    json!(loan_id.to_string()),
                ],
            )
            .await
            .context("Failed to call get_loan view function")?;

        // Parse result
        if result.is_empty() {
            return Err(anyhow!("Loan {} not found", loan_id));
        }

        // The result is a vector of JSON values representing the Loan struct
        let loan_data = &result[0];

        Loan::from_contract_data(loan_data)
            .context("Failed to parse loan data from contract")
    }

    /// Get all pending loans awaiting FHE verification
    ///
    /// # Returns
    /// * Vector of loans in PENDING status
    ///
    /// # Note
    /// This queries the total loan count and fetches each loan individually.
    /// For production, implement batch queries or indexer integration.
    pub async fn get_pending_loans(&self) -> Result<Vec<Loan>> {
        debug!("Fetching pending loans");

        // Get total loan count
        let total_loans = self.get_total_loans().await?;

        debug!("Total loans: {}", total_loans);

        let mut pending_loans = Vec::new();

        // Fetch each loan and filter by status
        // Note: This is inefficient. In production, use events or indexer.
        for loan_id in 1..=total_loans {
            match self.get_loan(loan_id).await {
                Ok(loan) if loan.status == LoanStatus::Pending => {
                    pending_loans.push(loan);
                }
                Ok(_) => {} // Not pending
                Err(e) => {
                    warn!("Failed to fetch loan {}: {}", loan_id, e);
                }
            }
        }

        info!("Found {} pending loans", pending_loans.len());

        Ok(pending_loans)
    }

    /// Get total number of loans
    pub async fn get_total_loans(&self) -> Result<u64> {
        let result = self
            .aptos_client
            .call_view_function(
                &self.registry_address,
                &self.module_name,
                "get_total_loans",
                vec![],
                vec![json!(self.registry_address)],
            )
            .await
            .context("Failed to call get_total_loans")?;

        if result.is_empty() {
            return Ok(0);
        }

        result[0]
            .as_str()
            .ok_or_else(|| anyhow!("Invalid total_loans response"))?
            .parse::<u64>()
            .context("Failed to parse total_loans")
    }

    /// Get number of active loans
    pub async fn get_active_loans(&self) -> Result<u64> {
        let result = self
            .aptos_client
            .call_view_function(
                &self.registry_address,
                &self.module_name,
                "get_active_loans",
                vec![],
                vec![json!(self.registry_address)],
            )
            .await
            .context("Failed to call get_active_loans")?;

        if result.is_empty() {
            return Ok(0);
        }

        result[0]
            .as_str()
            .ok_or_else(|| anyhow!("Invalid active_loans response"))?
            .parse::<u64>()
            .context("Failed to parse active_loans")
    }

    /// Get all loans for a borrower
    pub async fn get_borrower_loans(&self, borrower: &str) -> Result<Vec<u64>> {
        let result = self
            .aptos_client
            .call_view_function(
                &self.registry_address,
                &self.module_name,
                "get_borrower_loans",
                vec![],
                vec![json!(borrower)],
            )
            .await
            .context("Failed to call get_borrower_loans")?;

        if result.is_empty() {
            return Ok(vec![]);
        }

        // Parse array of loan IDs
        let loan_ids = result[0]
            .as_array()
            .ok_or_else(|| anyhow!("Expected array of loan IDs"))?
            .iter()
            .map(|v| {
                v.as_str()
                    .ok_or_else(|| anyhow!("Invalid loan ID"))?
                    .parse::<u64>()
                    .context("Failed to parse loan ID")
            })
            .collect::<Result<Vec<u64>>>()?;

        Ok(loan_ids)
    }

    /// Activate loan after FHE consensus
    ///
    /// # Arguments
    /// * `loan_id` - Loan ID to activate
    /// * `admin_private_key` - Admin private key for signing
    ///
    /// # Note
    /// This is an admin-only function. Only the lending manager admin can activate loans.
    pub async fn activate_loan(&self, loan_id: u64, _admin_private_key: &str) -> Result<String> {
        info!("Activating loan {}", loan_id);

        // TODO: Implement transaction signing and submission
        // This requires calling execute_entry_function with admin credentials

        warn!("activate_loan not fully implemented - requires transaction signing");

        Ok("mock_tx_hash".to_string())
    }

    /// Check if loan exists
    pub async fn loan_exists(&self, loan_id: u64) -> Result<bool> {
        let result = self
            .aptos_client
            .call_view_function(
                &self.registry_address,
                &self.module_name,
                "loan_exists",
                vec![],
                vec![
                    json!(self.registry_address),
                    json!(loan_id.to_string()),
                ],
            )
            .await
            .context("Failed to call loan_exists")?;

        if result.is_empty() {
            return Ok(false);
        }

        result[0]
            .as_bool()
            .ok_or_else(|| anyhow!("Invalid loan_exists response"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let aptos_client = AptosClient::local().unwrap();
        let client = PrivateLendingClient::new(
            aptos_client,
            "0x0000000000000000000000000000000000000000000000000000000000000001".to_string(),
        );

        assert!(client.is_ok());
    }

    #[test]
    fn test_loan_request_validation() {
        use crate::types::EncryptedLoanData;

        let request = LoanRequest {
            borrower: "0x123".to_string(),
            collateral_type: "0x1::aptos_coin::AptosCoin".to_string(),
            collateral_amount: 1_000_000,
            borrow_amount: 500_000,
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

        assert!(request.validate().is_ok());
    }
}
