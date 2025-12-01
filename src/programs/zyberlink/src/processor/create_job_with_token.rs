use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    instruction::Instruction,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::{clock::Clock, Sysvar},
};
use zyberlink_types::{CircuitType, FheConsensusConfig, FheOperation};

// Token account size constant
const TOKEN_ACCOUNT_LEN: usize = 165;

// SPL Token instruction discriminators
const SPL_TOKEN_INITIALIZE_ACCOUNT: u8 = 1;
const SPL_TOKEN_TRANSFER: u8 = 3;

use crate::{
    error::ZyberLinkProgramError,
    state::{FheConsensusData, JobAccount, MarketplaceConfig},
};

/// Convert CircuitType enum to u8 ID
fn circuit_type_to_id(circuit_type: &CircuitType) -> u8 {
    match circuit_type {
        CircuitType::ZcashOrchard => JobAccount::CIRCUIT_ZCASH_ORCHARD,
        CircuitType::AnonymousVote => JobAccount::CIRCUIT_ANONYMOUS_VOTE,
        CircuitType::Credential => JobAccount::CIRCUIT_CREDENTIAL,
        CircuitType::FheComputation(op) => fhe_operation_to_circuit_id(op),
        CircuitType::Custom(_) => 255, // Custom
    }
}

/// Convert FheOperation to circuit type ID
fn fhe_operation_to_circuit_id(op: &FheOperation) -> u8 {
    match op {
        FheOperation::Add(_) => JobAccount::CIRCUIT_FHE_ADD,
        FheOperation::Multiply(_) => JobAccount::CIRCUIT_FHE_MULTIPLY,
        FheOperation::Sum { .. } => JobAccount::CIRCUIT_FHE_SUM,
        FheOperation::Threshold { .. } => JobAccount::CIRCUIT_FHE_THRESHOLD,
        FheOperation::RangeCheck { .. } => JobAccount::CIRCUIT_FHE_RANGE_CHECK,
        FheOperation::Average { .. } => JobAccount::CIRCUIT_FHE_AVERAGE,
        FheOperation::CountIf { .. } => JobAccount::CIRCUIT_FHE_COUNT_IF,
        FheOperation::Histogram { .. } => JobAccount::CIRCUIT_FHE_HISTOGRAM,
    }
}

/// Extract packed parameters from FheOperation
fn pack_fhe_params(op: &FheOperation) -> (u16, u8, u8) {
    match op {
        FheOperation::Add(v) => (*v as u16, 0, 0),
        FheOperation::Multiply(v) => (*v as u16, 0, 0),
        FheOperation::Sum { expected_count } => (*expected_count, 0, 0),
        FheOperation::Threshold {
            threshold,
            greater_or_equal,
        } => (*threshold as u16, if *greater_or_equal { 1 } else { 0 }, 0),
        FheOperation::RangeCheck { min, max } => (0, *min, *max),
        FheOperation::Average { expected_count } => (*expected_count, 0, 0),
        FheOperation::CountIf { expected_count, .. } => (*expected_count, 0, 0),
        FheOperation::Histogram { bins } => (bins.len() as u16, 0, 0),
    }
}

