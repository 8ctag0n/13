//! Common validation functions for job operations
//!
//! These validators are used by all generators to ensure consistent
//! validation of job creation, claiming, and cancellation.

use solana_program::{
    account_info::AccountInfo, clock::Clock, program_error::ProgramError, pubkey::Pubkey,
    sysvar::Sysvar,
};

use crate::JobCommon;

/// Minimum price for a job (in lamports) - 0.001 SOL
pub const MIN_JOB_PRICE: u64 = 1_000_000;

/// Maximum price for a job (in lamports) - 1000 SOL
pub const MAX_JOB_PRICE: u64 = 1_000_000_000_000;

/// Minimum timeout for a job (in seconds) - 1 minute
pub const MIN_TIMEOUT_SECONDS: i64 = 60;

/// Maximum timeout for a job (in seconds) - 24 hours
pub const MAX_TIMEOUT_SECONDS: i64 = 86_400;

/// Minimum witness size (bytes)
pub const MIN_WITNESS_SIZE: u32 = 1;

/// Maximum witness size (bytes) - 1 MB
pub const MAX_WITNESS_SIZE: u32 = 1_048_576;

/// Job validation errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobValidationError {
    /// Price is below minimum
    PriceTooLow,
    /// Price is above maximum
    PriceTooHigh,
    /// Timeout is below minimum
    TimeoutTooShort,
    /// Timeout is above maximum
    TimeoutTooLong,
    /// Witness size is below minimum
    WitnessTooSmall,
    /// Witness size is above maximum
    WitnessTooLarge,
    /// Witness hash is all zeros
    InvalidWitnessHash,
    /// Job is not in correct state for operation
    InvalidJobState,
    /// Caller is not authorized
    Unauthorized,
    /// Job has expired
    JobExpired,
    /// Job has not expired yet
    JobNotExpired,
    /// Prover is not registered
    ProverNotRegistered,
    /// Prover has insufficient stake
    InsufficientStake,
}

impl From<JobValidationError> for ProgramError {
    fn from(e: JobValidationError) -> Self {
        ProgramError::Custom(e as u32 + 1000)
    }
}

/// Validate job creation parameters
///
/// # Arguments
/// * `price_lamports` - Price offered for the job
/// * `timeout_seconds` - Timeout duration in seconds
/// * `witness_hash` - Hash of the witness data
/// * `witness_size` - Size of the witness data
pub fn validate_job_creation(
    price_lamports: u64,
    timeout_seconds: i64,
    witness_hash: &[u8; 32],
    witness_size: u32,
) -> Result<(), JobValidationError> {
    // Validate price
    if price_lamports < MIN_JOB_PRICE {
        return Err(JobValidationError::PriceTooLow);
    }
    if price_lamports > MAX_JOB_PRICE {
        return Err(JobValidationError::PriceTooHigh);
    }

    // Validate timeout
    if timeout_seconds < MIN_TIMEOUT_SECONDS {
        return Err(JobValidationError::TimeoutTooShort);
    }
    if timeout_seconds > MAX_TIMEOUT_SECONDS {
        return Err(JobValidationError::TimeoutTooLong);
    }

    // Validate witness
    if witness_size < MIN_WITNESS_SIZE {
        return Err(JobValidationError::WitnessTooSmall);
    }
    if witness_size > MAX_WITNESS_SIZE {
        return Err(JobValidationError::WitnessTooLarge);
    }

    // Witness hash should not be all zeros
    if witness_hash.iter().all(|&b| b == 0) {
        return Err(JobValidationError::InvalidWitnessHash);
    }

    Ok(())
}

/// Validate that a job can be claimed
///
/// # Arguments
/// * `job` - The job to validate
/// * `prover` - The prover attempting to claim
/// * `current_time` - Current Unix timestamp
pub fn validate_claim(
    job: &JobCommon,
    _prover: &Pubkey,
    current_time: i64,
) -> Result<(), JobValidationError> {
    // Job must be pending
    if !job.can_claim() {
        return Err(JobValidationError::InvalidJobState);
    }

    // Job must not have timed out (shouldn't happen for Pending, but check anyway)
    if current_time >= job.timeout_at {
        return Err(JobValidationError::JobExpired);
    }

    Ok(())
}

