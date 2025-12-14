//! Instruction builders for zk-generator program
//!
//! This module provides helper functions to build instructions for all
//! zk-generator operations.

use borsh::BorshSerialize;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
};
use zk_generator::ZkGeneratorInstruction;

use crate::pda::{derive_escrow_pda, derive_job_pda};

// Helper to serialize instructions
fn serialize_instruction(ix: ZkGeneratorInstruction) -> Vec<u8> {
    borsh::to_vec(&ix).expect("Failed to serialize instruction")
}

/// Create a new ZK proof job
///
/// # Accounts
/// 0. `[signer]` Creator wallet
/// 1. `[writable]` ZkJob PDA (derived)
/// 2. `[writable]` Escrow PDA (derived)
/// 3. `[]` System program
///
/// # Arguments
/// * `program_id` - The zk-generator program ID
/// * `creator` - The job creator's pubkey
/// * `job_pda` - The derived job PDA
/// * `escrow_pda` - The derived escrow PDA
/// * `job_id` - Unique job identifier
/// * `circuit_type` - The circuit type (0-50, see CircuitType)
/// * `witness_hash` - Hash of the encrypted witness data
/// * `witness_size` - Size of witness data in bytes
/// * `price_lamports` - Payment for the prover in lamports
/// * `timeout_seconds` - Time limit for job completion
///
/// # Example
/// ```no_run
/// use zk_generator_sdk::{instructions, derive_job_pda, derive_escrow_pda};
/// use solana_sdk::pubkey::Pubkey;
///
/// let program_id = Pubkey::default();
/// let creator = Pubkey::default();
/// let job_id = 12345u64;
///
/// let (job_pda, _) = derive_job_pda(&program_id, &creator, job_id);
/// let (escrow_pda, _) = derive_escrow_pda(&program_id, &job_pda);
///
/// let ix = instructions::create_job(
///     &program_id,
///     &creator,
///     &job_pda,
///     &escrow_pda,
///     10, // ProofOfInnocence
///     [0u8; 32],
///     1024,
///     100_000_000, // 0.1 SOL
///     3600, // 1 hour
/// );
/// ```
#[allow(clippy::too_many_arguments)]
pub fn create_job(
    program_id: &Pubkey,
    creator: &Pubkey,
    job_pda: &Pubkey,
    escrow_pda: &Pubkey,
    job_id: u64,
    circuit_type: u8,
    witness_hash: [u8; 32],
    witness_size: u32,
    price_lamports: u64,
    timeout_seconds: i64,
) -> Instruction {
    let data = serialize_instruction(ZkGeneratorInstruction::CreateJob {
        job_id,
        circuit_type,
        witness_hash,
        witness_size,
        price_lamports,
        timeout_seconds,
    });

    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*creator, true),
            AccountMeta::new(*job_pda, false),
            AccountMeta::new(*escrow_pda, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data,
    }
}

