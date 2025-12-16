//! UserEscrow account state - represents a user's deposit balance for a market

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{program_error::ProgramError, pubkey::Pubkey};

pub const USER_ESCROW_SEED: &[u8] = b"user_escrow";

/// UserEscrow account - holds user's deposited funds for betting on a market
///
/// # MVP Design (Phase 1 - Transparent Escrow)
///
/// In MVP, escrow balances are transparent on-chain. This allows:
/// - Simple verification by provers
/// - Lower computational overhead
/// - Clear audit trail
///
/// Privacy is achieved through:
/// - Individual bet amounts are private (ZK commitments + threshold encryption)
/// - Pool totals are encrypted
/// - Only escrow balance is visible (but not how it's distributed across bets)
///
/// # Future (Phase 2 - Private Escrow)
///
/// Post-runway, escrow can be upgraded to threshold/FHE encryption for full privacy.
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct UserEscrow {
    /// User who owns this escrow
    pub user: Pubkey,

    /// Market this escrow is for
    pub market: Pubkey,

    /// Total amount deposited (in lamports)
    /// This is transparent in MVP
    pub deposited: u64,

    /// Amount reserved in active bets (in lamports)
    /// reserved = sum of all active bet amounts
    pub reserved: u64,

    /// Amount available for new bets (in lamports)
    /// available = deposited - reserved
    /// This is calculated, but stored for quick access
    pub available: u64,

    /// Bump seed for PDA derivation
    pub bump: u8,
}

impl UserEscrow {
    /// Space needed for UserEscrow account
    ///
    /// Calculation:
    /// - user: 32
    /// - market: 32
    /// - deposited: 8
    /// - reserved: 8
    /// - available: 8
    /// - bump: 1
    ///
    /// Total: 89 bytes
    pub const SPACE: usize = 89;

    /// Create new UserEscrow with initial deposit
    pub fn new(user: Pubkey, market: Pubkey, initial_deposit: u64, bump: u8) -> Self {
        Self {
            user,
            market,
            deposited: initial_deposit,
            reserved: 0,
            available: initial_deposit,
            bump,
        }
    }

    /// Deposit additional funds
    pub fn deposit(&mut self, amount: u64) -> Result<(), ProgramError> {
        self.deposited = self
            .deposited
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        self.available = self
            .available
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        Ok(())
    }

