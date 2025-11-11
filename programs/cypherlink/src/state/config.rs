use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Global marketplace configuration
/// This account is initialized once and controls marketplace parameters
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct MarketplaceConfig {
    /// Authority that can update marketplace parameters
    pub authority: Pubkey,

    /// Platform fee in basis points (e.g., 1000 = 10%)
    pub fee_basis_points: u16,

    /// Minimum stake amount required for provers (in lamports)
    pub min_stake_amount: u64,

    /// Minimum reputation score required to claim jobs (0-1000)
    pub min_reputation_score: u32,

    /// Default job timeout in seconds
    pub default_job_timeout_seconds: i64,

    /// Recipient of protocol fees
    pub protocol_fee_recipient: Pubkey,

    /// Counter for generating unique job IDs
    pub next_job_id: u64,

    /// Total number of registered provers
    pub total_provers: u64,

    /// Total number of jobs created
    pub total_jobs_created: u64,

    /// Total number of jobs completed
    pub total_jobs_completed: u64,

    /// Whether the marketplace is currently paused (emergency stop)
    pub is_paused: bool,

    /// Bump seed for PDA derivation
    pub bump: u8,
}

impl MarketplaceConfig {
    pub const LEN: usize = 32   // authority
        + 2                      // fee_basis_points
        + 8                      // min_stake_amount
        + 4                      // min_reputation_score
        + 8                      // default_job_timeout_seconds
        + 32                     // protocol_fee_recipient
        + 8                      // next_job_id
        + 8                      // total_provers
        + 8                      // total_jobs_created
        + 8                      // total_jobs_completed
        + 1                      // is_paused
        + 1;                     // bump

    /// Create a new marketplace configuration with default values
    pub fn new(authority: Pubkey, protocol_fee_recipient: Pubkey, bump: u8) -> Self {
        Self {
            authority,
            fee_basis_points: 1000,        // 10% platform fee
            min_stake_amount: 5_000_000_000, // 5 SOL minimum stake
            min_reputation_score: 500,     // Minimum 500/1000 reputation
            default_job_timeout_seconds: 600, // 10 minutes default
            protocol_fee_recipient,
            next_job_id: 0,
            total_provers: 0,
            total_jobs_created: 0,
            total_jobs_completed: 0,
            is_paused: false,
            bump,
        }
    }

    /// Calculate platform fee for a given price
    pub fn calculate_platform_fee(&self, price_lamports: u64) -> u64 {
        (price_lamports * self.fee_basis_points as u64) / 10_000
    }

    /// Calculate prover payout (price minus platform fee)
    pub fn calculate_prover_payout(&self, price_lamports: u64) -> u64 {
        price_lamports - self.calculate_platform_fee(price_lamports)
    }

    /// Increment job counter and return new job ID
    pub fn next_job_id(&mut self) -> u64 {
        let id = self.next_job_id;
        self.next_job_id = self.next_job_id.saturating_add(1);
        self.total_jobs_created = self.total_jobs_created.saturating_add(1);
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_marketplace_config_len() {
        let config = MarketplaceConfig::new(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            255,
        );
        let serialized = borsh::to_vec(&config).unwrap();
        assert_eq!(serialized.len(), MarketplaceConfig::LEN);
    }

    #[test]
    fn test_fee_calculation() {
        let config = MarketplaceConfig::new(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            255,
        );

        // 10% fee
        assert_eq!(config.calculate_platform_fee(1_000_000), 100_000);
        assert_eq!(config.calculate_prover_payout(1_000_000), 900_000);
    }

    #[test]
    fn test_job_id_increment() {
        let mut config = MarketplaceConfig::new(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            255,
        );

        assert_eq!(config.next_job_id(), 0);
        assert_eq!(config.next_job_id(), 1);
        assert_eq!(config.next_job_id(), 2);
        assert_eq!(config.total_jobs_created, 3);
    }
}
