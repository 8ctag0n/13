//! GovernanceConfig - Optional PDA for markets with governance actions
//!
//! Extracted from Market to reduce base Market size.
//! Only created when market.has_governance = true

use borsh::{BorshDeserialize, BorshSerialize};
use super::executable_action::ExecutableAction;

pub const GOVERNANCE_CONFIG_SEED: &[u8] = b"governance";

/// GovernanceConfig account
/// Seeds: ["governance", market_id]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct GovernanceConfig {
    /// Market ID this config belongs to
    pub market_id: u64,

    /// The action to execute if threshold is met
    pub executable_action: ExecutableAction,

    /// Percentage of YES votes required (0-100)
    pub execution_threshold: u8,

    /// Duration in seconds before action can be executed after settlement
    pub timelock_duration: i64,

    /// When the timelock expires (set after settlement)
    pub timelock_expires_at: Option<i64>,

    /// Whether the action has been executed
    pub action_executed: bool,

    /// Bump seed for PDA derivation
    pub bump: u8,
}

impl GovernanceConfig {
    /// Space: 8 + 619 + 1 + 8 + 9 + 1 + 1 = ~647 bytes
    pub const SPACE: usize = 8 + ExecutableAction::MAX_SIZE + 1 + 8 + 9 + 1 + 1;

    pub fn new(
        market_id: u64,
        executable_action: ExecutableAction,
        execution_threshold: u8,
        timelock_duration: i64,
        bump: u8,
    ) -> Self {
        Self {
            market_id,
            executable_action,
            execution_threshold,
            timelock_duration,
            timelock_expires_at: None,
            action_executed: false,
            bump,
        }
    }

    pub fn seeds_with_bump<'a>(market_id: &'a [u8; 8], bump: &'a [u8]) -> [&'a [u8]; 3] {
        [GOVERNANCE_CONFIG_SEED, market_id, bump]
    }

    /// Start timelock countdown
    pub fn start_timelock(&mut self, current_time: i64) {
        self.timelock_expires_at = Some(current_time + self.timelock_duration);
    }

    /// Check if timelock has expired
    pub fn timelock_expired(&self, current_time: i64) -> bool {
        match self.timelock_expires_at {
            Some(expires_at) => current_time >= expires_at,
            None => false,
        }
    }

    /// Check if action can be executed
    pub fn can_execute(&self, current_time: i64, yes_percentage: u8) -> bool {
        !self.action_executed
            && yes_percentage >= self.execution_threshold
            && self.timelock_expired(current_time)
    }

    /// Mark action as executed
    pub fn mark_executed(&mut self) {
        self.action_executed = true;
    }
}
