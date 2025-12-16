//! Position account state - represents a user's bet

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

pub const POSITION_SEED: &[u8] = b"position";

/// Position account - represents a user's bet on a market
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct Position {
    /// User who placed the bet
    pub user: Pubkey,

    /// Market this position belongs to
    pub market: Pubkey,

    /// Bet commitment: Poseidon(amount, position, blinding)
    /// This is the public commitment that was verified by ZK proof
    pub bet_commitment: [u8; 32],

    /// Timestamp when bet was placed
    pub placed_at: i64,

    /// Whether this position has been claimed (for tracking)
    pub claimed: bool,

    /// Bump seed for PDA derivation
    pub bump: u8,
}

impl Position {
    /// Space needed for Position account
    ///
    /// Calculation:
    /// - user: 32
    /// - market: 32
    /// - bet_commitment: 32
    /// - placed_at: 8
    /// - claimed: 1
    /// - bump: 1
    ///
    /// Total: 106 bytes
    pub const SPACE: usize = 106;

    /// Check if position has been claimed
    pub fn is_claimed(&self) -> bool {
        self.claimed
    }

    /// Mark position as claimed
    pub fn mark_claimed(&mut self) {
        self.claimed = true;
    }
}