/// Validate that a job can be cancelled
///
/// # Arguments
/// * `job` - The job to validate
/// * `caller` - The account attempting to cancel
pub fn validate_cancel(job: &JobCommon, caller: &Pubkey) -> Result<(), JobValidationError> {
    // Only creator can cancel
    if &job.creator != caller {
        return Err(JobValidationError::Unauthorized);
    }

    // Job must be pending
    if !job.can_cancel() {
        return Err(JobValidationError::InvalidJobState);
    }

    Ok(())
}

/// Validate that a job can receive a proof submission
///
/// # Arguments
/// * `job` - The job to validate
/// * `prover` - The prover submitting the proof
/// * `current_time` - Current Unix timestamp
pub fn validate_submit(
    job: &JobCommon,
    prover: &Pubkey,
    current_time: i64,
) -> Result<(), JobValidationError> {
    // Job must be claimed
    if job.status != zyberlink_types::JobStatus::Claimed {
        return Err(JobValidationError::InvalidJobState);
    }

    // Submitter must be the prover who claimed
    if job.prover.as_ref() != Some(prover) {
        return Err(JobValidationError::Unauthorized);
    }

    // Job must not have timed out
    if job.is_expired(current_time) {
        return Err(JobValidationError::JobExpired);
    }

    Ok(())
}

/// Validate that a job can be finalized (for timeout/failure handling)
///
/// # Arguments
/// * `job` - The job to validate
/// * `current_time` - Current Unix timestamp
pub fn validate_finalize_timeout(
    job: &JobCommon,
    current_time: i64,
) -> Result<(), JobValidationError> {
    // Job must be claimed (not pending, completed, etc.)
    if job.status != zyberlink_types::JobStatus::Claimed {
        return Err(JobValidationError::InvalidJobState);
    }

    // Job must have timed out
    if !job.is_expired(current_time) {
        return Err(JobValidationError::JobNotExpired);
    }

    Ok(())
}

/// Get current Unix timestamp from Sysvar
pub fn get_current_time() -> Result<i64, ProgramError> {
    Ok(Clock::get()?.unix_timestamp)
}

/// Verify account is a signer
pub fn verify_signer(account: &AccountInfo) -> Result<(), ProgramError> {
    if !account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    Ok(())
}

