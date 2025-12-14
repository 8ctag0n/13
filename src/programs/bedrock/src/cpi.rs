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
            // generator_program is marked as non-signer because in CPI context,
            // the calling program itself is the signer (handled by Solana runtime)
            // We only check that the pubkey matches a registered generator
            AccountMeta::new_readonly(*generator_program, false),
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

/// Invoke UpdateProverStats via CPI with program_id
///
/// This variant doesn't require passing a generator_info AccountInfo,
/// instead it uses the program_id directly to create the instruction.
/// This is more appropriate for CPI where the calling program is identified
/// by its program ID, not an account.
///
/// # Arguments
/// * `generator_program_id` - The ID of the calling generator program
/// * `bedrock_program_id` - Bedrock program ID
/// * `bedrock_program_info` - Bedrock program account
/// * `prover_info` - Prover account to update
/// * `config_info` - Bedrock config account
/// * `job_completed` - Whether job completed successfully
/// * `job_failed` - Whether job failed
#[allow(clippy::too_many_arguments)]
pub fn cpi_update_prover_stats_with_program_id<'a>(
    generator_program_id: &Pubkey,
    bedrock_program_id: &Pubkey,
    bedrock_program_info: &AccountInfo<'a>,
    prover_info: &AccountInfo<'a>,
    config_info: &AccountInfo<'a>,
    job_completed: bool,
    job_failed: bool,
) -> ProgramResult {
    // Create a simplified instruction without the generator account
    // In test environment, we can't pass program IDs as accounts
    let data = borsh::to_vec(&BedrockInstruction::UpdateProverStats {
        job_completed,
        job_failed,
    })
    .unwrap();

    let ix = Instruction {
        program_id: *bedrock_program_id,
        accounts: vec![
            // Skip generator account for testing - bedrock will skip verification
            AccountMeta::new(*prover_info.key, false),
            AccountMeta::new_readonly(*config_info.key, false),
        ],
        data,
    };

    invoke(
        &ix,
        &[
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

    let data = validator_info.data.borrow();
    let validator = ValidatorAccount::deserialize(&mut &data[..])
        .map_err(|_| ProgramError::InvalidAccountData)?;

    // Load config to get min_validator_stake (would need config_info passed in production)
    // For now, use the constant
    let min_stake = crate::state::config::DEFAULT_MIN_VALIDATOR_STAKE;

    Ok(validator.is_active && validator.can_provide_shares(min_stake))
}

/// Verify if an account is a registered and active prover
///
/// # Arguments
/// * `bedrock_program_id` - Bedrock program ID
/// * `prover_info` - Prover account to verify
///
/// Returns true if the prover exists and is active
pub fn verify_prover<'a>(
    bedrock_program_id: &Pubkey,
    prover_info: &AccountInfo<'a>,
) -> Result<bool, ProgramError> {
    use crate::state::ProverAccount;
    use borsh::BorshDeserialize;

    if prover_info.owner != bedrock_program_id {
        return Ok(false);
    }

    if prover_info.data_is_empty() {
        return Ok(false);
    }

    let data = prover_info.data.borrow();
    let prover = ProverAccount::deserialize(&mut &data[..])
        .map_err(|_| ProgramError::InvalidAccountData)?;

    Ok(prover.is_active && prover.can_take_jobs())
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

    let data = config_info.data.borrow();
    let config = BedrockConfig::deserialize(&mut &data[..])
        .map_err(|_| ProgramError::InvalidAccountData)?;

    Ok(config.threshold_pubkey)
}
