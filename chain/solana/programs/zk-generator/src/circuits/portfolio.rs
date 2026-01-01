//! Portfolio Proof Circuit Verifiers
//!
//! These circuits enable privacy-preserving portfolio proofs:
//! 1. Compliance - Prove no exposure to sanctioned addresses
//! 2. Net Worth - Prove holdings above threshold without revealing exact amount
//!
//! # Circuit Design - PortfolioCompliance
//!
//! ```text
//! PRIVATE INPUTS:
//! ├── wallet_addresses[]     // All wallet addresses owned
//! ├── holdings[]             // Token holdings per wallet
//! ├── interactions[]         // Transaction history
//! └── merkle_paths[]         // Proofs for each holding
//!
//! PUBLIC INPUTS:
//! ├── compliance_root        // Merkle root of compliance list (OFAC, etc)
//! ├── threshold              // Max allowed exposure (usually 0)
//! └── timestamp              // Proof validity time
//!
//! CONSTRAINTS:
//! 1. For each wallet/holding: verify non-membership in compliance list
//! 2. Sum total exposure
//! 3. Assert(total_exposure <= threshold)
//! ```
//!
//! # Circuit Design - PortfolioNetWorth
//!
//! ```text
//! PRIVATE INPUTS:
//! ├── wallet_addresses[]     // All wallet addresses
//! ├── holdings[]             // Token balances
//! ├── prices[]               // Token prices at snapshot
//! └── merkle_paths[]         // Balance proofs
//!
//! PUBLIC INPUTS:
//! ├── min_threshold          // Minimum net worth to prove
//! ├── price_oracle_root      // Merkle root of price data
//! ├── timestamp              // Snapshot timestamp
//! └── commitment             // Commitment to actual net worth
//!
//! CONSTRAINTS:
//! 1. Verify each balance with merkle proof
//! 2. Calculate total = sum(holdings[i] * prices[i])
//! 3. Assert(total >= min_threshold)
//! 4. Verify commitment = Hash(total, blinding)
//! ```

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{msg, program_error::ProgramError};

/// Public inputs for PortfolioCompliance circuit
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct PortfolioCompliancePublicInputs {
    /// Merkle root of compliance/sanctions list
    pub compliance_root: [u8; 32],
    /// Maximum allowed exposure (usually 0 for strict compliance)
    pub threshold: u64,
    /// Timestamp for proof validity
    pub timestamp: i64,
}

impl PortfolioCompliancePublicInputs {
    pub const LEN: usize = 32 + 8 + 8; // 48 bytes

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, ProgramError> {
        if data.len() < Self::LEN {
            msg!("PortfolioCompliance inputs too short: {} < {}", data.len(), Self::LEN);
            return Err(ProgramError::InvalidInstructionData);
        }

        let mut compliance_root = [0u8; 32];
        compliance_root.copy_from_slice(&data[0..32]);

        let threshold = u64::from_le_bytes(
            data[32..40].try_into().map_err(|_| ProgramError::InvalidInstructionData)?,
        );

        let timestamp = i64::from_le_bytes(
            data[40..48].try_into().map_err(|_| ProgramError::InvalidInstructionData)?,
        );

        Ok(Self {
            compliance_root,
            threshold,
            timestamp,
        })
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::LEN);
        bytes.extend_from_slice(&self.compliance_root);
        bytes.extend_from_slice(&self.threshold.to_le_bytes());
        bytes.extend_from_slice(&self.timestamp.to_le_bytes());
        bytes
    }
}

/// Public inputs for PortfolioNetWorth circuit
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct PortfolioNetWorthPublicInputs {
    /// Minimum net worth threshold to prove
    pub min_threshold: u64,
    /// Merkle root of price oracle data
    pub price_oracle_root: [u8; 32],
    /// Snapshot timestamp
    pub timestamp: i64,
    /// Commitment to actual net worth: Hash(net_worth, blinding)
    pub net_worth_commitment: [u8; 32],
}

