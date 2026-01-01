//! Key share response state

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Seeds for key share response PDA
pub const KEYSHARE_RESPONSE_SEED: &[u8] = b"keyshare_response";

/// Maximum size for encrypted key share (256 bytes)
pub const MAX_ENCRYPTED_SHARE_SIZE: usize = 256;

/// Key share response account
///
/// Created when a validator submits their encrypted key share for a request.
/// Each validator can only submit one response per request.
///
/// PDA: ["keyshare_response", request_id, validator]
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct KeyShareResponse {
    /// Key share request this response is for
    pub request: Pubkey,

    /// Validator providing the key share
    pub validator: Pubkey,

    /// Encrypted key share (encrypted for the prover's public key)
    /// The prover will decrypt this off-chain with their private key
    pub encrypted_share: Vec<u8>,

    /// Timestamp when response was submitted
    pub submitted_at: i64,

    /// Bump seed for PDA
    pub bump: u8,
}

impl KeyShareResponse {
    /// Size: 32 + 32 + (4 + 256) + 8 + 1 = 333 bytes
    /// Allocate 400 for safety
    pub const SIZE: usize = 400;

    pub fn new(
        request: Pubkey,
        validator: Pubkey,
        encrypted_share: Vec<u8>,
        submitted_at: i64,
        bump: u8,
    ) -> Self {
        Self {
            request,
            validator,
            encrypted_share,
            submitted_at,
            bump,
        }
    }

    /// Validate encrypted share size
    pub fn is_valid_share_size(share: &[u8]) -> bool {
        !share.is_empty() && share.len() <= MAX_ENCRYPTED_SHARE_SIZE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyshare_response_creation() {
        let request = Pubkey::new_unique();
        let validator = Pubkey::new_unique();
        let share = vec![1, 2, 3, 4, 5];
        let timestamp = 1000;

        let response = KeyShareResponse::new(
            request,
            validator,
            share.clone(),
            timestamp,
            255,
        );

        assert_eq!(response.request, request);
        assert_eq!(response.validator, validator);
        assert_eq!(response.encrypted_share, share);
        assert_eq!(response.submitted_at, timestamp);
        assert_eq!(response.bump, 255);
    }

    #[test]
    fn test_share_size_validation() {
        assert!(!KeyShareResponse::is_valid_share_size(&[]));
        assert!(KeyShareResponse::is_valid_share_size(&[1]));
        assert!(KeyShareResponse::is_valid_share_size(&vec![0; 256]));
        assert!(!KeyShareResponse::is_valid_share_size(&vec![0; 257]));
    }
}
