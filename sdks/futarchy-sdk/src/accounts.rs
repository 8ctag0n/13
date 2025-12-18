use solana_program::pubkey::Pubkey;
use crate::state::*;

pub struct PdaDerivation {
    pub address: Pubkey,
    pub bump: u8,
}

pub fn find_market_pda(program_id: &Pubkey, market_id: u64) -> PdaDerivation {
    let (address, bump) = Pubkey::find_program_address(
        &[MARKET_SEED, &market_id.to_le_bytes()],
        program_id,
    );
    PdaDerivation { address, bump }
}

pub fn find_position_pda(
    program_id: &Pubkey,
    market_id: u64,
    user: &Pubkey,
) -> PdaDerivation {
    let (address, bump) = Pubkey::find_program_address(
        &[
            POSITION_SEED,
            &market_id.to_le_bytes(),
            user.as_ref(),
        ],
        program_id,
    );
    PdaDerivation { address, bump }
}

pub fn find_escrow_pda(program_id: &Pubkey, market_id: u64) -> PdaDerivation {
    let (address, bump) = Pubkey::find_program_address(
        &[ESCROW_SEED, &market_id.to_le_bytes()],
        program_id,
    );
    PdaDerivation { address, bump }
}

pub fn find_user_eligibility_pda(program_id: &Pubkey, user: &Pubkey) -> PdaDerivation {
    let (address, bump) = Pubkey::find_program_address(
        &[USER_ELIGIBILITY_SEED, user.as_ref()],
        program_id,
    );
    PdaDerivation { address, bump }
}

pub fn find_user_escrow_pda(
    program_id: &Pubkey,
    user: &Pubkey,
    market_id: u64,
) -> PdaDerivation {
    let (address, bump) = Pubkey::find_program_address(
        &[
            USER_ESCROW_SEED,
            user.as_ref(),
            &market_id.to_le_bytes(),
        ],
        program_id,
    );
    PdaDerivation { address, bump }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_pda_derivation() {
        let program_id = Pubkey::new_unique();
        let market_id = 42u64;

        let pda1 = find_market_pda(&program_id, market_id);
        let pda2 = find_market_pda(&program_id, market_id);

        assert_eq!(pda1.address, pda2.address);
        assert_eq!(pda1.bump, pda2.bump);
    }

    #[test]
    fn test_position_pda_derivation() {
        let program_id = Pubkey::new_unique();
        let market_id = 42u64;
        let user = Pubkey::new_unique();

        let pda1 = find_position_pda(&program_id, market_id, &user);
        let pda2 = find_position_pda(&program_id, market_id, &user);

        assert_eq!(pda1.address, pda2.address);
        assert_eq!(pda1.bump, pda2.bump);
    }

    #[test]
    fn test_escrow_pda_derivation() {
        let program_id = Pubkey::new_unique();
        let market_id = 42u64;

        let pda1 = find_escrow_pda(&program_id, market_id);
        let pda2 = find_escrow_pda(&program_id, market_id);

        assert_eq!(pda1.address, pda2.address);
        assert_eq!(pda1.bump, pda2.bump);
    }
}
