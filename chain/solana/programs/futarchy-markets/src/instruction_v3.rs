//! V3 Instruction definitions for Fully Blind Markets with FHE+ZK
//!
//! Key features:
//! - Encrypted pools using FHE (stored off-chain, hashes on-chain)
//! - ZK proofs for pool commitment updates (PlaceBetBlind circuit)
//! - Threshold decryption for settlement
//! - No public pool totals until settlement

use borsh::{BorshDeserialize, BorshSerialize};
use crate::state::ExecutableAction;

/// Instructions for Fully Blind Markets (V3)
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum FutarchyInstructionV3 {
    /// Create market V3 (fully blind with FHE)
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` Market creator/authority
    /// 1. `[writable]` MarketV3 PDA (will be created)
    /// 2. `[writable]` MarketVault PDA (will be created)
    /// 3. `[]` Oracle pubkey
    /// 4. `[]` System program
    /// 5. `[]` Clock sysvar
    ///
    /// If has_governance = true, additional accounts:
    /// 6. `[writable]` GovernanceConfig PDA (will be created)
    CreateMarketV3 {
        /// Unique market ID
        market_id: u64,
        /// Hash of market question/description
        question_hash: [u8; 32],
        /// Unix timestamp when betting ends
        end_time: i64,
        /// Maximum bet amount allowed
        max_bet: u64,
        /// Threshold public key for FHE decryption
        threshold_pubkey: [u8; 32],
        /// Number of threshold shares required (t in t-of-n)
        threshold_required: u8,
        /// Total number of threshold shares (n in t-of-n)
        threshold_shares: u8,
        /// Whether this market has governance
        has_governance: bool,
        /// Executable action (if has_governance = true)
        executable_action: Option<ExecutableAction>,
        /// Execution threshold 0-100 (if has_governance = true)
        execution_threshold: Option<u8>,
        /// Timelock duration in seconds (if has_governance = true)
        timelock_duration: Option<i64>,
    },

    /// Place blind bet on market V3 (with ZK proof, no side revealed)
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` User placing bet
    /// 1. `[writable]` PrivateBalance PDA
    /// 2. `[writable]` MarketV3 PDA
    /// 3. `[writable]` PositionV3 PDA (will be created)
    /// 4. `[writable]` ProtocolVault PDA (source of funds)
    /// 5. `[writable]` MarketVault PDA (destination)
    /// 6. `[]` ZK-generator program
    /// 7. `[]` System program
    /// 8. `[]` Clock sysvar
    PlaceBetBlind {
        /// Market ID to bet on
        market_id: u64,
        /// Hash of FHE-encrypted bet ciphertext (stored off-chain)
        bet_ciphertext_hash: [u8; 32],
        /// User's secret commitment: Poseidon(amount, side, secret)
        bet_secret_commitment: [u8; 32],
        /// New pool commitment after bet: Poseidon(pool_yes, pool_no, blinding)
        pool_commitment_after: [u8; 32],
        /// ZK proof of valid blind bet (PlaceBetBlind circuit 50)
        /// Proves: pool update is correct without revealing amount or side
        proof: Vec<u8>,
        /// Public inputs for proof verification:
        /// - pool_commitment_before (32)
        /// - pool_commitment_after (32)
        /// - bet_ciphertext_hash (32)
        /// - market_id (8)
        /// - max_bet (8)
        public_inputs: Vec<u8>,
        /// New balance commitment after bet
        new_balance_commitment: [u8; 32],
        /// Circuit type used (should be 50 for PlaceBetBlind)
        circuit_type: u8,
    },

    /// Update encrypted pool hashes (off-chain FHE operation)
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` Updater (prover/aggregator)
    /// 1. `[writable]` MarketV3 PDA
    UpdateEncryptedPools {
        /// Market ID
        market_id: u64,
        /// New hash of encrypted YES pool
        encrypted_pool_yes_hash: [u8; 32],
        /// New hash of encrypted NO pool
        encrypted_pool_no_hash: [u8; 32],
        /// Proof of correct FHE homomorphic addition (placeholder)
        fhe_proof: Vec<u8>,
    },

    /// Settle market V3 with threshold-decrypted pools
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` Oracle account
    /// 1. `[writable]` MarketV3 PDA
    /// 2. `[]` Clock sysvar
    ///
    /// If has_governance = true, additional accounts:
    /// 3. `[writable]` GovernanceConfig PDA
    SettleMarketV3 {
        /// Market ID to settle
        market_id: u64,
        /// Winning outcome (true = YES, false = NO)
        outcome: bool,
        /// Decrypted pool YES (revealed via threshold decryption)
        decrypted_pool_yes: u64,
        /// Decrypted pool NO (revealed via threshold decryption)
        decrypted_pool_no: u64,
        /// Threshold signatures proving decryption validity (t-of-n)
        /// Each signature is 64 bytes (BLS/Schnorr)
        threshold_signatures: Vec<[u8; 64]>,
    },

    /// Claim payout from settled market V3
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` User claiming
    /// 1. `[writable]` PrivateBalance PDA
    /// 2. `[]` MarketV3 PDA (must be settled)
    /// 3. `[writable]` PositionV3 PDA (will mark as claimed)
    /// 4. `[writable]` MarketVault PDA (source of payout)
    /// 5. `[writable]` ProtocolVault PDA (destination)
    /// 6. `[]` ZK-generator program
    /// 7. `[]` System program
    ClaimV3 {
        /// Market ID
        market_id: u64,
        /// Bet ciphertext hash (position identifier)
        bet_ciphertext_hash: [u8; 32],
        /// Bet secret commitment (for ownership proof)
        bet_secret_commitment: [u8; 32],
        /// ZK proof of:
        /// - Ownership of bet (knows secret matching commitment)
        /// - Bet was on winning side
        /// - Correct payout calculation from decrypted pools
        proof: Vec<u8>,
        /// Public inputs for proof verification
        public_inputs: Vec<u8>,
        /// New balance commitment after claim
        new_balance_commitment: [u8; 32],
        /// Circuit type used (ClaimBlind circuit)
        circuit_type: u8,
    },

    /// Execute governance action after settlement and timelock (V3)
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` Executor (anyone can call)
    /// 1. `[writable]` MarketV3 PDA
    /// 2. `[writable]` GovernanceConfig PDA
    /// 3. `[]` Clock sysvar
    /// ... Additional accounts depend on ExecutableAction type
    ExecuteGovernanceActionV3 {
        /// Market ID
        market_id: u64,
    },

    /// Cancel governance action (authority only, V3)
    ///
    /// Accounts expected:
    /// 0. `[writable, signer]` Market authority
    /// 1. `[]` MarketV3 PDA
    /// 2. `[writable]` GovernanceConfig PDA
    CancelGovernanceActionV3 {
        /// Market ID
        market_id: u64,
    },
}

