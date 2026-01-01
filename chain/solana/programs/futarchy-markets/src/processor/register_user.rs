//! RegisterUser instruction processor

use borsh::{BorshDeserialize, BorshSerialize};
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
    cpi::verify_poi_proof,
    state::{UserEligibility, USER_ELIGIBILITY_SEED},
};

/// Process RegisterUser instruction
///
/// Registers a user with Proof of Innocence verification
///
/// Accounts expected:
/// 0. `[writable, signer]` User registering
/// 1. `[writable]` UserEligibility account (PDA)
/// 2. `[]` ZK-generator program
/// 3. `[]` System program
/// 4. `[]` Clock sysvar
pub fn process_register_user(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    proof: Vec<u8>,
    public_inputs: Vec<u8>,
    blacklist_root: [u8; 32],
    blacklist_version: u32,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let user_info = next_account_info(account_info_iter)?;
    let user_eligibility_info = next_account_info(account_info_iter)?;
    let zk_program_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;
    let clock_info = next_account_info(account_info_iter)?;

    // Verify user is signer
    if !user_info.is_signer {
        msg!("User must be signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Get current time
    let clock = Clock::from_account_info(clock_info)?;
    let current_time = clock.unix_timestamp;

    msg!("Verifying Proof of Innocence (Circuit 10)");

    // Verify PoI proof via CPI to zk-generator
    verify_poi_proof(zk_program_info, &proof, &public_inputs)?;

    msg!("PoI proof verified successfully");

    // Derive UserEligibility PDA
    let eligibility_seeds = &[USER_ELIGIBILITY_SEED, user_info.key.as_ref()];
    let (eligibility_pda, eligibility_bump) =
        Pubkey::find_program_address(eligibility_seeds, program_id);

    if eligibility_pda != *user_eligibility_info.key {
        msg!("Invalid UserEligibility PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Calculate rent
    let rent = Rent::get()?;
    let eligibility_space = UserEligibility::SPACE;
    let eligibility_lamports = rent.minimum_balance(eligibility_space);

    msg!("Creating UserEligibility account");

    // Create UserEligibility account
    let eligibility_seeds_with_bump = &[
        USER_ELIGIBILITY_SEED,
        user_info.key.as_ref(),
        &[eligibility_bump],
    ];

    invoke_signed(
        &system_instruction::create_account(
            user_info.key,
            user_eligibility_info.key,
            eligibility_lamports,
            eligibility_space as u64,
            program_id,
        ),
        &[
            user_info.clone(),
            user_eligibility_info.clone(),
            system_program_info.clone(),
        ],
        &[eligibility_seeds_with_bump],
    )?;

    // Calculate expiry time (24 hours from now)
    let expires_at = current_time
        .checked_add(UserEligibility::VALIDITY_WINDOW)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    // Initialize UserEligibility state
    // Note: poi_job_id would come from zk-generator in production
    // For MVP, we use a placeholder (0)
    let user_eligibility = UserEligibility {
        user: *user_info.key,
        poi_job_id: 0, // TODO: Get from zk-generator job ID
        blacklist_root,
        registered_at: current_time,
        expires_at,
        is_active: true,
        blacklist_version,
        bump: eligibility_bump,
    };

    // Serialize and write
    let mut eligibility_data = user_eligibility_info.try_borrow_mut_data()?;
    user_eligibility.serialize(&mut &mut eligibility_data[..])?;

    msg!("User registered successfully");
    msg!("  User: {}", user_info.key);
    msg!("  Blacklist version: {}", blacklist_version);
    msg!("  Valid until: {}", expires_at);
    msg!(
        "  Blacklist root: {:?}",
        &blacklist_root[..8]
    );

    Ok(())
}