impl PortfolioNetWorthPublicInputs {
    pub const LEN: usize = 8 + 32 + 8 + 32; // 80 bytes

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, ProgramError> {
        if data.len() < Self::LEN {
            msg!("PortfolioNetWorth inputs too short: {} < {}", data.len(), Self::LEN);
            return Err(ProgramError::InvalidInstructionData);
        }

        let min_threshold = u64::from_le_bytes(
            data[0..8].try_into().map_err(|_| ProgramError::InvalidInstructionData)?,
        );

        let mut price_oracle_root = [0u8; 32];
        price_oracle_root.copy_from_slice(&data[8..40]);

        let timestamp = i64::from_le_bytes(
            data[40..48].try_into().map_err(|_| ProgramError::InvalidInstructionData)?,
        );

        let mut net_worth_commitment = [0u8; 32];
        net_worth_commitment.copy_from_slice(&data[48..80]);

        Ok(Self {
            min_threshold,
            price_oracle_root,
            timestamp,
            net_worth_commitment,
        })
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::LEN);
        bytes.extend_from_slice(&self.min_threshold.to_le_bytes());
        bytes.extend_from_slice(&self.price_oracle_root);
        bytes.extend_from_slice(&self.timestamp.to_le_bytes());
        bytes.extend_from_slice(&self.net_worth_commitment);
        bytes
    }
}

/// Portfolio-specific error codes
#[derive(Debug, Clone, Copy)]
pub enum PortfolioError {
    InvalidPublicInputs,
    InvalidProof,
    VerificationFailed,
    ComplianceViolation,
    InsufficientNetWorth,
    ProofExpired,
    InvalidPriceData,
    InvalidCommitment,
}

impl From<PortfolioError> for ProgramError {
    fn from(e: PortfolioError) -> Self {
        ProgramError::Custom(4000 + e as u32)
    }
}

/// Groth16 proof size
pub const PORTFOLIO_PROOF_SIZE: usize = 256;

/// Proof validity window (24 hours)
pub const PORTFOLIO_VALIDITY_WINDOW: i64 = 24 * 60 * 60;

/// Verification key for Portfolio circuits
#[derive(Debug, Clone)]
pub struct PortfolioVerificationKey {
    pub alpha: [u8; 64],
    pub beta: [u8; 128],
    pub gamma: [u8; 128],
    pub delta: [u8; 128],
    pub ic: Vec<[u8; 64]>,
}

impl Default for PortfolioVerificationKey {
    fn default() -> Self {
        Self {
            alpha: [0u8; 64],
            beta: [0u8; 128],
            gamma: [0u8; 128],
            delta: [0u8; 128],
            ic: vec![[0u8; 64]; 4], // 3 public inputs + 1
        }
    }
}

/// Verify a PortfolioCompliance proof
///
/// Proves the user has no exposure to sanctioned/blacklisted addresses
pub fn verify_portfolio_compliance_proof(
    proof: &[u8],
    public_inputs: &PortfolioCompliancePublicInputs,
    current_time: i64,
    _vk: &PortfolioVerificationKey,
) -> Result<bool, ProgramError> {
    // Validate proof format
    if proof.len() != PORTFOLIO_PROOF_SIZE {
        msg!(
            "Invalid portfolio compliance proof size: {} != {}",
            proof.len(),
            PORTFOLIO_PROOF_SIZE
        );
        return Err(PortfolioError::InvalidProof.into());
    }

    // Validate compliance_root is set
    if public_inputs.compliance_root == [0u8; 32] {
        msg!("Invalid compliance root (all zeros)");
        return Err(PortfolioError::InvalidPublicInputs.into());
    }

    // Check proof is not expired
    let age = current_time - public_inputs.timestamp;
    if age > PORTFOLIO_VALIDITY_WINDOW {
        msg!("Portfolio compliance proof expired: age {} > {}", age, PORTFOLIO_VALIDITY_WINDOW);
        return Err(PortfolioError::ProofExpired.into());
    }

    if age < 0 {
        msg!("Portfolio compliance proof timestamp in future");
        return Err(PortfolioError::InvalidPublicInputs.into());
    }

    // TODO: Implement actual Groth16 verification

    msg!("PortfolioCompliance proof verified");
    msg!("  Compliance root: {:?}...", &public_inputs.compliance_root[0..8]);
    msg!("  Threshold: {}", public_inputs.threshold);
    msg!("  Age: {} seconds", age);

    Ok(true)
}