/// Claim a pending ZK job
///
/// # Accounts (without CPI verification - testing only)
/// 0. `[signer]` Prover wallet
/// 1. `[writable]` ZkJob account
///
/// # Accounts (with CPI verification - production)
/// 0. `[signer]` Prover wallet
/// 1. `[writable]` ZkJob account
/// 2. `[]` Prover PDA in bedrock (derived from bedrock program)
/// 3. `[]` Bedrock program
///
/// # Arguments
/// * `program_id` - The zk-generator program ID
/// * `prover` - The prover's pubkey
/// * `job_pda` - The job PDA to claim
/// * `prover_pda_bedrock` - Optional prover PDA in bedrock (for CPI verification)
/// * `bedrock_program` - Optional bedrock program ID (for CPI verification)
///
/// # Example
/// ```no_run
/// use zk_generator_sdk::instructions;
/// use solana_sdk::pubkey::Pubkey;
///
/// let program_id = Pubkey::default();
/// let bedrock_program_id = Pubkey::default();
/// let prover = Pubkey::default();
/// let job_pda = Pubkey::default();
///
/// // Derive prover PDA (from bedrock program)
/// // Seeds: [b"prover", prover.as_ref()]
/// let (prover_pda, _) = Pubkey::find_program_address(
///     &[b"prover", prover.as_ref()],
///     &bedrock_program_id
/// );
///
/// // Production: with CPI verification
/// let ix = instructions::claim_job(
///     &program_id,
///     &prover,
///     &job_pda,
///     Some(&prover_pda),
///     Some(&bedrock_program_id),
/// );
/// ```
pub fn claim_job(
    program_id: &Pubkey,
    prover: &Pubkey,
    job_pda: &Pubkey,
    prover_pda_bedrock: Option<&Pubkey>,
    bedrock_program: Option<&Pubkey>,
) -> Instruction {
    let data = serialize_instruction(ZkGeneratorInstruction::ClaimJob);

    let mut accounts = vec![
        AccountMeta::new(*prover, true),
        AccountMeta::new(*job_pda, false),
    ];

    // Add bedrock accounts for CPI verification if provided
    if let (Some(prover_pda), Some(bedrock_prog)) = (prover_pda_bedrock, bedrock_program) {
        accounts.push(AccountMeta::new_readonly(*prover_pda, false));
        accounts.push(AccountMeta::new_readonly(*bedrock_prog, false));
    }

    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

/// Submit a proof hash for a claimed job
///
/// # Accounts
/// 0. `[signer]` Prover wallet
/// 1. `[writable]` ZkJob account
/// 2. `[writable]` Escrow PDA
/// 3. `[writable]` Protocol fee recipient
/// 4. `[]` Bedrock program
/// 5. `[writable]` Prover PDA in bedrock
/// 6. `[]` Bedrock config PDA
///
/// # Arguments
/// * `program_id` - The zk-generator program ID
/// * `prover` - The prover's pubkey
/// * `job_pda` - The job PDA
/// * `escrow_pda` - The escrow PDA
/// * `fee_recipient` - Protocol fee recipient
/// * `bedrock_program` - Bedrock program ID
/// * `prover_pda` - Prover PDA in bedrock
/// * `bedrock_config` - Bedrock config PDA
/// * `proof_hash` - Hash of the proof (32 bytes)
///
/// # Example
/// ```no_run
/// use zk_generator_sdk::instructions;
/// use solana_sdk::pubkey::Pubkey;
///
/// let program_id = Pubkey::default();
/// let bedrock_program_id = Pubkey::default();
/// let prover = Pubkey::default();
/// let job_pda = Pubkey::default();
/// let escrow_pda = Pubkey::default();
/// let fee_recipient = Pubkey::default();
///
/// // Derive prover and config PDAs (from bedrock program)
/// let (prover_pda, _) = Pubkey::find_program_address(
///     &[b"prover", prover.as_ref()],
///     &bedrock_program_id
/// );
/// let (config_pda, _) = Pubkey::find_program_address(
///     &[b"config"],
///     &bedrock_program_id
/// );
///
/// let ix = instructions::submit_proof(
///     &program_id,
///     &prover,
///     &job_pda,
///     &escrow_pda,
///     &fee_recipient,
///     &bedrock_program_id,
///     &prover_pda,
///     &config_pda,
///     [0u8; 32], // proof_hash
/// );
/// ```
#[allow(clippy::too_many_arguments)]
pub fn submit_proof(
    program_id: &Pubkey,
    prover: &Pubkey,
    job_pda: &Pubkey,
    escrow_pda: &Pubkey,
    fee_recipient: &Pubkey,
    bedrock_program: &Pubkey,
    prover_pda: &Pubkey,
    bedrock_config: &Pubkey,
    proof_hash: [u8; 32],
) -> Instruction {
    let data = serialize_instruction(ZkGeneratorInstruction::SubmitProof { proof_hash });

    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*prover, true),
            AccountMeta::new(*job_pda, false),
            AccountMeta::new(*escrow_pda, false),
            AccountMeta::new(*fee_recipient, false),
            AccountMeta::new_readonly(*bedrock_program, false),
            AccountMeta::new(*prover_pda, false),
            AccountMeta::new_readonly(*bedrock_config, false),
        ],
        data,
    }
}

