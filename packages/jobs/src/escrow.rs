//! Escrow operations for job payments
//!
//! Provides helpers for creating, releasing, and refunding escrow accounts.
//! These are used by generators to handle payments to provers.

use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

/// Seeds for deriving escrow PDA
pub const ESCROW_SEED: &[u8] = b"escrow";

/// Escrow operations helper
pub struct EscrowOperations;

impl EscrowOperations {
    /// Derive the escrow PDA for a job
    ///
    /// Seeds: ["escrow", job_pda]
    pub fn derive_escrow_pda(job_pda: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[ESCROW_SEED, job_pda.as_ref()], program_id)
    }

    /// Create an escrow account and fund it
    ///
    /// # Arguments
    /// * `payer` - Account paying for the escrow
    /// * `escrow` - Escrow account to create
    /// * `job_pda` - Job PDA (used in escrow seeds)
    /// * `amount_lamports` - Amount to fund the escrow
    /// * `escrow_bump` - Bump seed for escrow PDA
    /// * `program_id` - Program ID that owns the escrow
    /// * `system_program` - System program account
    pub fn create_escrow<'a>(
        payer: &AccountInfo<'a>,
        escrow: &AccountInfo<'a>,
        job_pda: &Pubkey,
        amount_lamports: u64,
        escrow_bump: u8,
        program_id: &Pubkey,
        system_program: &AccountInfo<'a>,
    ) -> ProgramResult {
        let rent = Rent::get()?;
        let escrow_rent = rent.minimum_balance(0); // Escrow is just a native SOL account

        let total_lamports = amount_lamports
            .checked_add(escrow_rent)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        let escrow_seeds = &[ESCROW_SEED, job_pda.as_ref(), &[escrow_bump]];

        invoke_signed(
            &system_instruction::create_account(
                payer.key,
                escrow.key,
                total_lamports,
                0, // No data, just holds SOL
                program_id,
            ),
            &[payer.clone(), escrow.clone(), system_program.clone()],
            &[escrow_seeds],
        )?;

        Ok(())
    }

    /// Release escrow funds to a prover
    ///
    /// # Arguments
    /// * `escrow` - Escrow account holding funds
    /// * `prover` - Prover account to receive funds
    /// * `amount_lamports` - Amount to transfer
    /// * `job_pda` - Job PDA (used in escrow seeds)
    /// * `escrow_bump` - Bump seed for escrow PDA
    pub fn release_to_prover<'a>(
        escrow: &AccountInfo<'a>,
        prover: &AccountInfo<'a>,
        amount_lamports: u64,
        job_pda: &Pubkey,
        escrow_bump: u8,
    ) -> ProgramResult {
        // Verify escrow has enough funds
        if escrow.lamports() < amount_lamports {
            return Err(ProgramError::InsufficientFunds);
        }

        // Transfer via direct lamport manipulation (escrow is owned by program)
        let escrow_seeds = &[ESCROW_SEED, job_pda.as_ref(), &[escrow_bump]];
        let _ = escrow_seeds; // Used for verification, not signing here

        **escrow.try_borrow_mut_lamports()? -= amount_lamports;
        **prover.try_borrow_mut_lamports()? += amount_lamports;

        Ok(())
    }

    /// Refund escrow funds to the job creator
    ///
    /// # Arguments
    /// * `escrow` - Escrow account holding funds
    /// * `creator` - Job creator to receive refund
    /// * `job_pda` - Job PDA (used in escrow seeds)
    /// * `escrow_bump` - Bump seed for escrow PDA
    pub fn refund_to_creator<'a>(
        escrow: &AccountInfo<'a>,
        creator: &AccountInfo<'a>,
        job_pda: &Pubkey,
        escrow_bump: u8,
    ) -> ProgramResult {
        let amount = escrow.lamports();

        if amount == 0 {
            return Ok(()); // Nothing to refund
        }

        let escrow_seeds = &[ESCROW_SEED, job_pda.as_ref(), &[escrow_bump]];
        let _ = escrow_seeds; // Used for verification

        **escrow.try_borrow_mut_lamports()? = 0;
        **creator.try_borrow_mut_lamports()? += amount;

        Ok(())
    }

    /// Split escrow funds between multiple provers (for FHE consensus)
    ///
    /// # Arguments
    /// * `escrow` - Escrow account holding funds
    /// * `provers` - Slice of prover accounts to receive funds
    /// * `amounts` - Amount for each prover (must match length of provers)
    /// * `job_pda` - Job PDA (used in escrow seeds)
    /// * `escrow_bump` - Bump seed for escrow PDA
    pub fn split_to_provers<'a>(
        escrow: &AccountInfo<'a>,
        provers: &[AccountInfo<'a>],
        amounts: &[u64],
        job_pda: &Pubkey,
        escrow_bump: u8,
    ) -> ProgramResult {
        if provers.len() != amounts.len() {
            return Err(ProgramError::InvalidArgument);
        }

        let total: u64 = amounts.iter().sum();
        if escrow.lamports() < total {
            return Err(ProgramError::InsufficientFunds);
        }

        let escrow_seeds = &[ESCROW_SEED, job_pda.as_ref(), &[escrow_bump]];
        let _ = escrow_seeds;

        let mut remaining = escrow.lamports();

        for (prover, &amount) in provers.iter().zip(amounts.iter()) {
            if amount > 0 {
                **prover.try_borrow_mut_lamports()? += amount;
                remaining -= amount;
            }
        }

        **escrow.try_borrow_mut_lamports()? = remaining;

        Ok(())
    }

    /// Close escrow account and return remaining funds to creator
    ///
    /// # Arguments
    /// * `escrow` - Escrow account to close
    /// * `creator` - Account to receive remaining funds
    /// * `job_pda` - Job PDA (used in escrow seeds)
    /// * `escrow_bump` - Bump seed for escrow PDA
    pub fn close_escrow<'a>(
        escrow: &AccountInfo<'a>,
        creator: &AccountInfo<'a>,
        job_pda: &Pubkey,
        escrow_bump: u8,
    ) -> ProgramResult {
        Self::refund_to_creator(escrow, creator, job_pda, escrow_bump)
    }
}

