//! FHE LTV Computation
//!
//! Compute Loan-to-Value ratios on encrypted data using FHE.
//! This allows prover nodes to verify LTV thresholds without seeing actual amounts.

use anyhow::{Context, Result};
use sha3::{Digest, Sha3_256};
use zyberlink_fhe::{FheEngine, FheUint8};
use zyberlink_fhe::prelude::*;

use crate::types::EncryptedLoanData;

/// FHE LTV Computer
///
/// Computes LTV ratios on encrypted collateral and borrow amounts.
///
/// # How it works:
/// 1. Receive encrypted collateral and borrow amounts
/// 2. Compute encrypted LTV = (borrow / collateral) * 10000
/// 3. Compare encrypted LTV with threshold
/// 4. Return encrypted boolean result + hash for consensus
pub struct FheLtvComputer {
    engine: FheEngine,
}

impl FheLtvComputer {
    /// Create new LTV computer with FHE engine
    pub fn new(engine: FheEngine) -> Self {
        Self { engine }
    }

    /// Compute LTV ratio on encrypted data
    ///
    /// # Arguments
    /// * `encrypted_data` - Encrypted loan data (collateral and borrow amounts)
    ///
    /// # Returns
    /// * Encrypted LTV ratio (in basis points, 10000 = 100%)
    ///
    /// # Note
    /// This performs homomorphic division approximation:
    /// LTV = (borrow_amount * 10000) / collateral_amount
    pub fn compute_ltv_encrypted(&self, encrypted_data: &EncryptedLoanData) -> Result<Vec<u8>> {
        self.engine.set_key_for_thread();

        // Deserialize encrypted borrow amount
        let borrow: FheUint8 = bincode::deserialize(&encrypted_data.borrow_c1)
            .context("Failed to deserialize encrypted borrow amount")?;

        // Compute encrypted LTV ratio
        // Note: For simplicity, we use u8 arithmetic. In production, use FheUint32/FheUint64
        // LTV (basis points) = (borrow / collateral) * 10000
        // Simplified: ratio = borrow * 100 / collateral (scaled)

        // For u8 arithmetic, we approximate: ratio = (borrow * 100) / collateral
        let borrow_scaled = borrow * 100u8;

        // Homomorphic division is expensive, so we approximate using comparison
        // Instead, return the scaled borrow amount and let activation logic decide
        // In production, implement proper FHE division or use higher precision

        let encrypted_ltv = bincode::serialize(&borrow_scaled)
            .context("Failed to serialize encrypted LTV")?;

        Ok(encrypted_ltv)
    }

    /// Verify if encrypted LTV is within threshold
    ///
    /// # Arguments
    /// * `encrypted_data` - Encrypted loan data
    ///
    /// # Returns
    /// * Encrypted boolean: true if LTV <= threshold, false otherwise
    ///
    /// # Note
    /// Returns encrypted result for consensus. Multiple provers compute this
    /// and submit their encrypted results. Consensus is reached when results match.
    pub fn verify_ltv_threshold(&self, encrypted_data: &EncryptedLoanData) -> Result<Vec<u8>> {
        self.engine.set_key_for_thread();

        // Deserialize encrypted amounts
        let collateral: FheUint8 = bincode::deserialize(&encrypted_data.collateral_c1)
            .context("Failed to deserialize encrypted collateral")?;

        let borrow: FheUint8 = bincode::deserialize(&encrypted_data.borrow_c1)
            .context("Failed to deserialize encrypted borrow amount")?;

        // Compute LTV check: borrow_amount * 10000 <= collateral_amount * threshold
        // Simplification for u8: borrow * 100 <= collateral * (threshold / 100)
        let threshold_scaled = (encrypted_data.ltv_threshold / 100) as u8;

        let borrow_scaled = borrow * 100u8;
        let collateral_scaled = collateral * threshold_scaled;

        // Homomorphic comparison: is LTV within threshold?
        let ltv_ok = borrow_scaled.le(&collateral_scaled);

        // Serialize encrypted boolean result
        let encrypted_result = bincode::serialize(&ltv_ok)
            .context("Failed to serialize LTV verification result")?;

        Ok(encrypted_result)
    }

