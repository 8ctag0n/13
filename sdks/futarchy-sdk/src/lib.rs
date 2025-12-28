#![allow(clippy::too_many_arguments)]

pub mod accounts;
pub mod client;
pub mod error;
pub mod instruction;
pub mod instructions;
pub mod queries;
pub mod state;

pub use accounts::*;
pub use client::*;
pub use error::{FutarchyError, Result};
pub use instruction::FutarchyInstruction;
pub use instructions::*;
pub use queries::*;
pub use state::*;