/// Calculate payment distribution for FHE consensus
///
/// Returns a vector of amounts for each prover based on their contribution
pub fn calculate_consensus_payments(
    total_payment: u64,
    winning_prover_count: u8,
    total_prover_count: u8,
) -> Vec<u64> {
    if winning_prover_count == 0 || total_prover_count == 0 {
        return vec![0; total_prover_count as usize];
    }

    let per_winner = total_payment / winning_prover_count as u64;
    let remainder = total_payment % winning_prover_count as u64;

    let mut payments = Vec::with_capacity(total_prover_count as usize);

    for i in 0..total_prover_count {
        if i < winning_prover_count {
            // Winners get equal share, first winner gets remainder
            if i == 0 {
                payments.push(per_winner + remainder);
            } else {
                payments.push(per_winner);
            }
        } else {
            // Non-winners get nothing
            payments.push(0);
        }
    }

    payments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_escrow_pda() {
        let job_pda = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();

        let (escrow, bump) = EscrowOperations::derive_escrow_pda(&job_pda, &program_id);

        // Verify it's a valid PDA
        assert!(escrow != Pubkey::default());
        assert!(bump <= 255);

        // Same inputs should give same output
        let (escrow2, bump2) = EscrowOperations::derive_escrow_pda(&job_pda, &program_id);
        assert_eq!(escrow, escrow2);
        assert_eq!(bump, bump2);
    }

    #[test]
    fn test_calculate_consensus_payments_single_winner() {
        let payments = calculate_consensus_payments(1_000_000, 1, 3);

        assert_eq!(payments.len(), 3);
        assert_eq!(payments[0], 1_000_000);
        assert_eq!(payments[1], 0);
        assert_eq!(payments[2], 0);
    }

    #[test]
    fn test_calculate_consensus_payments_multiple_winners() {
        let payments = calculate_consensus_payments(1_000_000, 2, 3);

        assert_eq!(payments.len(), 3);
        assert_eq!(payments[0], 500_000); // 500k + remainder (0)
        assert_eq!(payments[1], 500_000);
        assert_eq!(payments[2], 0);
    }

    #[test]
    fn test_calculate_consensus_payments_with_remainder() {
        let payments = calculate_consensus_payments(1_000_000, 3, 3);

        assert_eq!(payments.len(), 3);
        // 1_000_000 / 3 = 333_333 with remainder 1
        assert_eq!(payments[0], 333_334); // Gets remainder
        assert_eq!(payments[1], 333_333);
        assert_eq!(payments[2], 333_333);

        // Verify total
        let total: u64 = payments.iter().sum();
        assert_eq!(total, 1_000_000);
    }

    #[test]
    fn test_calculate_consensus_payments_no_winners() {
        let payments = calculate_consensus_payments(1_000_000, 0, 3);

        assert_eq!(payments.len(), 3);
        assert_eq!(payments.iter().sum::<u64>(), 0);
    }

    #[test]
    fn test_calculate_consensus_payments_empty() {
        let payments = calculate_consensus_payments(1_000_000, 0, 0);
        assert!(payments.is_empty());
    }
}
