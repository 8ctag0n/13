//! Vault PDAs - Only hold lamports, no data
//!
//! ProtocolVault: Global vault for all deposited funds
//! MarketVault: Per-market vault for active bets

use solana_program::pubkey::Pubkey;

pub const PROTOCOL_VAULT_SEED: &[u8] = b"protocol_vault";
pub const MARKET_VAULT_SEED: &[u8] = b"market_vault";

/// ProtocolVault - Global vault for all user deposits
/// PDA: ["protocol_vault"]
/// Data: 0 bytes (only lamports)
pub struct ProtocolVault;

impl ProtocolVault {
    pub fn seeds() -> [&'static [u8]; 1] {
        [PROTOCOL_VAULT_SEED]
    }

    pub fn seeds_with_bump(bump: &[u8]) -> [&[u8]; 2] {
        [PROTOCOL_VAULT_SEED, bump]
    }

    pub fn find_pda(program_id: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[PROTOCOL_VAULT_SEED], program_id)
    }
}

/// MarketVault - Per-market vault for active bets
/// PDA: ["market_vault", market_id]
/// Data: 0 bytes (only lamports)
pub struct MarketVault;

impl MarketVault {
    pub fn seeds(market_id: &[u8; 8]) -> [&[u8]; 2] {
        [MARKET_VAULT_SEED, market_id]
    }

    pub fn seeds_with_bump<'a>(market_id: &'a [u8; 8], bump: &'a [u8]) -> [&'a [u8]; 3] {
        [MARKET_VAULT_SEED, market_id, bump]
    }

    pub fn find_pda(market_id: u64, program_id: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[MARKET_VAULT_SEED, &market_id.to_le_bytes()],
            program_id,
        )
    }
}
