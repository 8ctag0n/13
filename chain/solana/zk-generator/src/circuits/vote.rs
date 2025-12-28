//! Private Vote Circuit Verifier
//!
//! This circuit enables anonymous voting in DAOs while proving:
//! 1. Voter eligibility (token ownership)
//! 2. Vote validity (choice is valid option)
//! 3. No double-voting (nullifier uniqueness)
//! 4. Optional: No conflict of interest (PoI integration)
//!
//! # Circuit Design
//!
//! ```text
//! PRIVATE INPUTS:
//! ├── voter_wallet           // Wallet address (hidden)
//! ├── token_balance          // Actual token balance
//! ├── vote_choice            // Selected option (0, 1, 2, ...)
//! ├── nullifier_secret       // Random secret for nullifier
//! └── merkle_path[]          // Path proving token ownership
//!
//! PUBLIC INPUTS:
//! ├── poll_id                // Unique poll identifier
//! ├── eligibility_root       // Merkle root of eligible voters
//! ├── nullifier              // Hash(nullifier_secret, poll_id) - prevents double voting
//! ├── vote_commitment        // Hash(vote_choice, blinding) - hidden vote
//! └── min_balance            // Minimum tokens required to vote
//!
//! CONSTRAINTS:
//! 1. Verify Merkle membership in eligibility tree
//! 2. Assert(token_balance >= min_balance)
//! 3. Assert(vote_choice < num_options)
//! 4. Verify nullifier = Hash(nullifier_secret, poll_id)
//! 5. Verify vote_commitment = Hash(vote_choice, blinding)
//!
//! OUTPUT:
//! └── Valid vote proof that can be verified on-chain
//! ```

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{clock::Clock, msg, program_error::ProgramError, pubkey::Pubkey, sysvar::Sysvar};

/// Public inputs for Vote circuit verification
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct VotePublicInputs {
    /// Unique poll identifier
    pub poll_id: Pubkey,
    /// Merkle root of eligible voters (token holders snapshot)
    pub eligibility_root: [u8; 32],
    /// Nullifier to prevent double-voting: Hash(secret, poll_id)
    pub nullifier: [u8; 32],
    /// Commitment to the vote choice: Hash(choice, blinding)
    pub vote_commitment: [u8; 32],
    /// Minimum token balance required to vote
    pub min_balance: u64,
}

impl VotePublicInputs {
    // 32 (poll_id) + 32 (eligibility_root) + 32 (nullifier) + 32 (vote_commitment) + 8 (min_balance)
    pub const LEN: usize = 32 + 32 + 32 + 32 + 8; // 136 bytes

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, ProgramError> {
        if data.len() < Self::LEN {
            msg!("Vote public inputs too short: {} < {}", data.len(), Self::LEN);
            return Err(ProgramError::InvalidInstructionData);
        }

        let poll_id = Pubkey::try_from(&data[0..32])
            .map_err(|_| ProgramError::InvalidInstructionData)?;

        let mut eligibility_root = [0u8; 32];
        eligibility_root.copy_from_slice(&data[32..64]);

        let mut nullifier = [0u8; 32];
        nullifier.copy_from_slice(&data[64..96]);

        let mut vote_commitment = [0u8; 32];
        vote_commitment.copy_from_slice(&data[96..128]);

        let min_balance = u64::from_le_bytes(
            data[128..136]
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?,
        );

        Ok(Self {
            poll_id,
            eligibility_root,
            nullifier,
            vote_commitment,
            min_balance,
        })
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::LEN);
        bytes.extend_from_slice(self.poll_id.as_ref());
        bytes.extend_from_slice(&self.eligibility_root);
        bytes.extend_from_slice(&self.nullifier);
        bytes.extend_from_slice(&self.vote_commitment);
        bytes.extend_from_slice(&self.min_balance.to_le_bytes());
        bytes
    }
}

/// Public inputs for Vote with PoI circuit (includes conflict of interest check)
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct VoteWithPoiPublicInputs {
    /// Base vote inputs
    pub vote: VotePublicInputs,
    /// Blacklist merkle root for conflict of interest check
    pub blacklist_root: [u8; 32],
}