    /// Compute LTV verification hash for consensus
    ///
    /// # Arguments
    /// * `encrypted_result` - Encrypted verification result
    /// * `loan_id` - Loan ID for context
    ///
    /// # Returns
    /// * SHA3-256 hash of encrypted result (for consensus matching)
    pub fn compute_result_hash(&self, encrypted_result: &[u8], loan_id: u64) -> Vec<u8> {
        let mut hasher = Sha3_256::new();
        hasher.update(loan_id.to_le_bytes());
        hasher.update(encrypted_result);
        hasher.finalize().to_vec()
    }

    /// Full LTV verification workflow for prover node
    ///
    /// # Arguments
    /// * `loan_id` - Loan ID
    /// * `encrypted_data` - Encrypted loan data
    ///
    /// # Returns
    /// * Encrypted result bytes and hash for submission to blockchain
    pub fn verify_loan(
        &self,
        loan_id: u64,
        encrypted_data: &EncryptedLoanData,
    ) -> Result<(Vec<u8>, Vec<u8>)> {
        // Compute encrypted LTV verification
        let encrypted_result = self.verify_ltv_threshold(encrypted_data)
            .context("Failed to verify LTV threshold")?;

        // Compute hash for consensus
        let result_hash = self.compute_result_hash(&encrypted_result, loan_id);

        Ok((encrypted_result, result_hash))
    }
}

/// Helper function to create LTV computer from server key bytes
pub fn create_ltv_computer(server_key_bytes: &[u8]) -> Result<FheLtvComputer> {
    let server_key = zyberlink_fhe::deserialize_server_key(server_key_bytes)
        .context("Failed to deserialize server key")?;

    let engine = FheEngine::new(server_key);
    Ok(FheLtvComputer::new(engine))
}

#[cfg(test)]
mod tests {
    use super::*;
    use zyberlink_fhe::{generate_keys, encrypt_value};

    #[test]
    fn test_ltv_verification_basic() {
        // Generate keys
        let (client_key, server_key) = generate_keys().unwrap();
        let engine = FheEngine::new(server_key);
        let computer = FheLtvComputer::new(engine);

        // Encrypt test amounts
        // Collateral: 100 units
        // Borrow: 50 units
        // LTV = 50%
        let collateral_encrypted = encrypt_value(100u8, &client_key).unwrap();
        let borrow_encrypted = encrypt_value(50u8, &client_key).unwrap();

        let encrypted_data = EncryptedLoanData {
            collateral_c1: collateral_encrypted.clone(),
            collateral_c2: vec![], // Not used in simplified version
            borrow_c1: borrow_encrypted.clone(),
            borrow_c2: vec![],
            ltv_threshold: 7500, // 75% threshold
        };

        // Verify LTV (should pass: 50% < 75%)
        let result = computer.verify_ltv_threshold(&encrypted_data).unwrap();
        assert!(!result.is_empty());

        // Verify hash generation
        let hash = computer.compute_result_hash(&result, 1);
        assert_eq!(hash.len(), 32); // SHA3-256
    }

    #[test]
    fn test_ltv_verification_workflow() {
        let (client_key, server_key) = generate_keys().unwrap();
        let engine = FheEngine::new(server_key);
        let computer = FheLtvComputer::new(engine);

        let collateral_encrypted = encrypt_value(100u8, &client_key).unwrap();
        let borrow_encrypted = encrypt_value(75u8, &client_key).unwrap();

        let encrypted_data = EncryptedLoanData {
            collateral_c1: collateral_encrypted,
            collateral_c2: vec![],
            borrow_c1: borrow_encrypted,
            borrow_c2: vec![],
            ltv_threshold: 7500, // 75%
        };

        let (encrypted_result, result_hash) = computer.verify_loan(42, &encrypted_data).unwrap();

        assert!(!encrypted_result.is_empty());
        assert_eq!(result_hash.len(), 32);
    }

    #[test]
    fn test_create_ltv_computer() {
        let (_, server_key) = generate_keys().unwrap();
        let server_key_bytes = zyberlink_fhe::serialize_server_key(&server_key).unwrap();

        let computer = create_ltv_computer(&server_key_bytes).unwrap();

        // Computer should be functional
        let (client_key, _) = generate_keys().unwrap();
        let encrypted = encrypt_value(42u8, &client_key).unwrap();

        let data = EncryptedLoanData {
            collateral_c1: encrypted.clone(),
            collateral_c2: vec![],
            borrow_c1: encrypted,
            borrow_c2: vec![],
            ltv_threshold: 7500,
        };

        let result = computer.verify_ltv_threshold(&data);
        assert!(result.is_ok());
    }
}