/// Dispute a submitted proof by providing the full proof for on-chain verification
///
/// # Accounts
/// 0. `[signer]` Disputor wallet
/// 1. `[writable]` ZkJob account
/// 2. `[writable]` Prover wallet (for slashing if dispute succeeds)
/// 3. `[writable]` Protocol treasury (receives portion of slash)
/// 4. `[]` Bedrock program
/// 5. `[writable]` Prover PDA in bedrock
/// 6. `[]` Bedrock config PDA
///
/// # Arguments
/// * `program_id` - The zk-generator program ID
/// * `disputor` - The disputor's pubkey
/// * `job_pda` - The job PDA to dispute
/// * `prover` - The prover's wallet (to slash if proof invalid)
/// * `treasury` - Protocol treasury
/// * `bedrock_program` - Bedrock program ID
/// * `prover_pda` - Prover PDA in bedrock
/// * `bedrock_config` - Bedrock config PDA
/// * `proof` - The full proof bytes (256 bytes for Groth16)
/// * `public_inputs` - Public inputs for circuit verification
///
/// # Example
/// ```no_run
/// use zk_generator_sdk::instructions;
/// use solana_sdk::pubkey::Pubkey;
///
/// let program_id = Pubkey::default();
/// let bedrock_program_id = Pubkey::default();
/// let disputor = Pubkey::default();
/// let job_pda = Pubkey::default();
/// let prover = Pubkey::default();
/// let treasury = Pubkey::default();
///
/// // Derive prover and config PDAs (from bedrock program)
/// let (prover_pda, _) = Pubkey::find_program_address(
///     &[b"prover", prover.as_ref()],
///     &bedrock_program_id
/// );
/// let (config_pda, _) = Pubkey::find_program_address(
///     &[b"config"],
///     &bedrock_program_id
/// );
///
/// let proof = vec![0u8; 256];
/// let public_inputs = vec![0u8; 96];
///
/// let ix = instructions::dispute_proof(
///     &program_id,
///     &disputor,
///     &job_pda,
///     &prover,
///     &treasury,
///     &bedrock_program_id,
///     &prover_pda,
///     &config_pda,
///     proof,
///     public_inputs,
/// );
/// ```
#[allow(clippy::too_many_arguments)]
pub fn dispute_proof(
    program_id: &Pubkey,
    disputor: &Pubkey,
    job_pda: &Pubkey,
    prover: &Pubkey,
    treasury: &Pubkey,
    bedrock_program: &Pubkey,
    prover_pda: &Pubkey,
    bedrock_config: &Pubkey,
    proof: Vec<u8>,
    public_inputs: Vec<u8>,
) -> Instruction {
    let data = serialize_instruction(ZkGeneratorInstruction::DisputeProof {
        proof,
        public_inputs,
    });

    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*disputor, true),
            AccountMeta::new(*job_pda, false),
            AccountMeta::new(*prover, false),
            AccountMeta::new(*treasury, false),
            AccountMeta::new_readonly(*bedrock_program, false),
            AccountMeta::new(*prover_pda, false),
            AccountMeta::new_readonly(*bedrock_config, false),
        ],
        data,
    }
}

/// Cancel a pending job (creator only)
///
/// # Accounts
/// 0. `[signer]` Job creator wallet
/// 1. `[writable]` ZkJob account
/// 2. `[writable]` Escrow PDA
///
/// # Arguments
/// * `program_id` - The zk-generator program ID
/// * `creator` - The job creator's pubkey
/// * `job_pda` - The job PDA to cancel
/// * `escrow_pda` - The escrow PDA
///
/// # Example
/// ```no_run
/// use zk_generator_sdk::instructions;
/// use solana_sdk::pubkey::Pubkey;
///
/// let program_id = Pubkey::default();
/// let creator = Pubkey::default();
/// let job_pda = Pubkey::default();
/// let escrow_pda = Pubkey::default();
///
/// let ix = instructions::cancel_job(
///     &program_id,
///     &creator,
///     &job_pda,
///     &escrow_pda,
/// );
/// ```
pub fn cancel_job(
    program_id: &Pubkey,
    creator: &Pubkey,
    job_pda: &Pubkey,
    escrow_pda: &Pubkey,
) -> Instruction {
    let data = serialize_instruction(ZkGeneratorInstruction::CancelJob);

    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*creator, true),
            AccountMeta::new(*job_pda, false),
            AccountMeta::new(*escrow_pda, false),
        ],
        data,
    }
}

