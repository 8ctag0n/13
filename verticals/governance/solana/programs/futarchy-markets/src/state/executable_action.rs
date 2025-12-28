//! Executable actions for governance markets

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Account metadata for executable actions
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq)]
pub struct ActionAccount {
    /// Account public key
    pub pubkey: Pubkey,
    /// Whether account must sign
    pub is_signer: bool,
    /// Whether account is writable
    pub is_writable: bool,
}

impl ActionAccount {
    /// Size of ActionAccount when serialized
    pub const SIZE: usize = 32 + 1 + 1; // pubkey + is_signer + is_writable

    /// Create new ActionAccount
    pub fn new(pubkey: Pubkey, is_signer: bool, is_writable: bool) -> Self {
        Self {
            pubkey,
            is_signer,
            is_writable,
        }
    }
}

/// Executable actions that can be triggered by governance markets
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq)]
pub enum ExecutableAction {
    /// No action - simple prediction market
    None,

    /// Transfer SPL tokens from a treasury
    ///
    /// Executes a token transfer via CPI to SPL Token program
    TransferTokens {
        /// Token mint address
        token_mint: Pubkey,
        /// Source token account (treasury)
        from_treasury: Pubkey,
        /// Destination token account
        to: Pubkey,
        /// Amount to transfer (in token's smallest unit)
        amount: u64,
    },

    /// Update a parameter in another program
    ///
    /// Executes a CPI to update configuration/parameter
    UpdateParameter {
        /// Target program to call
        target_program: Pubkey,
        /// Parameter key/identifier
        parameter_key: Vec<u8>,
        /// New parameter value
        parameter_value: Vec<u8>,
    },

    /// Execute a custom instruction
    ///
    /// Generic action that can call any program with arbitrary data
    CustomInstruction {
        /// Program to invoke
        program_id: Pubkey,
        /// Accounts required by the instruction
        accounts: Vec<ActionAccount>,
        /// Instruction data
        data: Vec<u8>,
    },
}

impl ExecutableAction {
    /// Maximum size for serialized action (to prevent account overflow)
    /// This is conservative - actual size depends on variant
    pub const MAX_SIZE: usize = 1 + // discriminator
        32 + // program_id (worst case)
        4 + (ActionAccount::SIZE * 10) + // accounts vec (max 10 accounts)
        4 + 512; // data vec (max 512 bytes)

    /// Get the minimum size for this action variant
    pub fn size(&self) -> usize {
        match self {
            ExecutableAction::None => 1, // just discriminator
            ExecutableAction::TransferTokens { .. } => {
                1 + 32 + 32 + 32 + 8 // discriminator + 3 pubkeys + u64
            }
            ExecutableAction::UpdateParameter {
                parameter_key,
                parameter_value,
                ..
            } => {
                1 + 32 + // discriminator + program_id
                4 + parameter_key.len() + // key vec
                4 + parameter_value.len() // value vec
            }
            ExecutableAction::CustomInstruction { accounts, data, .. } => {
                1 + 32 + // discriminator + program_id
                4 + (accounts.len() * ActionAccount::SIZE) + // accounts vec
                4 + data.len() // data vec
            }
        }
    }

    /// Check if action is None
    pub fn is_none(&self) -> bool {
        matches!(self, ExecutableAction::None)
    }

    /// Validate action doesn't exceed size limits
    pub fn validate_size(&self) -> bool {
        self.size() <= Self::MAX_SIZE
    }

    /// Get human-readable description of action type
    pub fn action_type(&self) -> &str {
        match self {
            ExecutableAction::None => "None",
            ExecutableAction::TransferTokens { .. } => "TransferTokens",
            ExecutableAction::UpdateParameter { .. } => "UpdateParameter",
            ExecutableAction::CustomInstruction { .. } => "CustomInstruction",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_account_size() {
        let account = ActionAccount::new(Pubkey::new_unique(), true, false);
        let serialized = borsh::to_vec(&account).unwrap();
        assert_eq!(serialized.len(), ActionAccount::SIZE);
    }

    #[test]
    fn test_executable_action_none() {
        let action = ExecutableAction::None;
        assert!(action.is_none());
        assert_eq!(action.action_type(), "None");
        assert_eq!(action.size(), 1);
    }

    #[test]
    fn test_executable_action_transfer_tokens() {
        let action = ExecutableAction::TransferTokens {
            token_mint: Pubkey::new_unique(),
            from_treasury: Pubkey::new_unique(),
            to: Pubkey::new_unique(),
            amount: 1_000_000,
        };

        assert!(!action.is_none());
        assert_eq!(action.action_type(), "TransferTokens");
        assert!(action.validate_size());

        let serialized = borsh::to_vec(&action).unwrap();
        let deserialized: ExecutableAction = borsh::from_slice(&serialized).unwrap();
        assert_eq!(action, deserialized);
    }

    #[test]
    fn test_executable_action_update_parameter() {
        let action = ExecutableAction::UpdateParameter {
            target_program: Pubkey::new_unique(),
            parameter_key: b"max_bet".to_vec(),
            parameter_value: 5_000_000_000u64.to_le_bytes().to_vec(),
        };

        assert!(!action.is_none());
        assert_eq!(action.action_type(), "UpdateParameter");
        assert!(action.validate_size());
    }

    #[test]
    fn test_executable_action_custom_instruction() {
        let action = ExecutableAction::CustomInstruction {
            program_id: Pubkey::new_unique(),
            accounts: vec![
                ActionAccount::new(Pubkey::new_unique(), true, true),
                ActionAccount::new(Pubkey::new_unique(), false, true),
            ],
            data: vec![1, 2, 3, 4, 5],
        };

        assert!(!action.is_none());
        assert_eq!(action.action_type(), "CustomInstruction");
        assert!(action.validate_size());

        let serialized = borsh::to_vec(&action).unwrap();
        let deserialized: ExecutableAction = borsh::from_slice(&serialized).unwrap();
        assert_eq!(action, deserialized);
    }

    #[test]
    fn test_max_size_validation() {
        // Create action that's too large
        let large_data = vec![0u8; 1000]; // Exceeds max
        let action = ExecutableAction::CustomInstruction {
            program_id: Pubkey::new_unique(),
            accounts: vec![],
            data: large_data,
        };

        assert!(!action.validate_size());
    }

    #[test]
    fn test_serialization_round_trip() {
        let actions = vec![
            ExecutableAction::None,
            ExecutableAction::TransferTokens {
                token_mint: Pubkey::new_unique(),
                from_treasury: Pubkey::new_unique(),
                to: Pubkey::new_unique(),
                amount: 12345,
            },
            ExecutableAction::UpdateParameter {
                target_program: Pubkey::new_unique(),
                parameter_key: b"test_key".to_vec(),
                parameter_value: b"test_value".to_vec(),
            },
        ];

        for action in actions {
            let serialized = borsh::to_vec(&action).unwrap();
            let deserialized: ExecutableAction = borsh::from_slice(&serialized).unwrap();
            assert_eq!(action, deserialized);
        }
    }
}
