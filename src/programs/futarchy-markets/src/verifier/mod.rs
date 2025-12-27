//! Groth16 verification module for Futarchy V2 circuits
//!
//! Uses groth16-solana with Solana's native BN254 syscalls for efficient on-chain verification.
//! Verification takes < 200,000 compute units per proof.
//!
//! # Features
//!
//! - `mock-proofs`: Bypasses Groth16 verification for testing. NEVER enable in production!

mod claim_private_v2_vk;
mod place_bet_private_vk;
mod withdraw_private_vk;

pub use claim_private_v2_vk::CLAIM_PRIVATE_V2_VK;
pub use place_bet_private_vk::PLACE_BET_PRIVATE_VK;
pub use withdraw_private_vk::WITHDRAW_PRIVATE_VK;

#[cfg(not(feature = "mock-proofs"))]
use groth16_solana::groth16::Groth16Verifier;
use solana_program::{msg, program_error::ProgramError};

/// Groth16 proof size (A: 64, B: 128, C: 64)
pub const GROTH16_PROOF_SIZE: usize = 256;

/// Number of public inputs for PlaceBetPrivate circuit
pub const PLACE_BET_NR_INPUTS: usize = 5;

/// Number of public inputs for ClaimPrivateV2 circuit
pub const CLAIM_NR_INPUTS: usize = 7;

/// Number of public inputs for WithdrawPrivate circuit
pub const WITHDRAW_NR_INPUTS: usize = 3;

/// Error codes for verification
#[derive(Debug, Clone, Copy)]
pub enum VerifyError {
    InvalidProofSize,
    InvalidPublicInputsSize,
    VerificationFailed,
    ProofParseError,
}

impl From<VerifyError> for ProgramError {
    fn from(e: VerifyError) -> Self {
        ProgramError::Custom(5000 + e as u32)
    }
}

/// Parse Groth16 proof from bytes
///
/// Proof format (256 bytes):
/// - A: 64 bytes (G1 point, already negated for groth16-solana)
/// - B: 128 bytes (G2 point)
/// - C: 64 bytes (G1 point)
fn parse_proof(proof: &[u8]) -> Result<([u8; 64], [u8; 128], [u8; 64]), VerifyError> {
    if proof.len() != GROTH16_PROOF_SIZE {
        msg!("Invalid proof size: {} != {}", proof.len(), GROTH16_PROOF_SIZE);
        return Err(VerifyError::InvalidProofSize);
    }

    let mut proof_a = [0u8; 64];
    let mut proof_b = [0u8; 128];
    let mut proof_c = [0u8; 64];

    proof_a.copy_from_slice(&proof[0..64]);
    proof_b.copy_from_slice(&proof[64..192]);
    proof_c.copy_from_slice(&proof[192..256]);

    Ok((proof_a, proof_b, proof_c))
}

/// Verify a PlaceBetPrivate proof
///
/// Public inputs (5 field elements = 160 bytes):
/// 1. old_balance_commitment (32 bytes)
/// 2. new_balance_commitment (32 bytes)
/// 3. bet_commitment (32 bytes)
/// 4. max_bet (32 bytes - padded)
/// 5. bet_amount (32 bytes - padded)
#[cfg(not(feature = "mock-proofs"))]
pub fn verify_place_bet_proof(proof: &[u8], public_inputs: &[u8]) -> Result<bool, ProgramError> {
    msg!("Verifying PlaceBetPrivate proof on-chain (Groth16)");

    let (proof_a, proof_b, proof_c) = parse_proof(proof)?;

    // Place bet has 5 public inputs
    let expected_size = PLACE_BET_NR_INPUTS * 32;
    if public_inputs.len() < expected_size {
        msg!("Invalid public inputs size: {} < {}", public_inputs.len(), expected_size);
        return Err(VerifyError::InvalidPublicInputsSize.into());
    }

    let mut inputs: [[u8; 32]; PLACE_BET_NR_INPUTS] = [[0u8; 32]; PLACE_BET_NR_INPUTS];
    for i in 0..PLACE_BET_NR_INPUTS {
        inputs[i].copy_from_slice(&public_inputs[i * 32..(i + 1) * 32]);
    }

    let mut verifier = Groth16Verifier::<PLACE_BET_NR_INPUTS>::new(
        &proof_a,
        &proof_b,
        &proof_c,
        &inputs,
        &PLACE_BET_PRIVATE_VK,
    )
    .map_err(|e| {
        msg!("Verifier init error: {:?}", e);
        VerifyError::ProofParseError
    })?;

    match verifier.verify() {
        Ok(()) => {
            msg!("PlaceBetPrivate proof VERIFIED");
            Ok(true)
        }
        Err(e) => {
            msg!("PlaceBetPrivate proof INVALID: {:?}", e);
            Ok(false)
        }
    }
}

