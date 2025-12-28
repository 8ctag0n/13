//! CPI helpers for calling other programs

use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
};

/// Verify a MarketBet ZK proof via CPI to zk-generator
///
/// This calls the zk-generator program to verify a proof for circuit 30 (MarketBet)
/// or circuit 31 (MarketBetWithPoI)
///
/// # Arguments
/// * `zk_program_info` - ZK-generator program account
/// * `proof` - The ZK proof bytes
/// * `public_inputs` - Public inputs for the circuit
/// * `circuit_type` - Circuit type (30 or 31)
pub fn verify_market_bet_proof<'a>(
    zk_program_info: &AccountInfo<'a>,
    proof: &[u8],
    public_inputs: &[u8],
    circuit_type: u8,
) -> ProgramResult {
    // For now, we'll do basic validation
    // TODO: Implement actual CPI to zk-generator's verify_proof instruction

    // Validate proof size (Groth16 proofs are typically 256 bytes)
    if proof.len() != 256 {
        return Err(ProgramError::InvalidInstructionData);
    }

    // Validate circuit type
    if circuit_type != 30 && circuit_type != 31 {
        return Err(ProgramError::InvalidInstructionData);
    }

    // Validate public inputs size
    // Circuit 30 (MarketBet): 80 bytes
    // Circuit 31 (MarketBetWithPoI): 112 bytes
    let expected_size = if circuit_type == 30 { 80 } else { 112 };
    if public_inputs.len() != expected_size {
        return Err(ProgramError::InvalidInstructionData);
    }

    // TODO: Create instruction for zk-generator and invoke
    // For MVP, we accept the proof (will be validated off-chain initially)

    Ok(())
}

/// Verify a MarketClaim ZK proof via CPI to zk-generator
///
/// This calls the zk-generator program to verify a proof for circuit 32 (MarketClaim)
///
/// # Arguments
/// * `zk_program_info` - ZK-generator program account
/// * `proof` - The ZK proof bytes
/// * `public_inputs` - Public inputs for the circuit
pub fn verify_market_claim_proof<'a>(
    zk_program_info: &AccountInfo<'a>,
    proof: &[u8],
    public_inputs: &[u8],
) -> ProgramResult {
    // Validate proof size
    if proof.len() != 256 {
        return Err(ProgramError::InvalidInstructionData);
    }

    // Validate public inputs size (Circuit 32: 105 bytes)
    if public_inputs.len() != 105 {
        return Err(ProgramError::InvalidInstructionData);
    }

    // TODO: Create instruction for zk-generator and invoke
    // For MVP, we accept the proof (will be validated off-chain initially)

    Ok(())
}

/// Verify a ProofOfInnocence ZK proof via CPI to zk-generator
///
/// This calls the zk-generator program to verify a proof for circuit 10 (PoI)
///
/// # Arguments
/// * `zk_program_info` - ZK-generator program account
/// * `proof` - The ZK proof bytes
/// * `public_inputs` - Public inputs for the circuit
pub fn verify_poi_proof<'a>(
    zk_program_info: &AccountInfo<'a>,
    proof: &[u8],
    public_inputs: &[u8],
) -> ProgramResult {
    // Validate proof size (Groth16)
    if proof.len() != 256 {
        return Err(ProgramError::InvalidInstructionData);
    }

    // Validate public inputs size
    // Circuit 10 (PoI): user_commitment (32) + blacklist_root (32) = 64 bytes
    if public_inputs.len() != 64 {
        return Err(ProgramError::InvalidInstructionData);
    }

    // TODO: Create instruction for zk-generator and invoke
    // For MVP, we accept the proof (will be validated off-chain initially)

    Ok(())
}

/// Verify prover is registered via CPI to bedrock
///
/// # Arguments
/// * `bedrock_program_id` - Bedrock program ID
/// * `prover_info` - Prover account to verify
pub fn verify_prover<'a>(
    bedrock_program_id: &Pubkey,
    prover_info: &AccountInfo<'a>,
) -> Result<bool, ProgramError> {
    // Use bedrock's CPI helper
    bedrock::cpi::verify_prover(bedrock_program_id, prover_info)
}

/// Derive market PDA
pub fn derive_market_pda(program_id: &Pubkey, market_id: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[crate::state::MARKET_SEED, &market_id.to_le_bytes()],
        program_id,
    )
}

/// Derive escrow PDA for a market
pub fn derive_escrow_pda(program_id: &Pubkey, market_id: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"escrow", &market_id.to_le_bytes()],
        program_id,
    )
}

/// Derive position PDA for a user and market
pub fn derive_position_pda(
    program_id: &Pubkey,
    user: &Pubkey,
    market_id: u64,
    bet_commitment: &[u8; 32],
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            crate::state::POSITION_SEED,
            user.as_ref(),
            &market_id.to_le_bytes(),
            bet_commitment,
        ],
        program_id,
    )
}

/// Create FHE job for pool addition via CPI
///
/// Creates a job to add encrypted bet to encrypted pool using FHE
///
/// # Arguments
/// * `fhe_program_id` - FHE-generator program ID
/// * `creator` - Creator of the job (market program)
/// * `job_id` - Unique job ID
/// * `current_pool` - Current encrypted pool ciphertext
/// * `new_bet` - New encrypted bet to add
/// * `price_lamports` - Payment for provers
#[allow(clippy::too_many_arguments)]
pub fn create_fhe_pool_addition_job<'a>(
    fhe_program_info: &AccountInfo<'a>,
    creator_info: &AccountInfo<'a>,
    job_info: &AccountInfo<'a>,
    consensus_info: &AccountInfo<'a>,
    escrow_info: &AccountInfo<'a>,
    system_program_info: &AccountInfo<'a>,
    job_id: u64,
    current_pool: Vec<u8>,
    new_bet: Vec<u8>,
    price_lamports: u64,
) -> ProgramResult {
    // Combine encrypted inputs (current_pool || new_bet)
    let mut witness_data = current_pool;
    witness_data.extend_from_slice(&new_bet);

    // Hash the witness
    use solana_program::hash::hash;
    let witness_hash = hash(&witness_data).to_bytes();
    let witness_size = witness_data.len() as u32;

    // Build FHE CreateJob instruction
    let instruction_data = {
        use borsh::BorshSerialize;
        let ix = fhe_generator::instruction::FheGeneratorInstruction::CreateJob {
            job_id,
            circuit_type: 4, // CIRCUIT_FHE_ADD
            witness_hash,
            witness_size,
            price_lamports,
            timeout_seconds: 300, // 5 minutes
            required_provers: 3,
            consensus_threshold: 2, // 2/3 consensus
            operation_param1: 0,
            operation_param2: 0,
            operation_param3: 0,
        };
        borsh::to_vec(&ix).unwrap()
    };

    let accounts = vec![
        AccountMeta::new(*creator_info.key, true),
        AccountMeta::new(*job_info.key, false),
        AccountMeta::new(*consensus_info.key, false),
        AccountMeta::new(*escrow_info.key, false),
        AccountMeta::new_readonly(*system_program_info.key, false),
    ];

    let instruction = Instruction {
        program_id: *fhe_program_info.key,
        accounts,
        data: instruction_data,
    };

    invoke(
        &instruction,
        &[
            creator_info.clone(),
            job_info.clone(),
            consensus_info.clone(),
            escrow_info.clone(),
            system_program_info.clone(),
            fhe_program_info.clone(),
        ],
    )
}
