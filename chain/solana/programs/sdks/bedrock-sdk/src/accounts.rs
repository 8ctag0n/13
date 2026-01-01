//! Account metadata helpers for bedrock instructions

use solana_program::pubkey::Pubkey;
use solana_sdk::instruction::AccountMeta;

/// Accounts required for the Initialize instruction
pub struct InitializeAccounts {
    /// Admin authority (signer)
    pub admin: Pubkey,
    /// Config PDA (writable)
    pub config: Pubkey,
}

impl InitializeAccounts {
    /// Convert to vec of AccountMeta for instruction building
    pub fn to_account_metas(&self) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new_readonly(self.admin, true),
            AccountMeta::new(self.config, false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
        ]
    }
}

/// Accounts required for the RegisterProver instruction
pub struct RegisterProverAccounts {
    /// Prover wallet (signer, payer)
    pub prover_wallet: Pubkey,
    /// Prover account PDA (writable)
    pub prover_account: Pubkey,
    /// Config PDA (writable - to update total_provers)
    pub config: Pubkey,
}

impl RegisterProverAccounts {
    /// Convert to vec of AccountMeta for instruction building
    pub fn to_account_metas(&self) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(self.prover_wallet, true),
            AccountMeta::new(self.prover_account, false),
            AccountMeta::new(self.config, false), // writable for total_provers update
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
        ]
    }
}

/// Accounts required for the UpdateProverStats instruction
pub struct UpdateProverStatsAccounts {
    /// Generator program (must be registered in config, signer)
    pub generator_program: Pubkey,
    /// Prover account PDA (writable)
    pub prover_account: Pubkey,
    /// Config PDA (readonly)
    pub config: Pubkey,
}

impl UpdateProverStatsAccounts {
    /// Convert to vec of AccountMeta for instruction building
    pub fn to_account_metas(&self) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new_readonly(self.generator_program, true),
            AccountMeta::new(self.prover_account, false),
            AccountMeta::new_readonly(self.config, false),
        ]
    }
}

/// Accounts required for the SlashProver instruction
pub struct SlashProverAccounts {
    /// Generator program (must be registered in config, signer)
    pub generator_program: Pubkey,
    /// Prover account PDA (writable)
    pub prover_account: Pubkey,
    /// Config PDA (readonly)
    pub config: Pubkey,
}

impl SlashProverAccounts {
    /// Convert to vec of AccountMeta for instruction building
    pub fn to_account_metas(&self) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new_readonly(self.generator_program, true),
            AccountMeta::new(self.prover_account, false),
            AccountMeta::new_readonly(self.config, false),
        ]
    }
}

/// Accounts required for the RegisterValidator instruction
pub struct RegisterValidatorAccounts {
    /// Validator wallet (signer, payer)
    pub validator_wallet: Pubkey,
    /// Validator account PDA (writable)
    pub validator_account: Pubkey,
    /// Config PDA (writable, for incrementing total_validators)
    pub config: Pubkey,
}

impl RegisterValidatorAccounts {
    /// Convert to vec of AccountMeta for instruction building
    pub fn to_account_metas(&self) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(self.validator_wallet, true),
            AccountMeta::new(self.validator_account, false),
            AccountMeta::new(self.config, false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
        ]
    }
}

/// Accounts required for the SlashValidator instruction
pub struct SlashValidatorAccounts {
    /// Admin authority (signer)
    pub admin: Pubkey,
    /// Validator account PDA (writable)
    pub validator_account: Pubkey,
    /// Config PDA (readonly)
    pub config: Pubkey,
}

impl SlashValidatorAccounts {
    /// Convert to vec of AccountMeta for instruction building
    pub fn to_account_metas(&self) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new_readonly(self.admin, true),
            AccountMeta::new(self.validator_account, false),
            AccountMeta::new_readonly(self.config, false),
        ]
    }
}
