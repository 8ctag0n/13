use borsh::{BorshDeserialize, BorshSerialize};
use crate::state::ExecutableAction;

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum FutarchyInstruction {
    CreateMarket {
        market_id: u64,
        question_hash: [u8; 32],
        end_time: i64,
        max_bet: u64,
    },

    PlaceBet {
        market_id: u64,
        bet_commitment: [u8; 32],
        proof: Vec<u8>,
        public_inputs: Vec<u8>,
        amount: u64,
        circuit_type: u8,
        encrypted_bet_amount: Option<Vec<u8>>,
        side: Option<bool>,
    },

    SettleMarket {
        market_id: u64,
        outcome: bool,
    },

    ClaimPayout {
        market_id: u64,
        claim_nullifier: [u8; 32],
        proof: Vec<u8>,
        public_inputs: Vec<u8>,
        payout_amount: u64,
    },

    CancelMarket {
        market_id: u64,
    },

    UpdatePool {
        market_id: u64,
        fhe_job_id: u64,
        encrypted_result: Vec<u8>,
        side: bool,
    },

    RegisterUser {
        proof: Vec<u8>,
        public_inputs: Vec<u8>,
        blacklist_root: [u8; 32],
        blacklist_version: u32,
    },

    CreateMarketWithGovernance {
        market_id: u64,
        question_hash: [u8; 32],
        end_time: i64,
        max_bet: u64,
        executable_action: ExecutableAction,
        execution_threshold: u8,
        timelock_duration: i64,
    },

    ExecuteGovernanceAction {
        market_id: u64,
    },

    CancelGovernanceAction {
        market_id: u64,
    },

    DepositToMarket {
        market_id: u64,
        amount: u64,
    },

    WithdrawFromEscrow {
        market_id: u64,
        amount: u64,
    },
}

impl FutarchyInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, std::io::Error> {
        borsh::from_slice(input)
    }

    pub fn pack(&self) -> Result<Vec<u8>, std::io::Error> {
        borsh::to_vec(self)
    }
}
