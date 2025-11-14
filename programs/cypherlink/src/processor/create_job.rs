use cypherlink_types::{CircuitType, FheConsensusConfig};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::{clock::Clock, Sysvar},
};

use crate::{
    error::CypherLinkProgramError,
    state::{JobAccount, MarketplaceConfig},
};

/// Process CreateJob instruction
#[allow(clippy::too_many_arguments)]
pub fn process_create_job(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    circuit_type: CircuitType,
    witness_commitment: [u8; 32],
    witness_size: u32,
    price_lamports: u64,
    timeout_seconds: i64,
    fhe_config: Option<FheConsensusConfig>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let creator_info = next_account_info(account_info_iter)?;
    let job_info = next_account_info(account_info_iter)?;
    let config_info = next_account_info(account_info_iter)?;
    let escrow_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;

    // Verify creator is signer
    if !creator_info.is_signer {
        msg!("Creator must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load and verify marketplace config
    let (config_pda, _) = Pubkey::find_program_address(&[b"config"], program_id);
    if config_info.key != &config_pda {
        msg!("Invalid config account");
        return Err(CypherLinkProgramError::InvalidAccount.into());
    }

    // Check marketplace is initialized
    if config_info.owner != program_id {
        msg!("Marketplace not initialized");
        return Err(CypherLinkProgramError::MarketplaceNotInitialized.into());
    }

    // Deserialize config to get next job ID
    let mut config: MarketplaceConfig = borsh::from_slice(&config_info.data.borrow())?;

    // Check marketplace is not paused
    if config.is_paused {
        msg!("Marketplace is paused");
        return Err(CypherLinkProgramError::MarketplacePaused.into());
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
        return Err(CypherLinkProgramError::InvalidAccount.into());
    }

    // Derive and verify escrow PDA
    let (escrow_pda, escrow_bump) =
        Pubkey::find_program_address(&[b"escrow", job_pda.as_ref()], program_id);

    if escrow_info.key != &escrow_pda {
        msg!("Invalid escrow account");
        return Err(CypherLinkProgramError::InvalidAccount.into());
    }

    // Validate price
    if price_lamports == 0 {
        msg!("Price must be greater than zero");
        return Err(CypherLinkProgramError::InvalidPrice.into());
    }

    // Validate FHE configuration
    match &circuit_type {
        CircuitType::FheComputation(_) => {
            // FHE job MUST have config
            let config = fhe_config
                .as_ref()
                .ok_or(CypherLinkProgramError::MissingFheConfig)?;

            // Validate config
            config.validate()
                .map_err(|_| CypherLinkProgramError::InvalidFheConfig)?;

            // Validate minimum price for multi-prover
            let min_price = config.required_provers as u64 * 1_000_000; // 0.001 SOL per prover
            if price_lamports < min_price {
                msg!("Price too low for FHE job: {} < {}", price_lamports, min_price);
                return Err(CypherLinkProgramError::InvalidPrice.into());
            }
        }
        _ => {
            // Non-FHE job should NOT have config
            if fhe_config.is_some() {
                msg!("Non-FHE job should not have FHE config");
                return Err(CypherLinkProgramError::UnexpectedFheConfig.into());
            }
        }
    }

    // Get current time
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;

    // Determine timeout
    let actual_timeout = if timeout_seconds > 0 {
        timeout_seconds
    } else {
        config.default_job_timeout_seconds
    };

    // Calculate rent for job account
    let rent = Rent::get()?;
    let job_rent_lamports = rent.minimum_balance(JobAccount::LEN);

    msg!("Creating job account");

    // Create job account
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
        &[&[b"job", creator_info.key.as_ref(), &job_id_bytes, &[job_bump]]],
    )?;

    msg!("Creating escrow account");

    // Create escrow account - needs to hold the price
    let escrow_rent_lamports = rent.minimum_balance(0); // Escrow holds no data, just lamports

    invoke_signed(
        &system_instruction::create_account(
            creator_info.key,
            escrow_info.key,
            escrow_rent_lamports + price_lamports, // Rent + payment
            0,
            program_id,
        ),
        &[
            creator_info.clone(),
            escrow_info.clone(),
            system_program_info.clone(),
        ],
        &[&[b"escrow", job_pda.as_ref(), &[escrow_bump]]],
    )?;

    // Initialize job account
    let job = JobAccount::new(
        job_id,
        *creator_info.key,
        circuit_type.clone(),
        witness_commitment,
        witness_size,
        price_lamports,
        escrow_pda,
        current_time,
        actual_timeout,
        job_bump,
        fhe_config,
    );

    // Serialize job to account
    let mut job_data = job_info.try_borrow_mut_data()?;
    borsh::to_writer(&mut job_data[..], &job)?;

    // Update config with new job counter
    let mut config_data = config_info.try_borrow_mut_data()?;
    borsh::to_writer(&mut config_data[..], &config)?;

    msg!("Job created successfully");
    msg!("  Job ID: {}", job_id);
    msg!("  Circuit: {:?}", circuit_type);
    msg!("  Price: {} lamports", price_lamports);
    msg!("  Timeout: {} seconds", actual_timeout);

    Ok(())
}