/// Mock proof verification for testing
#[cfg(feature = "mock-proofs")]
pub fn verify_place_bet_proof(proof: &[u8], public_inputs: &[u8]) -> Result<bool, ProgramError> {
    msg!("MOCK: Bypassing PlaceBetPrivate proof verification (mock-proofs feature enabled)");

    // Basic size validation only
    if proof.len() != GROTH16_PROOF_SIZE {
        msg!("Invalid proof size: {} != {}", proof.len(), GROTH16_PROOF_SIZE);
        return Err(VerifyError::InvalidProofSize.into());
    }

    let expected_size = PLACE_BET_NR_INPUTS * 32;
    if public_inputs.len() < expected_size {
        msg!("Invalid public inputs size: {} < {}", public_inputs.len(), expected_size);
        return Err(VerifyError::InvalidPublicInputsSize.into());
    }

    msg!("PlaceBetPrivate proof ACCEPTED (mock mode)");
    Ok(true)
}

/// Verify a ClaimPrivateV2 proof
///
/// Public inputs (7 field elements = 224 bytes):
/// 1. bet_commitment (32 bytes)
/// 2. resolution (32 bytes - padded bool)
/// 3. nullifier_hash (32 bytes)
/// 4. bet_amount (32 bytes - padded)
/// 5. bet_side (32 bytes - padded bool)
/// 6. old_balance_commitment (32 bytes)
/// 7. new_balance_commitment (32 bytes)
#[cfg(not(feature = "mock-proofs"))]
pub fn verify_claim_proof(proof: &[u8], public_inputs: &[u8]) -> Result<bool, ProgramError> {
    msg!("Verifying ClaimPrivateV2 proof on-chain (Groth16)");

    let (proof_a, proof_b, proof_c) = parse_proof(proof)?;

    // Claim has 7 public inputs
    let expected_size = CLAIM_NR_INPUTS * 32;
    if public_inputs.len() < expected_size {
        msg!("Invalid public inputs size: {} < {}", public_inputs.len(), expected_size);
        return Err(VerifyError::InvalidPublicInputsSize.into());
    }

    let mut inputs: [[u8; 32]; CLAIM_NR_INPUTS] = [[0u8; 32]; CLAIM_NR_INPUTS];
    for i in 0..CLAIM_NR_INPUTS {
        inputs[i].copy_from_slice(&public_inputs[i * 32..(i + 1) * 32]);
    }

    let mut verifier = Groth16Verifier::<CLAIM_NR_INPUTS>::new(
        &proof_a,
        &proof_b,
        &proof_c,
        &inputs,
        &CLAIM_PRIVATE_V2_VK,
    )
    .map_err(|e| {
        msg!("Verifier init error: {:?}", e);
        VerifyError::ProofParseError
    })?;

    match verifier.verify() {
        Ok(()) => {
            msg!("ClaimPrivateV2 proof VERIFIED");
            Ok(true)
        }
        Err(e) => {
            msg!("ClaimPrivateV2 proof INVALID: {:?}", e);
            Ok(false)
        }
    }
}

