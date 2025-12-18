use thiserror::Error;

#[derive(Error, Debug)]
pub enum FutarchyError {
    #[error("Solana error: {0}")]
    SolanaError(#[from] solana_sdk::program_error::ProgramError),

    #[error("RPC error: {0}")]
    RpcError(#[from] Box<solana_client::client_error::ClientError>),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] std::io::Error),

    #[error("Invalid instruction data")]
    InvalidInstructionData,

    #[error("Invalid account")]
    InvalidAccount,

    #[error("Market not found")]
    MarketNotFound,

    #[error("Position not found")]
    PositionNotFound,

    #[error("Invalid PDA derivation")]
    InvalidPda,

    #[error("Missing required account")]
    MissingAccount,

    #[error("Custom error: {0}")]
    Custom(String),
}

pub type Result<T> = std::result::Result<T, FutarchyError>;
