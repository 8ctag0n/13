//! State account definitions

// V1 modules
pub mod market;
pub mod nullifier;
pub mod position;
pub mod user_eligibility;
pub mod user_escrow;
pub mod executable_action;

// V2 modules (PrivateBalance architecture)
pub mod private_balance;
pub mod vaults;
pub mod governance_config;

// V1 exports
pub use market::{Market, MarketStatus, MARKET_SEED};
pub use nullifier::{Nullifier, NULLIFIER_SEED};
pub use position::{Position, POSITION_SEED};
pub use user_eligibility::{UserEligibility, USER_ELIGIBILITY_SEED};
pub use user_escrow::{UserEscrow, USER_ESCROW_SEED};
pub use executable_action::{ExecutableAction, ActionAccount};

// V2 exports
pub use market::{MarketV2, MARKET_V2_SEED};
pub use position::{PositionV2, POSITION_V2_SEED};
pub use private_balance::{PrivateBalance, PRIVATE_BALANCE_SEED, MAX_FHE_CIPHERTEXT_SIZE};
pub use vaults::{ProtocolVault, MarketVault, PROTOCOL_VAULT_SEED, MARKET_VAULT_SEED};
pub use governance_config::{GovernanceConfig, GOVERNANCE_CONFIG_SEED};