/// Mock claim proof verification for testing
#[cfg(feature = "mock-proofs")]
pub fn verify_claim_proof(proof: &[u8], public_inputs: &[u8]) -> Result<bool, ProgramError> {
    msg!("MOCK: Bypassing ClaimPrivateV2 proof verification (mock-proofs feature enabled)");

    // Basic size validation only
    if proof.len() != GROTH16_PROOF_SIZE {
        msg!("Invalid proof size: {} != {}", proof.len(), GROTH16_PROOF_SIZE);
        return Err(VerifyError::InvalidProofSize.into());
    }

    let expected_size = CLAIM_NR_INPUTS * 32;
    if public_inputs.len() < expected_size {
        msg!("Invalid public inputs size: {} < {}", public_inputs.len(), expected_size);
        return Err(VerifyError::InvalidPublicInputsSize.into());
    }

    msg!("ClaimPrivateV2 proof ACCEPTED (mock mode)");
    Ok(true)
}

/// Verify a WithdrawPrivate proof
///
/// Public inputs (3 field elements = 96 bytes):
/// 1. old_balance_commitment (32 bytes)
/// 2. new_balance_commitment (32 bytes)
/// 3. withdraw_amount (32 bytes - padded)
#[cfg(not(feature = "mock-proofs"))]
pub fn verify_withdraw_proof(proof: &[u8], public_inputs: &[u8]) -> Result<bool, ProgramError> {
    msg!("Verifying WithdrawPrivate proof on-chain (Groth16)");

    let (proof_a, proof_b, proof_c) = parse_proof(proof)?;

    // Withdraw has 3 public inputs
    let expected_size = WITHDRAW_NR_INPUTS * 32;
    if public_inputs.len() < expected_size {
        msg!("Invalid public inputs size: {} < {}", public_inputs.len(), expected_size);
        return Err(VerifyError::InvalidPublicInputsSize.into());
    }

    let mut inputs: [[u8; 32]; WITHDRAW_NR_INPUTS] = [[0u8; 32]; WITHDRAW_NR_INPUTS];
    for i in 0..WITHDRAW_NR_INPUTS {
        inputs[i].copy_from_slice(&public_inputs[i * 32..(i + 1) * 32]);
    }

    let mut verifier = Groth16Verifier::<WITHDRAW_NR_INPUTS>::new(
        &proof_a,
        &proof_b,
        &proof_c,
        &inputs,
        &WITHDRAW_PRIVATE_VK,
    )
    .map_err(|e| {
        msg!("Verifier init error: {:?}", e);
        VerifyError::ProofParseError
    })?;

    match verifier.verify() {
        Ok(()) => {
            msg!("WithdrawPrivate proof VERIFIED");
            Ok(true)
        }
        Err(e) => {
            msg!("WithdrawPrivate proof INVALID: {:?}", e);
            Ok(false)
        }
    }
}

/// Mock withdraw proof verification for testing
#[cfg(feature = "mock-proofs")]
pub fn verify_withdraw_proof(proof: &[u8], public_inputs: &[u8]) -> Result<bool, ProgramError> {
    msg!("MOCK: Bypassing WithdrawPrivate proof verification (mock-proofs feature enabled)");

    // Basic size validation only
    if proof.len() != GROTH16_PROOF_SIZE {
        msg!("Invalid proof size: {} != {}", proof.len(), GROTH16_PROOF_SIZE);
        return Err(VerifyError::InvalidProofSize.into());
    }

    let expected_size = WITHDRAW_NR_INPUTS * 32;
    if public_inputs.len() < expected_size {
        msg!("Invalid public inputs size: {} < {}", public_inputs.len(), expected_size);
        return Err(VerifyError::InvalidPublicInputsSize.into());
    }

    msg!("WithdrawPrivate proof ACCEPTED (mock mode)");
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_proof_valid_size() {
        let proof = [0u8; 256];
        let result = parse_proof(&proof);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_proof_invalid_size() {
        let proof = [0u8; 128];
        let result = parse_proof(&proof);
        assert!(matches!(result, Err(VerifyError::InvalidProofSize)));
    }
}