/// Verify account is writable
pub fn verify_writable(account: &AccountInfo) -> Result<(), ProgramError> {
    if !account.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

/// Verify account owner
pub fn verify_owner(account: &AccountInfo, expected_owner: &Pubkey) -> Result<(), ProgramError> {
    if account.owner != expected_owner {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_job_creation_valid() {
        let result = validate_job_creation(
            10_000_000,  // 0.01 SOL
            300,         // 5 minutes
            &[1u8; 32],  // Valid hash
            1024,        // 1 KB witness
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_job_creation_price_too_low() {
        let result = validate_job_creation(100, 300, &[1u8; 32], 1024);
        assert_eq!(result, Err(JobValidationError::PriceTooLow));
    }

    #[test]
    fn test_validate_job_creation_price_too_high() {
        let result = validate_job_creation(2_000_000_000_000, 300, &[1u8; 32], 1024);
        assert_eq!(result, Err(JobValidationError::PriceTooHigh));
    }

    #[test]
    fn test_validate_job_creation_timeout_too_short() {
        let result = validate_job_creation(10_000_000, 30, &[1u8; 32], 1024);
        assert_eq!(result, Err(JobValidationError::TimeoutTooShort));
    }

    #[test]
    fn test_validate_job_creation_timeout_too_long() {
        let result = validate_job_creation(10_000_000, 100_000, &[1u8; 32], 1024);
        assert_eq!(result, Err(JobValidationError::TimeoutTooLong));
    }

    #[test]
    fn test_validate_job_creation_witness_too_small() {
        let result = validate_job_creation(10_000_000, 300, &[1u8; 32], 0);
        assert_eq!(result, Err(JobValidationError::WitnessTooSmall));
    }

    #[test]
    fn test_validate_job_creation_witness_too_large() {
        let result = validate_job_creation(10_000_000, 300, &[1u8; 32], 2_000_000);
        assert_eq!(result, Err(JobValidationError::WitnessTooLarge));
    }

    #[test]
    fn test_validate_job_creation_invalid_hash() {
        let result = validate_job_creation(10_000_000, 300, &[0u8; 32], 1024);
        assert_eq!(result, Err(JobValidationError::InvalidWitnessHash));
    }

    #[test]
    fn test_validate_claim_valid() {
        let job = JobCommon::new(
            1,
            Pubkey::new_unique(),
            [1u8; 32],
            1024,
            10_000_000,
            Pubkey::new_unique(),
            1000,
            600,
            255,
        );

        let prover = Pubkey::new_unique();
        let result = validate_claim(&job, &prover, 1100);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_claim_already_claimed() {
        let mut job = JobCommon::new(
            1,
            Pubkey::new_unique(),
            [1u8; 32],
            1024,
            10_000_000,
            Pubkey::new_unique(),
            1000,
            600,
            255,
        );
        job.claim(Pubkey::new_unique());

        let prover = Pubkey::new_unique();
        let result = validate_claim(&job, &prover, 1100);
        assert_eq!(result, Err(JobValidationError::InvalidJobState));
    }

    #[test]
    fn test_validate_claim_expired() {
        let job = JobCommon::new(
            1,
            Pubkey::new_unique(),
            [1u8; 32],
            1024,
            10_000_000,
            Pubkey::new_unique(),
            1000,
            600,
            255,
        );

        let prover = Pubkey::new_unique();
        let result = validate_claim(&job, &prover, 2000);
        assert_eq!(result, Err(JobValidationError::JobExpired));
    }

    #[test]
    fn test_validate_cancel_valid() {
        let creator = Pubkey::new_unique();
        let job = JobCommon::new(
            1,
            creator,
            [1u8; 32],
            1024,
            10_000_000,
            Pubkey::new_unique(),
            1000,
            600,
            255,
        );

        let result = validate_cancel(&job, &creator);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_cancel_unauthorized() {
        let creator = Pubkey::new_unique();
        let other = Pubkey::new_unique();
        let job = JobCommon::new(
            1,
            creator,
            [1u8; 32],
            1024,
            10_000_000,
            Pubkey::new_unique(),
            1000,
            600,
            255,
        );

        let result = validate_cancel(&job, &other);
        assert_eq!(result, Err(JobValidationError::Unauthorized));
    }

    #[test]
    fn test_validate_cancel_not_pending() {
        let creator = Pubkey::new_unique();
        let mut job = JobCommon::new(
            1,
            creator,
            [1u8; 32],
            1024,
            10_000_000,
            Pubkey::new_unique(),
            1000,
            600,
            255,
        );
        job.claim(Pubkey::new_unique());

        let result = validate_cancel(&job, &creator);
        assert_eq!(result, Err(JobValidationError::InvalidJobState));
    }

    #[test]
    fn test_validate_submit_valid() {
        let prover = Pubkey::new_unique();
        let mut job = JobCommon::new(
            1,
            Pubkey::new_unique(),
            [1u8; 32],
            1024,
            10_000_000,
            Pubkey::new_unique(),
            1000,
            600,
            255,
        );
        job.claim(prover);

        let result = validate_submit(&job, &prover, 1100);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_submit_wrong_prover() {
        let prover = Pubkey::new_unique();
        let other = Pubkey::new_unique();
        let mut job = JobCommon::new(
            1,
            Pubkey::new_unique(),
            [1u8; 32],
            1024,
            10_000_000,
            Pubkey::new_unique(),
            1000,
            600,
            255,
        );
        job.claim(prover);

        let result = validate_submit(&job, &other, 1100);
        assert_eq!(result, Err(JobValidationError::Unauthorized));
    }

    #[test]
    fn test_validate_submit_expired() {
        let prover = Pubkey::new_unique();
        let mut job = JobCommon::new(
            1,
            Pubkey::new_unique(),
            [1u8; 32],
            1024,
            10_000_000,
            Pubkey::new_unique(),
            1000,
            600,
            255,
        );
        job.claim(prover);

        let result = validate_submit(&job, &prover, 2000);
        assert_eq!(result, Err(JobValidationError::JobExpired));
    }
}
