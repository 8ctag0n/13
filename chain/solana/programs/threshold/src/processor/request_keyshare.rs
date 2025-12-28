//! Process RequestKeyShare instruction

use borsh::{BorshDeserialize, BorshSerialize as _};
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
use zyberlink_types::JobStatus;

use crate::{
    error::ThresholdError,
    state::{KeyShareRequest, KEYSHARE_REQUEST_SEED, MAX_CID_LENGTH},
};

pub fn process_request_keyshare(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    encrypted_witness_cid: String,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let prover_info = next_account_info(account_info_iter)?;
    let request_pda_info = next_account_info(account_info_iter)?;
    let job_info = next_account_info(account_info_iter)?;
    let prover_account_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;

    // Verify prover is signer
    if !prover_info.is_signer {
        msg!("Prover must be signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Validate CID length
    if encrypted_witness_cid.is_empty() || encrypted_witness_cid.len() > MAX_CID_LENGTH {
        msg!("Invalid CID length: {}", encrypted_witness_cid.len());
        return Err(ThresholdError::InvalidCid.into());
    }

    // Read and verify the ZK job
    let job_data = job_info.try_borrow_data()?;

    // Deserialize JobCommon (first 200 bytes as per JobCommon::SIZE)
    let job_common = zyberlink_jobs::JobCommon::deserialize(&mut &job_data[..200])
        .map_err(|_| ThresholdError::InvalidJob)?;

    // Verify job is claimed
    if job_common.status != JobStatus::Claimed {
        msg!("Job is not claimed");
        return Err(ThresholdError::JobNotClaimed.into());
    }

    // Verify prover claimed this job
    if job_common.prover != Some(*prover_info.key) {
        msg!("Job not claimed by this prover");
        return Err(ThresholdError::Unauthorized.into());
    }

    // Verify prover account exists and belongs to bedrock
    if prover_account_info.data_is_empty() {
        msg!("Prover not registered");
        return Err(ThresholdError::InvalidProver.into());
    }

    // Get current time
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;

    // Derive and verify request PDA
    let (request_pda, bump) = Pubkey::find_program_address(
        &[KEYSHARE_REQUEST_SEED, job_info.key.as_ref()],
        program_id,
    );

    if request_pda != *request_pda_info.key {
        msg!("Invalid request PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Check if request already exists
    if !request_pda_info.data_is_empty() {
        msg!("Key share request already exists for this job");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    // Create the request account
    let request = KeyShareRequest::new(
        *job_info.key,
        *prover_info.key,
        encrypted_witness_cid.clone(),
        current_time,
        bump,
    );

    let request_size = KeyShareRequest::SIZE;
    let rent = Rent::get()?;
    let required_lamports = rent.minimum_balance(request_size);

    // Create request PDA
    invoke_signed(
        &system_instruction::create_account(
            prover_info.key,
            request_pda_info.key,
            required_lamports,
            request_size as u64,
            program_id,
        ),
        &[
            prover_info.clone(),
            request_pda_info.clone(),
            system_program_info.clone(),
        ],
        &[&[KEYSHARE_REQUEST_SEED, job_info.key.as_ref(), &[bump]]],
    )?;

    // Write request data
    let mut request_data = request_pda_info.try_borrow_mut_data()?;
    request.serialize(&mut &mut request_data[..])?;

    msg!(
        "KeyShareRequested: job={}, prover={}, cid={}",
        job_info.key,
        prover_info.key,
        encrypted_witness_cid
    );

    Ok(())
}
