//! PrivateBalance - FHE + ZK encrypted user balance
//!
//! Dual representation:
//! - encrypted_balance: FHE ciphertext for homomorphic operations
//! - balance_commitment: Poseidon hash for ZK proof verification

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

pub const PRIVATE_BALANCE_SEED: &[u8] = b"private_balance";
pub const MAX_FHE_CIPHERTEXT_SIZE: usize = 512;

/// PrivateBalance account
/// Seeds: ["private_balance", user]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct PrivateBalance {
    /// User pubkey - owner of this balance
    pub user: Pubkey,

    /// FHE encrypted balance (TFHE ciphertext)
    /// Updated by FHE provers through consensus
    pub encrypted_balance: Vec<u8>,

    /// Poseidon commitment: Poseidon(balance, nonce)
    /// Used in ZK proofs to verify balance constraints
    pub balance_commitment: [u8; 32],

    /// Nonce for commitment freshness and replay protection
    /// Incremented on each operation
    pub nonce: u64,

    /// Bump seed for PDA derivation
    pub bump: u8,
}

impl PrivateBalance {
    /// Space: 32 + 4 + 512 + 32 + 8 + 1 = 589 bytes
    pub const SPACE: usize = 32 + 4 + MAX_FHE_CIPHERTEXT_SIZE + 32 + 8 + 1;

    pub fn new(
        user: Pubkey,
        encrypted_balance: Vec<u8>,
        balance_commitment: [u8; 32],
        bump: u8,
    ) -> Self {
        Self {
            user,
            encrypted_balance,
            balance_commitment,
            nonce: 0,
            bump,
        }
    }

    pub fn increment_nonce(&mut self) {
        self.nonce = self.nonce.saturating_add(1);
    }

    pub fn update_commitment(&mut self, new_commitment: [u8; 32]) {
        self.balance_commitment = new_commitment;
        self.increment_nonce();
    }

    pub fn update_encrypted_balance(&mut self, new_encrypted: Vec<u8>) {
        self.encrypted_balance = new_encrypted;
    }

    pub fn seeds_with_bump<'a>(user: &'a Pubkey, bump: &'a [u8]) -> [&'a [u8]; 3] {
        [PRIVATE_BALANCE_SEED, user.as_ref(), bump]
    }

    pub fn validate_encrypted_size(&self) -> bool {
        self.encrypted_balance.len() <= MAX_FHE_CIPHERTEXT_SIZE
    }
}
