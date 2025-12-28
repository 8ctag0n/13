//! ExecuteGovernanceAction instruction processor

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::Sysvar,
};

use crate::{
    error::FutarchyError,
    state::{ExecutableAction, Market, MARKET_SEED},
};

/// Process ExecuteGovernanceAction instruction
///
/// Executes the governance action after settlement and timelock
///
/// Accounts expected:
/// 0. `[writable, signer]` Executor (anyone can call)
/// 1. `[writable]` Market account (PDA)
/// 2. `[]` Clock sysvar
/// ... Additional accounts depend on ExecutableAction type
pub fn process_execute_governance_action(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    market_id: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let executor_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    let clock_info = next_account_info(account_info_iter)?;

    // Verify executor is signer (permissionless, but must sign)
    if !executor_info.is_signer {
        msg!("Executor must be signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Get current time
    let clock = Clock::from_account_info(clock_info)?;
    let current_time = clock.unix_timestamp;

    msg!("Executing governance action for market {}", market_id);

    // Derive market PDA
    let market_seeds = &[MARKET_SEED, &market_id.to_le_bytes()];
    let (market_pda, market_bump) = Pubkey::find_program_address(market_seeds, program_id);

    if market_pda != *market_info.key {
        msg!("Invalid market PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Load market
    let mut market_data = market_info.try_borrow_mut_data()?;
    let mut market = Market::deserialize(&mut &market_data[..])?;

    // Verify market is settled
    if !market.is_settled() {
        msg!("Market not settled yet");
        return Err(FutarchyError::MarketNotSettled.into());
    }

    // Verify is governance market
    if !market.is_governance_market() {
        msg!("Market has no executable action");
        return Err(FutarchyError::NoActionConfigured.into());
    }

    // Verify threshold was met
    if !market.threshold_met() {
        msg!("Execution threshold not met");
        msg!("  YES votes: {}%", market.yes_vote_percentage());
        msg!("  Required: {}%", market.execution_threshold);
        return Err(FutarchyError::ThresholdNotMet.into());
    }

    // Verify timelock expired
    if !market.timelock_expired(current_time) {
        msg!("Timelock not expired yet");
        if let Some(expires_at) = market.timelock_expires_at {
            msg!("  Current time: {}", current_time);
            msg!("  Expires at: {}", expires_at);
            msg!("  Remaining: {}s", expires_at - current_time);
        }
        return Err(FutarchyError::TimelockNotExpired.into());
    }

    // Verify action not already executed
    if market.action_executed {
        msg!("Governance action already executed");
        return Err(FutarchyError::ActionAlreadyExecuted.into());
    }

    msg!("Validation passed, executing action");
    msg!("  Action type: {}", market.executable_action.action_type());

    // Execute action based on type
    match &market.executable_action {
        ExecutableAction::None => {
            // Should never happen (checked above)
            return Err(FutarchyError::NoActionConfigured.into());
        }

        ExecutableAction::TransferTokens {
            token_mint,
            from_treasury,
            to,
            amount,
        } => {
            msg!("Executing TransferTokens");
            msg!("  Token mint: {}", token_mint);
            msg!("  From: {}", from_treasury);
            msg!("  To: {}", to);
            msg!("  Amount: {}", amount);

            // TODO: Implement SPL token transfer
            // Requires adding spl-token dependency to Cargo.toml
            msg!("TransferTokens implementation pending");
            msg!("  Add dependency: spl-token = \"...\", spl-associated-token-account = \"...\"");
            msg!("  For now, action is validated but not executed");

            // Placeholder - would need:
            // 1. Get token_program, from_account, to_account from remaining accounts
            // 2. Verify accounts match action parameters
            // 3. Create transfer instruction via spl_token::instruction::transfer()
            // 4. Invoke with market PDA as signer

            msg!("Token transfer placeholder executed");
        }

        ExecutableAction::UpdateParameter {
            target_program,
            parameter_key,
            parameter_value,
        } => {
            msg!("Executing UpdateParameter");
            msg!("  Target program: {}", target_program);
            msg!("  Parameter key: {:?}", parameter_key);
            msg!("  Parameter value: {:?}", parameter_value);

            // Get target program account
            let target_program_info = next_account_info(account_info_iter)?;

            if target_program_info.key != target_program {
                msg!("Target program mismatch");
                return Err(ProgramError::InvalidAccountData);
            }

            // Build update instruction (format depends on target program)
            // For now, this is a placeholder - actual implementation depends on target
            msg!("UpdateParameter: Implementation depends on target program");
            msg!("  This is a placeholder - customize for specific targets");

            // Market PDA would call target program with update instruction
            // invoke_signed(...) with custom instruction
        }

        ExecutableAction::CustomInstruction {
            program_id: target_program_id,
            accounts: action_accounts,
            data,
        } => {
            msg!("Executing CustomInstruction");
            msg!("  Target program: {}", target_program_id);
            msg!("  Accounts: {}", action_accounts.len());
            msg!("  Data size: {} bytes", data.len());

            // Collect account infos
            let mut account_metas = Vec::new();
            let mut account_infos = Vec::new();

            for action_account in action_accounts {
                let account_info = next_account_info(account_info_iter)?;

                // Verify account matches
                if account_info.key != &action_account.pubkey {
                    msg!("Account mismatch: expected {}, got {}", action_account.pubkey, account_info.key);
                    return Err(ProgramError::InvalidAccountData);
                }

                account_metas.push(AccountMeta {
                    pubkey: action_account.pubkey,
                    is_signer: action_account.is_signer,
                    is_writable: action_account.is_writable,
                });

                account_infos.push(account_info.clone());
            }

            // Build custom instruction
            let custom_ix = Instruction {
                program_id: *target_program_id,
                accounts: account_metas,
                data: data.clone(),
            };

            // Invoke with market PDA as signer
            let market_seeds_with_bump = &[
                MARKET_SEED,
                &market_id.to_le_bytes(),
                &[market_bump],
            ];

            invoke_signed(&custom_ix, &account_infos, &[market_seeds_with_bump])?;

            msg!("Custom instruction executed successfully");
        }
    }

    // Mark action as executed
    market.action_executed = true;

    // Save market state
    drop(market_data); // Release borrow
    let mut market_data = market_info.try_borrow_mut_data()?;
    market.serialize(&mut &mut market_data[..])?;

    msg!("Governance action executed successfully");

    Ok(())
}
