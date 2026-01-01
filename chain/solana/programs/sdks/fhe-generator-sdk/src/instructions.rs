//! Instruction builders for FHE Generator program

use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
};

use crate::{derive_consensus_pda, derive_escrow_pda, derive_job_pda, FheGeneratorInstruction};

/// Create a new FHE computation job
///
/// # Arguments
/// * `program_id` - FHE Generator program ID
/// * `creator` - Job creator wallet (signer, payer)
/// * `job_id` - Job ID (must be unique per creator)
/// * `circuit_type` - FHE circuit type (4-11)
/// * `witness_hash` - Hash of encrypted witness data
/// * `witness_size` - Size of witness data in bytes
/// * `price_lamports` - Payment amount in lamports
/// * `timeout_seconds` - Job timeout in seconds
/// * `required_provers` - Number of provers required (2-5)
/// * `consensus_threshold` - Minimum matching results for consensus
/// * `operation_param1` - Operation-specific parameter 1
/// * `operation_param2` - Operation-specific parameter 2
/// * `operation_param3` - Operation-specific parameter 3
#[allow(clippy::too_many_arguments)]
pub fn create_job(
    program_id: &Pubkey,
    creator: &Pubkey,
    job_id: u64,
    circuit_type: u8,
    witness_hash: [u8; 32],
    witness_size: u32,
    price_lamports: u64,
    timeout_seconds: i64,
    required_provers: u8,
    consensus_threshold: u8,
    operation_param1: u16,
    operation_param2: u8,
    operation_param3: u8,
) -> Instruction {
    let (job_pda, _) = derive_job_pda(program_id, creator, job_id);
    let (consensus_pda, _) = derive_consensus_pda(program_id, job_id);
    let (escrow_pda, _) = derive_escrow_pda(program_id, &job_pda);

    let instruction_data = FheGeneratorInstruction::CreateJob {
        job_id,
        circuit_type,
        witness_hash,
        witness_size,
        price_lamports,
        timeout_seconds,
        required_provers,
        consensus_threshold,
        operation_param1,
        operation_param2,
        operation_param3,
    };

    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*creator, true),              // Creator (signer, payer)
            AccountMeta::new(job_pda, false),              // FheJob PDA
            AccountMeta::new(consensus_pda, false),        // FheConsensusData PDA
            AccountMeta::new(escrow_pda, false),           // Escrow PDA
            AccountMeta::new_readonly(system_program::id(), false), // System program
        ],
        data: borsh::to_vec(&instruction_data).expect("Failed to serialize CreateJob instruction"),
    }
}

/// Claim a job as a prover
///
/// # Arguments
/// * `program_id` - FHE Generator program ID
/// * `prover` - Prover wallet (signer)
/// * `job_pda` - FheJob PDA address
/// * `consensus_pda` - FheConsensusData PDA address
/// * `prover_pda_bedrock` - Prover PDA in Bedrock program
/// * `bedrock_program` - Bedrock program ID
pub fn claim_job(
    program_id: &Pubkey,
    prover: &Pubkey,
    job_pda: &Pubkey,
    consensus_pda: &Pubkey,
    prover_pda_bedrock: &Pubkey,
    bedrock_program: &Pubkey,
) -> Instruction {
    let instruction_data = FheGeneratorInstruction::ClaimJob;

    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*prover, true),                 // Prover wallet (signer)
            AccountMeta::new(*job_pda, false),               // FheJob PDA
            AccountMeta::new(*consensus_pda, false),         // FheConsensusData PDA
            AccountMeta::new_readonly(*prover_pda_bedrock, false), // Prover PDA (bedrock)
            AccountMeta::new_readonly(*bedrock_program, false),    // Bedrock program
        ],
        data: borsh::to_vec(&instruction_data).expect("Failed to serialize ClaimJob instruction"),
    }
}

