//! V2 Instruction definitions for PrivateBalance architecture
//!
//! Key differences from V1:
//! - User balances are private (FHE encrypted + Poseidon commitments)
//! - Markets use separate vault PDAs
//! - All operations require ZK proofs
//! - Governance config is separate PDA

use borsh::{BorshDeserialize, BorshSerialize};
use crate::state::ExecutableAction;

/// Instructions for PrivateBalance architecture (V2)
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum FutarchyInstructionV2 {
    /// Initialize protocol vault (one-time setup)
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` Protocol authority/admin
    /// 1. `[writable]` ProtocolVault PDA (will be created)
    /// 2. `[]` System program
    InitializeProtocol,

    /// Create private balance account for user
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` User
    /// 1. `[writable]` PrivateBalance PDA (will be created)
    /// 2. `[]` System program
    CreatePrivateBalance {
        /// Initial encrypted balance (FHE ciphertext)
        /// Should be encryption of 0 for new accounts
        initial_encrypted_balance: Vec<u8>,
        /// Initial balance commitment: Poseidon(0, nonce)
        initial_commitment: [u8; 32],
    },

    /// Deposit SOL to protocol vault (updates private balance)
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` User depositing
    /// 1. `[writable]` PrivateBalance PDA
    /// 2. `[writable]` ProtocolVault PDA
    /// 3. `[]` System program
    ///
    /// Note: FHE balance update is async (off-chain provers)
    /// Commitment is updated immediately with new_commitment
    Deposit {
        /// Amount to deposit (in lamports)
        amount: u64,
        /// New balance commitment: Poseidon(new_balance, new_nonce)
        new_commitment: [u8; 32],
    },

    /// Withdraw SOL from protocol vault (with ZK proof)
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` User withdrawing
    /// 1. `[writable]` PrivateBalance PDA
    /// 2. `[writable]` ProtocolVault PDA
    /// 3. `[]` ZK-generator program
    /// 4. `[]` System program
    Withdraw {
        /// Amount to withdraw (in lamports)
        amount: u64,
        /// ZK proof of sufficient balance
        proof: Vec<u8>,
        /// Public inputs for proof verification
        public_inputs: Vec<u8>,
        /// New balance commitment after withdrawal
        new_balance_commitment: [u8; 32],
    },

    /// Create market V2 (with separate vault)
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` Market creator/authority
    /// 1. `[writable]` MarketV2 PDA (will be created)
    /// 2. `[writable]` MarketVault PDA (will be created)
    /// 3. `[]` Oracle pubkey
    /// 4. `[]` System program
    /// 5. `[]` Clock sysvar
    ///
    /// If has_governance = true, additional accounts:
    /// 6. `[writable]` GovernanceConfig PDA (will be created)
    CreateMarketV2 {
        /// Unique market ID
        market_id: u64,
        /// Hash of market question/description
        question_hash: [u8; 32],
        /// Unix timestamp when betting ends
        end_time: i64,
        /// Maximum bet amount allowed
        max_bet: u64,
        /// Whether this market has governance
        has_governance: bool,
        /// Executable action (if has_governance = true)
        executable_action: Option<ExecutableAction>,
        /// Execution threshold 0-100 (if has_governance = true)
        execution_threshold: Option<u8>,
        /// Timelock duration in seconds (if has_governance = true)
        timelock_duration: Option<i64>,
    },

    /// Place private bet on market V2
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` User placing bet
    /// 1. `[writable]` PrivateBalance PDA
    /// 2. `[writable]` MarketV2 PDA
    /// 3. `[writable]` PositionV2 PDA (will be created)
    /// 4. `[writable]` ProtocolVault PDA (source of funds)
    /// 5. `[writable]` MarketVault PDA (destination)
    /// 6. `[]` ZK-generator program
    /// 7. `[]` System program
    /// 8. `[]` Clock sysvar
    PlaceBetV2 {
        /// Market ID to bet on
        market_id: u64,
        /// Bet commitment: Poseidon(amount, side, secret)
        bet_commitment: [u8; 32],
        /// Bet side: true = YES, false = NO (visible for V2, enables proportional payout)
        bet_side: bool,
        /// ZK proof of valid bet (sufficient balance, valid amount)
        proof: Vec<u8>,
        /// Public inputs for proof verification
        public_inputs: Vec<u8>,
        /// New balance commitment after bet
        new_balance_commitment: [u8; 32],
        /// Encrypted bet data (for FHE pool update in V3)
        /// Contains: side (YES/NO) and amount
        encrypted_bet: Vec<u8>,
        /// Circuit type used (for proof verification)
        circuit_type: u8,
    },

    /// Claim payout from settled market V2
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` User claiming (doesn't prove ownership on-chain)
    /// 1. `[writable]` PrivateBalance PDA
    /// 2. `[]` MarketV2 PDA (must be settled)
    /// 3. `[writable]` PositionV2 PDA (will mark as claimed)
    /// 4. `[writable]` MarketVault PDA (source of payout)
    /// 5. `[writable]` ProtocolVault PDA (destination)
    /// 6. `[]` ZK-generator program
    /// 7. `[]` System program
    ClaimV2 {
        /// Market ID
        market_id: u64,
        /// Nullifier hash to prevent double claim
        /// nullifier = Poseidon(bet_commitment, secret)
        nullifier_hash: [u8; 32],
        /// ZK proof of:
        /// - Ownership of bet (knows secret matching bet_commitment)
        /// - Bet was on winning side
        /// - Correct payout calculation
        proof: Vec<u8>,
        /// Public inputs for proof verification
        public_inputs: Vec<u8>,
        /// New balance commitment after claim
        new_balance_commitment: [u8; 32],
        /// Encrypted payout data (for FHE balance update)
        encrypted_payout: Vec<u8>,
        /// Bet commitment being claimed
        bet_commitment: [u8; 32],
        /// Circuit type used
        circuit_type: u8,
    },

    /// Settle market V2 (oracle only)
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` Oracle account
    /// 1. `[writable]` MarketV2 PDA
    /// 2. `[]` Clock sysvar
    ///
    /// If has_governance = true, additional accounts:
    /// 3. `[writable]` GovernanceConfig PDA
    SettleMarketV2 {
        /// Market ID to settle
        market_id: u64,
        /// Winning outcome (true = YES, false = NO)
        outcome: bool,
    },

    /// Update pool state root after FHE consensus
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` Updater (anyone after consensus)
    /// 1. `[writable]` MarketV2 PDA
    UpdatePoolState {
        /// Market ID
        market_id: u64,
        /// New pool state root commitment
        /// pool_state_root = Poseidon(encrypted_yes || encrypted_no || nonce)
        new_pool_state_root: [u8; 32],
        /// Proof of FHE consensus (placeholder for now)
        consensus_proof: Vec<u8>,
    },

    /// Execute governance action after settlement and timelock
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` Executor (anyone can call)
    /// 1. `[writable]` MarketV2 PDA
    /// 2. `[writable]` GovernanceConfig PDA
    /// 3. `[]` Clock sysvar
    /// ... Additional accounts depend on ExecutableAction type
    ExecuteGovernanceActionV2 {
        /// Market ID
        market_id: u64,
    },

    /// Cancel governance action (authority only)
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` Market authority
    /// 1. `[]` MarketV2 PDA
    /// 2. `[writable]` GovernanceConfig PDA
    CancelGovernanceActionV2 {
        /// Market ID
        market_id: u64,
    },
}

impl FutarchyInstructionV2 {
    /// Deserialize instruction from instruction data
    pub fn unpack(input: &[u8]) -> Result<Self, std::io::Error> {
        borsh::from_slice(input)
    }

    /// Serialize instruction to instruction data
    pub fn pack(&self) -> Result<Vec<u8>, std::io::Error> {
        borsh::to_vec(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize_protocol_serialization() {
        let instruction = FutarchyInstructionV2::InitializeProtocol;

        let packed = instruction.pack().unwrap();
        let unpacked = FutarchyInstructionV2::unpack(&packed).unwrap();

        assert_eq!(instruction, unpacked);
    }

    #[test]
    fn test_deposit_serialization() {
        let instruction = FutarchyInstructionV2::Deposit {
            amount: 1_000_000_000,
            new_commitment: [5u8; 32],
        };

        let packed = instruction.pack().unwrap();
        let unpacked = FutarchyInstructionV2::unpack(&packed).unwrap();

        assert_eq!(instruction, unpacked);
    }

    #[test]
    fn test_place_bet_v2_serialization() {
        let instruction = FutarchyInstructionV2::PlaceBetV2 {
            market_id: 1,
            bet_commitment: [1u8; 32],
            bet_side: true,
            proof: vec![0u8; 256],
            public_inputs: vec![1u8; 80],
            new_balance_commitment: [2u8; 32],
            encrypted_bet: vec![3u8; 128],
            circuit_type: 40,
        };

        let packed = instruction.pack().unwrap();
        let unpacked = FutarchyInstructionV2::unpack(&packed).unwrap();

        assert_eq!(instruction, unpacked);
    }

    #[test]
    fn test_claim_v2_serialization() {
        let instruction = FutarchyInstructionV2::ClaimV2 {
            market_id: 1,
            nullifier_hash: [1u8; 32],
            proof: vec![0u8; 256],
            public_inputs: vec![1u8; 105],
            new_balance_commitment: [2u8; 32],
            encrypted_payout: vec![3u8; 128],
            bet_commitment: [4u8; 32],
            circuit_type: 42,
        };

        let packed = instruction.pack().unwrap();
        let unpacked = FutarchyInstructionV2::unpack(&packed).unwrap();

        assert_eq!(instruction, unpacked);
    }
}
