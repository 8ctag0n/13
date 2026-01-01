//! Process SubmitKeyShare instruction

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

use crate::{
    error::ThresholdError,
    state::{
        KeyShareRequest, KeyShareResponse, RequestStatus, KEYSHARE_RESPONSE_SEED,
        MAX_ENCRYPTED_SHARE_SIZE,
    },
};

pub fn process_submit_keyshare(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    encrypted_share: Vec<u8>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let validator_info = next_account_info(account_info_iter)?;
    let response_pda_info = next_account_info(account_info_iter)?;
    let request_info = next_account_info(account_info_iter)?;
    let validator_account_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;
    let clock_info = next_account_info(account_info_iter)?;

    // Verify validator is signer
    if !validator_info.is_signer {
        msg!("Validator must be signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Validate encrypted share size
    if !KeyShareResponse::is_valid_share_size(&encrypted_share) {
        msg!("Invalid share size: {}", encrypted_share.len());
        return Err(ThresholdError::ShareTooLarge.into());
    }

    if encrypted_share.len() > MAX_ENCRYPTED_SHARE_SIZE {
        msg!("Share too large: {} > {}", encrypted_share.len(), MAX_ENCRYPTED_SHARE_SIZE);
        return Err(ThresholdError::ShareTooLarge.into());
    }

    // Verify validator account exists and is active
    if validator_account_info.data_is_empty() {
        msg!("Validator not registered");
        return Err(ThresholdError::InvalidValidator.into());
    }

    // Read validator account to check if active
    let validator_data = validator_account_info.try_borrow_data()?;
    let validator =
        bedrock::ValidatorAccount::try_from_slice(&validator_data)
            .map_err(|_| ThresholdError::InvalidValidator)?;

    if !validator.is_active {
        msg!("Validator is not active");
        return Err(ThresholdError::ValidatorNotActive.into());
    }

    // Verify validator authority matches
    if validator.authority != *validator_info.key {
        msg!("Validator authority mismatch");
        return Err(ThresholdError::Unauthorized.into());
    }

    // Read and verify the request
    let mut request_data = request_info.try_borrow_mut_data()?;
    let mut request = KeyShareRequest::try_from_slice(&request_data)
        .map_err(|_| ThresholdError::RequestNotFound)?;

    // Get current time
    let clock = Clock::from_account_info(clock_info)?;
    let current_time = clock.unix_timestamp;

    // Check if request is expired
    if request.is_expired(current_time) {
        msg!("Request expired");
        request.mark_expired();
        request.serialize(&mut &mut request_data[..])?;
        return Err(ThresholdError::RequestExpired.into());
    }

    // Check if request can accept responses
    if !request.can_accept_responses(current_time) {
        msg!("Request cannot accept responses: {:?}", request.status);
        return Err(ThresholdError::RequestCompleted.into());
    }

    // Derive and verify response PDA
    let (response_pda, bump) = Pubkey::find_program_address(
        &[
            KEYSHARE_RESPONSE_SEED,
            request_info.key.as_ref(),
            validator_info.key.as_ref(),
        ],
        program_id,
    );

    if response_pda != *response_pda_info.key {
        msg!("Invalid response PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Check if validator already responded
    if !response_pda_info.data_is_empty() {
        msg!("Validator already responded to this request");
        return Err(ThresholdError::AlreadyResponded.into());
    }

    // Create the response account
    let response = KeyShareResponse::new(
        *request_info.key,
        *validator_info.key,
        encrypted_share.clone(),
        current_time,
        bump,
    );

    let response_size = KeyShareResponse::SIZE;
    let rent = Rent::get()?;
    let required_lamports = rent.minimum_balance(response_size);

    // Create response PDA
    invoke_signed(
        &system_instruction::create_account(
            validator_info.key,
            response_pda_info.key,
            required_lamports,
            response_size as u64,
            program_id,
        ),
        &[
            validator_info.clone(),
            response_pda_info.clone(),
            system_program_info.clone(),
        ],
        &[&[
            KEYSHARE_RESPONSE_SEED,
            request_info.key.as_ref(),
            validator_info.key.as_ref(),
            &[bump],
        ]],
    )?;

    // Write response data
    let mut response_data = response_pda_info.try_borrow_mut_data()?;
    response.serialize(&mut &mut response_data[..])?;

    // Update request: increment response counter and update status if needed
    request.record_response();
    request.serialize(&mut &mut request_data[..])?;

    msg!(
        "KeyShareSubmitted: request={}, validator={}, responses={}/{}, status={:?}",
        request_info.key,
        validator_info.key,
        request.responses_received,
        crate::state::MIN_RESPONSES_REQUIRED,
        request.status
    );

    // Update validator stats (record successful share provision)
    // Note: In a complete implementation, you'd make a CPI call to bedrock here
    // to update validator.total_shares_provided

    Ok(())
}
