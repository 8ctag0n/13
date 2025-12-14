//! Instruction builders for bedrock program

use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

use bedrock::{BedrockInstruction, SlashReason, ValidatorRegion};

use crate::accounts::{
    InitializeAccounts, RegisterProverAccounts, RegisterValidatorAccounts, SlashProverAccounts,
    SlashValidatorAccounts, UpdateProverStatsAccounts,
};

/// Build an Initialize instruction
///
/// # Arguments
/// * `program_id` - The bedrock program ID
/// * `admin` - Admin authority pubkey
/// * `config` - Config PDA pubkey
/// * `zk_generator` - ZK generator program ID
/// * `fhe_generator` - FHE generator program ID
///
/// # Returns
/// * `Instruction` - Ready-to-send instruction
///
/// # Example
/// ```
/// use bedrock_sdk::instructions::initialize;
/// use solana_sdk::pubkey::Pubkey;
///
/// let program_id = Pubkey::default();
/// let admin = Pubkey::default();
/// let config = Pubkey::default();
/// let zk_gen = Pubkey::default();
/// let fhe_gen = Pubkey::default();
///
/// let ix = initialize(&program_id, &admin, &config, &zk_gen, &fhe_gen);
/// ```
pub fn initialize(
    program_id: &Pubkey,
    admin: &Pubkey,
    config: &Pubkey,
    zk_generator: &Pubkey,
    fhe_generator: &Pubkey,
) -> Instruction {
    let accounts = InitializeAccounts {
        admin: *admin,
        config: *config,
    };

    let instruction = BedrockInstruction::Initialize {
        zk_generator_program: *zk_generator,
        fhe_generator_program: *fhe_generator,
    };
    let data = borsh::to_vec(&instruction).expect("Failed to serialize Initialize instruction");

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(),
        data,
    }
}

/// Build a RegisterProver instruction
///
/// # Arguments
/// * `program_id` - The bedrock program ID
/// * `prover_wallet` - Prover's wallet pubkey (signer)
/// * `prover_account` - Prover account PDA pubkey
/// * `stake_lamports` - Amount to stake (minimum 0.1 SOL = 100_000_000 lamports)
///
/// # Returns
/// * `Instruction` - Ready-to-send instruction
///
/// # Example
/// ```
/// use bedrock_sdk::{instructions::register_prover, derive_prover_pda};
/// use solana_sdk::pubkey::Pubkey;
///
/// let program_id = Pubkey::default();
/// let prover_wallet = Pubkey::default();
/// let (prover_pda, _) = derive_prover_pda(&program_id, &prover_wallet);
///
/// let ix = register_prover(&program_id, &prover_wallet, &prover_pda, 100_000_000);
/// ```
pub fn register_prover(
    program_id: &Pubkey,
    prover_wallet: &Pubkey,
    prover_account: &Pubkey,
    stake_lamports: u64,
) -> Instruction {
    let config = crate::derive_config_pda(program_id).0;

    let accounts = RegisterProverAccounts {
        prover_wallet: *prover_wallet,
        prover_account: *prover_account,
        config,
    };

    let instruction = BedrockInstruction::RegisterProver { stake_lamports };
    let data = borsh::to_vec(&instruction).expect("Failed to serialize RegisterProver instruction");

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(),
        data,
    }
}