    /// Reserve funds for a bet
    ///
    /// Called when user places a bet. Moves funds from available → reserved.
    pub fn reserve(&mut self, amount: u64) -> Result<(), ProgramError> {
        if amount > self.available {
            return Err(ProgramError::InsufficientFunds);
        }

        self.reserved = self
            .reserved
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        self.available = self
            .available
            .checked_sub(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        Ok(())
    }

    /// Release reserved funds back to available
    ///
    /// Called when a bet is cancelled or market is cancelled (refund).
    pub fn release(&mut self, amount: u64) -> Result<(), ProgramError> {
        if amount > self.reserved {
            return Err(ProgramError::InvalidArgument);
        }

        self.reserved = self
            .reserved
            .checked_sub(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        self.available = self
            .available
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        Ok(())
    }

    /// Withdraw funds (only available, not reserved)
    ///
    /// Called by WithdrawFromEscrow instruction.
    /// Can only withdraw funds that are not locked in active bets.
    pub fn withdraw(&mut self, amount: u64) -> Result<(), ProgramError> {
        if amount > self.available {
            return Err(ProgramError::InsufficientFunds);
        }

        self.deposited = self
            .deposited
            .checked_sub(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        self.available = self
            .available
            .checked_sub(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        Ok(())
    }

    /// Transfer reserved funds to market escrow
    ///
    /// Called after reserve() when funds are physically transferred to market escrow.
    /// Decrements both deposited and reserved since funds left the escrow.
    pub fn transfer_to_market(&mut self, amount: u64) -> Result<(), ProgramError> {
        if amount > self.reserved {
            return Err(ProgramError::InvalidArgument);
        }

        self.deposited = self
            .deposited
            .checked_sub(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        self.reserved = self
            .reserved
            .checked_sub(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        Ok(())
    }

    /// Claim winning bet funds
    ///
    /// Called when user claims payout.
    /// Note: This is a no-op now since funds were already transferred via transfer_to_market().
    /// Kept for backward compatibility and semantic clarity.
    pub fn claim(&mut self, _bet_amount: u64) -> Result<(), ProgramError> {
        // Funds were already transferred to market escrow via transfer_to_market()
        // Nothing to do here
        Ok(())
    }

    /// Check if user has sufficient available funds
    pub fn has_available(&self, amount: u64) -> bool {
        self.available >= amount
    }

    /// Get total balance (deposited - reserved = available)
    pub fn verify_invariant(&self) -> bool {
        self.deposited >= self.reserved && self.available == self.deposited - self.reserved
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_escrow() {
        let user = Pubkey::new_unique();
        let market = Pubkey::new_unique();
        let escrow = UserEscrow::new(user, market, 1_000_000_000, 255);

        assert_eq!(escrow.deposited, 1_000_000_000);
        assert_eq!(escrow.reserved, 0);
        assert_eq!(escrow.available, 1_000_000_000);
        assert!(escrow.verify_invariant());
    }

    #[test]
    fn test_deposit() {
        let mut escrow = UserEscrow::new(Pubkey::new_unique(), Pubkey::new_unique(), 1_000_000_000, 255);

        escrow.deposit(500_000_000).unwrap();

        assert_eq!(escrow.deposited, 1_500_000_000);
        assert_eq!(escrow.available, 1_500_000_000);
        assert!(escrow.verify_invariant());
    }

    #[test]
    fn test_reserve() {
        let mut escrow = UserEscrow::new(Pubkey::new_unique(), Pubkey::new_unique(), 1_000_000_000, 255);

        escrow.reserve(300_000_000).unwrap();

        assert_eq!(escrow.deposited, 1_000_000_000);
        assert_eq!(escrow.reserved, 300_000_000);
        assert_eq!(escrow.available, 700_000_000);
        assert!(escrow.verify_invariant());
    }

    #[test]
    fn test_reserve_insufficient() {
        let mut escrow = UserEscrow::new(Pubkey::new_unique(), Pubkey::new_unique(), 1_000_000_000, 255);

        let result = escrow.reserve(1_500_000_000);

        assert!(result.is_err());
        assert!(escrow.verify_invariant());
    }

    #[test]
    fn test_release() {
        let mut escrow = UserEscrow::new(Pubkey::new_unique(), Pubkey::new_unique(), 1_000_000_000, 255);
        escrow.reserve(300_000_000).unwrap();

        escrow.release(100_000_000).unwrap();

        assert_eq!(escrow.reserved, 200_000_000);
        assert_eq!(escrow.available, 800_000_000);
        assert!(escrow.verify_invariant());
    }

    #[test]
    fn test_withdraw() {
        let mut escrow = UserEscrow::new(Pubkey::new_unique(), Pubkey::new_unique(), 1_000_000_000, 255);
        escrow.reserve(300_000_000).unwrap();

        // Can only withdraw available
        escrow.withdraw(500_000_000).unwrap();

        assert_eq!(escrow.deposited, 500_000_000);
        assert_eq!(escrow.reserved, 300_000_000);
        assert_eq!(escrow.available, 200_000_000);
        assert!(escrow.verify_invariant());
    }

    #[test]
    fn test_withdraw_insufficient() {
        let mut escrow = UserEscrow::new(Pubkey::new_unique(), Pubkey::new_unique(), 1_000_000_000, 255);
        escrow.reserve(300_000_000).unwrap();

        // Try to withdraw more than available
        let result = escrow.withdraw(800_000_000);

        assert!(result.is_err());
        assert!(escrow.verify_invariant());
    }

    #[test]
    fn test_claim() {
        let mut escrow = UserEscrow::new(Pubkey::new_unique(), Pubkey::new_unique(), 1_000_000_000, 255);

        // Reserve funds for bet
        escrow.reserve(300_000_000).unwrap();
        assert_eq!(escrow.deposited, 1_000_000_000);
        assert_eq!(escrow.reserved, 300_000_000);
        assert_eq!(escrow.available, 700_000_000);

        // Transfer to market escrow (simulates physical lamport transfer)
        escrow.transfer_to_market(300_000_000).unwrap();
        assert_eq!(escrow.deposited, 700_000_000);  // Decremented
        assert_eq!(escrow.reserved, 0);  // Decremented
        assert_eq!(escrow.available, 700_000_000);  // Unchanged

        // Claim is now a no-op (funds already transferred)
        escrow.claim(300_000_000).unwrap();
        assert_eq!(escrow.deposited, 700_000_000);
        assert_eq!(escrow.reserved, 0);
        assert_eq!(escrow.available, 700_000_000);

        assert!(escrow.verify_invariant());
    }

    #[test]
    fn test_full_lifecycle() {
        let mut escrow = UserEscrow::new(Pubkey::new_unique(), Pubkey::new_unique(), 5_000_000_000, 255);

        // Place bet 1: reserve + transfer to market escrow
        escrow.reserve(1_500_000_000).unwrap();
        assert_eq!(escrow.deposited, 5_000_000_000);
        assert_eq!(escrow.reserved, 1_500_000_000);
        assert_eq!(escrow.available, 3_500_000_000);

        escrow.transfer_to_market(1_500_000_000).unwrap();
        assert_eq!(escrow.deposited, 3_500_000_000);  // Decremented
        assert_eq!(escrow.reserved, 0);
        assert_eq!(escrow.available, 3_500_000_000);

        // Place bet 2: reserve + transfer
        escrow.reserve(1_000_000_000).unwrap();
        assert_eq!(escrow.deposited, 3_500_000_000);
        assert_eq!(escrow.reserved, 1_000_000_000);
        assert_eq!(escrow.available, 2_500_000_000);

        escrow.transfer_to_market(1_000_000_000).unwrap();
        assert_eq!(escrow.deposited, 2_500_000_000);  // Decremented again
        assert_eq!(escrow.reserved, 0);
        assert_eq!(escrow.available, 2_500_000_000);

        // Claim bet 1 (no-op, funds already transferred)
        escrow.claim(1_500_000_000).unwrap();
        assert_eq!(escrow.deposited, 2_500_000_000);  // Unchanged
        assert_eq!(escrow.reserved, 0);
        assert_eq!(escrow.available, 2_500_000_000);

        // Withdraw available funds
        escrow.withdraw(1_500_000_000).unwrap();
        assert_eq!(escrow.deposited, 1_000_000_000);
        assert_eq!(escrow.available, 1_000_000_000);

        assert!(escrow.verify_invariant());
    }
}
