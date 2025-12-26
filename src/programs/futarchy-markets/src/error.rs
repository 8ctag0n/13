//! Error types for futarchy-markets program

use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Error, Debug, Copy, Clone)]
pub enum FutarchyError {
    #[error("Invalid instruction")]
    InvalidInstruction,

    #[error("Market not found")]
    MarketNotFound,

    #[error("Market already settled")]
    MarketAlreadySettled,

    #[error("Market not settled yet")]
    MarketNotSettled,

    #[error("Betting period ended")]
    BettingClosed,

    #[error("Invalid oracle")]
    InvalidOracle,

    #[error("Invalid bet commitment")]
    InvalidBetCommitment,

    #[error("Invalid ZK proof")]
    InvalidProof,

    #[error("Claim nullifier already used")]
    NullifierUsed,

    #[error("Bet exceeds maximum allowed")]
    BetExceedsMax,

    #[error("Invalid outcome")]
    InvalidOutcome,

    #[error("Insufficient escrow balance")]
    InsufficientEscrow,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Market not active")]
    MarketNotActive,

    #[error("Invalid account")]
    InvalidAccount,

    #[error("Numerical overflow")]
    Overflow,

    #[error("User not eligible (PoI verification required)")]
    UserNotEligible,

    #[error("Invalid execution threshold (must be 0-100)")]
    InvalidThreshold,

    #[error("Execution threshold not met")]
    ThresholdNotMet,

    #[error("Timelock not expired yet")]
    TimelockNotExpired,

    #[error("Governance action already executed")]
    ActionAlreadyExecuted,

    #[error("No governance action configured")]
    NoActionConfigured,

    #[error("Invalid governance action")]
    InvalidAction,

    #[error("Invalid owner")]
    InvalidOwner,

    #[error("Position already claimed")]
    PositionAlreadyClaimed,

    #[error("Already claimed (nullifier exists)")]
    AlreadyClaimed,

    #[error("Unauthorized oracle")]
    UnauthorizedOracle,

    #[error("Betting period not closed")]
    BettingNotClosed,

    #[error("Market is not a governance market")]
    NotGovernanceMarket,

    #[error("Unauthorized authority")]
    UnauthorizedAuthority,

    #[error("No winners in market")]
    NoWinners,
}

impl From<FutarchyError> for ProgramError {
    fn from(e: FutarchyError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