/// Build an UpdateProverStats instruction
///
/// # Arguments
/// * `program_id` - The bedrock program ID
/// * `generator_program` - Generator program ID (must be registered, signer)
/// * `prover_account` - Prover account PDA pubkey
/// * `job_completed` - Whether the job was completed successfully
/// * `job_failed` - Whether the job failed
///
/// # Returns
/// * `Instruction` - Ready-to-send instruction (for CPI usage)
///
/// # Example
/// ```
/// use bedrock_sdk::instructions::update_prover_stats;
/// use solana_sdk::pubkey::Pubkey;
///
/// let program_id = Pubkey::default();
/// let generator = Pubkey::default();
/// let prover_account = Pubkey::default();
///
/// let ix = update_prover_stats(&program_id, &generator, &prover_account, true, false);
/// ```
pub fn update_prover_stats(
    program_id: &Pubkey,
    generator_program: &Pubkey,
    prover_account: &Pubkey,
    job_completed: bool,
    job_failed: bool,
) -> Instruction {
    let config = crate::derive_config_pda(program_id).0;

    let accounts = UpdateProverStatsAccounts {
        generator_program: *generator_program,
        prover_account: *prover_account,
        config,
    };

    let instruction = BedrockInstruction::UpdateProverStats {
        job_completed,
        job_failed,
    };
    let data = borsh::to_vec(&instruction)
        .expect("Failed to serialize UpdateProverStats instruction");

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(),
        data,
    }
}

/// Build a SlashProver instruction
///
/// # Arguments
/// * `program_id` - The bedrock program ID
/// * `generator_program` - Generator program ID (must be registered, signer)
/// * `prover_account` - Prover account PDA pubkey
/// * `amount` - Amount to slash (in lamports)
/// * `reason` - Reason for slashing
///
/// # Returns
/// * `Instruction` - Ready-to-send instruction (for CPI usage)
///
/// # Example
/// ```
/// use bedrock_sdk::{instructions::slash_prover, SlashReason};
/// use solana_sdk::pubkey::Pubkey;
///
/// let program_id = Pubkey::default();
/// let generator = Pubkey::default();
/// let prover_account = Pubkey::default();
///
/// let ix = slash_prover(
///     &program_id,
///     &generator,
///     &prover_account,
///     10_000_000,
///     SlashReason::Timeout,
/// );
/// ```
pub fn slash_prover(
    program_id: &Pubkey,
    generator_program: &Pubkey,
    prover_account: &Pubkey,
    amount: u64,
    reason: SlashReason,
) -> Instruction {
    let config = crate::derive_config_pda(program_id).0;

    let accounts = SlashProverAccounts {
        generator_program: *generator_program,
        prover_account: *prover_account,
        config,
    };

    let instruction = BedrockInstruction::SlashProver { amount, reason };
    let data = borsh::to_vec(&instruction).expect("Failed to serialize SlashProver instruction");

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(),
        data,
    }
}

/// Build a RegisterValidator instruction
///
/// # Arguments
/// * `program_id` - The bedrock program ID
/// * `validator_wallet` - Validator's wallet pubkey (signer)
/// * `validator_account` - Validator account PDA pubkey
/// * `endpoint` - Validator's endpoint URL (max 256 chars)
/// * `region` - Geographic region
/// * `stake` - Amount to stake (in lamports)
///
/// # Returns
/// * `Instruction` - Ready-to-send instruction
///
/// # Example
/// ```
/// use bedrock_sdk::{instructions::register_validator, derive_validator_pda, ValidatorRegion};
/// use solana_sdk::pubkey::Pubkey;
///
/// let program_id = Pubkey::default();
/// let validator_wallet = Pubkey::default();
/// let (validator_pda, _) = derive_validator_pda(&program_id, &validator_wallet);
///
/// let ix = register_validator(
///     &program_id,
///     &validator_wallet,
///     &validator_pda,
///     "https://validator.example.com".to_string(),
///     ValidatorRegion::NorthAmerica,
///     100_000_000,
/// );
/// ```
pub fn register_validator(
    program_id: &Pubkey,
    validator_wallet: &Pubkey,
    validator_account: &Pubkey,
    endpoint: String,
    region: ValidatorRegion,
    stake: u64,
) -> Instruction {
    let config = crate::derive_config_pda(program_id).0;

    let accounts = RegisterValidatorAccounts {
        validator_wallet: *validator_wallet,
        validator_account: *validator_account,
        config,
    };

    let instruction = BedrockInstruction::RegisterValidator {
        endpoint,
        region,
        stake,
    };
    let data =
        borsh::to_vec(&instruction).expect("Failed to serialize RegisterValidator instruction");

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(),
        data,
    }
}

