use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

pub const MARKET_SEED: &[u8] = b"market";
pub const POSITION_SEED: &[u8] = b"position";
pub const ESCROW_SEED: &[u8] = b"escrow";
pub const USER_ELIGIBILITY_SEED: &[u8] = b"user_eligibility";
pub const USER_ESCROW_SEED: &[u8] = b"user_escrow";

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[borsh(use_discriminant = true)]
pub enum MarketStatus {
    Active = 0,
    Paused = 1,
    Settled = 2,
    Cancelled = 3,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub enum ExecutableAction {
    None,
    TransferLamports {
        recipient: Pubkey,
        amount: u64,
    },
    TransferSplToken {
        token_mint: Pubkey,
        recipient: Pubkey,
        amount: u64,
    },
    UpdateProgramData {
        program_id: Pubkey,
        new_authority: Option<Pubkey>,
    },
    ExecuteCustom {
        target_program: Pubkey,
        instruction_data: Vec<u8>,
    },
}

impl ExecutableAction {
    pub fn is_none(&self) -> bool {
        matches!(self, ExecutableAction::None)
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct Market {
    pub authority: Pubkey,
    pub market_id: u64,
    pub oracle: Pubkey,
    pub question_hash: [u8; 32],
    pub end_time: i64,
    pub status: MarketStatus,
    pub max_bet: u64,
    pub total_yes_bets: u64,
    pub total_no_bets: u64,
    pub encrypted_pool_yes: Vec<u8>,
    pub encrypted_pool_no: Vec<u8>,
    pub pending_pool_update_job: Option<u64>,
    pub resolution: Option<bool>,
    pub settled_at: Option<i64>,
    pub escrow: Pubkey,
    pub claim_nullifiers: Vec<[u8; 32]>,
    pub executable_action: ExecutableAction,
    pub execution_threshold: u8,
    pub timelock_duration: i64,
    pub timelock_expires_at: Option<i64>,
    pub action_executed: bool,
    pub bump: u8,
    pub created_at: i64,
}

impl Market {
    pub fn is_active(&self) -> bool {
        matches!(self.status, MarketStatus::Active)
    }

    pub fn is_settled(&self) -> bool {
        matches!(self.status, MarketStatus::Settled)
    }

    pub fn is_betting_closed(&self, current_time: i64) -> bool {
        current_time >= self.end_time
    }

    pub fn total_pool(&self) -> u64 {
        self.total_yes_bets.saturating_add(self.total_no_bets)
    }

    pub fn has_pending_fhe_work(&self) -> bool {
        self.pending_pool_update_job.is_some()
            || !self.encrypted_pool_yes.is_empty()
            || !self.encrypted_pool_no.is_empty()
    }

    pub fn yes_vote_percentage(&self) -> u8 {
        let total = self.total_pool();
        if total == 0 {
            return 0;
        }
        let percentage = (self.total_yes_bets as u128 * 100) / total as u128;
        percentage.min(100) as u8
    }

    pub fn threshold_met(&self) -> bool {
        self.yes_vote_percentage() >= self.execution_threshold
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct Position {
    pub user: Pubkey,
    pub market: Pubkey,
    pub bet_commitment: [u8; 32],
    pub placed_at: i64,
    pub claimed: bool,
    pub bump: u8,
}

impl Position {
    pub fn is_claimed(&self) -> bool {
        self.claimed
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct UserEligibility {
    pub user: Pubkey,
    pub registered_at: i64,
    pub blacklist_root_at_registration: [u8; 32],
    pub blacklist_version: u32,
    pub is_eligible: bool,
    pub bump: u8,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct UserEscrow {
    pub user: Pubkey,
    pub market: Pubkey,
    pub deposited: u64,
    pub locked: u64,
    pub bump: u8,
}

impl UserEscrow {
    pub fn available(&self) -> u64 {
        self.deposited.saturating_sub(self.locked)
    }
}
