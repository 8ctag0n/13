use cypherlink_types::{CircuitType, FheConsensusConfig};
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

// Token account size constant
const TOKEN_ACCOUNT_LEN: usize = 165;

// SPL Token instruction discriminators
const SPL_TOKEN_INITIALIZE_ACCOUNT: u8 = 1;
const SPL_TOKEN_TRANSFER: u8 = 3;

use crate::{
    error::CypherLinkProgramError,
    state::{JobAccount, MarketplaceConfig},
};

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
        return Err(CypherLinkProgramError::InvalidAccount.into());
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

    // Validate price
    if price_token_amount == 0 {
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
            config
                .validate()
                .map_err(|_| CypherLinkProgramError::InvalidFheConfig)?;
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

    msg!("Creating job account with token payment");

    // Create job account
    invoke(
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
    )?;

    msg!("Creating and initializing token escrow account");

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
        &[&[b"token_escrow", job_pda.as_ref(), &[job_bump]]],
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

    // Initialize job account
    // Note: We store the token escrow address instead of SOL escrow
    let job = JobAccount::new(
        job_id,
        *creator_info.key,
        circuit_type.clone(),
        witness_commitment,
        witness_size,
        price_token_amount,     // Store token amount instead of lamports
        *token_escrow_info.key, // Store token escrow address
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

    msg!("Job created successfully with token payment");
    msg!("  Job ID: {}", job_id);
    msg!("  Circuit: {:?}", circuit_type);
    msg!("  Price: {} tokens", price_token_amount);
    msg!("  Token mint: {}", token_mint_info.key);
    msg!("  Timeout: {} seconds", actual_timeout);

    Ok(())
}