impl FutarchyInstructionV3 {
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
    fn test_create_market_v3_serialization() {
        let instruction = FutarchyInstructionV3::CreateMarketV3 {
            market_id: 123,
            question_hash: [1u8; 32],
            end_time: 1700000000,
            max_bet: 1_000_000_000,
            threshold_pubkey: [2u8; 32],
            threshold_required: 3,
            threshold_shares: 5,
            has_governance: false,
            executable_action: None,
            execution_threshold: None,
            timelock_duration: None,
        };

        let packed = instruction.pack().unwrap();
        let unpacked = FutarchyInstructionV3::unpack(&packed).unwrap();

        assert_eq!(instruction, unpacked);
    }

    #[test]
    fn test_place_bet_blind_serialization() {
        let instruction = FutarchyInstructionV3::PlaceBetBlind {
            market_id: 456,
            bet_ciphertext_hash: [3u8; 32],
            bet_secret_commitment: [4u8; 32],
            pool_commitment_after: [5u8; 32],
            proof: vec![0u8; 256],
            public_inputs: vec![0u8; 112],
            new_balance_commitment: [6u8; 32],
            circuit_type: 50,
        };

        let packed = instruction.pack().unwrap();
        let unpacked = FutarchyInstructionV3::unpack(&packed).unwrap();

        assert_eq!(instruction, unpacked);
    }

    #[test]
    fn test_settle_market_v3_serialization() {
        let instruction = FutarchyInstructionV3::SettleMarketV3 {
            market_id: 789,
            outcome: true,
            decrypted_pool_yes: 5_000_000_000,
            decrypted_pool_no: 3_000_000_000,
            threshold_signatures: vec![[7u8; 64], [8u8; 64], [9u8; 64]],
        };

        let packed = instruction.pack().unwrap();
        let unpacked = FutarchyInstructionV3::unpack(&packed).unwrap();

        assert_eq!(instruction, unpacked);
    }
}