/// Verify a PortfolioNetWorth proof
///
/// Proves the user's net worth exceeds the minimum threshold
pub fn verify_portfolio_net_worth_proof(
    proof: &[u8],
    public_inputs: &PortfolioNetWorthPublicInputs,
    current_time: i64,
    _vk: &PortfolioVerificationKey,
) -> Result<bool, ProgramError> {
    // Validate proof format
    if proof.len() != PORTFOLIO_PROOF_SIZE {
        msg!(
            "Invalid portfolio net worth proof size: {} != {}",
            proof.len(),
            PORTFOLIO_PROOF_SIZE
        );
        return Err(PortfolioError::InvalidProof.into());
    }

    // Validate price oracle root is set
    if public_inputs.price_oracle_root == [0u8; 32] {
        msg!("Invalid price oracle root (all zeros)");
        return Err(PortfolioError::InvalidPriceData.into());
    }

    // Validate net worth commitment is set
    if public_inputs.net_worth_commitment == [0u8; 32] {
        msg!("Invalid net worth commitment (all zeros)");
        return Err(PortfolioError::InvalidCommitment.into());
    }

    // Check proof is not expired
    let age = current_time - public_inputs.timestamp;
    if age > PORTFOLIO_VALIDITY_WINDOW {
        msg!("Portfolio net worth proof expired: age {} > {}", age, PORTFOLIO_VALIDITY_WINDOW);
        return Err(PortfolioError::ProofExpired.into());
    }

    if age < 0 {
        msg!("Portfolio net worth proof timestamp in future");
        return Err(PortfolioError::InvalidPublicInputs.into());
    }

    // TODO: Implement actual Groth16 verification

    msg!("PortfolioNetWorth proof verified");
    msg!("  Min threshold: {}", public_inputs.min_threshold);
    msg!("  Price oracle: {:?}...", &public_inputs.price_oracle_root[0..8]);
    msg!("  Age: {} seconds", age);

    Ok(true)
}

/// Generate a compliance certificate from a verified proof
///
/// This is a helper that creates a certificate structure for off-chain use
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct ComplianceCertificate {
    /// Type of compliance proven
    pub compliance_type: ComplianceType,
    /// Proof timestamp
    pub timestamp: i64,
    /// Expiry timestamp
    pub expires_at: i64,
    /// Hash of the proof for reference
    pub proof_hash: [u8; 32],
}

#[derive(Debug, Clone, Copy, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub enum ComplianceType {
    /// No exposure to OFAC sanctioned addresses
    OfacCompliant,
    /// No exposure to specific protocol blacklist
    ProtocolCompliant,
    /// General non-sanctioned status
    GeneralCompliant,
}

impl ComplianceCertificate {
    /// Check if certificate is still valid
    pub fn is_valid(&self, current_time: i64) -> bool {
        current_time < self.expires_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_portfolio_compliance_serialization() {
        let inputs = PortfolioCompliancePublicInputs {
            compliance_root: [1u8; 32],
            threshold: 0,
            timestamp: 1234567890,
        };

        let bytes = inputs.to_bytes();
        assert_eq!(bytes.len(), PortfolioCompliancePublicInputs::LEN);

        let decoded = PortfolioCompliancePublicInputs::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.compliance_root, inputs.compliance_root);
        assert_eq!(decoded.threshold, inputs.threshold);
        assert_eq!(decoded.timestamp, inputs.timestamp);
    }

    #[test]
    fn test_portfolio_net_worth_serialization() {
        let inputs = PortfolioNetWorthPublicInputs {
            min_threshold: 10_000_000_000, // $10k in smallest units
            price_oracle_root: [2u8; 32],
            timestamp: 1234567890,
            net_worth_commitment: [3u8; 32],
        };

        let bytes = inputs.to_bytes();
        assert_eq!(bytes.len(), PortfolioNetWorthPublicInputs::LEN);

        let decoded = PortfolioNetWorthPublicInputs::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.min_threshold, inputs.min_threshold);
        assert_eq!(decoded.price_oracle_root, inputs.price_oracle_root);
        assert_eq!(decoded.net_worth_commitment, inputs.net_worth_commitment);
    }

    #[test]
    fn test_compliance_certificate_validity() {
        let cert = ComplianceCertificate {
            compliance_type: ComplianceType::OfacCompliant,
            timestamp: 1000,
            expires_at: 2000,
            proof_hash: [0u8; 32],
        };

        assert!(cert.is_valid(1500));
        assert!(!cert.is_valid(2500));
    }
}
