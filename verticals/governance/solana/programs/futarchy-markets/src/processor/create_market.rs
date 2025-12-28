//! CreateMarket instruction processor

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

use crate::{
    error::FutarchyError,
    state::{ExecutableAction, Market, MarketStatus},
};

/// Process CreateMarket instruction
///
/// Creates a new prediction market with escrow
///
/// Accounts expected:
/// 0. `[writable, signer]` Market creator/authority
/// 1. `[writable]` Market account (PDA)
/// 2. `[writable]` Escrow account (PDA)
/// 3. `[]` Oracle account
/// 4. `[]` System program
/// 5. `[]` Clock sysvar
pub fn process_create_market(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    market_id: u64,
    question_hash: [u8; 32],
    end_time: i64,
    max_bet: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let authority_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    let escrow_info = next_account_info(account_info_iter)?;
    let oracle_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;
    let clock_info = next_account_info(account_info_iter)?;

    // Verify authority is signer
    if !authority_info.is_signer {
        msg!("Authority must be signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Get current time
    let clock = Clock::from_account_info(clock_info)?;
    let current_time = clock.unix_timestamp;

    // Validate end_time is in the future
    if end_time <= current_time {
        msg!("End time must be in the future");
        return Err(FutarchyError::InvalidInstruction.into());
    }

    // Derive market PDA
    let market_seeds = &[
        crate::state::MARKET_SEED,
        &market_id.to_le_bytes(),
    ];
    let (market_pda, market_bump) = Pubkey::find_program_address(market_seeds, program_id);

    if market_pda != *market_info.key {
        msg!("Invalid market PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Derive escrow PDA
    let market_id_bytes = market_id.to_le_bytes();
    let escrow_seeds = &[b"escrow" as &[u8], &market_id_bytes as &[u8]];
    let (escrow_pda, escrow_bump) = Pubkey::find_program_address(escrow_seeds, program_id);

    if escrow_pda != *escrow_info.key {
        msg!("Invalid escrow PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Create market account
    let rent = Rent::get()?;
    let market_space = Market::SPACE;
    let market_lamports = rent.minimum_balance(market_space);

    msg!("Creating market account: {} bytes, {} lamports", market_space, market_lamports);

    invoke_signed(
        &system_instruction::create_account(
            authority_info.key,
            market_info.key,
            market_lamports,
            market_space as u64,
            program_id,
        ),
        &[
            authority_info.clone(),
            market_info.clone(),
            system_program_info.clone(),
        ],
        &[&[
            crate::state::MARKET_SEED,
            &market_id.to_le_bytes(),
            &[market_bump],
        ]],
    )?;

    // Create escrow account (just a simple PDA, will hold lamports)
    let escrow_space = 0; // Empty account, just holds lamports
    let escrow_lamports = rent.minimum_balance(escrow_space);

    invoke_signed(
        &system_instruction::create_account(
            authority_info.key,
            escrow_info.key,
            escrow_lamports,
            escrow_space as u64,
            program_id,
        ),
        &[
            authority_info.clone(),
            escrow_info.clone(),
            system_program_info.clone(),
        ],
        &[&[b"escrow", &market_id.to_le_bytes(), &[escrow_bump]]],
    )?;

    // Initialize market state (simple market, no governance)
    let market = Market {
        authority: *authority_info.key,
        market_id,
        oracle: *oracle_info.key,
        question_hash,
        end_time,
        status: MarketStatus::Active,
        max_bet,
        total_yes_bets: 0,
        total_no_bets: 0,
        encrypted_pool_yes: Vec::new(),  // Empty for now, will be set when first bet uses FHE
        encrypted_pool_no: Vec::new(),
        pending_pool_update_job: None,
        resolution: None,
        settled_at: None,
        escrow: *escrow_info.key,
        claim_nullifiers: Vec::new(),
        // Governance fields (defaults for simple markets)
        executable_action: ExecutableAction::None,
        execution_threshold: 0,
        timelock_duration: 0,
        timelock_expires_at: None,
        action_executed: false,
        bump: market_bump,
        created_at: current_time,
    };

    // Serialize market to account data
    let mut market_data = market_info.try_borrow_mut_data()?;
    market.serialize(&mut &mut market_data[..])?;

    msg!("Market created successfully");
    msg!("  Market ID: {}", market_id);
    msg!("  Authority: {}", authority_info.key);
    msg!("  Oracle: {}", oracle_info.key);
    msg!("  End time: {}", end_time);
    msg!("  Max bet: {}", max_bet);

    Ok(())
}
