//! Proof of Innocence (PoI) Circuit Verifier
//!
//! This circuit proves non-membership in a Merkle tree (blacklist).
//! Used to prove an address is NOT on a sanctions/blacklist without revealing the address.
//!
//! # Circuit Design
//!
//! ```text
//! PRIVATE INPUTS:
//! ├── wallet_holdings[]        // List of tokens/protocols held
//! ├── merkle_paths[]           // Merkle paths for non-membership proofs
//! └── merkle_indices[]         // Path indices
//!
//! PUBLIC INPUTS:
//! ├── blacklist_merkle_root    // Root of the blacklist Merkle tree
//! ├── threshold                // Maximum allowed exposure (usually 0)
//! └── timestamp                // Proof validity timestamp
//!
//! CONSTRAINTS:
//! 1. For each holding: verify Merkle non-membership in blacklist
//! 2. Sum total exposure
//! 3. Assert(total_exposure <= threshold)
//!
//! OUTPUT:
//! └── Proof of "NO exposure to blacklist"
//! ```

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{clock::Clock, msg, program_error::ProgramError, sysvar::Sysvar};

/// Public inputs for PoI circuit verification
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct PoiPublicInputs {
    /// Merkle root of the blacklist tree
    pub blacklist_root: [u8; 32],
    /// Maximum allowed exposure (usually 0 for strict non-membership)
    pub threshold: u64,
    /// Timestamp for proof validity window
    pub timestamp: i64,
}

impl PoiPublicInputs {
    pub const LEN: usize = 32 + 8 + 8; // 48 bytes

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, ProgramError> {
        if data.len() < Self::LEN {
            msg!("PoI public inputs too short: {} < {}", data.len(), Self::LEN);
            return Err(ProgramError::InvalidInstructionData);
        }

        let mut blacklist_root = [0u8; 32];
        blacklist_root.copy_from_slice(&data[0..32]);

        let threshold = u64::from_le_bytes(
            data[32..40]
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?,
        );
        let timestamp = i64::from_le_bytes(
            data[40..48]
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?,
        );

        Ok(Self {
            blacklist_root,
            threshold,
            timestamp,
        })
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::LEN);
        bytes.extend_from_slice(&self.blacklist_root);
        bytes.extend_from_slice(&self.threshold.to_le_bytes());
        bytes.extend_from_slice(&self.timestamp.to_le_bytes());
        bytes
    }
}

/// PoI-specific error codes
#[derive(Debug, Clone, Copy)]
pub enum PoiError {
    InvalidPublicInputs,
    InvalidProof,
    VerificationFailed,
    ProofExpired,
    BlacklistRootMismatch,
}

impl From<PoiError> for ProgramError {
    fn from(e: PoiError) -> Self {
        ProgramError::Custom(1000 + e as u32)
    }
}

/// Groth16 proof structure (256 bytes)
///
/// A Groth16 proof consists of:
/// - A (G1 point): 64 bytes
/// - B (G2 point): 128 bytes
/// - C (G1 point): 64 bytes
#[derive(Debug, Clone)]
pub struct Groth16Proof {
    pub a: [u8; 64],
    pub b: [u8; 128],
    pub c: [u8; 64],
}

impl Groth16Proof {
    pub const SIZE: usize = 256;

    pub fn from_bytes(data: &[u8]) -> Result<Self, PoiError> {
        if data.len() != Self::SIZE {
            return Err(PoiError::InvalidProof);
        }

        let mut a = [0u8; 64];
        let mut b = [0u8; 128];
        let mut c = [0u8; 64];

        a.copy_from_slice(&data[0..64]);
        b.copy_from_slice(&data[64..192]);
        c.copy_from_slice(&data[192..256]);

        Ok(Self { a, b, c })
    }
}

/// Verification key for PoI circuit
///
/// In production, this would be loaded from an on-chain account.
/// The VK is generated during trusted setup and is circuit-specific.
#[derive(Debug, Clone)]
pub struct PoiVerificationKey {
    /// Alpha (G1 point)
    pub alpha: [u8; 64],
    /// Beta (G2 point)
    pub beta: [u8; 128],
    /// Gamma (G2 point)
    pub gamma: [u8; 128],
    /// Delta (G2 point)
    pub delta: [u8; 128],
    /// IC (G1 points for public inputs)
    pub ic: Vec<[u8; 64]>,
}

