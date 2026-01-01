//! PDA (Program Derived Address) derivation utilities

use solana_program::pubkey::Pubkey;

use bedrock::{CONFIG_SEED, PROVER_SEED, VALIDATOR_SEED};

/// Derive the config PDA for the bedrock program
///
/// # Arguments
/// * `program_id` - The bedrock program ID
///
/// # Returns
/// * `(Pubkey, u8)` - The derived PDA and bump seed
///
/// # Example
/// ```
/// use bedrock_sdk::derive_config_pda;
/// use solana_sdk::pubkey::Pubkey;
///
/// let program_id = Pubkey::default();
/// let (config_pda, bump) = derive_config_pda(&program_id);
/// ```
pub fn derive_config_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[CONFIG_SEED], program_id)
}

/// Derive the prover account PDA
///
/// # Arguments
/// * `program_id` - The bedrock program ID
/// * `prover_wallet` - The prover's wallet address
///
/// # Returns
/// * `(Pubkey, u8)` - The derived PDA and bump seed
///
/// # Example
/// ```
/// use bedrock_sdk::derive_prover_pda;
/// use solana_sdk::pubkey::Pubkey;
///
/// let program_id = Pubkey::default();
/// let prover_wallet = Pubkey::default();
/// let (prover_pda, bump) = derive_prover_pda(&program_id, &prover_wallet);
/// ```
pub fn derive_prover_pda(program_id: &Pubkey, prover_wallet: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[PROVER_SEED, prover_wallet.as_ref()], program_id)
}

/// Derive the validator account PDA
///
/// # Arguments
/// * `program_id` - The bedrock program ID
/// * `validator_wallet` - The validator's wallet address
///
/// # Returns
/// * `(Pubkey, u8)` - The derived PDA and bump seed
///
/// # Example
/// ```
/// use bedrock_sdk::derive_validator_pda;
/// use solana_sdk::pubkey::Pubkey;
///
/// let program_id = Pubkey::default();
/// let validator_wallet = Pubkey::default();
/// let (validator_pda, bump) = derive_validator_pda(&program_id, &validator_wallet);
/// ```
pub fn derive_validator_pda(program_id: &Pubkey, validator_wallet: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[VALIDATOR_SEED, validator_wallet.as_ref()], program_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_pda_derivation() {
        let program_id = Pubkey::new_unique();
        let (pda, bump) = derive_config_pda(&program_id);

        // Verify PDA can be recreated with bump
        let expected_pda =
            Pubkey::create_program_address(&[CONFIG_SEED, &[bump]], &program_id).unwrap();
        assert_eq!(pda, expected_pda);
    }

    #[test]
    fn test_prover_pda_derivation() {
        let program_id = Pubkey::new_unique();
        let prover_wallet = Pubkey::new_unique();
        let (pda, bump) = derive_prover_pda(&program_id, &prover_wallet);

        // Verify PDA can be recreated with bump
        let expected_pda = Pubkey::create_program_address(
            &[PROVER_SEED, prover_wallet.as_ref(), &[bump]],
            &program_id,
        )
        .unwrap();
        assert_eq!(pda, expected_pda);
    }

    #[test]
    fn test_validator_pda_derivation() {
        let program_id = Pubkey::new_unique();
        let validator_wallet = Pubkey::new_unique();
        let (pda, bump) = derive_validator_pda(&program_id, &validator_wallet);

        // Verify PDA can be recreated with bump
        let expected_pda = Pubkey::create_program_address(
            &[VALIDATOR_SEED, validator_wallet.as_ref(), &[bump]],
            &program_id,
        )
        .unwrap();
        assert_eq!(pda, expected_pda);
    }

    #[test]
    fn test_different_wallets_different_pdas() {
        let program_id = Pubkey::new_unique();
        let wallet1 = Pubkey::new_unique();
        let wallet2 = Pubkey::new_unique();

        let (pda1, _) = derive_prover_pda(&program_id, &wallet1);
        let (pda2, _) = derive_prover_pda(&program_id, &wallet2);

        assert_ne!(pda1, pda2);
    }
}