/// Helper: Create job with automatic PDA derivation
///
/// This is a convenience function that derives the job and escrow PDAs
/// automatically based on the creator and job_id.
///
/// # Example
/// ```no_run
/// use zk_generator_sdk::instructions;
/// use solana_sdk::pubkey::Pubkey;
///
/// let program_id = Pubkey::default();
/// let creator = Pubkey::default();
/// let job_id = 12345u64;
///
/// let (ix, job_pda, escrow_pda) = instructions::create_job_with_derived_pdas(
///     &program_id,
///     &creator,
///     job_id,
///     10, // ProofOfInnocence
///     [0u8; 32],
///     1024,
///     100_000_000,
///     3600,
/// );
/// ```
#[allow(clippy::too_many_arguments)]
pub fn create_job_with_derived_pdas(
    program_id: &Pubkey,
    creator: &Pubkey,
    job_id: u64,
    circuit_type: u8,
    witness_hash: [u8; 32],
    witness_size: u32,
    price_lamports: u64,
    timeout_seconds: i64,
) -> (Instruction, Pubkey, Pubkey) {
    let (job_pda, _) = derive_job_pda(program_id, creator, job_id);
    let (escrow_pda, _) = derive_escrow_pda(program_id, &job_pda);

    let ix = create_job(
        program_id,
        creator,
        &job_pda,
        &escrow_pda,
        job_id,
        circuit_type,
        witness_hash,
        witness_size,
        price_lamports,
        timeout_seconds,
    );

    (ix, job_pda, escrow_pda)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_job_instruction() {
        let program_id = Pubkey::new_unique();
        let creator = Pubkey::new_unique();
        let job_pda = Pubkey::new_unique();
        let escrow_pda = Pubkey::new_unique();

        let ix = create_job(
            &program_id,
            &creator,
            &job_pda,
            &escrow_pda,
            12345,
            10,
            [0u8; 32],
            1024,
            100_000_000,
            3600,
        );

        assert_eq!(ix.program_id, program_id);
        assert_eq!(ix.accounts.len(), 4);
        assert!(ix.accounts[0].is_signer);
        assert!(ix.accounts[0].is_writable);
    }

    #[test]
    fn test_claim_job_instruction_no_cpi() {
        let program_id = Pubkey::new_unique();
        let prover = Pubkey::new_unique();
        let job_pda = Pubkey::new_unique();

        let ix = claim_job(&program_id, &prover, &job_pda, None, None);

        assert_eq!(ix.program_id, program_id);
        assert_eq!(ix.accounts.len(), 2); // Without CPI accounts
    }

    #[test]
    fn test_claim_job_instruction_with_cpi() {
        let program_id = Pubkey::new_unique();
        let prover = Pubkey::new_unique();
        let job_pda = Pubkey::new_unique();
        let prover_pda = Pubkey::new_unique();
        let bedrock_program = Pubkey::new_unique();

        let ix = claim_job(
            &program_id,
            &prover,
            &job_pda,
            Some(&prover_pda),
            Some(&bedrock_program),
        );

        assert_eq!(ix.program_id, program_id);
        assert_eq!(ix.accounts.len(), 4); // With CPI accounts
    }

    #[test]
    fn test_create_job_with_derived_pdas() {
        let program_id = Pubkey::new_unique();
        let creator = Pubkey::new_unique();
        let job_id = 12345u64;

        let (ix, job_pda, escrow_pda) = create_job_with_derived_pdas(
            &program_id,
            &creator,
            job_id,
            10,
            [0u8; 32],
            1024,
            100_000_000,
            3600,
        );

        // Verify PDAs are correctly derived
        let (expected_job_pda, _) = derive_job_pda(&program_id, &creator, job_id);
        let (expected_escrow_pda, _) = derive_escrow_pda(&program_id, &job_pda);

        assert_eq!(job_pda, expected_job_pda);
        assert_eq!(escrow_pda, expected_escrow_pda);
        assert_eq!(ix.program_id, program_id);
    }
}