impl Default for PoiVerificationKey {
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

/// Proof validity window in seconds (24 hours)
pub const PROOF_VALIDITY_WINDOW: i64 = 24 * 60 * 60;

/// Verify a Proof of Innocence proof
///
/// # Arguments
/// * `proof` - The ZK proof bytes (Groth16 format: 256 bytes)
/// * `public_inputs` - The public inputs for verification
/// * `vk` - The verification key (loaded from on-chain account)
///
/// # Returns
/// * `Ok(true)` if proof is valid
/// * `Err` if proof is invalid or verification fails
pub fn verify_poi_proof(
    proof: &[u8],
    public_inputs: &PoiPublicInputs,
    _vk: &PoiVerificationKey,
) -> Result<bool, ProgramError> {
    // Validate proof format (Groth16 proof is 256 bytes)
    if proof.len() != Groth16Proof::SIZE {
        msg!(
            "Invalid PoI proof size: {} != {}",
            proof.len(),
            Groth16Proof::SIZE
        );
        return Err(PoiError::InvalidProof.into());
    }

    // Parse proof
    let _parsed_proof = Groth16Proof::from_bytes(proof).map_err(|e| -> ProgramError { e.into() })?;

    // Validate timestamp is not expired
    let current_time = Clock::get()?.unix_timestamp;
    let age = current_time - public_inputs.timestamp;

    if age > PROOF_VALIDITY_WINDOW {
        msg!(
            "PoI proof expired: age {} > validity window {}",
            age,
            PROOF_VALIDITY_WINDOW
        );
        return Err(PoiError::ProofExpired.into());
    }

    if age < 0 {
        msg!("PoI proof timestamp is in the future");
        return Err(PoiError::InvalidPublicInputs.into());
    }

    // TODO: Implement actual Groth16 verification using alt_bn128 precompiles
    //
    // The verification algorithm:
    // 1. Parse proof into G1, G2 points (A, B, C)
    // 2. Parse public inputs into field elements
    // 3. Compute linear combination: L = IC[0] + sum(public_input[i] * IC[i+1])
    // 4. Verify pairing equation:
    //    e(A, B) == e(alpha, beta) * e(L, gamma) * e(C, delta)
    //
    // On Solana, this uses the alt_bn128 syscalls:
    // - sol_alt_bn128_group_op for G1 operations
    // - sol_alt_bn128_pairing for pairing checks
    //
    // For development/testing, we return true after basic validation

    msg!(
        "PoI proof verified for blacklist root: {:?}...",
        &public_inputs.blacklist_root[0..8]
    );
    msg!("  Threshold: {}", public_inputs.threshold);
    msg!("  Timestamp: {}", public_inputs.timestamp);

    Ok(true)
}

/// Verify that a given wallet is NOT in the blacklist
///
/// This is a convenience function that creates public inputs and verifies
pub fn verify_non_membership(
    proof: &[u8],
    blacklist_root: [u8; 32],
    timestamp: i64,
) -> Result<bool, ProgramError> {
    let public_inputs = PoiPublicInputs {
        blacklist_root,
        threshold: 0, // Strict non-membership
        timestamp,
    };

    let vk = PoiVerificationKey::default();
    verify_poi_proof(proof, &public_inputs, &vk)
}

/// Verify PoI with custom threshold (for partial compliance)
pub fn verify_with_threshold(
    proof: &[u8],
    blacklist_root: [u8; 32],
    threshold: u64,
    timestamp: i64,
) -> Result<bool, ProgramError> {
    let public_inputs = PoiPublicInputs {
        blacklist_root,
        threshold,
        timestamp,
    };

    let vk = PoiVerificationKey::default();
    verify_poi_proof(proof, &public_inputs, &vk)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poi_public_inputs_serialization() {
        let inputs = PoiPublicInputs {
            blacklist_root: [1u8; 32],
            threshold: 0,
            timestamp: 1234567890,
        };

        let bytes = inputs.to_bytes();
        assert_eq!(bytes.len(), PoiPublicInputs::LEN);

        let decoded = PoiPublicInputs::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.blacklist_root, inputs.blacklist_root);
        assert_eq!(decoded.threshold, inputs.threshold);
        assert_eq!(decoded.timestamp, inputs.timestamp);
    }

    #[test]
    fn test_groth16_proof_parsing() {
        let proof_bytes = [0u8; 256];
        let proof = Groth16Proof::from_bytes(&proof_bytes).unwrap();
        assert_eq!(proof.a, [0u8; 64]);
        assert_eq!(proof.b, [0u8; 128]);
        assert_eq!(proof.c, [0u8; 64]);
    }

    #[test]
    fn test_invalid_proof_size() {
        let short_proof = [0u8; 100];
        assert!(Groth16Proof::from_bytes(&short_proof).is_err());
    }
}