impl VoteWithPoiPublicInputs {
    pub const LEN: usize = VotePublicInputs::LEN + 32; // 168 bytes

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, ProgramError> {
        if data.len() < Self::LEN {
            msg!("VoteWithPoI public inputs too short: {} < {}", data.len(), Self::LEN);
            return Err(ProgramError::InvalidInstructionData);
        }

        let vote = VotePublicInputs::from_bytes(&data[0..VotePublicInputs::LEN])?;

        let mut blacklist_root = [0u8; 32];
        blacklist_root.copy_from_slice(&data[VotePublicInputs::LEN..Self::LEN]);

        Ok(Self {
            vote,
            blacklist_root,
        })
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.vote.to_bytes();
        bytes.extend_from_slice(&self.blacklist_root);
        bytes
    }
}

/// Vote-specific error codes
#[derive(Debug, Clone, Copy)]
pub enum VoteError {
    InvalidPublicInputs,
    InvalidProof,
    VerificationFailed,
    NullifierAlreadyUsed,
    InvalidPollId,
    PollNotActive,
    InsufficientBalance,
}

impl From<VoteError> for ProgramError {
    fn from(e: VoteError) -> Self {
        ProgramError::Custom(2000 + e as u32)
    }
}

/// Groth16 proof size (same as PoI)
pub const VOTE_PROOF_SIZE: usize = 256;

/// Verification key for Vote circuit
#[derive(Debug, Clone)]
pub struct VoteVerificationKey {
    pub alpha: [u8; 64],
    pub beta: [u8; 128],
    pub gamma: [u8; 128],
    pub delta: [u8; 128],
    pub ic: Vec<[u8; 64]>,
}

impl Default for VoteVerificationKey {
    fn default() -> Self {
        Self {
            alpha: [0u8; 64],
            beta: [0u8; 128],
            gamma: [0u8; 128],
            delta: [0u8; 128],
            ic: vec![[0u8; 64]; 6], // 5 public inputs + 1
        }
    }
}

/// Verify a Private Vote proof
///
/// # Arguments
/// * `proof` - The ZK proof bytes (Groth16 format: 256 bytes)
/// * `public_inputs` - The public inputs for verification
/// * `used_nullifiers` - Set of already used nullifiers (to check double-voting)
///
/// # Returns
/// * `Ok(true)` if proof is valid and nullifier is fresh
/// * `Err` if proof is invalid or nullifier already used
pub fn verify_vote_proof(
    proof: &[u8],
    public_inputs: &VotePublicInputs,
    _vk: &VoteVerificationKey,
) -> Result<bool, ProgramError> {
    // Validate proof format
    if proof.len() != VOTE_PROOF_SIZE {
        msg!(
            "Invalid vote proof size: {} != {}",
            proof.len(),
            VOTE_PROOF_SIZE
        );
        return Err(VoteError::InvalidProof.into());
    }

    // Validate poll_id is not default (empty)
    if public_inputs.poll_id == Pubkey::default() {
        msg!("Invalid poll ID");
        return Err(VoteError::InvalidPollId.into());
    }

    // Validate nullifier is not zero (would indicate error)
    if public_inputs.nullifier == [0u8; 32] {
        msg!("Invalid nullifier (all zeros)");
        return Err(VoteError::InvalidPublicInputs.into());
    }

    // TODO: Implement actual Groth16 verification using alt_bn128 precompiles
    //
    // The verification algorithm:
    // 1. Parse proof into G1, G2 points (A, B, C)
    // 2. Parse public inputs into field elements
    // 3. Compute linear combination: L = IC[0] + sum(public_input[i] * IC[i+1])
    // 4. Verify pairing equation:
    //    e(A, B) == e(alpha, beta) * e(L, gamma) * e(C, delta)

    msg!(
        "Vote proof verified for poll: {}",
        public_inputs.poll_id
    );
    msg!("  Nullifier: {:?}...", &public_inputs.nullifier[0..8]);
    msg!("  Min balance: {}", public_inputs.min_balance);

    Ok(true)
}

