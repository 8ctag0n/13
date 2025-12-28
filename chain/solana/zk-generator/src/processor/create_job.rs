//! CreateJob instruction processor

use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};
use zyberlink_jobs::{validation, JobCommon};

use crate::{
    error::ZkGeneratorError,
    state::{ZkJob, ZK_JOB_SEED, CIRCUIT_CREDENTIAL},
};

/// Seeds for escrow PDA
const ESCROW_SEED: &[u8] = b"zk_escrow";

/// Process CreateJob instruction
///
/// Accounts:
/// 0. `[signer]` Creator wallet
/// 1. `[writable]` ZkJob PDA
/// 2. `[writable]` Escrow PDA
/// 3. `[]` System program
#[allow(clippy::too_many_arguments)]
pub fn process_create_job(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    job_id: u64,
    circuit_type: u8,
    witness_hash: [u8; 32],
    witness_size: u32,
    price_lamports: u64,
    timeout_seconds: i64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let creator_info = next_account_info(account_info_iter)?;
    let job_info = next_account_info(account_info_iter)?;
    let escrow_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;

    // Verify creator is signer
    if !creator_info.is_signer {
        msg!("Creator must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Validate circuit type is ZK (0-3)
    if !ZkJob::is_valid_circuit(circuit_type) {
        msg!("Invalid ZK circuit type: {}", circuit_type);
        return Err(ZkGeneratorError::InvalidProof.into());
    }

    // Validate job creation parameters using shared library
    validation::validate_job_creation(price_lamports, timeout_seconds, &witness_hash, witness_size)
        .map_err(|_| ZkGeneratorError::InvalidInstruction)?;

    // Get current time
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;

    let job_id_bytes = job_id.to_le_bytes();

    // Derive and verify job PDA
    let (job_pda, job_bump) = Pubkey::find_program_address(
        &[ZK_JOB_SEED, creator_info.key.as_ref(), &job_id_bytes],
        program_id,
    );

    if job_info.key != &job_pda {
        msg!("Invalid job account PDA");
        return Err(ZkGeneratorError::InvalidInstruction.into());
    }

    // Derive and verify escrow PDA
    let (escrow_pda, escrow_bump) =
        Pubkey::find_program_address(&[ESCROW_SEED, job_pda.as_ref()], program_id);

    if escrow_info.key != &escrow_pda {
        msg!("Invalid escrow account PDA");
        return Err(ZkGeneratorError::InvalidInstruction.into());
    }

    // Calculate rent
    let rent = Rent::get()?;
    let job_rent = rent.minimum_balance(ZkJob::SIZE);
    let escrow_rent = rent.minimum_balance(0);

    msg!("Creating ZK job account ({} bytes)", ZkJob::SIZE);

    // Create job account
    invoke_signed(
        &system_instruction::create_account(
            creator_info.key,
            job_info.key,
            job_rent,
            ZkJob::SIZE as u64,
            program_id,
        ),
        &[
            creator_info.clone(),
            job_info.clone(),
            system_program_info.clone(),
        ],
        &[&[ZK_JOB_SEED, creator_info.key.as_ref(), &job_id_bytes, &[job_bump]]],
    )?;

    msg!("Creating escrow account with {} lamports", price_lamports);

    // Create escrow account with payment
    invoke_signed(
        &system_instruction::create_account(
            creator_info.key,
            escrow_info.key,
            escrow_rent + price_lamports,
            0,
            program_id,
        ),
        &[
            creator_info.clone(),
            escrow_info.clone(),
            system_program_info.clone(),
        ],
        &[&[ESCROW_SEED, job_pda.as_ref(), &[escrow_bump]]],
    )?;

    // Create JobCommon
    let common = JobCommon::new(
        job_id,
        *creator_info.key,
        witness_hash,
        witness_size,
        price_lamports,
        escrow_pda,
        current_time,
        timeout_seconds,
        job_bump,
    );

    // Create ZkJob
    let zk_job = ZkJob {
        common,
        circuit_type,
    };

    // Serialize to account
    let mut job_data = job_info.try_borrow_mut_data()?;
    zk_job.serialize(&mut &mut job_data[..])?;

    msg!("ZK job created successfully");
    msg!("  Job ID: {}", job_id);
    msg!("  Circuit Type: {}", circuit_type);
    msg!("  Price: {} lamports", price_lamports);
    msg!("  Timeout: {} seconds", timeout_seconds);

    Ok(())
}
