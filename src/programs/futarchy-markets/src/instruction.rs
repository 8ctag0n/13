//! Instruction definitions for futarchy-markets program

use borsh::{BorshDeserialize, BorshSerialize};

/// Instructions supported by the futarchy-markets program
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum FutarchyInstruction {
    /// Create a new prediction market
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` Market creator/authority
    /// 1. `[writable]` Market account (PDA)
    /// 2. `[writable]` Escrow account (PDA)
    /// 3. `[]` Oracle account (pubkey that can settle)
    /// 4. `[]` System program
    /// 5. `[]` Clock sysvar
    CreateMarket {
        /// Unique market ID
        market_id: u64,
        /// Hash of market question/description
        question_hash: [u8; 32],
        /// Unix timestamp when betting ends
        end_time: i64,
        /// Maximum bet amount allowed
        max_bet: u64,
    },

    /// Place a bet on a market (with ZK proof)
    ///
    /// Accounts expected (without FHE):
    /// 0. `[writable, signer]` User placing bet
    /// 1. `[writable]` Market account (PDA)
    /// 2. `[writable]` Position account (PDA)
    /// 3. `[writable]` Escrow account (PDA)
    /// 4. `[]` ZK-generator program
    /// 5. `[]` System program
    /// 6. `[]` Clock sysvar
    ///
    /// Additional accounts (if encrypted_bet_amount is Some):
    /// 7. `[writable]` FHE job account (PDA)
    /// 8. `[writable]` FHE consensus account (PDA)
    /// 9. `[writable]` FHE escrow account (PDA)
    /// 10. `[]` FHE-generator program
    PlaceBet {
        /// Market ID to bet on
        market_id: u64,
        /// Bet commitment (from ZK proof)
        bet_commitment: [u8; 32],
        /// ZK proof data (MarketBet circuit 30 or MarketBetWithPoI circuit 31)
        proof: Vec<u8>,
        /// Public inputs for proof verification
        public_inputs: Vec<u8>,
        /// Bet amount (in lamports)
        amount: u64,
        /// Circuit type (30 = MarketBet, 31 = MarketBetWithPoI)
        circuit_type: u8,
        /// Encrypted bet amount (FHE ciphertext, optional)
        /// If provided, will create FHE job to add to pool
        encrypted_bet_amount: Option<Vec<u8>>,
        /// Which side (true = YES, false = NO)
        /// Only used if encrypted_bet_amount is Some
        side: Option<bool>,
    },

    /// Settle a market (oracle only)
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` Oracle account
    /// 1. `[writable]` Market account (PDA)
    /// 2. `[]` Clock sysvar
    SettleMarket {
        /// Market ID to settle
        market_id: u64,
        /// Winning outcome (true = YES, false = NO)
        outcome: bool,
    },

    /// Claim payout from a settled market (with ZK proof)
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` User claiming payout
    /// 1. `[writable]` Market account (PDA)
    /// 2. `[writable]` Position account (PDA, optional)
    /// 3. `[writable]` Escrow account (PDA)
    /// 4. `[]` ZK-generator program
    /// 5. `[]` System program
    ClaimPayout {
        /// Market ID
        market_id: u64,
        /// Claim nullifier (prevents double claim)
        claim_nullifier: [u8; 32],
        /// ZK proof data (MarketClaim circuit 32)
        proof: Vec<u8>,
        /// Public inputs for proof verification
        public_inputs: Vec<u8>,
        /// Payout amount claimed
        payout_amount: u64,
    },

    /// Cancel a market (authority only, before settlement)
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` Market authority
    /// 1. `[writable]` Market account (PDA)
    CancelMarket {
        /// Market ID to cancel
        market_id: u64,
    },

    /// Update encrypted pool after FHE consensus
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` Updater (anyone can call after consensus)
    /// 1. `[writable]` Market account (PDA)
    /// 2. `[]` FHE job account
    /// 3. `[]` FHE consensus account
    UpdatePool {
        /// Market ID
        market_id: u64,
        /// FHE job ID that computed the new pool
        fhe_job_id: u64,
        /// Encrypted result from FHE computation
        encrypted_result: Vec<u8>,
        /// Which pool to update (true = YES, false = NO)
        side: bool,
    },

    /// Register user with Proof of Innocence (PoI)
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` User registering
    /// 1. `[writable]` UserEligibility account (PDA)
    /// 2. `[]` ZK-generator program
    /// 3. `[]` System program
    /// 4. `[]` Clock sysvar
    RegisterUser {
        /// ZK proof data (ProofOfInnocence circuit 10)
        proof: Vec<u8>,
        /// Public inputs for proof verification
        public_inputs: Vec<u8>,
        /// Blacklist merkle root used in proof
        blacklist_root: [u8; 32],
        /// Blacklist version
        blacklist_version: u32,
    },

    /// Create a governance market (with executable action)
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` Market creator/authority
    /// 1. `[writable]` Market account (PDA)
    /// 2. `[writable]` Escrow account (PDA)
    /// 3. `[]` Oracle account (pubkey that can settle)
    /// 4. `[]` System program
    /// 5. `[]` Clock sysvar
    CreateMarketWithGovernance {
        /// Unique market ID
        market_id: u64,
        /// Hash of market question/description
        question_hash: [u8; 32],
        /// Unix timestamp when betting ends
        end_time: i64,
        /// Maximum bet amount allowed
        max_bet: u64,
        /// Executable action if governance threshold met
        executable_action: crate::state::ExecutableAction,
        /// Execution threshold (0-100, percentage of YES votes needed)
        execution_threshold: u8,
        /// Timelock duration in seconds
        timelock_duration: i64,
    },

    /// Execute governance action after settlement and timelock
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` Executor (anyone can call)
    /// 1. `[writable]` Market account (PDA)
    /// 2. `[]` Clock sysvar
    /// ... Additional accounts depend on ExecutableAction type
    ExecuteGovernanceAction {
        /// Market ID
        market_id: u64,
    },

    /// Cancel governance action (authority only)
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` Market authority
    /// 1. `[writable]` Market account (PDA)
    CancelGovernanceAction {
        /// Market ID
        market_id: u64,
    },

    /// Deposit funds to user escrow for a market
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` User depositing
    /// 1. `[writable]` UserEscrow account (PDA)
    /// 2. `[]` Market account (PDA) - to validate escrow is for correct market
    /// 3. `[]` System program
    DepositToMarket {
        /// Market ID
        market_id: u64,
        /// Amount to deposit (in lamports)
        amount: u64,
    },

    /// Withdraw funds from user escrow
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` User withdrawing
    /// 1. `[writable]` UserEscrow account (PDA)
    /// 2. `[]` Market account (PDA) - to validate market is active
    /// 3. `[]` System program
    WithdrawFromEscrow {
        /// Market ID
        market_id: u64,
        /// Amount to withdraw (in lamports)
        /// Must be <= user_escrow.available
        amount: u64,
    },
}

impl FutarchyInstruction {
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
    fn test_create_market_serialization() {
        let instruction = FutarchyInstruction::CreateMarket {
            market_id: 1,
            question_hash: [1u8; 32],
            end_time: 1234567890,
            max_bet: 1_000_000_000,
        };

        let packed = instruction.pack().unwrap();
        let unpacked = FutarchyInstruction::unpack(&packed).unwrap();

        assert_eq!(instruction, unpacked);
    }

    #[test]
    fn test_place_bet_serialization() {
        let instruction = FutarchyInstruction::PlaceBet {
            market_id: 1,
            bet_commitment: [2u8; 32],
            proof: vec![0u8; 256],
            public_inputs: vec![1u8; 80],
            amount: 500_000_000,
            circuit_type: 30,
            encrypted_bet_amount: None,
            side: None,
        };

        let packed = instruction.pack().unwrap();
        let unpacked = FutarchyInstruction::unpack(&packed).unwrap();

        assert_eq!(instruction, unpacked);
    }

    #[test]
    fn test_settle_market_serialization() {
        let instruction = FutarchyInstruction::SettleMarket {
            market_id: 1,
            outcome: true,
        };

        let packed = instruction.pack().unwrap();
        let unpacked = FutarchyInstruction::unpack(&packed).unwrap();

        assert_eq!(instruction, unpacked);
    }

    #[test]
    fn test_claim_payout_serialization() {
        let instruction = FutarchyInstruction::ClaimPayout {
            market_id: 1,
            claim_nullifier: [3u8; 32],
            proof: vec![0u8; 256],
            public_inputs: vec![1u8; 105],
            payout_amount: 1_500_000_000,
        };

        let packed = instruction.pack().unwrap();
        let unpacked = FutarchyInstruction::unpack(&packed).unwrap();

        assert_eq!(instruction, unpacked);
    }
}