/// Process CreateJobWithToken instruction for SPL token payments (e.g. wZEC)
#[allow(clippy::too_many_arguments)]
pub fn process_create_job_with_token(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    circuit_type: CircuitType,
    witness_commitment: [u8; 32],
    witness_size: u32,
    price_token_amount: u64,
    timeout_seconds: i64,
    fhe_config: Option<FheConsensusConfig>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let creator_info = next_account_info(account_info_iter)?;
    let job_info = next_account_info(account_info_iter)?;
    let config_info = next_account_info(account_info_iter)?;
    let token_escrow_info = next_account_info(account_info_iter)?;
    let creator_token_account_info = next_account_info(account_info_iter)?;
    let token_mint_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;
    let rent_sysvar_info = next_account_info(account_info_iter)?;

    // For FHE jobs, we need an additional account for FheConsensusData
    let fhe_consensus_info = if matches!(circuit_type, CircuitType::FheComputation(_)) {
        Some(next_account_info(account_info_iter)?)
    } else {
        None
    };

    // Verify creator is signer
    if !creator_info.is_signer {
        msg!("Creator must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // GATEKEEPING: Only accept wZEC token
    // TODO: Make this configurable per environment (localhost/devnet/mainnet)
    let wzec_mint = match Pubkey::try_from("sXpG9BWgA6hxz9BTVLNTqWSHpbbQKa2LqKH6qD2fCAZ") {
        Ok(pubkey) => pubkey,
        Err(_) => return Err(ProgramError::InvalidArgument),
    };

    if token_mint_info.key != &wzec_mint {
        msg!("Only wZEC token is accepted for payment");
        msg!("  Expected mint: {}", wzec_mint);
        msg!("  Provided mint: {}", token_mint_info.key);
        return Err(ZyberLinkProgramError::InvalidAccount.into());
    }

    // Load and verify marketplace config
    let (config_pda, _) = Pubkey::find_program_address(&[b"config"], program_id);
    if config_info.key != &config_pda {
        msg!("Invalid config account");
        return Err(ZyberLinkProgramError::InvalidAccount.into());
    }

    // Check marketplace is initialized
    if config_info.owner != program_id {
        msg!("Marketplace not initialized");
        return Err(ZyberLinkProgramError::MarketplaceNotInitialized.into());
    }

    // Deserialize config to get next job ID
    let mut config: MarketplaceConfig = borsh::from_slice(&config_info.data.borrow())?;

    // Check marketplace is not paused
    if config.is_paused {
        msg!("Marketplace is paused");
        return Err(ZyberLinkProgramError::MarketplacePaused.into());
    }

    // Get next job ID
    let job_id = config.next_job_id();

    // Derive and verify job PDA
    let job_id_bytes = job_id.to_le_bytes();
    let (job_pda, job_bump) = Pubkey::find_program_address(
        &[b"job", creator_info.key.as_ref(), &job_id_bytes],
        program_id,
    );

    if job_info.key != &job_pda {
        msg!("Invalid job account");
        return Err(ZyberLinkProgramError::InvalidAccount.into());
    }

    // Validate price
    if price_token_amount == 0 {
        msg!("Price must be greater than zero");
        return Err(ZyberLinkProgramError::InvalidPrice.into());
    }

    // Convert circuit type to ID
    let circuit_type_id = circuit_type_to_id(&circuit_type);

    // Validate FHE configuration and get bump if FHE job
    let fhe_consensus_bump = match &circuit_type {
        CircuitType::FheComputation(fhe_op) => {
            // FHE job MUST have config
            let fhe_consensus_config = fhe_config
                .as_ref()
                .ok_or(ZyberLinkProgramError::MissingFheConfig)?;

            // Validate consensus config
            fhe_consensus_config
                .validate()
                .map_err(|_| ZyberLinkProgramError::InvalidFheConfig)?;

            // Verify FheConsensusData account was provided
            let fhe_info =
                fhe_consensus_info.ok_or(ZyberLinkProgramError::MissingFheConsensusAccount)?;

            // Derive and verify FheConsensusData PDA
            let (fhe_pda, fhe_bump) =
                Pubkey::find_program_address(&[b"fhe_consensus", &job_id_bytes], program_id);

            if fhe_info.key != &fhe_pda {
                msg!("Invalid FHE consensus account");
                return Err(ZyberLinkProgramError::InvalidAccount.into());
            }

            // Get dynamic cost configuration based on operation complexity
            let cost_config = fhe_op.get_cost_config();

            // Calculate minimum price: operation cost × number of provers
            let min_price_per_prover = cost_config.min_payment_lamports;
            let total_min_price =
                min_price_per_prover * (fhe_consensus_config.required_provers as u64);

            if price_token_amount < total_min_price {
                msg!(
                    "Price too low for FHE operation '{}' (tier {}): {} < {} ({}×{} provers)",
                    fhe_op.name(),
                    cost_config.complexity_tier,
                    price_token_amount,
                    total_min_price,
                    min_price_per_prover,
                    fhe_consensus_config.required_provers
                );
                return Err(ZyberLinkProgramError::InvalidPrice.into());
            }

            msg!(
                "FHE job pricing validated - Op: {}, Tier: {}, Min: {} tokens",
                fhe_op.name(),
                cost_config.complexity_tier,
                total_min_price
            );

            Some(fhe_bump)
        }
        _ => {
            // Non-FHE job should NOT have config
            if fhe_config.is_some() {
                msg!("Non-FHE job should not have FHE config");
                return Err(ZyberLinkProgramError::UnexpectedFheConfig.into());
            }
            None
        }
    };

    // Get current time
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;

    // Determine timeout with dynamic calculation for FHE operations
    let actual_timeout = if timeout_seconds > 0 {
        timeout_seconds
    } else {
        match &circuit_type {
            CircuitType::FheComputation(fhe_op) => {
                let cost_config = fhe_op.get_cost_config();
                msg!(
                    "Using dynamic timeout for {} operation: {} seconds",
                    fhe_op.name(),
                    cost_config.timeout_seconds
                );
                cost_config.timeout_seconds
            }
            _ => config.default_job_timeout_seconds,
        }
    };

    // Calculate rent for job account
    let rent = Rent::get()?;
    let job_rent_lamports = rent.minimum_balance(JobAccount::LEN);

    msg!("Creating job account with token payment");

    // Create job account using invoke_signed (since job is a PDA)
    invoke_signed(
        &system_instruction::create_account(
            creator_info.key,
            job_info.key,
            job_rent_lamports,
            JobAccount::LEN as u64,
            program_id,
        ),
        &[
            creator_info.clone(),
            job_info.clone(),
            system_program_info.clone(),
        ],
        &[&[
            b"job",
            creator_info.key.as_ref(),
            &job_id_bytes,
            &[job_bump],
        ]],
    )?;

    msg!("Creating and initializing token escrow account");

    // Derive token escrow PDA
    let (token_escrow_pda, token_escrow_bump) =
        Pubkey::find_program_address(&[b"token_escrow", job_pda.as_ref()], program_id);

    if token_escrow_info.key != &token_escrow_pda {
        msg!("Invalid token escrow account");
        return Err(ZyberLinkProgramError::InvalidAccount.into());
    }

    // Calculate rent for token account
    let token_account_rent = rent.minimum_balance(TOKEN_ACCOUNT_LEN);

    // Create token escrow account
    invoke_signed(
        &system_instruction::create_account(
            creator_info.key,
            token_escrow_info.key,
            token_account_rent,
            TOKEN_ACCOUNT_LEN as u64,
            token_program_info.key,
        ),
        &[
            creator_info.clone(),
            token_escrow_info.clone(),
            system_program_info.clone(),
        ],
        &[&[b"token_escrow", job_pda.as_ref(), &[token_escrow_bump]]],
    )?;

    // Initialize token account manually
    let init_account_ix = Instruction {
        program_id: *token_program_info.key,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(*token_escrow_info.key, false),
            solana_program::instruction::AccountMeta::new_readonly(*token_mint_info.key, false),
            solana_program::instruction::AccountMeta::new_readonly(job_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(*rent_sysvar_info.key, false),
        ],
        data: vec![SPL_TOKEN_INITIALIZE_ACCOUNT],
    };

    invoke(
        &init_account_ix,
        &[
            token_escrow_info.clone(),
            token_mint_info.clone(),
            job_info.clone(),
            rent_sysvar_info.clone(),
        ],
    )?;

    msg!("Transferring tokens to escrow");

    // Transfer tokens from creator to escrow manually
    let mut transfer_data = vec![SPL_TOKEN_TRANSFER];
    transfer_data.extend_from_slice(&price_token_amount.to_le_bytes());

    let transfer_ix = Instruction {
        program_id: *token_program_info.key,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(*creator_token_account_info.key, false),
            solana_program::instruction::AccountMeta::new(*token_escrow_info.key, false),
            solana_program::instruction::AccountMeta::new_readonly(*creator_info.key, true),
        ],
        data: transfer_data,
    };

    invoke(
        &transfer_ix,
        &[
            creator_token_account_info.clone(),
            token_escrow_info.clone(),
            creator_info.clone(),
            token_program_info.clone(),
        ],
    )?;

    // Create FheConsensusData account if FHE job
    if let (Some(fhe_info), Some(fhe_bump), CircuitType::FheComputation(fhe_op)) =
        (fhe_consensus_info, fhe_consensus_bump, &circuit_type)
    {
        let fhe_cfg = fhe_config.as_ref().unwrap();
        let fhe_rent_lamports = rent.minimum_balance(FheConsensusData::LEN);

        msg!(
            "Creating FHE consensus account ({} bytes)",
            FheConsensusData::LEN
        );

        invoke_signed(
            &system_instruction::create_account(
                creator_info.key,
                fhe_info.key,
                fhe_rent_lamports,
                FheConsensusData::LEN as u64,
                program_id,
            ),
            &[
                creator_info.clone(),
                fhe_info.clone(),
                system_program_info.clone(),
            ],
            &[&[b"fhe_consensus", &job_id_bytes, &[fhe_bump]]],
        )?;

        // Initialize FheConsensusData
        let (param1, param2, param3) = pack_fhe_params(fhe_op);
        let fhe_data = FheConsensusData::new(
            job_id,
            circuit_type_id,
            param1,
            param2,
            param3,
            fhe_cfg.required_provers,
            fhe_cfg.consensus_threshold,
            current_time + fhe_cfg.submission_timeout_secs,
            fhe_bump,
        );

        let mut fhe_account_data = fhe_info.try_borrow_mut_data()?;
        borsh::to_writer(&mut fhe_account_data[..], &fhe_data)?;
    }

    // Initialize job account
    let job = if let Some(fhe_bump) = fhe_consensus_bump {
        JobAccount::new_fhe(
            job_id,
            *creator_info.key,
            circuit_type_id,
            witness_commitment,
            witness_size,
            price_token_amount,     // Store token amount instead of lamports
            *token_escrow_info.key, // Store token escrow address
            current_time,
            actual_timeout,
            job_bump,
            fhe_bump,
        )
    } else {
        JobAccount::new_zk(
            job_id,
            *creator_info.key,
            circuit_type_id,
            witness_commitment,
            witness_size,
            price_token_amount,
            *token_escrow_info.key,
            current_time,
            actual_timeout,
            job_bump,
        )
    };

    // Serialize job to account
    let mut job_data = job_info.try_borrow_mut_data()?;
    borsh::to_writer(&mut job_data[..], &job)?;

    // Update config with new job counter
    let mut config_data = config_info.try_borrow_mut_data()?;
    borsh::to_writer(&mut config_data[..], &config)?;

    msg!("Job created successfully with token payment");
    msg!("  Job ID: {}", job_id);
    msg!("  Circuit Type ID: {}", circuit_type_id);
    msg!("  Price: {} tokens", price_token_amount);
    msg!("  Token mint: {}", token_mint_info.key);
    msg!("  Timeout: {} seconds", actual_timeout);
    if fhe_consensus_bump.is_some() {
        msg!("  FHE Consensus: enabled");
    }

    Ok(())
}
