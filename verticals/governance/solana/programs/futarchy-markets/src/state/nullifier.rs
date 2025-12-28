//! Nullifier account state - prevents double claims
//!
//! Each nullifier is stored as a separate PDA, allowing unlimited claims
//! per market (vs the previous vec-based approach limited to 100).

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

pub const NULLIFIER_SEED: &[u8] = b"nullifier";

/// Nullifier account - represents a used claim nullifier
///
/// The existence of this PDA proves the nullifier has been used.
/// Seeds: [b"nullifier", market_id.to_le_bytes(), nullifier_hash]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct Nullifier {
    /// Market this nullifier belongs to
    pub market_id: u64,

    /// The nullifier hash (32 bytes)
    /// This is Poseidon(secret, bet_commitment) from the ZK circuit
    pub nullifier_hash: [u8; 32],

    /// User who claimed (for auditing/tracking)
    pub claimed_by: Pubkey,

    /// Timestamp when claimed
    pub claimed_at: i64,

    /// Payout amount that was claimed
    pub payout_amount: u64,

    /// Bump seed for PDA derivation
    pub bump: u8,
}

impl Nullifier {
    /// Space needed for Nullifier account
    ///
    /// Calculation:
    /// - market_id: 8
    /// - nullifier_hash: 32
    /// - claimed_by: 32
    /// - claimed_at: 8
    /// - payout_amount: 8
    /// - bump: 1
    ///
    /// Total: 89 bytes
    pub const SPACE: usize = 89;

    /// Create a new Nullifier
    pub fn new(
        market_id: u64,
        nullifier_hash: [u8; 32],
        claimed_by: Pubkey,
        claimed_at: i64,
        payout_amount: u64,
        bump: u8,
    ) -> Self {
        Self {
            market_id,
            nullifier_hash,
            claimed_by,
            claimed_at,
            payout_amount,
            bump,
        }
    }
}