/// Submit computation result for a claimed job
///
/// # Arguments
/// * `program_id` - FHE Generator program ID
/// * `prover` - Prover wallet (signer)
/// * `job_pda` - FheJob PDA address
/// * `consensus_pda` - FheConsensusData PDA address
/// * `result_hash` - Hash of the computation result
pub fn submit_result(
    program_id: &Pubkey,
    prover: &Pubkey,
    job_pda: &Pubkey,
    consensus_pda: &Pubkey,
    result_hash: [u8; 32],
) -> Instruction {
    let instruction_data = FheGeneratorInstruction::SubmitResult { result_hash };

    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*prover, true),         // Prover wallet (signer)
            AccountMeta::new_readonly(*job_pda, false), // FheJob PDA (read-only)
            AccountMeta::new(*consensus_pda, false), // FheConsensusData PDA
        ],
        data: borsh::to_vec(&instruction_data).expect("Failed to serialize SubmitResult instruction"),
    }
}

/// Finalize job and distribute payments
///
/// # Arguments
/// * `program_id` - FHE Generator program ID
/// * `finalizer` - Anyone can trigger finalization (signer)
/// * `job_pda` - FheJob PDA address
/// * `consensus_pda` - FheConsensusData PDA address
/// * `escrow_pda` - Escrow PDA address
/// * `creator` - Job creator wallet (for refunds)
/// * `fee_recipient` - Protocol fee recipient
/// * `bedrock_program` - Bedrock program ID
/// * `bedrock_config` - Bedrock config PDA
/// * `prover_wallets` - List of prover wallets (in claim order)
/// * `prover_pdas` - List of prover PDAs in Bedrock (in claim order)
///
/// Note: prover_wallets and prover_pdas must be in the same order as they claimed the job
#[allow(clippy::too_many_arguments)]
pub fn finalize_job(
    program_id: &Pubkey,
    finalizer: &Pubkey,
    job_pda: &Pubkey,
    consensus_pda: &Pubkey,
    escrow_pda: &Pubkey,
    creator: &Pubkey,
    fee_recipient: &Pubkey,
    bedrock_program: &Pubkey,
    bedrock_config: &Pubkey,
    prover_wallets: &[Pubkey],
    prover_pdas: &[Pubkey],
) -> Instruction {
    let instruction_data = FheGeneratorInstruction::FinalizeJob;

    let mut accounts = vec![
        AccountMeta::new(*finalizer, true),                 // Finalizer (signer)
        AccountMeta::new(*job_pda, false),                  // FheJob PDA
        AccountMeta::new(*consensus_pda, false),            // FheConsensusData PDA
        AccountMeta::new(*escrow_pda, false),               // Escrow PDA
        AccountMeta::new(*creator, false),                  // Creator wallet
        AccountMeta::new(*fee_recipient, false),            // Fee recipient
        AccountMeta::new_readonly(*bedrock_program, false), // Bedrock program
        AccountMeta::new_readonly(*bedrock_config, false),  // Bedrock config
    ];

    // Add prover wallets
    for wallet in prover_wallets {
        accounts.push(AccountMeta::new(*wallet, false));
    }

    // Add prover PDAs
    for pda in prover_pdas {
        accounts.push(AccountMeta::new(*pda, false));
    }

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction_data).expect("Failed to serialize FinalizeJob instruction"),
    }
}

/// Cancel a pending job
///
/// # Arguments
/// * `program_id` - FHE Generator program ID
/// * `creator` - Job creator wallet (signer)
/// * `job_pda` - FheJob PDA address
/// * `consensus_pda` - FheConsensusData PDA address
/// * `escrow_pda` - Escrow PDA address
pub fn cancel_job(
    program_id: &Pubkey,
    creator: &Pubkey,
    job_pda: &Pubkey,
    consensus_pda: &Pubkey,
    escrow_pda: &Pubkey,
) -> Instruction {
    let instruction_data = FheGeneratorInstruction::CancelJob;

    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*creator, true),        // Creator wallet (signer)
            AccountMeta::new(*job_pda, false),       // FheJob PDA
            AccountMeta::new(*consensus_pda, false), // FheConsensusData PDA
            AccountMeta::new(*escrow_pda, false),    // Escrow PDA
        ],
        data: borsh::to_vec(&instruction_data).expect("Failed to serialize CancelJob instruction"),
    }
}
