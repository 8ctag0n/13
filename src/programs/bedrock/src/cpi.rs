//! CPI helpers for calling Bedrock from generator programs
//!
//! These functions create the instruction data and account metas needed
//! for generators to call Bedrock instructions via CPI.

use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::instruction::{BedrockInstruction, SlashReason};

/// Create instruction to update prover stats
///
/// # Arguments
/// * `bedrock_program_id` - Bedrock program ID
/// * `generator_program` - Calling generator program (signer via CPI)
/// * `prover_account` - Prover PDA to update
/// * `config` - Bedrock config account
/// * `job_completed` - Whether job was completed successfully
/// * `job_failed` - Whether job failed
pub fn update_prover_stats_instruction(
    bedrock_program_id: &Pubkey,
    generator_program: &Pubkey,
    prover_account: &Pubkey,
    config: &Pubkey,
    job_completed: bool,
    job_failed: bool,
) -> Instruction {
    let data = borsh::to_vec(&BedrockInstruction::UpdateProverStats {
        job_completed,
        job_failed,
    })
    .unwrap();

    Instruction {
        program_id: *bedrock_program_id,
        accounts: vec![
            AccountMeta::new_readonly(*generator_program, true), // signer
            AccountMeta::new(*prover_account, false),
            AccountMeta::new_readonly(*config, false),
        ],
        data,
    }
}

/// Create instruction to slash a prover
///
/// # Arguments
/// * `bedrock_program_id` - Bedrock program ID
/// * `generator_program` - Calling generator program (signer via CPI)
/// * `prover_account` - Prover PDA to slash
/// * `config` - Bedrock config account
/// * `recipient` - Account to receive slashed funds
/// * `amount` - Amount to slash in lamports
/// * `reason` - Reason for slashing
pub fn slash_prover_instruction(
    bedrock_program_id: &Pubkey,
    generator_program: &Pubkey,
    prover_account: &Pubkey,
    config: &Pubkey,
    recipient: &Pubkey,
    amount: u64,
    reason: SlashReason,
) -> Instruction {
    let data = borsh::to_vec(&BedrockInstruction::SlashProver { amount, reason }).unwrap();

    Instruction {
        program_id: *bedrock_program_id,
        accounts: vec![
            AccountMeta::new_readonly(*generator_program, true), // signer
            AccountMeta::new(*prover_account, false),
            AccountMeta::new_readonly(*config, false),
            AccountMeta::new(*recipient, false),
        ],
        data,
    }
}

/// Invoke UpdateProverStats via CPI
///
/// # Arguments
/// * `bedrock_program_id` - Bedrock program ID
/// * `generator_info` - Generator program account (will sign)
/// * `prover_info` - Prover account to update
/// * `config_info` - Bedrock config account
/// * `job_completed` - Whether job completed successfully
/// * `job_failed` - Whether job failed
pub fn cpi_update_prover_stats<'a>(
    bedrock_program_id: &Pubkey,
    bedrock_program_info: &AccountInfo<'a>,
    generator_info: &AccountInfo<'a>,
    prover_info: &AccountInfo<'a>,
    config_info: &AccountInfo<'a>,
    job_completed: bool,
    job_failed: bool,
) -> ProgramResult {
    let ix = update_prover_stats_instruction(
        bedrock_program_id,
        generator_info.key,
        prover_info.key,
        config_info.key,
        job_completed,
        job_failed,
    );

    invoke(
        &ix,
        &[
            generator_info.clone(),
            prover_info.clone(),
            config_info.clone(),
            bedrock_program_info.clone(),
        ],
    )
}

/// Invoke SlashProver via CPI
///
/// # Arguments
/// * `bedrock_program_id` - Bedrock program ID
/// * `generator_info` - Generator program account (will sign)
/// * `prover_info` - Prover account to slash
/// * `config_info` - Bedrock config account
/// * `recipient_info` - Account to receive slashed funds
/// * `amount` - Amount to slash
/// * `reason` - Reason for slashing
#[allow(clippy::too_many_arguments)]
pub fn cpi_slash_prover<'a>(
    bedrock_program_id: &Pubkey,
    bedrock_program_info: &AccountInfo<'a>,
    generator_info: &AccountInfo<'a>,
    prover_info: &AccountInfo<'a>,
    config_info: &AccountInfo<'a>,
    recipient_info: &AccountInfo<'a>,
    amount: u64,
    reason: SlashReason,
) -> ProgramResult {
    let ix = slash_prover_instruction(
        bedrock_program_id,
        generator_info.key,
        prover_info.key,
        config_info.key,
        recipient_info.key,
        amount,
        reason,
    );

    invoke(
        &ix,
        &[
            generator_info.clone(),
            prover_info.clone(),
            config_info.clone(),
            recipient_info.clone(),
            bedrock_program_info.clone(),
        ],
    )
}

/// Derive prover PDA for a given wallet
pub fn derive_prover_pda(bedrock_program_id: &Pubkey, prover_wallet: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"prover", prover_wallet.as_ref()], bedrock_program_id)
}

/// Derive config PDA
pub fn derive_config_pda(bedrock_program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"config"], bedrock_program_id)
}

/// Derive validator PDA for a given wallet
pub fn derive_validator_pda(bedrock_program_id: &Pubkey, validator_wallet: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"validator", validator_wallet.as_ref()], bedrock_program_id)
}

/// Verify if an account is a registered and active validator
///
/// # Arguments
/// * `bedrock_program_id` - Bedrock program ID
/// * `validator_info` - Validator account to verify
///
/// Returns true if the validator exists and is active
pub fn verify_validator<'a>(
    bedrock_program_id: &Pubkey,
    validator_info: &AccountInfo<'a>,
) -> Result<bool, ProgramError> {
    use crate::state::ValidatorAccount;
    use borsh::BorshDeserialize;

    if validator_info.owner != bedrock_program_id {
        return Ok(false);
    }

    if validator_info.data_is_empty() {
        return Ok(false);
    }

    let validator = ValidatorAccount::try_from_slice(&validator_info.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    // Load config to get min_validator_stake (would need config_info passed in production)
    // For now, use the constant
    let min_stake = crate::state::config::DEFAULT_MIN_VALIDATOR_STAKE;

    Ok(validator.is_active && validator.can_provide_shares(min_stake))
}

/// Get threshold public key from config
///
/// # Arguments
/// * `config_info` - Config account
///
/// Returns the threshold encryption public key
pub fn get_threshold_pubkey<'a>(
    config_info: &AccountInfo<'a>,
) -> Result<Pubkey, ProgramError> {
    use crate::state::BedrockConfig;
    use borsh::BorshDeserialize;

    let config = BedrockConfig::try_from_slice(&config_info.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    Ok(config.threshold_pubkey)
}
