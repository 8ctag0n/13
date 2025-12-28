//! CreateMarketV2 - Create a new prediction market with optional governance
//!
//! Creates:
//! 1. MarketV2 PDA (minimal market state)
//! 2. MarketVault PDA (lamport-only escrow for bets)
//! 3. GovernanceConfig PDA (if has_governance=true)

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

use crate::state::{
    ExecutableAction, GovernanceConfig, MarketV2, MarketVault,
    GOVERNANCE_CONFIG_SEED, MARKET_V2_SEED, MARKET_VAULT_SEED,
};

/// Process CreateMarketV2 instruction
///
/// Creates a new prediction market with optional governance
///
/// Accounts expected:
/// 0. `[writable, signer]` Market creator/authority (payer)
/// 1. `[writable]` MarketV2 PDA (will be created)
/// 2. `[writable]` MarketVault PDA (will be created)
/// 3. `[]` Oracle pubkey
/// 4. `[]` System program
/// 5. `[]` Clock sysvar
///
/// If has_governance = true:
/// 6. `[writable]` GovernanceConfig PDA (will be created)
#[allow(clippy::too_many_arguments)]
pub fn process_create_market_v2(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    market_id: u64,
    question_hash: [u8; 32],
    end_time: i64,
    max_bet: u64,
    has_governance: bool,
    executable_action: Option<ExecutableAction>,
    execution_threshold: Option<u8>,
    timelock_duration: Option<i64>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let authority_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    let market_vault_info = next_account_info(account_info_iter)?;
    let oracle_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;
    let clock_sysvar_info = next_account_info(account_info_iter)?;

    // Verify authority is signer
    if !authority_info.is_signer {
        msg!("Authority must be signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Get clock for timestamps
    let clock = Clock::from_account_info(clock_sysvar_info)?;
    let current_time = clock.unix_timestamp;

    // Validate end_time is in the future
    if end_time <= current_time {
        msg!("End time must be in the future");
        return Err(ProgramError::InvalidArgument);
    }

    // Validate max_bet
    if max_bet == 0 {
        msg!("Max bet must be greater than 0");
        return Err(ProgramError::InvalidArgument);
    }

    // Validate governance params if has_governance
    if has_governance {
        if executable_action.is_none() {
            msg!("Governance market requires executable_action");
            return Err(ProgramError::InvalidArgument);
        }
        if execution_threshold.is_none() || execution_threshold.unwrap() > 100 {
            msg!("Governance market requires valid execution_threshold (0-100)");
            return Err(ProgramError::InvalidArgument);
        }
        if timelock_duration.is_none() || timelock_duration.unwrap() < 0 {
            msg!("Governance market requires valid timelock_duration");
            return Err(ProgramError::InvalidArgument);
        }
    }

    let market_id_bytes = market_id.to_le_bytes();

    // Derive and verify MarketV2 PDA
    let (market_pda, market_bump) = Pubkey::find_program_address(
        &[MARKET_V2_SEED, &market_id_bytes],
        program_id,
    );
    if market_pda != *market_info.key {
        msg!("Invalid MarketV2 PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Check if market already exists
    if market_info.data_len() > 0 {
        msg!("Market already exists");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    // Derive and verify MarketVault PDA
    let (market_vault_pda, vault_bump) = MarketVault::find_pda(market_id, program_id);
    if market_vault_pda != *market_vault_info.key {
        msg!("Invalid MarketVault PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    let rent = Rent::get()?;

    // Create MarketV2 PDA
    let market_space = MarketV2::SPACE;
    let market_lamports = rent.minimum_balance(market_space);
    let market_seeds = &[MARKET_V2_SEED, &market_id_bytes, &[market_bump]];

    msg!("Creating MarketV2 PDA");
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
        &[market_seeds],
    )?;

    // Create MarketVault PDA (0 bytes, lamports only)
    let vault_lamports = rent.minimum_balance(0);
    let vault_seeds = &[MARKET_VAULT_SEED, &market_id_bytes, &[vault_bump]];

    msg!("Creating MarketVault PDA");
    invoke_signed(
        &system_instruction::create_account(
            authority_info.key,
            market_vault_info.key,
            vault_lamports,
            0,
            program_id,
        ),
        &[
            authority_info.clone(),
            market_vault_info.clone(),
            system_program_info.clone(),
        ],
        &[vault_seeds],
    )?;

    // Initialize MarketV2 state
    let market = MarketV2::new(
        *authority_info.key,
        market_id,
        *oracle_info.key,
        question_hash,
        end_time,
        max_bet,
        has_governance,
        market_bump,
        current_time,
    );

    let mut market_data = market_info.try_borrow_mut_data()?;
    market.serialize(&mut &mut market_data[..])?;
    drop(market_data);

    msg!("MarketV2 created");
    msg!("  Market ID: {}", market_id);
    msg!("  Authority: {}", authority_info.key);
    msg!("  Oracle: {}", oracle_info.key);
    msg!("  End time: {}", end_time);
    msg!("  Max bet: {} lamports", max_bet);
    msg!("  Has governance: {}", has_governance);

    // Create GovernanceConfig if has_governance
    if has_governance {
        let governance_info = next_account_info(account_info_iter)?;

        let (governance_pda, governance_bump) = Pubkey::find_program_address(
            &[GOVERNANCE_CONFIG_SEED, &market_id_bytes],
            program_id,
        );
        if governance_pda != *governance_info.key {
            msg!("Invalid GovernanceConfig PDA");
            return Err(ProgramError::InvalidSeeds);
        }

        let governance_space = GovernanceConfig::SPACE;
        let governance_lamports = rent.minimum_balance(governance_space);
        let governance_seeds = &[GOVERNANCE_CONFIG_SEED, &market_id_bytes, &[governance_bump]];

        msg!("Creating GovernanceConfig PDA");
        invoke_signed(
            &system_instruction::create_account(
                authority_info.key,
                governance_info.key,
                governance_lamports,
                governance_space as u64,
                program_id,
            ),
            &[
                authority_info.clone(),
                governance_info.clone(),
                system_program_info.clone(),
            ],
            &[governance_seeds],
        )?;

        let governance_config = GovernanceConfig::new(
            market_id,
            executable_action.unwrap(),
            execution_threshold.unwrap(),
            timelock_duration.unwrap(),
            governance_bump,
        );

        let mut governance_data = governance_info.try_borrow_mut_data()?;
        governance_config.serialize(&mut &mut governance_data[..])?;

        msg!("GovernanceConfig created");
        msg!("  Threshold: {}%", execution_threshold.unwrap());
        msg!("  Timelock: {} seconds", timelock_duration.unwrap());
    }

    Ok(())
}
