//! PDA derivation functions for zk-generator program

use solana_sdk::pubkey::Pubkey;

/// Seed for ZK job PDAs
pub const ZK_JOB_SEED: &[u8] = b"zk_job";

/// Seed for escrow PDAs
pub const ESCROW_SEED: &[u8] = b"zk_escrow";

/// Derive the PDA for a ZK job
///
/// # Arguments
/// * `program_id` - The zk-generator program ID
/// * `creator` - The job creator's pubkey
/// * `job_id` - The job ID (u64)
///
/// # Returns
/// * `(Pubkey, u8)` - The derived PDA and bump seed
///
/// # Example
/// ```no_run
/// use zk_generator_sdk::derive_job_pda;
/// use solana_sdk::pubkey::Pubkey;
///
/// let program_id = Pubkey::default();
/// let creator = Pubkey::default();
/// let job_id = 12345u64;
/// let (job_pda, bump) = derive_job_pda(&program_id, &creator, job_id);
/// ```
pub fn derive_job_pda(program_id: &Pubkey, creator: &Pubkey, job_id: u64) -> (Pubkey, u8) {
    let job_id_bytes = job_id.to_le_bytes();
    Pubkey::find_program_address(
        &[ZK_JOB_SEED, creator.as_ref(), &job_id_bytes],
        program_id,
    )
}

/// Derive the PDA for a job's escrow account
///
/// # Arguments
/// * `program_id` - The zk-generator program ID
/// * `job_pda` - The job PDA
///
/// # Returns
/// * `(Pubkey, u8)` - The derived PDA and bump seed
///
/// # Example
/// ```no_run
/// use zk_generator_sdk::derive_escrow_pda;
/// use solana_sdk::pubkey::Pubkey;
///
/// let program_id = Pubkey::default();
/// let job_pda = Pubkey::default();
/// let (escrow_pda, bump) = derive_escrow_pda(&program_id, &job_pda);
/// ```
pub fn derive_escrow_pda(program_id: &Pubkey, job_pda: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[ESCROW_SEED, job_pda.as_ref()], program_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_job_pda() {
        let program_id = Pubkey::new_unique();
        let creator = Pubkey::new_unique();
        let job_id = 12345u64;

        let (pda1, bump1) = derive_job_pda(&program_id, &creator, job_id);
        let (pda2, bump2) = derive_job_pda(&program_id, &creator, job_id);

        // Deterministic
        assert_eq!(pda1, pda2);
        assert_eq!(bump1, bump2);

        // Different job_id produces different PDA
        let (pda3, _) = derive_job_pda(&program_id, &creator, job_id + 1);
        assert_ne!(pda1, pda3);
    }

    #[test]
    fn test_derive_escrow_pda() {
        let program_id = Pubkey::new_unique();
        let job_pda = Pubkey::new_unique();

        let (pda1, bump1) = derive_escrow_pda(&program_id, &job_pda);
        let (pda2, bump2) = derive_escrow_pda(&program_id, &job_pda);

        // Deterministic
        assert_eq!(pda1, pda2);
        assert_eq!(bump1, bump2);
    }
}
