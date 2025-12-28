use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    sysvar,
};

#[allow(deprecated)]
use solana_sdk::system_program;
use crate::{
    accounts::*,
    error::Result,
    instruction::FutarchyInstruction,
    state::ExecutableAction,
};

pub fn build_create_market_ix(
    program_id: &Pubkey,
    authority: &Pubkey,
    market_id: u64,
    question_hash: [u8; 32],
    oracle: &Pubkey,
    end_time: i64,
    max_bet: u64,
) -> Result<Instruction> {
    let market_pda = find_market_pda(program_id, market_id);
    let escrow_pda = find_escrow_pda(program_id, market_id);

    let instruction_data = FutarchyInstruction::CreateMarket {
        market_id,
        question_hash,
        end_time,
        max_bet,
    };

    let accounts = vec![
        AccountMeta::new(*authority, true),
        AccountMeta::new(market_pda.address, false),
        AccountMeta::new(escrow_pda.address, false),
        AccountMeta::new_readonly(*oracle, false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: instruction_data.pack()?,
    })
}

pub fn build_place_bet_ix(
    program_id: &Pubkey,
    bettor: &Pubkey,
    market_id: u64,
    bet_commitment: [u8; 32],
    proof: Vec<u8>,
    public_inputs: Vec<u8>,
    amount: u64,
    circuit_type: u8,
    ciphertext_hash: Option<[u8; 32]>,
    encrypted_bet_amount: Option<Vec<u8>>,
    side: Option<bool>,
    zk_generator_program: &Pubkey,
    fhe_accounts: Option<FheAccounts>,
) -> Result<Instruction> {
    let market_pda = find_market_pda(program_id, market_id);
    let position_pda = find_position_pda(program_id, market_id, bettor);
    let user_escrow_pda = find_user_escrow_pda(program_id, bettor, market_id);
    let market_escrow_pda = find_escrow_pda(program_id, market_id);

    let instruction_data = FutarchyInstruction::PlaceBet {
        market_id,
        bet_commitment,
        proof,
        public_inputs,
        amount,
        circuit_type,
        ciphertext_hash,
        encrypted_bet_amount,
        side,
    };

    let mut accounts = vec![
        AccountMeta::new(*bettor, true),
        AccountMeta::new(market_pda.address, false),
        AccountMeta::new(position_pda.address, false),
        AccountMeta::new(user_escrow_pda.address, false),
        AccountMeta::new(market_escrow_pda.address, false),
        AccountMeta::new_readonly(*zk_generator_program, false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
    ];

    if let Some(fhe) = fhe_accounts {
        accounts.push(AccountMeta::new(fhe.fhe_job, false));
        accounts.push(AccountMeta::new(fhe.fhe_consensus, false));
        accounts.push(AccountMeta::new(fhe.fhe_escrow, false));
        accounts.push(AccountMeta::new_readonly(fhe.fhe_generator_program, false));
    }

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: instruction_data.pack()?,
    })
}

pub fn build_settle_market_ix(
    program_id: &Pubkey,
    oracle: &Pubkey,
    market_id: u64,
    outcome: bool,
) -> Result<Instruction> {
    let market_pda = find_market_pda(program_id, market_id);

    let instruction_data = FutarchyInstruction::SettleMarket {
        market_id,
        outcome,
    };

    let accounts = vec![
        AccountMeta::new(*oracle, true),
        AccountMeta::new(market_pda.address, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: instruction_data.pack()?,
    })
}

pub fn build_claim_payout_ix(
    program_id: &Pubkey,
    user: &Pubkey,
    market_id: u64,
    claim_nullifier: [u8; 32],
    proof: Vec<u8>,
    public_inputs: Vec<u8>,
    payout_amount: u64,
    zk_generator_program: &Pubkey,
    include_position: bool,
) -> Result<Instruction> {
    let market_pda = find_market_pda(program_id, market_id);
    let escrow_pda = find_escrow_pda(program_id, market_id);

    let instruction_data = FutarchyInstruction::ClaimPayout {
        market_id,
        claim_nullifier,
        proof,
        public_inputs,
        payout_amount,
    };

    let mut accounts = vec![
        AccountMeta::new(*user, true),
        AccountMeta::new(market_pda.address, false),
    ];

    if include_position {
        let position_pda = find_position_pda(program_id, market_id, user);
        accounts.push(AccountMeta::new(position_pda.address, false));
    }

    accounts.extend_from_slice(&[
        AccountMeta::new(escrow_pda.address, false),
        AccountMeta::new_readonly(*zk_generator_program, false),
        AccountMeta::new_readonly(system_program::id(), false),
    ]);

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: instruction_data.pack()?,
    })
}

pub fn build_update_pool_ix(
    program_id: &Pubkey,
    updater: &Pubkey,
    market_id: u64,
    fhe_job_id: u64,
    encrypted_result: Vec<u8>,
    side: bool,
    fhe_job_account: &Pubkey,
    fhe_consensus_account: &Pubkey,
) -> Result<Instruction> {
    let market_pda = find_market_pda(program_id, market_id);

    let instruction_data = FutarchyInstruction::UpdatePool {
        market_id,
        fhe_job_id,
        encrypted_result,
        side,
    };

    let accounts = vec![
        AccountMeta::new(*updater, true),
        AccountMeta::new(market_pda.address, false),
        AccountMeta::new_readonly(*fhe_job_account, false),
        AccountMeta::new_readonly(*fhe_consensus_account, false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: instruction_data.pack()?,
    })
}

