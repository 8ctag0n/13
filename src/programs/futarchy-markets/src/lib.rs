//! Futarchy Markets - Private Prediction Markets
//!
//! This program implements futarchy-style governance through prediction markets:
//! - Create conditional prediction markets (YES/NO)
//! - Place private bets using ZK proofs
//! - Settle markets based on oracle data
//! - Claim payouts with ZK proofs
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │              FUTARCHY-MARKETS (this)                         │
//! │    Market { question, end_time, status, pools }             │
//! │    Position { user, market, commitment }                    │
//! │                                                              │
//! │    Instructions:                                             │
//! │    - CreateMarket  → creates Market + escrow                │
//! │    - PlaceBet      → creates Position (private bet)         │
//! │    - SettleMarket  → oracle resolves market                 │
//! │    - ClaimPayout   → user claims winnings (ZK proof)        │
//! └─────────────────────────────────────────────────────────────┘
//!                               │ CPI
//!                               ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                   ZK-GENERATOR                               │
//! │    verify MarketBet proof (circuit 30/31)                   │
//! │    verify MarketClaim proof (circuit 32)                    │
//! └─────────────────────────────────────────────────────────────┘
//!                               │ CPI
//!                               ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      BEDROCK                                 │
//! │    verify_prover() - optional prover verification           │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Features
//!
//! ## MVP (Phase 1 - Transparent)
//! - Create YES/NO prediction markets
//! - Place bets with ZK commitments (amounts hidden)
//! - Settle markets via oracle
//! - Claim payouts with ZK proof of winning bet
//!
//! ## Future (Phase 2 - Privacy)
//! - FHE encrypted bet amounts
//! - Private pool calculations
//! - PoI integration (insider trading prevention)

use solana_program::declare_id;

declare_id!("FutMkts111111111111111111111111111111111111");

pub mod error;
pub mod instruction;
pub mod instruction_v2;
pub mod processor;
pub mod processor_v2;
pub mod state;
pub mod cpi;
pub mod verifier;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

pub use error::*;
pub use instruction::*;
pub use processor::*;
pub use state::*;