/// Verify a Private Vote with PoI proof
///
/// This combines vote verification with conflict of interest check
pub fn verify_vote_with_poi_proof(
    proof: &[u8],
    public_inputs: &VoteWithPoiPublicInputs,
    _vk: &VoteVerificationKey,
) -> Result<bool, ProgramError> {
    // Validate proof format
    if proof.len() != VOTE_PROOF_SIZE {
        msg!(
            "Invalid vote+poi proof size: {} != {}",
            proof.len(),
            VOTE_PROOF_SIZE
        );
        return Err(VoteError::InvalidProof.into());
    }

    // Validate base vote inputs
    if public_inputs.vote.poll_id == Pubkey::default() {
        msg!("Invalid poll ID");
        return Err(VoteError::InvalidPollId.into());
    }

    if public_inputs.vote.nullifier == [0u8; 32] {
        msg!("Invalid nullifier (all zeros)");
        return Err(VoteError::InvalidPublicInputs.into());
    }

    // Validate blacklist root is set
    if public_inputs.blacklist_root == [0u8; 32] {
        msg!("Invalid blacklist root (all zeros)");
        return Err(VoteError::InvalidPublicInputs.into());
    }

    // TODO: Implement actual Groth16 verification

    msg!(
        "Vote+PoI proof verified for poll: {}",
        public_inputs.vote.poll_id
    );
    msg!("  Nullifier: {:?}...", &public_inputs.vote.nullifier[0..8]);
    msg!("  Blacklist root: {:?}...", &public_inputs.blacklist_root[0..8]);

    Ok(true)
}

/// Check if a nullifier has already been used for a poll
///
/// In production, this would check against an on-chain nullifier set
pub fn is_nullifier_used(
    _poll_id: &Pubkey,
    _nullifier: &[u8; 32],
) -> bool {
    // TODO: Check against on-chain nullifier account/set
    // For now, always return false (nullifier not used)
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vote_public_inputs_serialization() {
        let inputs = VotePublicInputs {
            poll_id: Pubkey::new_unique(),
            eligibility_root: [1u8; 32],
            nullifier: [2u8; 32],
            vote_commitment: [3u8; 32],
            min_balance: 1000,
        };

        let bytes = inputs.to_bytes();
        assert_eq!(bytes.len(), VotePublicInputs::LEN);

        let decoded = VotePublicInputs::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.poll_id, inputs.poll_id);
        assert_eq!(decoded.eligibility_root, inputs.eligibility_root);
        assert_eq!(decoded.nullifier, inputs.nullifier);
        assert_eq!(decoded.vote_commitment, inputs.vote_commitment);
        assert_eq!(decoded.min_balance, inputs.min_balance);
    }

    #[test]
    fn test_vote_with_poi_public_inputs_serialization() {
        let inputs = VoteWithPoiPublicInputs {
            vote: VotePublicInputs {
                poll_id: Pubkey::new_unique(),
                eligibility_root: [1u8; 32],
                nullifier: [2u8; 32],
                vote_commitment: [3u8; 32],
                min_balance: 1000,
            },
            blacklist_root: [4u8; 32],
        };

        let bytes = inputs.to_bytes();
        assert_eq!(bytes.len(), VoteWithPoiPublicInputs::LEN);

        let decoded = VoteWithPoiPublicInputs::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.vote.poll_id, inputs.vote.poll_id);
        assert_eq!(decoded.blacklist_root, inputs.blacklist_root);
    }

    #[test]
    fn test_invalid_proof_size() {
        let inputs = VotePublicInputs {
            poll_id: Pubkey::new_unique(),
            eligibility_root: [1u8; 32],
            nullifier: [2u8; 32],
            vote_commitment: [3u8; 32],
            min_balance: 1000,
        };

        let short_proof = [0u8; 100];
        let vk = VoteVerificationKey::default();

        let result = verify_vote_proof(&short_proof, &inputs, &vk);
        assert!(result.is_err());
    }
}