pub fn build_register_user_ix(
    program_id: &Pubkey,
    user: &Pubkey,
    proof: Vec<u8>,
    public_inputs: Vec<u8>,
    blacklist_root: [u8; 32],
    blacklist_version: u32,
    zk_generator_program: &Pubkey,
) -> Result<Instruction> {
    let user_eligibility_pda = find_user_eligibility_pda(program_id, user);

    let instruction_data = FutarchyInstruction::RegisterUser {
        proof,
        public_inputs,
        blacklist_root,
        blacklist_version,
    };

    let accounts = vec![
        AccountMeta::new(*user, true),
        AccountMeta::new(user_eligibility_pda.address, false),
        AccountMeta::new_readonly(*zk_generator_program, false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: instruction_data.pack()?,
    })
}

pub fn build_create_market_with_governance_ix(
    program_id: &Pubkey,
    authority: &Pubkey,
    market_id: u64,
    question_hash: [u8; 32],
    oracle: &Pubkey,
    end_time: i64,
    max_bet: u64,
    executable_action: ExecutableAction,
    execution_threshold: u8,
    timelock_duration: i64,
) -> Result<Instruction> {
    let market_pda = find_market_pda(program_id, market_id);
    let escrow_pda = find_escrow_pda(program_id, market_id);

    let instruction_data = FutarchyInstruction::CreateMarketWithGovernance {
        market_id,
        question_hash,
        end_time,
        max_bet,
        executable_action,
        execution_threshold,
        timelock_duration,
    };

    let accounts = vec![
        AccountMeta::new(*authority, true),
        AccountMeta::new(market_pda.address, false),
        AccountMeta::new(escrow_pda.address, false),
        AccountMeta::new_readonly(*oracle, false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: instruction_data.pack()?,
    })
}

pub fn build_cancel_market_ix(
    program_id: &Pubkey,
    authority: &Pubkey,
    market_id: u64,
) -> Result<Instruction> {
    let market_pda = find_market_pda(program_id, market_id);

    let instruction_data = FutarchyInstruction::CancelMarket { market_id };

    let accounts = vec![
        AccountMeta::new(*authority, true),
        AccountMeta::new(market_pda.address, false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: instruction_data.pack()?,
    })
}

pub fn build_deposit_to_market_ix(
    program_id: &Pubkey,
    user: &Pubkey,
    market_id: u64,
    amount: u64,
) -> Result<Instruction> {
    let user_escrow_pda = find_user_escrow_pda(program_id, user, market_id);
    let market_pda = find_market_pda(program_id, market_id);

    let instruction_data = FutarchyInstruction::DepositToMarket { market_id, amount };

    let accounts = vec![
        AccountMeta::new(*user, true),
        AccountMeta::new(user_escrow_pda.address, false),
        AccountMeta::new_readonly(market_pda.address, false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: instruction_data.pack()?,
    })
}

pub fn build_withdraw_from_escrow_ix(
    program_id: &Pubkey,
    user: &Pubkey,
    market_id: u64,
    amount: u64,
) -> Result<Instruction> {
    let user_escrow_pda = find_user_escrow_pda(program_id, user, market_id);
    let market_pda = find_market_pda(program_id, market_id);

    let instruction_data = FutarchyInstruction::WithdrawFromEscrow { market_id, amount };

    let accounts = vec![
        AccountMeta::new(*user, true),
        AccountMeta::new(user_escrow_pda.address, false),
        AccountMeta::new_readonly(market_pda.address, false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: instruction_data.pack()?,
    })
}

pub fn prepare_unsigned_transaction(
    instructions: &[Instruction],
    payer: &Pubkey,
) -> Result<String> {
    use solana_sdk::{
        message::Message,
        transaction::Transaction,
    };
    use base64::{Engine as _, engine::general_purpose};

    let message = Message::new(instructions, Some(payer));
    let transaction = Transaction::new_unsigned(message);

    let serialized = bincode::serialize(&transaction)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    Ok(general_purpose::STANDARD.encode(&serialized))
}

#[derive(Debug, Clone)]
pub struct FheAccounts {
    pub fhe_job: Pubkey,
    pub fhe_consensus: Pubkey,
    pub fhe_escrow: Pubkey,
    pub fhe_generator_program: Pubkey,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_create_market_ix() {
        let program_id = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let oracle = Pubkey::new_unique();

        let ix = build_create_market_ix(
            &program_id,
            &authority,
            1,
            [0u8; 32],
            &oracle,
            1234567890,
            1_000_000_000,
        )
        .unwrap();

        assert_eq!(ix.program_id, program_id);
        assert_eq!(ix.accounts.len(), 6);
        assert!(ix.data.len() > 0);
    }

    #[test]
    fn test_build_settle_market_ix() {
        let program_id = Pubkey::new_unique();
        let oracle = Pubkey::new_unique();

        let ix = build_settle_market_ix(&program_id, &oracle, 1, true).unwrap();

        assert_eq!(ix.program_id, program_id);
        assert_eq!(ix.accounts.len(), 3);
        assert!(ix.data.len() > 0);
    }
}
