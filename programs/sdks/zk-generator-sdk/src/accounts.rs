//! Account helpers for zk-generator program
//!
//! This module provides helper functions to build AccountMeta arrays
//! for complex instructions that involve CPI calls to bedrock.

use solana_sdk::{instruction::AccountMeta, pubkey::Pubkey, system_program};

/// Build accounts for CreateJob instruction
///
/// # Arguments
/// * `creator` - The job creator (signer, writable)
/// * `job_pda` - The job PDA (writable)
/// * `escrow_pda` - The escrow PDA (writable)
///
/// # Returns
/// Vec of AccountMeta for CreateJob instruction
pub fn create_job_accounts(
    creator: &Pubkey,
    job_pda: &Pubkey,
    escrow_pda: &Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new(*creator, true),
        AccountMeta::new(*job_pda, false),
        AccountMeta::new(*escrow_pda, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ]
}

/// Build accounts for ClaimJob instruction (without CPI)
///
/// # Arguments
/// * `prover` - The prover (signer)
/// * `job_pda` - The job PDA (writable)
///
/// # Returns
/// Vec of AccountMeta for ClaimJob instruction
pub fn claim_job_accounts_no_cpi(prover: &Pubkey, job_pda: &Pubkey) -> Vec<AccountMeta> {
    vec![AccountMeta::new(*prover, true), AccountMeta::new(*job_pda, false)]
}

/// Build accounts for ClaimJob instruction (with CPI verification)
///
/// # Arguments
/// * `prover` - The prover (signer)
/// * `job_pda` - The job PDA (writable)
/// * `prover_pda_bedrock` - Prover PDA in bedrock (readonly)
/// * `bedrock_program` - Bedrock program ID (readonly)
///
/// # Returns
/// Vec of AccountMeta for ClaimJob instruction with CPI accounts
pub fn claim_job_accounts_with_cpi(
    prover: &Pubkey,
    job_pda: &Pubkey,
    prover_pda_bedrock: &Pubkey,
    bedrock_program: &Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new(*prover, true),
        AccountMeta::new(*job_pda, false),
        AccountMeta::new_readonly(*prover_pda_bedrock, false),
        AccountMeta::new_readonly(*bedrock_program, false),
    ]
}

/// Build accounts for SubmitProof instruction
///
/// # Arguments
/// * `prover` - The prover (signer, writable)
/// * `job_pda` - The job PDA (writable)
/// * `escrow_pda` - The escrow PDA (writable)
/// * `fee_recipient` - Protocol fee recipient (writable)
/// * `bedrock_program` - Bedrock program ID (readonly)
/// * `prover_pda` - Prover PDA in bedrock (writable)
/// * `bedrock_config` - Bedrock config PDA (readonly)
///
/// # Returns
/// Vec of AccountMeta for SubmitProof instruction
#[allow(clippy::too_many_arguments)]
pub fn submit_proof_accounts(
    prover: &Pubkey,
    job_pda: &Pubkey,
    escrow_pda: &Pubkey,
    fee_recipient: &Pubkey,
    bedrock_program: &Pubkey,
    prover_pda: &Pubkey,
    bedrock_config: &Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new(*prover, true),
        AccountMeta::new(*job_pda, false),
        AccountMeta::new(*escrow_pda, false),
        AccountMeta::new(*fee_recipient, false),
        AccountMeta::new_readonly(*bedrock_program, false),
        AccountMeta::new(*prover_pda, false),
        AccountMeta::new_readonly(*bedrock_config, false),
    ]
}

/// Build accounts for DisputeProof instruction
///
/// # Arguments
/// * `disputor` - The disputor (signer, writable)
/// * `job_pda` - The job PDA (writable)
/// * `prover` - The prover wallet (writable, for slashing)
/// * `treasury` - Protocol treasury (writable)
/// * `bedrock_program` - Bedrock program ID (readonly)
/// * `prover_pda` - Prover PDA in bedrock (writable)
/// * `bedrock_config` - Bedrock config PDA (readonly)
///
/// # Returns
/// Vec of AccountMeta for DisputeProof instruction
#[allow(clippy::too_many_arguments)]
pub fn dispute_proof_accounts(
    disputor: &Pubkey,
    job_pda: &Pubkey,
    prover: &Pubkey,
    treasury: &Pubkey,
    bedrock_program: &Pubkey,
    prover_pda: &Pubkey,
    bedrock_config: &Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new(*disputor, true),
        AccountMeta::new(*job_pda, false),
        AccountMeta::new(*prover, false),
        AccountMeta::new(*treasury, false),
        AccountMeta::new_readonly(*bedrock_program, false),
        AccountMeta::new(*prover_pda, false),
        AccountMeta::new_readonly(*bedrock_config, false),
    ]
}

