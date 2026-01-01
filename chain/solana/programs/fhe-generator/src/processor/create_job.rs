//! CreateJob instruction processor for FHE jobs

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
    error::FheGeneratorError,
    state::{FheConsensusData, FheJob, FHE_CONSENSUS_SEED, FHE_JOB_SEED},
};

/// Seeds for escrow PDA
const ESCROW_SEED: &[u8] = b"fhe_escrow";

/// Process CreateJob instruction for FHE jobs
///
/// Creates both the FheJob PDA and FheConsensusData PDA for multi-prover consensus.
///
/// Accounts:
/// 0. `[signer]` Creator wallet
/// 1. `[writable]` FheJob PDA
/// 2. `[writable]` FheConsensusData PDA
/// 3. `[writable]` Escrow PDA
/// 4. `[]` System program
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
    required_provers: u8,
    consensus_threshold: u8,
    operation_param1: u16,
    operation_param2: u8,
    operation_param3: u8,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let creator_info = next_account_info(account_info_iter)?;
    let job_info = next_account_info(account_info_iter)?;
    let consensus_info = next_account_info(account_info_iter)?;
    let escrow_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;

    // Verify creator is signer
    if !creator_info.is_signer {
        msg!("Creator must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Validate circuit type is FHE (4-11)
    if !FheJob::is_valid_circuit(circuit_type) {
        msg!("Invalid FHE circuit type: {}", circuit_type);
        return Err(FheGeneratorError::InvalidOperation.into());
    }

    // Validate consensus parameters
    if required_provers == 0 || required_provers > 5 {
        msg!("Required provers must be 1-5, got: {}", required_provers);
        return Err(FheGeneratorError::InvalidInstruction.into());
    }

    if consensus_threshold == 0 || consensus_threshold > required_provers {
        msg!(
            "Consensus threshold must be 1-{}, got: {}",
            required_provers,
            consensus_threshold
        );
        return Err(FheGeneratorError::InvalidInstruction.into());
    }

    // Validate job creation parameters using shared library
    validation::validate_job_creation(price_lamports, timeout_seconds, &witness_hash, witness_size)
        .map_err(|_| FheGeneratorError::InvalidInstruction)?;

    // Get current time
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;

    let job_id_bytes = job_id.to_le_bytes();

    // Derive and verify job PDA
    let (job_pda, job_bump) = Pubkey::find_program_address(
        &[FHE_JOB_SEED, creator_info.key.as_ref(), &job_id_bytes],
        program_id,
    );

    if job_info.key != &job_pda {
        msg!("Invalid FHE job account PDA");
        return Err(FheGeneratorError::InvalidInstruction.into());
    }

    // Derive and verify consensus PDA
    let (consensus_pda, consensus_bump) =
        Pubkey::find_program_address(&[FHE_CONSENSUS_SEED, &job_id_bytes], program_id);

    if consensus_info.key != &consensus_pda {
        msg!("Invalid FHE consensus account PDA");
        return Err(FheGeneratorError::InvalidInstruction.into());
    }

    // Derive and verify escrow PDA
    let (escrow_pda, escrow_bump) =
        Pubkey::find_program_address(&[ESCROW_SEED, job_pda.as_ref()], program_id);

    if escrow_info.key != &escrow_pda {
        msg!("Invalid escrow account PDA");
        return Err(FheGeneratorError::InvalidInstruction.into());
    }

    // Calculate rent
    let rent = Rent::get()?;
    let job_rent = rent.minimum_balance(FheJob::SIZE);
    let consensus_rent = rent.minimum_balance(FheConsensusData::SIZE);
    let escrow_rent = rent.minimum_balance(0);

    msg!("Creating FHE job account ({} bytes)", FheJob::SIZE);

    // Create job account
    invoke_signed(
        &system_instruction::create_account(
            creator_info.key,
            job_info.key,
            job_rent,
            FheJob::SIZE as u64,
            program_id,
        ),
        &[
            creator_info.clone(),
            job_info.clone(),
            system_program_info.clone(),
        ],
        &[&[
            FHE_JOB_SEED,
            creator_info.key.as_ref(),
            &job_id_bytes,
            &[job_bump],
        ]],
    )?;

    msg!(
        "Creating FHE consensus account ({} bytes)",
        FheConsensusData::SIZE
    );

    // Create consensus account
    invoke_signed(
        &system_instruction::create_account(
            creator_info.key,
            consensus_info.key,
            consensus_rent,
            FheConsensusData::SIZE as u64,
            program_id,
        ),
        &[
            creator_info.clone(),
            consensus_info.clone(),
            system_program_info.clone(),
        ],
        &[&[FHE_CONSENSUS_SEED, &job_id_bytes, &[consensus_bump]]],
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

    // Create FheJob
    let fhe_job = FheJob {
        common,
        circuit_type,
        fhe_consensus_bump: consensus_bump,
    };

    // Create FheConsensusData
    let fhe_consensus = FheConsensusData::new(
        job_id,
        circuit_type,
        operation_param1,
        operation_param2,
        operation_param3,
        required_provers,
        consensus_threshold,
        current_time + timeout_seconds,
        consensus_bump,
    );

    // Serialize to accounts
    let mut job_data = job_info.try_borrow_mut_data()?;
    fhe_job.serialize(&mut &mut job_data[..])?;

    let mut consensus_data = consensus_info.try_borrow_mut_data()?;
    fhe_consensus.serialize(&mut &mut consensus_data[..])?;

    msg!("FHE job created successfully");
    msg!("  Job ID: {}", job_id);
    msg!("  Circuit Type: {}", circuit_type);
    msg!("  Required Provers: {}", required_provers);
    msg!("  Consensus Threshold: {}", consensus_threshold);
    msg!("  Price: {} lamports", price_lamports);
    msg!("  Timeout: {} seconds", timeout_seconds);

    Ok(())
}