/// Build a SlashValidator instruction
///
/// # Arguments
/// * `program_id` - The bedrock program ID
/// * `admin` - Admin authority pubkey (signer)
/// * `validator_account` - Validator account PDA pubkey
/// * `amount` - Amount to slash (in lamports)
/// * `reason` - Reason for slashing
///
/// # Returns
/// * `Instruction` - Ready-to-send instruction
///
/// # Example
/// ```
/// use bedrock_sdk::{instructions::slash_validator, SlashReason};
/// use solana_sdk::pubkey::Pubkey;
///
/// let program_id = Pubkey::default();
/// let admin = Pubkey::default();
/// let validator_account = Pubkey::default();
///
/// let ix = slash_validator(
///     &program_id,
///     &admin,
///     &validator_account,
///     10_000_000,
///     SlashReason::InvalidKeyShare,
/// );
/// ```
pub fn slash_validator(
    program_id: &Pubkey,
    admin: &Pubkey,
    validator_account: &Pubkey,
    amount: u64,
    reason: SlashReason,
) -> Instruction {
    let config = crate::derive_config_pda(program_id).0;

    let accounts = SlashValidatorAccounts {
        admin: *admin,
        validator_account: *validator_account,
        config,
    };

    let instruction = BedrockInstruction::SlashValidator { amount, reason };
    let data = borsh::to_vec(&instruction).expect("Failed to serialize SlashValidator instruction");

    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(),
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize_instruction() {
        let program_id = Pubkey::new_unique();
        let admin = Pubkey::new_unique();
        let config = Pubkey::new_unique();
        let zk_gen = Pubkey::new_unique();
        let fhe_gen = Pubkey::new_unique();

        let ix = initialize(&program_id, &admin, &config, &zk_gen, &fhe_gen);

        assert_eq!(ix.program_id, program_id);
        assert_eq!(ix.accounts.len(), 3);
        assert!(ix.accounts[0].is_signer); // admin
        assert!(ix.accounts[1].is_writable); // config
    }

    #[test]
    fn test_register_prover_instruction() {
        let program_id = Pubkey::new_unique();
        let prover_wallet = Pubkey::new_unique();
        let prover_account = Pubkey::new_unique();

        let ix = register_prover(&program_id, &prover_wallet, &prover_account, 100_000_000);

        assert_eq!(ix.program_id, program_id);
        assert_eq!(ix.accounts.len(), 4);
        assert!(ix.accounts[0].is_signer); // prover_wallet
        assert!(ix.accounts[0].is_writable); // prover_wallet (pays for account)
        assert!(ix.accounts[1].is_writable); // prover_account
    }

    #[test]
    fn test_update_prover_stats_instruction() {
        let program_id = Pubkey::new_unique();
        let generator = Pubkey::new_unique();
        let prover_account = Pubkey::new_unique();

        let ix = update_prover_stats(&program_id, &generator, &prover_account, true, false);

        assert_eq!(ix.program_id, program_id);
        assert_eq!(ix.accounts.len(), 3);
        assert!(ix.accounts[0].is_signer); // generator
        assert!(ix.accounts[1].is_writable); // prover_account
    }

    #[test]
    fn test_slash_prover_instruction() {
        let program_id = Pubkey::new_unique();
        let generator = Pubkey::new_unique();
        let prover_account = Pubkey::new_unique();

        let ix = slash_prover(
            &program_id,
            &generator,
            &prover_account,
            10_000_000,
            SlashReason::Timeout,
        );

        assert_eq!(ix.program_id, program_id);
        assert_eq!(ix.accounts.len(), 3);
        assert!(ix.accounts[0].is_signer); // generator
        assert!(ix.accounts[1].is_writable); // prover_account
    }
}