/// Build accounts for CancelJob instruction
///
/// # Arguments
/// * `creator` - The job creator (signer, writable)
/// * `job_pda` - The job PDA (writable)
/// * `escrow_pda` - The escrow PDA (writable)
///
/// # Returns
/// Vec of AccountMeta for CancelJob instruction
pub fn cancel_job_accounts(
    creator: &Pubkey,
    job_pda: &Pubkey,
    escrow_pda: &Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new(*creator, true),
        AccountMeta::new(*job_pda, false),
        AccountMeta::new(*escrow_pda, false),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_job_accounts() {
        let creator = Pubkey::new_unique();
        let job_pda = Pubkey::new_unique();
        let escrow_pda = Pubkey::new_unique();

        let accounts = create_job_accounts(&creator, &job_pda, &escrow_pda);

        assert_eq!(accounts.len(), 4);
        assert_eq!(accounts[0].pubkey, creator);
        assert!(accounts[0].is_signer);
        assert!(accounts[0].is_writable);
        assert_eq!(accounts[1].pubkey, job_pda);
        assert!(accounts[1].is_writable);
        assert_eq!(accounts[3].pubkey, system_program::ID);
    }

    #[test]
    fn test_claim_job_accounts_no_cpi() {
        let prover = Pubkey::new_unique();
        let job_pda = Pubkey::new_unique();

        let accounts = claim_job_accounts_no_cpi(&prover, &job_pda);

        assert_eq!(accounts.len(), 2);
        assert_eq!(accounts[0].pubkey, prover);
        assert!(accounts[0].is_signer);
    }

    #[test]
    fn test_claim_job_accounts_with_cpi() {
        let prover = Pubkey::new_unique();
        let job_pda = Pubkey::new_unique();
        let prover_pda = Pubkey::new_unique();
        let bedrock = Pubkey::new_unique();

        let accounts =
            claim_job_accounts_with_cpi(&prover, &job_pda, &prover_pda, &bedrock);

        assert_eq!(accounts.len(), 4);
        assert_eq!(accounts[2].pubkey, prover_pda);
        assert_eq!(accounts[3].pubkey, bedrock);
    }

    #[test]
    fn test_submit_proof_accounts() {
        let prover = Pubkey::new_unique();
        let job_pda = Pubkey::new_unique();
        let escrow_pda = Pubkey::new_unique();
        let fee_recipient = Pubkey::new_unique();
        let bedrock = Pubkey::new_unique();
        let prover_pda = Pubkey::new_unique();
        let config = Pubkey::new_unique();

        let accounts = submit_proof_accounts(
            &prover,
            &job_pda,
            &escrow_pda,
            &fee_recipient,
            &bedrock,
            &prover_pda,
            &config,
        );

        assert_eq!(accounts.len(), 7);
        assert_eq!(accounts[0].pubkey, prover);
        assert!(accounts[0].is_signer);
        assert_eq!(accounts[4].pubkey, bedrock);
    }

    #[test]
    fn test_dispute_proof_accounts() {
        let disputor = Pubkey::new_unique();
        let job_pda = Pubkey::new_unique();
        let prover = Pubkey::new_unique();
        let treasury = Pubkey::new_unique();
        let bedrock = Pubkey::new_unique();
        let prover_pda = Pubkey::new_unique();
        let config = Pubkey::new_unique();

        let accounts = dispute_proof_accounts(
            &disputor,
            &job_pda,
            &prover,
            &treasury,
            &bedrock,
            &prover_pda,
            &config,
        );

        assert_eq!(accounts.len(), 7);
        assert_eq!(accounts[0].pubkey, disputor);
        assert!(accounts[0].is_signer);
    }

    #[test]
    fn test_cancel_job_accounts() {
        let creator = Pubkey::new_unique();
        let job_pda = Pubkey::new_unique();
        let escrow_pda = Pubkey::new_unique();

        let accounts = cancel_job_accounts(&creator, &job_pda, &escrow_pda);

        assert_eq!(accounts.len(), 3);
        assert_eq!(accounts[0].pubkey, creator);
        assert!(accounts[0].is_signer);
    }
}
