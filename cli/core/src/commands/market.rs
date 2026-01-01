//! Market commands for prediction markets
//!
//! Implements CLI commands for creating and participating in prediction markets.
//! Supports both ZK circuits and FHE encrypted bets.

use anyhow::{Context, Result};
use clap::{Args, Subcommand, ValueEnum};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

use crate::client::{CreateJobRequest, ZkClient};
use crate::client::futarchy::{FutarchyClient, PrepareBetRequest, SubmitBetRequest, encode_base64};
use borsh::{BorshSerialize, BorshDeserialize};

// =============================================================================
// V2 Instruction Enum (matches on-chain program)
// =============================================================================

/// V2 Instructions for PrivateBalance architecture
/// Must match futarchy-markets/src/instruction_v2.rs exactly
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum FutarchyInstructionV2 {
    /// Initialize protocol vault (one-time setup)
    InitializeProtocol,

    /// Create private balance account for user
    CreatePrivateBalance {
        initial_encrypted_balance: Vec<u8>,
        initial_commitment: [u8; 32],
    },

    /// Deposit SOL to protocol vault
    Deposit {
        amount: u64,
        new_commitment: [u8; 32],
    },

    /// Withdraw SOL from protocol vault (with ZK proof)
    Withdraw {
        amount: u64,
        proof: Vec<u8>,
        public_inputs: Vec<u8>,
        new_balance_commitment: [u8; 32],
    },

    /// Create market V2 (with separate vault)
    CreateMarketV2 {
        market_id: u64,
        question_hash: [u8; 32],
        end_time: i64,
        max_bet: u64,
        has_governance: bool,
        executable_action: Option<ExecutableActionV2>,
        execution_threshold: Option<u8>,
        timelock_duration: Option<i64>,
    },

    /// Place private bet on market V2
    PlaceBetV2 {
        market_id: u64,
        bet_commitment: [u8; 32],
        bet_side: bool, // true = YES, false = NO (visible for V2)
        proof: Vec<u8>,
        public_inputs: Vec<u8>,
        new_balance_commitment: [u8; 32],
        encrypted_bet: Vec<u8>,
        circuit_type: u8,
    },

    /// Claim payout from settled market V2
    ClaimV2 {
        market_id: u64,
        nullifier_hash: [u8; 32],
        proof: Vec<u8>,
        public_inputs: Vec<u8>,
        new_balance_commitment: [u8; 32],
        encrypted_payout: Vec<u8>,
        bet_commitment: [u8; 32],
        circuit_type: u8,
    },

    /// Settle market V2 (oracle only)
    SettleMarketV2 { market_id: u64, outcome: bool },

    /// Update pool state after FHE consensus
    UpdatePoolState {
        market_id: u64,
        new_pool_state_root: [u8; 32],
        consensus_proof: Vec<u8>,
    },

    /// Execute governance action after settlement
    ExecuteGovernanceActionV2 { market_id: u64 },

    /// Cancel governance action
    CancelGovernanceActionV2 { market_id: u64 },
}

/// Executable action type for governance
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum ExecutableActionV2 {
    TransferSol { recipient: [u8; 32], amount: u64 },
    TransferToken { mint: [u8; 32], recipient: [u8; 32], amount: u64 },
    UpdateConfig { config_key: [u8; 32], new_value: Vec<u8> },
    Custom { program_id: [u8; 32], instruction_data: Vec<u8> },
}

// =============================================================================
// V3 Instruction Enum (Fully Blind Markets with FHE+ZK)
// =============================================================================

/// V3 Instructions for Fully Blind Markets
/// Must match futarchy-markets/src/instruction_v3.rs exactly
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum FutarchyInstructionV3 {
    /// Create market V3 (fully blind with FHE)
    CreateMarketV3 {
        market_id: u64,
        question_hash: [u8; 32],
        end_time: i64,
        max_bet: u64,
        threshold_pubkey: [u8; 32],
        threshold_required: u8,
        threshold_shares: u8,
        has_governance: bool,
        executable_action: Option<ExecutableActionV2>,
        execution_threshold: Option<u8>,
        timelock_duration: Option<i64>,
    },

    /// Place blind bet on market V3 (with ZK proof, no side revealed)
    PlaceBetBlind {
        market_id: u64,
        bet_ciphertext_hash: [u8; 32],
        bet_secret_commitment: [u8; 32],
        pool_commitment_after: [u8; 32],
        proof: Vec<u8>,
        public_inputs: Vec<u8>,
        new_balance_commitment: [u8; 32],
        circuit_type: u8,
    },

    /// Update encrypted pool hashes (off-chain FHE operation)
    UpdateEncryptedPools {
        market_id: u64,
        encrypted_pool_yes_hash: [u8; 32],
        encrypted_pool_no_hash: [u8; 32],
        fhe_proof: Vec<u8>,
    },

    /// Settle market V3 with threshold-decrypted pools
    SettleMarketV3 {
        market_id: u64,
        outcome: bool,
        decrypted_pool_yes: u64,
        decrypted_pool_no: u64,
        threshold_signatures: Vec<[u8; 64]>,
    },

    /// Claim payout from settled market V3
    ClaimV3 {
        market_id: u64,
        bet_ciphertext_hash: [u8; 32],
        bet_secret_commitment: [u8; 32],
        proof: Vec<u8>,
        public_inputs: Vec<u8>,
        new_balance_commitment: [u8; 32],
        circuit_type: u8,
    },

    /// Execute governance action after settlement (V3)
    ExecuteGovernanceActionV3 { market_id: u64 },

    /// Cancel governance action (V3)
    CancelGovernanceActionV3 { market_id: u64 },
}

impl FutarchyInstructionV3 {
    /// Serialize instruction to bytes using borsh
    pub fn pack(&self) -> Result<Vec<u8>> {
        borsh::to_vec(self).context("Failed to serialize FutarchyInstructionV3")
    }
}

// Circuit type 30 reserved for future market prediction circuit
const CIRCUIT_MARKET_BET: u8 = 31;
const CIRCUIT_MARKET_SETTLEMENT: u8 = 32;
const CIRCUIT_FHE_BET: u8 = 35;

// V3 Circuits (Fully Blind Markets)
const CIRCUIT_PLACE_BET_BLIND: u8 = 50;
const CIRCUIT_CLAIM_BLIND: u8 = 51;

/// Side of a futarchy bet
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq)]
pub enum BetSide {
    Yes,
    No,
}

/// Bet witness data saved locally for later claim
#[derive(Debug, Serialize, Deserialize)]
pub struct BetWitness {
    pub market_id: u64,
    pub secret: String,          // hex 32 bytes
    pub blinding: String,         // hex 32 bytes
    pub bet_amount: u64,
    pub bet_side: u8,             // 0=NO, 1=YES
    pub bet_commitment: String,   // hex 32 bytes
}

/// Market data response from server
#[derive(Debug, Serialize, Deserialize)]
pub struct MarketData {
    pub market_id: u64,
    pub total_pool: u64,
    pub winning_pool: u64,
    pub resolution: Option<u8>,  // 0 = NO won, 1 = YES won
    pub total_yes_bets: u64,
    pub total_no_bets: u64,
}

#[derive(Subcommand, Debug)]
pub enum MarketCommands {
    /// Create a new prediction market
    Create(CreateArgs),
    /// Deposit funds to market escrow (required before betting)
    Deposit(DepositArgs),
    /// Place a private bet
    Bet(BetArgs),
    /// Claim market winnings
    Claim(ClaimArgs),
    /// Verify market settlement
    Verify(VerifyArgs),

    // === V2 Commands (PrivateBalance architecture) ===

    /// Initialize protocol (one-time setup)
    InitProtocol(InitProtocolArgs),
    /// Create a V2 market with optional governance
    CreateV2(CreateV2Args),
    /// Create private balance account for user
    CreateBalance(CreateBalanceArgs),
    /// Deposit to private balance (V2)
    DepositPrivate(DepositPrivateArgs),
    /// Place bet using private balance (V2)
    BetPrivate(BetPrivateArgs),
    /// Claim winnings to private balance (V2)
    ClaimPrivate(ClaimPrivateArgs),
    /// Withdraw from private balance (V2)
    WithdrawPrivate(WithdrawPrivateArgs),
    /// Settle market V2 (oracle only)
    SettleV2(SettleV2Args),

    // === V3 Commands (Fully Blind Markets with FHE+ZK) ===

    /// Create market V3 (fully blind with encrypted pools)
    CreateV3(CreateV3Args),
    /// Place blind bet on V3 market (ZK proof, no side revealed)
    BetBlind(BetBlindArgs),
    /// Settle market V3 with threshold decryption
    SettleV3(SettleV3Args),
    /// Claim winnings from V3 market
    ClaimV3(ClaimV3Args),
}

#[derive(Args, Debug)]
pub struct CreateArgs {
    /// Market question
    #[arg(long)]
    pub question: String,

    /// Numeric market ID (u64)
    #[arg(long)]
    pub market_id: u64,

    /// Oracle public key (who can settle the market)
    #[arg(long)]
    pub oracle: Option<String>,

    /// Resolution window in seconds (default: 1 day)
    #[arg(long, default_value = "86400")]
    pub resolution_window: i64,

    /// Maximum bet amount in lamports
    #[arg(long, default_value = "1000000000")]
    pub max_bet: u64,

    /// Path to Solana keypair for signing
    #[arg(long)]
    pub keypair: PathBuf,

    /// Solana RPC URL
    #[arg(long, default_value = "http://localhost:8899")]
    pub rpc_url: String,

    /// Futarchy server URL
    #[arg(long, default_value = "http://localhost:9000")]
    pub server: String,
}

#[derive(Args, Debug)]
pub struct DepositArgs {
    /// Market ID
    #[arg(long)]
    pub market_id: u64,

    /// Amount to deposit in lamports
    #[arg(long)]
    pub amount: u64,

    /// Path to Solana keypair for signing
    #[arg(long)]
    pub keypair: PathBuf,

    /// Solana RPC URL
    #[arg(long, default_value = "http://localhost:8899")]
    pub rpc_url: String,
}

#[derive(Args, Debug)]
pub struct BetArgs {
    /// Market ID
    #[arg(long)]
    pub market_id: String,

    /// Selected outcome (0-indexed) - for ZK mode
    #[arg(long, required_unless_present = "use_fhe")]
    pub outcome: Option<u8>,

    /// Bet side (yes/no) - for FHE mode
    #[arg(long, value_enum)]
    pub side: Option<BetSide>,

    /// Bet amount (lamports)
    #[arg(long)]
    pub amount: u64,

    /// Bettor public key
    #[arg(long)]
    pub bettor: String,

    /// Server URL (ZK server) - overrides config and BACKEND_URL env var
    #[arg(long)]
    pub server: Option<String>,

    /// Futarchy server URL (for FHE mode) - overrides config
    #[arg(long)]
    pub futarchy_server: Option<String>,

    /// Use FHE encryption for the bet (enables encrypted pool aggregation)
    #[arg(long)]
    pub use_fhe: bool,

    /// Path to output witness file
    #[arg(long, default_value = "./bet_witness.json")]
    pub witness_output: PathBuf,

    /// Path to Solana keypair for signing transactions - overrides config and USER_KEYPAIR env var
    #[arg(long)]
    pub keypair: Option<PathBuf>,

    /// Solana RPC URL - overrides config and SOLANA_RPC_URL env var
    #[arg(long)]
    pub rpc_url: Option<String>,

    /// Config file path (default: zyb.toml in current directory or ~/.config/zyb/zyb.toml)
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Profile name from config (overrides common.profile)
    #[arg(long)]
    pub profile: Option<String>,

    /// Skip payment confirmation
    #[arg(long)]
    pub skip_confirm: bool,

    /// Path to save FHE keys (for later decryption)
    #[arg(long, default_value = "~/.zyb/fhe_keys")]
    pub fhe_keys_dir: String,

    /// Path to shared FHE keys directory (load instead of generate)
    /// Directory should contain fhe_client_key.bin and fhe_server_key.bin
    #[arg(long)]
    pub shared_keys_dir: Option<String>,
}

#[derive(Args, Debug)]
pub struct ClaimArgs {
    /// Market ID
    #[arg(long)]
    pub market_id: String,

    /// Bet ID (job ID from bet placement)
    #[arg(long)]
    pub bet_id: i64,

    /// Claimer public key
    #[arg(long)]
    pub claimer: String,

    /// Server URL
    #[arg(long, default_value = "http://localhost:3000")]
    pub server: String,

    /// Path to output witness file
    #[arg(long, default_value = "./claim_witness.json")]
    pub witness_output: PathBuf,

    /// Path to Solana keypair for payment
    #[arg(long)]
    pub keypair: Option<PathBuf>,

    /// Solana RPC URL
    #[arg(long, default_value = "https://api.devnet.solana.com")]
    pub rpc_url: String,

    /// Futarchy Markets program ID
    #[arg(long, default_value = "5B8x1aJEHsMLqqKDYVQe2dTA38hbie1QXX2JSmPAxWJT")]
    pub futarchy_program: String,

    /// ZK Generator program ID
    #[arg(long, default_value = "ZkGenerator11111111111111111111111111111111")]
    pub zk_generator_program: String,

    /// Skip payment confirmation
    #[arg(long)]
    pub skip_confirm: bool,

    /// Path to bet witness file (contains secret, blinding, bet data)
    #[arg(long)]
    pub bet_witness: Option<PathBuf>,

    /// Path to circuit WASM file
    #[arg(long, default_value = "circuits/market/market_claim_js/market_claim.wasm")]
    pub circuit_wasm: PathBuf,

    /// Path to circuit zkey file
    #[arg(long, default_value = "circuits/market/market_claim_final.zkey")]
    pub circuit_zkey: PathBuf,

    /// Generate proof locally (vs delegate to prover)
    #[arg(long)]
    pub local_proof: bool,
}

#[derive(Args, Debug)]
pub struct VerifyArgs {
    /// Settlement job ID
    #[arg(long)]
    pub job_id: i64,

    /// Server URL
    #[arg(long, default_value = "http://localhost:3000")]
    pub server: String,
}

// === V2 Args (PrivateBalance architecture) ===

#[derive(Args, Debug)]
pub struct InitProtocolArgs {
    /// Path to Solana keypair (authority) - overrides config and USER_KEYPAIR env var
    #[arg(long)]
    pub keypair: Option<PathBuf>,

    /// Solana RPC URL - overrides config and SOLANA_RPC_URL env var
    #[arg(long)]
    pub rpc_url: Option<String>,

    /// Config file path (default: zyb.toml in current directory or ~/.config/zyb/zyb.toml)
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Profile name from config (overrides common.profile)
    #[arg(long)]
    pub profile: Option<String>,

    /// Futarchy Markets program ID - overrides config and FUTARCHY_PROGRAM_ID env var
    #[arg(long)]
    pub futarchy_program_id: Option<String>,
}

#[derive(Args, Debug)]
pub struct CreateV2Args {
    /// Numeric market ID
    #[arg(long)]
    pub market_id: u64,

    /// Oracle public key (who can settle)
    #[arg(long)]
    pub oracle: Option<String>,

    /// Market end time in unix timestamp
    #[arg(long)]
    pub end_time: i64,

    /// Maximum bet in lamports
    #[arg(long, default_value = "10000000000")]
    pub max_bet: u64,

    /// Enable governance for this market
    #[arg(long)]
    pub governance: bool,

    /// Governance execution threshold (0-100)
    #[arg(long, default_value = "51")]
    pub threshold: u8,

    /// Governance timelock in seconds
    #[arg(long, default_value = "86400")]
    pub timelock: i64,

    /// Path to Solana keypair - overrides config and USER_KEYPAIR env var
    #[arg(long)]
    pub keypair: Option<PathBuf>,

    /// Solana RPC URL - overrides config and SOLANA_RPC_URL env var
    #[arg(long)]
    pub rpc_url: Option<String>,

    /// Config file path (default: zyb.toml in current directory or ~/.config/zyb/zyb.toml)
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Profile name from config (overrides common.profile)
    #[arg(long)]
    pub profile: Option<String>,

    /// Futarchy Markets program ID - overrides config and FUTARCHY_PROGRAM_ID env var
    #[arg(long)]
    pub futarchy_program_id: Option<String>,
}

#[derive(Args, Debug)]
pub struct CreateBalanceArgs {
    /// Path to Solana keypair (user)
    #[arg(long)]
    pub keypair: PathBuf,

    /// Solana RPC URL
    #[arg(long, default_value = "http://localhost:8899")]
    pub rpc_url: String,
}

#[derive(Args, Debug)]
pub struct DepositPrivateArgs {
    /// Amount to deposit in lamports
    #[arg(long)]
    pub amount: u64,

    /// Path to Solana keypair
    #[arg(long)]
    pub keypair: PathBuf,

    /// Solana RPC URL
    #[arg(long, default_value = "http://localhost:8899")]
    pub rpc_url: String,
}

#[derive(Args, Debug)]
pub struct BetPrivateArgs {
    /// Market ID
    #[arg(long)]
    pub market_id: u64,

    /// Bet side (yes/no)
    #[arg(long, value_enum)]
    pub side: BetSide,

    /// Bet amount in lamports
    #[arg(long)]
    pub amount: u64,

    /// Path to Solana keypair
    #[arg(long)]
    pub keypair: PathBuf,

    /// Path to save bet witness
    #[arg(long, default_value = "./bet_witness_v2.json")]
    pub witness_output: PathBuf,

    /// Solana RPC URL
    #[arg(long, default_value = "http://localhost:8899")]
    pub rpc_url: String,

    /// Path to circuit WASM (for local proof)
    #[arg(long)]
    pub circuit_wasm: Option<PathBuf>,

    /// Path to circuit zkey (for local proof)
    #[arg(long)]
    pub circuit_zkey: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct ClaimPrivateArgs {
    /// Market ID
    #[arg(long)]
    pub market_id: u64,

    /// Path to bet witness file
    #[arg(long)]
    pub bet_witness: PathBuf,

    /// Path to Solana keypair
    #[arg(long)]
    pub keypair: PathBuf,

    /// Solana RPC URL
    #[arg(long, default_value = "http://localhost:8899")]
    pub rpc_url: String,

    /// Path to circuit WASM (for real proof generation)
    #[arg(long)]
    pub circuit_wasm: Option<PathBuf>,

    /// Path to circuit zkey (for real proof generation)
    #[arg(long)]
    pub circuit_zkey: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct WithdrawPrivateArgs {
    /// Amount to withdraw in lamports
    #[arg(long)]
    pub amount: u64,

    /// Path to Solana keypair
    #[arg(long)]
    pub keypair: PathBuf,

    /// Solana RPC URL
    #[arg(long, default_value = "http://localhost:8899")]
    pub rpc_url: String,

    /// Path to circuit WASM (for real proof generation)
    #[arg(long)]
    pub circuit_wasm: Option<PathBuf>,

    /// Path to circuit zkey (for real proof generation)
    #[arg(long)]
    pub circuit_zkey: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct SettleV2Args {
    /// Market ID to settle
    #[arg(long)]
    pub market_id: u64,

    /// Winning outcome (yes/no)
    #[arg(long, value_enum)]
    pub outcome: BetSide,

    /// Path to oracle keypair
    #[arg(long)]
    pub keypair: PathBuf,

    /// Solana RPC URL
    #[arg(long, default_value = "http://localhost:8899")]
    pub rpc_url: String,
}

// =============================================================================
// V3 Args (Fully Blind Markets)
// =============================================================================

#[derive(Args, Debug)]
pub struct CreateV3Args {
    /// Numeric market ID
    #[arg(long)]
    pub market_id: u64,

    /// Market question (will be hashed)
    #[arg(long)]
    pub question: String,

    /// Oracle public key (who can settle)
    #[arg(long)]
    pub oracle: Option<String>,

    /// Market end time in unix timestamp
    #[arg(long)]
    pub end_time: i64,

    /// Maximum bet in lamports
    #[arg(long, default_value = "10000000000")]
    pub max_bet: u64,

    /// Threshold public key for FHE decryption (32 bytes hex)
    #[arg(long)]
    pub threshold_pubkey: String,

    /// Number of threshold shares required (t in t-of-n)
    #[arg(long, default_value = "3")]
    pub threshold_required: u8,

    /// Total number of threshold shares (n in t-of-n)
    #[arg(long, default_value = "5")]
    pub threshold_shares: u8,

    /// Enable governance for this market
    #[arg(long)]
    pub has_governance: bool,

    /// Path to Solana keypair
    #[arg(long)]
    pub keypair: PathBuf,

    /// Solana RPC URL
    #[arg(long, default_value = "http://localhost:8899")]
    pub rpc_url: String,
}

#[derive(Args, Debug)]
pub struct BetBlindArgs {
    /// Market ID to bet on
    #[arg(long)]
    pub market_id: u64,

    /// Bet amount in lamports
    #[arg(long)]
    pub amount: u64,

    /// Bet side (yes/no)
    #[arg(long, value_enum)]
    pub side: BetSide,

    /// Path to Solana keypair
    #[arg(long)]
    pub keypair: PathBuf,

    /// Solana RPC URL
    #[arg(long, default_value = "http://localhost:8899")]
    pub rpc_url: String,

    /// Path to PlaceBetBlind circuit WASM
    #[arg(long, default_value = "circuits/blind/place_bet_blind_js/place_bet_blind.wasm")]
    pub circuit_wasm: PathBuf,

    /// Path to witness calculator JS
    #[arg(long, default_value = "circuits/blind/place_bet_blind_js/witness_calculator.js")]
    pub witness_calc: PathBuf,

    /// Path to zkey file
    #[arg(long, default_value = "circuits/blind/place_bet_blind.zkey")]
    pub circuit_zkey: PathBuf,

    /// Output file for bet witness (for claim)
    #[arg(long, default_value = "bet_witness_v3.json")]
    pub witness_output: PathBuf,

    /// FHE key file path
    #[arg(long, default_value = "fhe_keys.json")]
    pub fhe_keys: PathBuf,
}

#[derive(Args, Debug)]
pub struct SettleV3Args {
    /// Market ID to settle
    #[arg(long)]
    pub market_id: u64,

    /// Winning outcome (yes/no)
    #[arg(long, value_enum)]
    pub outcome: BetSide,

    /// Decrypted YES pool amount
    #[arg(long)]
    pub decrypted_yes: u64,

    /// Decrypted NO pool amount
    #[arg(long)]
    pub decrypted_no: u64,

    /// Path to threshold signatures file (JSON array of base64 signatures)
    #[arg(long)]
    pub threshold_sigs: PathBuf,

    /// Path to oracle keypair
    #[arg(long)]
    pub keypair: PathBuf,

    /// Solana RPC URL
    #[arg(long, default_value = "http://localhost:8899")]
    pub rpc_url: String,
}

#[derive(Args, Debug)]
pub struct ClaimV3Args {
    /// Market ID to claim from
    #[arg(long)]
    pub market_id: u64,

    /// Path to bet witness file
    #[arg(long)]
    pub witness_file: PathBuf,

    /// Path to Solana keypair
    #[arg(long)]
    pub keypair: PathBuf,

    /// Solana RPC URL
    #[arg(long, default_value = "http://localhost:8899")]
    pub rpc_url: String,

    /// Path to ClaimBlind circuit WASM
    #[arg(long)]
    pub circuit_wasm: Option<PathBuf>,

    /// Path to ClaimBlind zkey
    #[arg(long)]
    pub circuit_zkey: Option<PathBuf>,
}

pub fn handle_market_command(cmd: MarketCommands) -> Result<()> {
    match cmd {
        MarketCommands::Create(args) => create_market(args),
        MarketCommands::Deposit(args) => deposit_funds(args),
        MarketCommands::Bet(args) => place_bet(args),
        MarketCommands::Claim(args) => claim_winnings(args),
        MarketCommands::Verify(args) => verify_settlement(args),
        // V2 commands
        MarketCommands::InitProtocol(args) => init_protocol_v2(args),
        MarketCommands::CreateV2(args) => create_market_v2(args),
        MarketCommands::CreateBalance(args) => create_balance_v2(args),
        MarketCommands::DepositPrivate(args) => deposit_private_v2(args),
        MarketCommands::BetPrivate(args) => bet_private_v2(args),
        MarketCommands::ClaimPrivate(args) => claim_private_v2(args),
        MarketCommands::WithdrawPrivate(args) => withdraw_private_v2(args),
        MarketCommands::SettleV2(args) => settle_market_v2(args),
        // V3 commands
        MarketCommands::CreateV3(args) => create_market_v3(args),
        MarketCommands::BetBlind(args) => bet_blind_v3(args),
        MarketCommands::SettleV3(args) => settle_market_v3(args),
        MarketCommands::ClaimV3(args) => claim_v3(args),
    }
}

#[tokio::main]
async fn deposit_funds(args: DepositArgs) -> Result<()> {
    use solana_sdk::signature::{read_keypair_file, Signer};
    use solana_sdk::transaction::Transaction;

    println!("{}", "Depositing funds to market escrow...".cyan().bold());
    println!();
    println!("Market ID: {}", args.market_id);
    println!("Amount: {} lamports ({:.4} SOL)", args.amount, args.amount as f64 / 1_000_000_000.0);
    println!();

    // Load keypair
    let keypair = read_keypair_file(&args.keypair)
        .map_err(|e| anyhow::anyhow!("Failed to read keypair: {}", e))?;
    let user = keypair.pubkey();

    println!("User: {}", user);
    println!();

    // Get program ID from env or use default
    let futarchy_program_id_str = std::env::var("FUTARCHY_PROGRAM_ID")
        .unwrap_or_else(|_| "AQUUuRSwDhB1eeC2Caa8GPVGV4YzZkJ1YiSvZd3BBPij".to_string());
    let futarchy_program_id = solana_sdk::pubkey::Pubkey::from_str(&futarchy_program_id_str)
        .context("Invalid FUTARCHY_PROGRAM_ID")?;

    // Build deposit instruction
    println!("{}", "Building deposit transaction...".cyan());
    let instruction = futarchy_sdk::build_deposit_to_market_ix(
        &futarchy_program_id,
        &user,
        args.market_id,
        args.amount,
    ).context("Failed to build deposit instruction")?;

    // Create and sign transaction
    let rpc_client = solana_client::rpc_client::RpcClient::new(&args.rpc_url);
    let blockhash = rpc_client.get_latest_blockhash()
        .context("Failed to get blockhash")?;

    let mut tx = Transaction::new_with_payer(&[instruction], Some(&user));
    tx.sign(&[&keypair], blockhash);

    println!("{}", "Transaction signed.".green());
    println!();

    // Send transaction
    println!("{}", "Sending transaction to Solana...".cyan());
    let signature = rpc_client.send_and_confirm_transaction(&tx)
        .context("Failed to send transaction")?;

    println!();
    println!("{}", "Deposit successful!".green().bold());
    println!();
    println!("TX Signature: {}", signature.to_string().yellow().bold());
    println!("Amount deposited: {} lamports", args.amount);
    println!();
    println!("{}", "Next steps:".cyan());
    println!("  Place bet: zyb market bet --market-id {} --use-fhe --amount <AMOUNT> --side yes/no --keypair <PATH>", args.market_id);

    Ok(())
}

#[tokio::main]
async fn create_market(args: CreateArgs) -> Result<()> {
    use solana_sdk::signature::{read_keypair_file, Signer};
    use solana_sdk::transaction::Transaction;

    println!("{}", "Creating prediction market on-chain...".cyan().bold());
    println!();
    println!("Question: {}", args.question.green());
    println!("Market ID: {}", args.market_id);
    println!("Max bet: {} lamports", args.max_bet);
    println!();

    // Load keypair
    let keypair = read_keypair_file(&args.keypair)
        .map_err(|e| anyhow::anyhow!("Failed to read keypair: {}", e))?;
    let creator = keypair.pubkey();
    let creator_str = creator.to_string();
    let oracle = args.oracle
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or(&creator_str);

    println!("Creator: {}", creator);
    println!("Oracle: {}", oracle);
    println!();

    // Step 1: Request unsigned transaction from server
    println!("{}", "Requesting transaction from server...".cyan());

    let client = reqwest::Client::new();
    let request_body = serde_json::json!({
        "question": args.question,
        "creator": creator.to_string(),
        "oracle": oracle,
        "market_id": args.market_id,
        "max_bet_lamports": args.max_bet as i64,
        "resolution_window_secs": args.resolution_window,
    });

    let response = client
        .post(&format!("{}/api/futarchy/markets/validate-and-build", args.server))
        .json(&request_body)
        .send()
        .await
        .context("Failed to request transaction")?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        anyhow::bail!("Server error: {}", error_text);
    }

    let resp_data: serde_json::Value = response.json().await?;
    let unsigned_tx_b64 = resp_data["unsigned_transaction"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing unsigned_transaction in response"))?;

    println!("{}", "Transaction received.".green());

    // Step 2: Decode and sign transaction
    println!("{}", "Signing transaction...".cyan());

    let unsigned_tx_bytes = crate::client::futarchy::decode_base64(unsigned_tx_b64)
        .context("Failed to decode transaction")?;
    let mut tx: Transaction = bincode::deserialize(&unsigned_tx_bytes)
        .context("Failed to deserialize transaction")?;

    let rpc_client = solana_client::rpc_client::RpcClient::new(&args.rpc_url);
    let blockhash = rpc_client.get_latest_blockhash()
        .context("Failed to get blockhash")?;

    tx.sign(&[&keypair], blockhash);

    println!("{}", "Transaction signed.".green());

    // Step 3: Send transaction
    println!("{}", "Sending transaction to Solana...".cyan());

    let signature = rpc_client.send_and_confirm_transaction(&tx)
        .context("Failed to send transaction")?;

    println!();
    println!("{}", "Market created successfully!".green().bold());
    println!();
    println!("TX Signature: {}", signature.to_string().yellow().bold());
    println!("Market ID: {}", args.market_id);
    println!();
    println!("{}", "Next steps:".cyan());
    println!("  1. Place bets: zyb market bet --market-id {} --use-fhe", args.market_id);
    println!("  2. Settle: POST /api/futarchy/markets/{}/settle", args.market_id);
    println!("  3. Claim: zyb market claim --market-id {} --local-proof", args.market_id);

    Ok(())
}

#[tokio::main]
async fn place_bet(mut args: BetArgs) -> Result<()> {
    // Load global config
    let global_config = crate::config::load_global_config(args.config.clone())?;

    // Get active profile
    let profile = if let Some(ref cfg) = global_config {
        if let Some(profile_name) = args.profile.as_ref().or_else(|| cfg.common.profile.as_ref()) {
            cfg.profiles.get(profile_name).cloned()
        } else {
            None
        }
    } else {
        None
    };

    // Apply cascade for configuration fields
    if args.rpc_url.is_none() {
        args.rpc_url = Some(
            profile.as_ref().and_then(|p| p.rpc_url.clone())
                .or_else(|| std::env::var("SOLANA_RPC_URL").ok())
                .unwrap_or_else(|| "http://localhost:8899".to_string())
        );
    }

    if args.server.is_none() {
        args.server = Some(
            profile.as_ref().and_then(|p| p.backend_url.clone())
                .or_else(|| std::env::var("BACKEND_URL").ok())
                .unwrap_or_else(|| "http://localhost:3000".to_string())
        );
    }

    if args.futarchy_server.is_none() {
        args.futarchy_server = Some(
            profile.as_ref().and_then(|p| p.backend_url.clone())
                .or_else(|| std::env::var("FUTARCHY_SERVER_URL").ok())
                .unwrap_or_else(|| "http://localhost:9000".to_string())
        );
    }

    if args.keypair.is_none() {
        args.keypair = profile.as_ref().and_then(|p| p.keypair.clone())
            .or_else(|| std::env::var("USER_KEYPAIR").ok().map(PathBuf::from))
            .map(|p| crate::config::expand_tilde(&p));
    }

    // Validate amount
    if args.amount == 0 {
        anyhow::bail!("Bet amount must be greater than 0");
    }

    // Route to appropriate handler based on mode
    if args.use_fhe {
        place_bet_fhe(args).await
    } else {
        place_bet_zk(args).await
    }
}

/// Place bet using FHE encryption (encrypted pool aggregation)
async fn place_bet_fhe(args: BetArgs) -> Result<()> {
    use zyberlink_fhe::futarchy::{encrypt_bet_with_hash, hash_ciphertext_hex};
    use zyberlink_fhe::{generate_keys, serialize_client_key, serialize_server_key};
    use solana_sdk::signature::{read_keypair_file, Signer};
    use solana_sdk::transaction::Transaction;

    println!("{}", "Placing FHE-encrypted bet...".cyan().bold());
    println!();

    // Validate side is provided for FHE mode
    let side = args.side.ok_or_else(|| {
        anyhow::anyhow!("--side (yes/no) is required for FHE mode")
    })?;
    let side_bool = matches!(side, BetSide::Yes);

    let market_id = args.market_id.clone();
    let market_id_u64: u64 = market_id.parse()
        .context("Invalid market_id format (must be numeric)")?;

    println!("Market ID: {}", market_id);
    println!("Side: {}", if side_bool { "YES" } else { "NO" });
    println!("Amount: {} lamports", args.amount);
    println!("Mode: FHE encrypted");
    println!();

    // Generate cryptographic secrets for claim proof (32 bytes each)
    use rand::RngCore;
    let mut rng = rand::thread_rng();

    let mut secret = [0u8; 32];
    let mut blinding = [0u8; 32];
    rng.fill_bytes(&mut secret);
    rng.fill_bytes(&mut blinding);

    // Calculate bet_commitment = hash(secret || amount || side || blinding)
    // Note: Production should use Poseidon hash to match circom circuit
    let bet_commitment = {
        use sha3::{Digest, Keccak256};
        let mut hasher = Keccak256::new();
        hasher.update(&secret);
        hasher.update(&args.amount.to_le_bytes());
        hasher.update(&[if side_bool { 1u8 } else { 0u8 }]);
        hasher.update(&blinding);
        let result = hasher.finalize();
        let mut commitment = [0u8; 32];
        commitment.copy_from_slice(&result);
        commitment
    };

    // Require keypair for FHE mode (needed to sign TX)
    let keypair_path = args.keypair.as_ref().ok_or_else(|| {
        anyhow::anyhow!("--keypair is required for FHE mode (needed to sign transaction)")
    })?;

    let keypair = read_keypair_file(keypair_path)
        .map_err(|e| anyhow::anyhow!("Failed to read keypair: {}", e))?;

    let bettor_pubkey = keypair.pubkey().to_string();
    println!("Bettor: {}", bettor_pubkey);
    println!();

    // Step 1: Load or generate FHE keys
    let (client_key, server_key) = if let Some(ref shared_dir) = args.shared_keys_dir {
        let dir = shellexpand::tilde(shared_dir).to_string();
        let client_path = format!("{}/fhe_client_key.bin", dir);
        let server_path = format!("{}/fhe_server_key.bin", dir);

        println!("{}", format!("Loading shared FHE keys from: {}", dir).cyan());

        let client_bytes = fs::read(&client_path)
            .context("Failed to read fhe_client_key.bin")?;
        let server_bytes = fs::read(&server_path)
            .context("Failed to read fhe_server_key.bin")?;

        use zyberlink_fhe::{deserialize_client_key, deserialize_server_key};
        let client_key = deserialize_client_key(&client_bytes)
            .context("Failed to deserialize client key")?;
        let server_key = deserialize_server_key(&server_bytes)
            .context("Failed to deserialize server key")?;

        println!("{}", "Shared FHE keys loaded.".green());
        (client_key, server_key)
    } else {
        println!("{}", "Generating FHE keys (this may take ~30s)...".cyan());
        let keys = generate_keys()
            .context("Failed to generate FHE keys")?;
        println!("{}", "FHE keys generated.".green());
        keys
    };

    // Step 2: Encrypt the bet amount
    println!("{}", "Encrypting bet amount...".cyan());
    let encrypted_bet = encrypt_bet_with_hash(args.amount, &client_key)
        .context("Failed to encrypt bet amount")?;

    let ciphertext_hash = encrypted_bet.hash_hex();
    println!("Ciphertext size: {} bytes", encrypted_bet.size());
    println!("Ciphertext hash: {}...", &ciphertext_hash[..16]);
    println!();

    // Step 3: Save FHE keys for later decryption
    let keys_dir = shellexpand::tilde(&args.fhe_keys_dir).to_string();
    fs::create_dir_all(&keys_dir)?;

    let key_file = format!("{}/bet_{}_{}.keys", keys_dir, market_id, &ciphertext_hash[..8]);
    let client_key_bytes = serialize_client_key(&client_key)?;
    let server_key_bytes = serialize_server_key(&server_key)?;

    let keys_data = json!({
        "market_id": market_id,
        "ciphertext_hash": ciphertext_hash,
        "client_key": encode_base64(&client_key_bytes),
        "server_key_hash": hash_ciphertext_hex(&server_key_bytes),
        "created_at": chrono::Utc::now().to_rfc3339(),
    });
    fs::write(&key_file, serde_json::to_string_pretty(&keys_data)?)?;
    println!("{}", format!("Keys saved to: {}", key_file).green());
    println!();

    // Step 4: Call prepare endpoint
    println!("{}", "Preparing transaction...".cyan());
    let futarchy_server = args.futarchy_server.as_ref()
        .expect("futarchy_server should be resolved in place_bet");
    let futarchy_client = FutarchyClient::with_url(futarchy_server);

    // Generate placeholder proof (256 bytes for Groth16, 80 bytes public inputs)
    // In FHE mode, ZK verification is skipped but size validation still applies
    let proof_placeholder = encode_base64(&[0u8; 256]);
    let public_inputs_placeholder = encode_base64(&[0u8; 80]);

    // market_id_u64 already parsed at the start of function

    let prepare_req = PrepareBetRequest {
        market_id,
        bettor: bettor_pubkey.clone(),
        side: side_bool,
        amount_lamports: args.amount,
        ciphertext_hash: ciphertext_hash.clone(),
        proof: proof_placeholder,
        public_inputs: public_inputs_placeholder,
        // Circuit 30 = basic MarketBet (no eligibility required)
        // Circuit 31 = MarketBetWithPoI (requires RegisterUser first)
        // TODO: Add --require-eligibility flag to switch between 30/31
        circuit_type: 30,
    };

    let prepare_resp = futarchy_client.prepare_bet(prepare_req).await
        .context("Failed to prepare bet transaction")?;

    println!("{}", "Transaction prepared.".green());
    println!();

    // Step 5: Sign the transaction
    println!("{}", "Signing transaction...".cyan());
    let unsigned_tx_bytes = crate::client::futarchy::decode_base64(&prepare_resp.unsigned_transaction)
        .context("Failed to decode unsigned transaction")?;

    let mut tx: Transaction = bincode::deserialize(&unsigned_tx_bytes)
        .context("Failed to deserialize unsigned transaction")?;

    // Get recent blockhash
    let rpc_url = args.rpc_url.as_ref()
        .expect("rpc_url should be resolved in place_bet");
    let rpc_client = solana_client::rpc_client::RpcClient::new(rpc_url);
    let blockhash = rpc_client.get_latest_blockhash()
        .context("Failed to get recent blockhash")?;

    tx.sign(&[&keypair], blockhash);

    let signed_tx_bytes = bincode::serialize(&tx)
        .context("Failed to serialize signed transaction")?;
    println!("{}", "Transaction signed.".green());
    println!();

    // Step 6: Submit signed TX with ciphertext
    println!("{}", "Submitting bet to network...".cyan());
    let submit_req = SubmitBetRequest {
        signed_tx: encode_base64(&signed_tx_bytes),
        ciphertext: encode_base64(&encrypted_bet.ciphertext),
        server_key: Some(encode_base64(&server_key_bytes)),
        market_id: Some(market_id_u64),
        side: Some(side_bool),
    };

    let submit_resp = futarchy_client.submit_bet(submit_req).await
        .context("Failed to submit bet")?;

    println!();
    println!("{}", "FHE Bet placed successfully!".green().bold());
    println!();
    println!("TX Signature: {}", submit_resp.tx_signature.yellow().bold());
    println!("Ciphertext Hash: {}", submit_resp.ciphertext_hash);
    println!("Status: {}", submit_resp.status);
    println!();
    println!("{}", "The bet amount is encrypted. Provers will aggregate it".cyan());
    println!("{}", "to the pool using homomorphic encryption.".cyan());
    println!();

    // Save bet witness for later claim (contains private data for ZK proof)
    let bet_witness = BetWitness {
        market_id: market_id_u64,
        secret: hex::encode(secret),
        blinding: hex::encode(blinding),
        bet_amount: args.amount,
        bet_side: if side_bool { 1 } else { 0 },
        bet_commitment: hex::encode(bet_commitment),
    };

    let witness_json = serde_json::to_string_pretty(&bet_witness)?;
    fs::write(&args.witness_output, &witness_json)?;

    println!("{}", "Bet witness saved:".green());
    println!("  {:?}", args.witness_output);
    println!();
    println!("{}", format!("FHE keys saved: {}", key_file).yellow());
    println!();
    println!("{}", "IMPORTANT: Keep both files safe!".yellow().bold());
    println!("  - bet_witness.json → needed for claim proof");
    println!("  - FHE keys → needed for decryption");

    Ok(())
}

/// Place bet using ZK proofs (original flow)
async fn place_bet_zk(args: BetArgs) -> Result<()> {
    let outcome = args.outcome.ok_or_else(|| {
        anyhow::anyhow!("--outcome is required for ZK mode")
    })?;

    println!("{}", "Placing private bet (ZK mode)...".cyan().bold());
    println!();
    println!("Market ID: {}", args.market_id);
    println!("Selected outcome: {}", outcome);
    println!("Bet amount: {} lamports", args.amount);
    println!("Circuit: MarketBet (31)");
    println!();

    // Build witness
    println!("{}", "Building witness...".cyan());

    let witness = json!({
        "marketId": args.market_id,
        "outcome": outcome,
        "amount": args.amount.to_string(),
        "bettorPubkey": args.bettor,
        "timestamp": chrono::Utc::now().timestamp(),
        "publicInputs": []
    });

    // Save witness to file
    fs::write(&args.witness_output, serde_json::to_string_pretty(&witness)?)?;

    println!("{}", "Witness saved to:".green());
    println!("  {:?}", args.witness_output);
    println!();

    // Create ZK job
    println!("{}", "Submitting bet as ZK job...".cyan());

    let witness_str = serde_json::to_string(&witness)?;
    let witness_commitment = {
        use sha3::{Digest, Keccak256};
        let mut hasher = Keccak256::new();
        hasher.update(witness_str.as_bytes());
        hex::encode(hasher.finalize())
    };

    let server_url = args.server.as_ref()
        .expect("server should be resolved in place_bet");
    let client = ZkClient::with_url(server_url);

    let request = CreateJobRequest {
        circuit_type: CIRCUIT_MARKET_BET,
        witness_commitment,
        public_inputs: vec![],
        creator: args.bettor.clone(),
        timeout_seconds: None,
    };

    // Execute payment flow if keypair provided
    let response = if let Some(ref keypair) = args.keypair {
        use crate::commands::zk::execute_payment_flow_helper;

        let rpc_url = args.rpc_url.as_ref()
            .expect("rpc_url should be resolved in place_bet");

        execute_payment_flow_helper(
            &client,
            request,
            keypair,
            rpc_url,
            CIRCUIT_MARKET_BET,
            &args.bettor,
            args.skip_confirm,
        )
        .await?
    } else {
        client.create_job(request).await?
    };

    println!("{}", "Bet placed successfully!".green().bold());
    println!();
    println!("Job ID: {}", response.job_id.to_string().yellow().bold());
    println!("Status: {}", response.status);
    println!();
    println!("{}", "Your bet is private and will be proven using ZK.".cyan());
    println!(
        "{}",
        "Save this Job ID - you'll need it to claim winnings!".yellow().bold()
    );
    println!("Check status with: zyb zk status {}", response.job_id);

    Ok(())
}

#[tokio::main]
async fn claim_winnings(args: ClaimArgs) -> Result<()> {
    println!("{}", "Claiming market winnings...".cyan().bold());
    println!();
    println!("Market ID: {}", args.market_id);

    if args.local_proof {
        claim_winnings_local(args).await
    } else {
        claim_winnings_delegated(args).await
    }
}

/// Generate proof locally and submit to claim endpoint directly
async fn claim_winnings_local(args: ClaimArgs) -> Result<()> {
    use futarchy_sdk::claim::{
        generate_market_claim_proof, MarketClaimWitness, MarketClaimProofPaths, SnarkjsCommand,
    };

    println!("{}", "Mode: Local proof generation".cyan());
    println!();

    // Step 1: Load bet witness
    let bet_witness_path = args.bet_witness.as_ref().ok_or_else(|| {
        anyhow::anyhow!("--bet-witness is required for local proof generation")
    })?;

    println!("{}", format!("Loading bet witness from: {:?}", bet_witness_path).cyan());
    let bet_witness_data = fs::read_to_string(bet_witness_path)
        .context("Failed to read bet witness file")?;
    let bet_witness: BetWitness = serde_json::from_str(&bet_witness_data)
        .context("Failed to parse bet witness JSON")?;

    println!("{}", "Bet witness loaded.".green());
    println!();

    // Step 2: Fetch market data from server
    println!("{}", "Fetching market data...".cyan());
    let market_id = args.market_id.parse::<u64>()
        .context("Invalid market_id format")?;

    let client = reqwest::Client::new();
    let market_url = format!("{}/api/futarchy/markets/{}/claim-data", args.server, market_id);

    let market_data: MarketData = client
        .get(&market_url)
        .send()
        .await
        .context("Failed to fetch market data")?
        .json()
        .await
        .context("Failed to parse market data response")?;

    println!("Market data fetched:");
    println!("  Total pool: {} lamports", market_data.total_pool);
    println!("  Winning pool: {} lamports", market_data.winning_pool);
    println!("  Resolution: {:?}", market_data.resolution);
    println!();

    // Validate market is resolved
    let resolution = market_data.resolution.ok_or_else(|| {
        anyhow::anyhow!("Market {} is not resolved yet", market_id)
    })?;

    // Step 3: Parse bet witness fields
    let secret = hex::decode(&bet_witness.secret)
        .context("Invalid secret hex")?;
    let secret_bytes: [u8; 32] = secret.try_into()
        .map_err(|_| anyhow::anyhow!("Secret must be 32 bytes"))?;

    let blinding = hex::decode(&bet_witness.blinding)
        .context("Invalid blinding hex")?;
    let blinding_bytes: [u8; 32] = blinding.try_into()
        .map_err(|_| anyhow::anyhow!("Blinding must be 32 bytes"))?;

    let bet_commitment = hex::decode(&bet_witness.bet_commitment)
        .context("Invalid bet_commitment hex")?;
    let bet_commitment_bytes: [u8; 32] = bet_commitment.try_into()
        .map_err(|_| anyhow::anyhow!("Bet commitment must be 32 bytes"))?;

    // Step 4: Calculate payout
    println!("{}", "Calculating payout...".cyan());
    let payout_amount = futarchy_sdk::claim::calculate_payout(
        bet_witness.bet_amount,
        market_data.total_yes_bets,
        market_data.total_no_bets,
        resolution == 1,
        bet_witness.bet_side == 1,
    ).ok_or_else(|| anyhow::anyhow!("You didn't win this bet"))?;

    println!("Payout calculated: {} lamports", payout_amount);
    println!();

    // Step 5: Generate nullifier
    let nullifier = futarchy_sdk::claim::generate_nullifier(&secret_bytes, &bet_commitment_bytes);

    // Step 6: Build circuit witness
    println!("{}", "Building circuit witness...".cyan());
    let timestamp = chrono::Utc::now().timestamp();

    let circuit_witness = MarketClaimWitness {
        market_id,
        nullifier,
        payout_amount,
        resolution,
        total_pool: market_data.total_pool,
        winning_pool: market_data.winning_pool,
        bet_commitment: bet_commitment_bytes,
        timestamp,
        secret: secret_bytes,
        bet_amount: bet_witness.bet_amount,
        bet_side: bet_witness.bet_side,
        blinding: blinding_bytes,
    };

    println!("{}", "Witness built.".green());
    println!();

    // Step 7: Generate proof with snarkjs
    println!("{}", "Generating ZK proof locally (this may take 10-30s)...".cyan());

    let snarkjs = SnarkjsCommand::npx();
    let paths = MarketClaimProofPaths {
        wasm_path: args.circuit_wasm.clone(),
        zkey_path: args.circuit_zkey.clone(),
    };

    let proof = generate_market_claim_proof(&snarkjs, &paths, &circuit_witness)
        .context("Failed to generate proof")?;

    println!("{}", "Proof generated successfully!".green());
    println!("  Proof size: {} bytes", proof.proof.len());
    println!("  Public inputs size: {} bytes", proof.public_inputs.len());
    println!();

    // Step 8: Build and send transaction directly to Solana
    use solana_sdk::signature::{read_keypair_file, Signer};
    use solana_sdk::transaction::Transaction;
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    println!("{}", "Building claim transaction...".cyan());

    // Require keypair
    let keypair_path = args.keypair.as_ref().ok_or_else(|| {
        anyhow::anyhow!("--keypair is required to sign the claim transaction")
    })?;

    let keypair = read_keypair_file(keypair_path)
        .map_err(|e| anyhow::anyhow!("Failed to read keypair: {}", e))?;

    let user_pubkey = keypair.pubkey();
    println!("Claimer: {}", user_pubkey);

    // Parse program IDs
    let futarchy_program = Pubkey::from_str(&args.futarchy_program)
        .context("Invalid futarchy program ID")?;
    let zk_generator_program = Pubkey::from_str(&args.zk_generator_program)
        .context("Invalid zk generator program ID")?;

    // Build the claim instruction
    let claim_ix = futarchy_sdk::instructions::build_claim_payout_ix(
        &futarchy_program,
        &user_pubkey,
        market_id,
        nullifier,
        bet_commitment_bytes,
        proof.proof.clone(),
        proof.public_inputs.clone(),
        payout_amount,
        &zk_generator_program,
    ).context("Failed to build claim instruction")?;

    // Create and sign transaction
    let rpc_client = solana_client::rpc_client::RpcClient::new(&args.rpc_url);
    let blockhash = rpc_client.get_latest_blockhash()
        .context("Failed to get recent blockhash")?;

    let mut tx = Transaction::new_with_payer(&[claim_ix], Some(&user_pubkey));
    tx.sign(&[&keypair], blockhash);

    println!("{}", "Transaction signed.".green());
    println!();

    // Send transaction
    println!("{}", "Sending transaction to Solana...".cyan());

    let signature = rpc_client.send_and_confirm_transaction(&tx)
        .context("Failed to send transaction")?;

    println!();
    println!("{}", "Claim successful!".green().bold());
    println!();
    println!("TX Signature: {}", signature.to_string().yellow().bold());
    println!("Payout: {} lamports", payout_amount);
    println!("Nullifier: {}", hex::encode(nullifier));

    Ok(())
}

/// Delegate proof generation to ZK server (original flow)
async fn claim_winnings_delegated(args: ClaimArgs) -> Result<()> {
    println!("Bet ID: {}", args.bet_id);
    println!("Circuit: MarketSettlement (32)");
    println!();

    // First, verify the bet was successful
    let client = ZkClient::with_url(&args.server);

    println!("{}", "Verifying original bet...".cyan());
    let bet_status = client
        .get_job_status(args.bet_id)
        .await
        .context("Failed to fetch bet job status")?;

    if bet_status.status != "completed" {
        anyhow::bail!(
            "Bet job {} is not completed yet (status: {})",
            args.bet_id,
            bet_status.status
        );
    }

    println!("{}", "Bet verified!".green());
    println!();

    // Build claim witness
    println!("{}", "Building claim witness...".cyan());

    let witness = json!({
        "marketId": args.market_id,
        "betJobId": args.bet_id,
        "claimerPubkey": args.claimer,
        "timestamp": chrono::Utc::now().timestamp(),
        "publicInputs": []
    });

    // Save witness to file
    fs::write(&args.witness_output, serde_json::to_string_pretty(&witness)?)?;

    println!("{}", "Witness saved to:".green());
    println!("  {:?}", args.witness_output);
    println!();

    // Create ZK job for settlement
    println!("{}", "Submitting claim as ZK job...".cyan());

    let witness_str = serde_json::to_string(&witness)?;
    let witness_commitment = {
        use sha3::{Digest, Keccak256};
        let mut hasher = Keccak256::new();
        hasher.update(witness_str.as_bytes());
        hex::encode(hasher.finalize())
    };

    let request = CreateJobRequest {
        circuit_type: CIRCUIT_MARKET_SETTLEMENT,
        witness_commitment,
        public_inputs: vec![],
        creator: args.claimer.clone(),
        timeout_seconds: None,
    };

    // Execute payment flow if keypair provided
    let response = if let Some(keypair) = args.keypair {
        use crate::commands::zk::execute_payment_flow_helper;

        execute_payment_flow_helper(
            &client,
            request,
            &keypair,
            &args.rpc_url,
            CIRCUIT_MARKET_SETTLEMENT,
            &args.claimer,
            args.skip_confirm,
        )
        .await?
    } else {
        client.create_job(request).await?
    };

    println!("{}", "Claim submitted successfully!".green().bold());
    println!();
    println!("Job ID: {}", response.job_id.to_string().yellow().bold());
    println!("Status: {}", response.status);
    println!();
    println!("{}", "Your claim will be processed and settled privately.".cyan());
    println!("Check status with: zyb zk status {}", response.job_id);

    Ok(())
}

#[tokio::main]
async fn verify_settlement(args: VerifyArgs) -> Result<()> {
    println!("{}", "Verifying market settlement...".cyan().bold());
    println!();

    let client = ZkClient::with_url(&args.server);

    let status = client
        .get_job_status(args.job_id)
        .await
        .context("Failed to fetch job status")?;

    println!("Job ID: {}", args.job_id);
    println!("Status: {}", status.status);
    println!();

    if status.status == "completed" {
        println!("{}", "Settlement verified successfully!".green().bold());
        println!();
        if let Some(proof_hash) = status.proof_hash {
            println!("Proof hash: {}", proof_hash);
        }
        println!();
        println!("{}", "Market settled. Winners can claim their rewards.".cyan());
    } else {
        println!(
            "{}",
            format!("Settlement is still {}", status.status).yellow()
        );
    }

    Ok(())
}

// ============================================================================
// V2 Command Implementations (PrivateBalance architecture)
// ============================================================================

/// V2 Constants
const PROTOCOL_VAULT_SEED: &[u8] = b"protocol_vault";
const PRIVATE_BALANCE_SEED: &[u8] = b"private_balance";
const MARKET_V2_SEED: &[u8] = b"market_v2";
const MARKET_V3_SEED: &[u8] = b"market_v3";
const MARKET_VAULT_SEED: &[u8] = b"market_vault";
const POSITION_V2_SEED: &[u8] = b"position_v2";
const POSITION_V3_SEED: &[u8] = b"position_v3";
const GOVERNANCE_CONFIG_SEED: &[u8] = b"governance";
const NULLIFIER_SEED: &[u8] = b"nullifier";

/// Get futarchy program ID from env or default
fn get_futarchy_program_id() -> Result<solana_sdk::pubkey::Pubkey> {
    let id_str = std::env::var("FUTARCHY_PROGRAM_ID")
        .unwrap_or_else(|_| "5B8x1aJEHsMLqqKDYVQe2dTA38hbie1QXX2JSmPAxWJT".to_string());
    solana_sdk::pubkey::Pubkey::from_str(&id_str).context("Invalid FUTARCHY_PROGRAM_ID")
}

/// Get ZK generator program ID from env or default
fn get_zk_generator_program_id() -> Result<solana_sdk::pubkey::Pubkey> {
    let id_str = std::env::var("ZK_GENERATOR_PROGRAM_ID")
        .unwrap_or_else(|_| "BH7o2Ldmsy69bRPbK5A45QkBKCMtp2HH8f56JsZU4L8b".to_string());
    solana_sdk::pubkey::Pubkey::from_str(&id_str).context("Invalid ZK_GENERATOR_PROGRAM_ID")
}

#[tokio::main]
async fn init_protocol_v2(args: InitProtocolArgs) -> Result<()> {
    use solana_sdk::signature::{read_keypair_file, Signer};
    use solana_sdk::transaction::Transaction;
    use solana_sdk::instruction::{AccountMeta, Instruction};
    use solana_sdk::system_program;

    // Load global config
    let global_config = crate::config::load_global_config(args.config.clone())?;

    // Get active profile
    let profile = if let Some(ref cfg) = global_config {
        if let Some(profile_name) = args.profile.as_ref().or_else(|| cfg.common.profile.as_ref()) {
            cfg.profiles.get(profile_name).cloned()
        } else {
            None
        }
    } else {
        None
    };

    // Apply cascade for rpc_url
    let rpc_url = args.rpc_url
        .or_else(|| profile.as_ref().and_then(|p| p.rpc_url.clone()))
        .or_else(|| std::env::var("SOLANA_RPC_URL").ok())
        .unwrap_or_else(|| "http://localhost:8899".to_string());

    // Apply cascade for keypair
    let keypair_path = args.keypair
        .or_else(|| profile.as_ref().and_then(|p| p.keypair.clone()))
        .or_else(|| std::env::var("USER_KEYPAIR").ok().map(PathBuf::from))
        .map(|p| crate::config::expand_tilde(&p))
        .ok_or_else(|| anyhow::anyhow!("Keypair is required. Provide via --keypair, config, or USER_KEYPAIR env var"))?;

    // Apply cascade for futarchy_program_id
    let futarchy_program_id = args.futarchy_program_id
        .or_else(|| profile.as_ref().and_then(|p| p.futarchy_program_id.clone()))
        .or_else(|| std::env::var("FUTARCHY_PROGRAM_ID").ok());

    println!("{}", "Initializing Protocol V2...".cyan().bold());
    println!();

    let keypair = read_keypair_file(&keypair_path)
        .map_err(|e| anyhow::anyhow!("Failed to read keypair: {}", e))?;
    let authority = keypair.pubkey();
    let program_id = if let Some(pid) = futarchy_program_id {
        solana_sdk::pubkey::Pubkey::from_str(&pid)?
    } else {
        get_futarchy_program_id()?
    };

    let (protocol_vault_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[PROTOCOL_VAULT_SEED],
        &program_id,
    );

    println!("Authority: {}", authority);
    println!("ProtocolVault PDA: {}", protocol_vault_pda);
    println!();

    // Serialize with borsh to match on-chain program
    let ix_data = borsh::to_vec(&FutarchyInstructionV2::InitializeProtocol)?;
    let instruction = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(authority, true),
            AccountMeta::new(protocol_vault_pda, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: ix_data,
    };

    let rpc_client = solana_client::rpc_client::RpcClient::new(&rpc_url);
    let blockhash = rpc_client.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&[instruction], Some(&authority));
    tx.sign(&[&keypair], blockhash);

    println!("{}", "Sending transaction...".cyan());
    let signature = rpc_client.send_and_confirm_transaction(&tx)?;

    println!();
    println!("{}", "Protocol V2 initialized!".green().bold());
    println!("TX: {}", signature.to_string().yellow());
    Ok(())
}

#[tokio::main]
async fn create_market_v2(args: CreateV2Args) -> Result<()> {
    use solana_sdk::signature::{read_keypair_file, Signer};
    use solana_sdk::transaction::Transaction;
    use solana_sdk::instruction::{AccountMeta, Instruction};
    use solana_sdk::system_program;
    use solana_sdk::sysvar;
    use std::io::Write;

    // Load global config
    let global_config = crate::config::load_global_config(args.config.clone())?;

    // Get active profile
    let profile = if let Some(ref cfg) = global_config {
        if let Some(profile_name) = args.profile.as_ref().or_else(|| cfg.common.profile.as_ref()) {
            cfg.profiles.get(profile_name).cloned()
        } else {
            None
        }
    } else {
        None
    };

    // Apply cascade for rpc_url
    let rpc_url = args.rpc_url
        .or_else(|| profile.as_ref().and_then(|p| p.rpc_url.clone()))
        .or_else(|| std::env::var("SOLANA_RPC_URL").ok())
        .unwrap_or_else(|| "http://localhost:8899".to_string());

    // Apply cascade for keypair
    let keypair_path = args.keypair
        .or_else(|| profile.as_ref().and_then(|p| p.keypair.clone()))
        .or_else(|| std::env::var("USER_KEYPAIR").ok().map(PathBuf::from))
        .map(|p| crate::config::expand_tilde(&p))
        .ok_or_else(|| anyhow::anyhow!("Keypair is required. Provide via --keypair, config, or USER_KEYPAIR env var"))?;

    // Apply cascade for futarchy_program_id
    let futarchy_program_id = args.futarchy_program_id
        .or_else(|| profile.as_ref().and_then(|p| p.futarchy_program_id.clone()))
        .or_else(|| std::env::var("FUTARCHY_PROGRAM_ID").ok());

    println!("{}", "Creating Market V2...".cyan().bold());
    println!();

    let keypair = read_keypair_file(&keypair_path)
        .map_err(|e| anyhow::anyhow!("Failed to read keypair: {}", e))?;
    let authority = keypair.pubkey();
    let oracle = args.oracle.as_ref()
        .map(|o| solana_sdk::pubkey::Pubkey::from_str(o))
        .transpose()?
        .unwrap_or(authority);
    let program_id = if let Some(pid) = futarchy_program_id {
        solana_sdk::pubkey::Pubkey::from_str(&pid)?
    } else {
        get_futarchy_program_id()?
    };

    let (market_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[MARKET_V2_SEED, &args.market_id.to_le_bytes()], &program_id);
    let (market_vault_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[MARKET_VAULT_SEED, &args.market_id.to_le_bytes()], &program_id);
    let (governance_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[GOVERNANCE_CONFIG_SEED, &args.market_id.to_le_bytes()], &program_id);

    println!("Market ID: {}", args.market_id);
    println!("Oracle: {}", oracle);
    println!("Governance: {}", args.governance);
    println!();

    // Serialize with borsh to match on-chain program
    let ix_data = borsh::to_vec(&FutarchyInstructionV2::CreateMarketV2 {
        market_id: args.market_id,
        question_hash: [0u8; 32],
        end_time: args.end_time,
        max_bet: args.max_bet,
        has_governance: args.governance,
        executable_action: None,
        execution_threshold: if args.governance { Some(args.threshold) } else { None },
        timelock_duration: if args.governance { Some(args.timelock) } else { None },
    })?;

    let mut accounts = vec![
        AccountMeta::new(authority, true),
        AccountMeta::new(market_pda, false),
        AccountMeta::new(market_vault_pda, false),
        AccountMeta::new_readonly(oracle, false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
    ];
    if args.governance {
        accounts.push(AccountMeta::new(governance_pda, false));
    }

    let instruction = Instruction { program_id, accounts, data: ix_data };
    let rpc_client = solana_client::rpc_client::RpcClient::new(&rpc_url);
    let blockhash = rpc_client.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&[instruction], Some(&authority));
    tx.sign(&[&keypair], blockhash);

    println!("{}", "Sending transaction...".cyan());
    let signature = rpc_client.send_and_confirm_transaction(&tx)?;

    println!();
    println!("{}", "Market V2 created!".green().bold());
    println!("TX: {}", signature.to_string().yellow());
    Ok(())
}

#[tokio::main]
async fn create_balance_v2(args: CreateBalanceArgs) -> Result<()> {
    use solana_sdk::signature::{read_keypair_file, Signer};
    use solana_sdk::transaction::Transaction;
    use solana_sdk::instruction::{AccountMeta, Instruction};
    use solana_sdk::system_program;
    use std::io::Write;

    println!("{}", "Creating Private Balance...".cyan().bold());

    let keypair = read_keypair_file(&args.keypair)
        .map_err(|e| anyhow::anyhow!("Failed to read keypair: {}", e))?;
    let user = keypair.pubkey();
    let program_id = get_futarchy_program_id()?;

    let (private_balance_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[PRIVATE_BALANCE_SEED, user.as_ref()], &program_id);

    println!("User: {}", user);
    println!("PDA: {}", private_balance_pda);

    // Calculate initial commitment = Poseidon(0, 0) to match circom circuit
    let initial_commitment = {
        use light_poseidon::{Poseidon, PoseidonBytesHasher};
        let mut poseidon = Poseidon::<ark_bn254::Fr>::new_circom(2).unwrap();
        let zero_bytes = [0u8; 32];
        poseidon.hash_bytes_be(&[&zero_bytes, &zero_bytes]).unwrap()
    };
    println!("Initial commitment (Poseidon(0,0)): {}", hex::encode(&initial_commitment));

    // Serialize with borsh to match on-chain program
    let ix_data = borsh::to_vec(&FutarchyInstructionV2::CreatePrivateBalance {
        initial_encrypted_balance: vec![0u8; 64],
        initial_commitment,
    })?;

    let instruction = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(user, true),
            AccountMeta::new(private_balance_pda, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: ix_data,
    };

    let rpc_client = solana_client::rpc_client::RpcClient::new(&args.rpc_url);
    let blockhash = rpc_client.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&[instruction], Some(&user));
    tx.sign(&[&keypair], blockhash);

    let signature = rpc_client.send_and_confirm_transaction(&tx)?;
    println!("{}", "Private Balance created!".green().bold());
    println!("TX: {}", signature.to_string().yellow());
    Ok(())
}

#[tokio::main]
async fn deposit_private_v2(args: DepositPrivateArgs) -> Result<()> {
    use solana_sdk::signature::{read_keypair_file, Signer};
    use solana_sdk::transaction::Transaction;
    use solana_sdk::instruction::{AccountMeta, Instruction};
    use solana_sdk::system_program;

    println!("{}", "Depositing to Private Balance...".cyan().bold());

    let keypair = read_keypair_file(&args.keypair)
        .map_err(|e| anyhow::anyhow!("Failed to read keypair: {}", e))?;
    let user = keypair.pubkey();
    let program_id = get_futarchy_program_id()?;

    let (private_balance_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[PRIVATE_BALANCE_SEED, user.as_ref()], &program_id);
    let (protocol_vault_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[PROTOCOL_VAULT_SEED], &program_id);

    // Load local state to calculate new commitment
    let user_str = user.to_string();
    let mut local_state = LocalPrivateBalanceState::load_or_create(&user_str)?;

    println!("Current local balance: {} lamports (nonce: {})", local_state.balance, local_state.nonce);
    println!("Amount to deposit: {} lamports ({:.4} SOL)", args.amount, args.amount as f64 / 1e9);

    // Calculate new balance and nonce
    let new_balance = local_state.balance.saturating_add(args.amount);
    let new_nonce = local_state.nonce + 1;

    // Calculate new commitment = Poseidon(new_balance, new_nonce)
    let new_commitment = {
        use light_poseidon::{Poseidon, PoseidonBytesHasher};
        let mut poseidon = Poseidon::<ark_bn254::Fr>::new_circom(2).unwrap();
        let mut balance_bytes = [0u8; 32];
        balance_bytes[24..32].copy_from_slice(&new_balance.to_be_bytes());
        let mut nonce_bytes = [0u8; 32];
        nonce_bytes[24..32].copy_from_slice(&new_nonce.to_be_bytes());
        poseidon.hash_bytes_be(&[&balance_bytes, &nonce_bytes]).unwrap()
    };

    println!("New balance will be: {} lamports (nonce: {})", new_balance, new_nonce);
    println!("New commitment: {}", hex::encode(&new_commitment));

    // Serialize with borsh to match on-chain program
    let ix_data = borsh::to_vec(&FutarchyInstructionV2::Deposit {
        amount: args.amount,
        new_commitment,
    })?;

    let instruction = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(user, true),
            AccountMeta::new(private_balance_pda, false),
            AccountMeta::new(protocol_vault_pda, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: ix_data,
    };

    let rpc_client = solana_client::rpc_client::RpcClient::new(&args.rpc_url);
    let blockhash = rpc_client.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&[instruction], Some(&user));
    tx.sign(&[&keypair], blockhash);

    let signature = rpc_client.send_and_confirm_transaction(&tx)?;

    // Update local balance state
    local_state.apply_deposit(args.amount);
    local_state.save()?;

    println!("{}", "Deposit successful!".green().bold());
    println!("TX: {}", signature.to_string().yellow());
    println!("New local balance: {} lamports (nonce: {})", local_state.balance, local_state.nonce);
    Ok(())
}

#[tokio::main]
async fn bet_private_v2(args: BetPrivateArgs) -> Result<()> {
    use solana_sdk::signature::{read_keypair_file, Signer};
    use solana_sdk::transaction::Transaction;
    use solana_sdk::instruction::{AccountMeta, Instruction};
    use solana_sdk::system_program;
    use solana_sdk::sysvar::clock;
    use rand::RngCore;
    use std::io::Write as IoWrite;

    println!("{}", "Placing Private Bet V2...".cyan().bold());
    println!();

    let keypair = read_keypair_file(&args.keypair)
        .map_err(|e| anyhow::anyhow!("Failed to read keypair: {}", e))?;
    let user = keypair.pubkey();
    let side_bool = matches!(args.side, BetSide::Yes);
    let side_u8 = if side_bool { 1u8 } else { 0u8 };

    println!("User: {}", user.to_string().yellow());
    println!("Market: {}", args.market_id);
    println!("Side: {}", if side_bool { "YES".green() } else { "NO".red() });
    println!("Amount: {} lamports ({:.4} SOL)", args.amount, args.amount as f64 / 1e9);
    println!();

    // Load local balance state
    let user_str = user.to_string();
    let mut local_state = LocalPrivateBalanceState::load_or_create(&user_str)?;
    println!("Local balance: {} lamports (nonce: {})", local_state.balance, local_state.nonce);

    if local_state.balance < args.amount {
        anyhow::bail!(
            "Insufficient local balance: {} < {}. Run deposit-private first.",
            local_state.balance, args.amount
        );
    }

    // Generate secret for this bet (must fit in BN254 field)
    // The field modulus is ~254 bits, so we limit secret to 31 bytes
    // with leading byte < 0x30 to ensure it's < field modulus
    let mut rng = rand::thread_rng();
    let mut secret = [0u8; 32];
    rng.fill_bytes(&mut secret[1..32]); // Leave first byte as 0
    secret[1] &= 0x1F; // Ensure high bits are limited

    // Fetch on-chain state
    println!("Fetching on-chain state...");
    let (private_balance_pda, old_commitment, chain_nonce) =
        fetch_private_balance_state(&args.rpc_url, &user).await?;

    // Validate nonce matches
    if chain_nonce != local_state.nonce {
        println!("{}", format!(
            "Warning: nonce mismatch (local: {}, chain: {}). Using chain nonce.",
            local_state.nonce, chain_nonce
        ).yellow());
        local_state.nonce = chain_nonce;
    }

    // Fetch market max_bet
    let max_bet = fetch_market_v2_state(&args.rpc_url, args.market_id).await?;
    println!("Market max_bet: {} lamports", max_bet);

    if args.amount > max_bet {
        anyhow::bail!("Bet amount {} exceeds market max_bet {}", args.amount, max_bet);
    }

    // Calculate new values
    let new_balance = local_state.balance.saturating_sub(args.amount);
    let new_nonce = local_state.nonce + 1;

    // Calculate commitments using same hash as circuit
    // Calculate commitments using Poseidon (compatible with circomlib)
    // old_balance_commitment should match chain - we read it directly
    // new_balance_commitment = Poseidon(new_balance, new_nonce)
    // bet_commitment = Poseidon(bet_amount, side, secret)

    let new_balance_commitment = {
        use light_poseidon::{Poseidon, PoseidonBytesHasher};
        let mut poseidon = Poseidon::<ark_bn254::Fr>::new_circom(2).unwrap();

        // Convert to field elements (u64 -> 32-byte big-endian)
        let mut balance_bytes = [0u8; 32];
        balance_bytes[24..32].copy_from_slice(&new_balance.to_be_bytes());
        let mut nonce_bytes = [0u8; 32];
        nonce_bytes[24..32].copy_from_slice(&new_nonce.to_be_bytes());

        poseidon.hash_bytes_be(&[&balance_bytes, &nonce_bytes]).unwrap()
    };

    let bet_commitment = {
        use light_poseidon::{Poseidon, PoseidonBytesHasher};
        let mut poseidon = Poseidon::<ark_bn254::Fr>::new_circom(3).unwrap();

        // Convert to field elements
        let mut amount_bytes = [0u8; 32];
        amount_bytes[24..32].copy_from_slice(&args.amount.to_be_bytes());
        let mut side_bytes = [0u8; 32];
        side_bytes[31] = side_u8;
        // secret is already [u8; 32]

        poseidon.hash_bytes_be(&[&amount_bytes, &side_bytes, &secret]).unwrap()
    };

    // Check if we should generate real proof or use mock
    let (proof_bytes, public_inputs) = if let (Some(wasm), Some(zkey)) =
        (&args.circuit_wasm, &args.circuit_zkey)
    {
        println!("Generating ZK proof with snarkjs...");
        let witness = PlaceBetPrivateWitness {
            old_balance_commitment: old_commitment,
            new_balance_commitment,
            bet_commitment,
            market_id: args.market_id,
            max_bet,
            balance: local_state.balance,
            new_balance,
            bet_amount: args.amount,
            side: side_u8,
            secret,
            nonce: local_state.nonce,
            new_nonce,
        };

        let proof = generate_place_bet_private_proof(wasm, zkey, &witness)?;
        println!("{}", "ZK proof generated successfully!".green());
        (proof.proof, proof.public_inputs)
    } else {
        println!("{}", "Using mock proof (no circuit paths provided)".yellow());
        // Mock proof: 256 zero bytes
        let mock_proof = vec![0u8; 256];
        // Public inputs: old_commitment(32) + new_commitment(32) + bet_commitment(32) + max_bet(8) + bet_amount(8)
        let mut mock_public = Vec::with_capacity(112);
        mock_public.extend_from_slice(&old_commitment);
        mock_public.extend_from_slice(&new_balance_commitment);
        mock_public.extend_from_slice(&bet_commitment);
        mock_public.extend_from_slice(&max_bet.to_le_bytes());
        mock_public.extend_from_slice(&args.amount.to_le_bytes());
        (mock_proof, mock_public)
    };

    // Save witness for later claim
    let witness_data = BetWitness {
        market_id: args.market_id,
        secret: hex::encode(secret),
        blinding: hex::encode([0u8; 32]),
        bet_amount: args.amount,
        bet_side: side_u8,
        bet_commitment: hex::encode(bet_commitment),
    };
    fs::write(&args.witness_output, serde_json::to_string_pretty(&witness_data)?)?;
    println!("Witness saved: {:?}", args.witness_output);

    // Build PlaceBetV2 instruction
    let program_id = get_futarchy_program_id()?;
    let zk_program_id = get_zk_generator_program_id()?;

    let market_id_bytes = args.market_id.to_le_bytes();
    let (market_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[MARKET_V2_SEED, &market_id_bytes], &program_id);
    let (position_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[POSITION_V2_SEED, &market_id_bytes, &bet_commitment], &program_id);
    let (protocol_vault_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[PROTOCOL_VAULT_SEED], &program_id);
    let (market_vault_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[MARKET_VAULT_SEED, &market_id_bytes], &program_id);

    // Serialize with borsh to match on-chain program
    let ix_data = borsh::to_vec(&FutarchyInstructionV2::PlaceBetV2 {
        market_id: args.market_id,
        bet_commitment,
        bet_side: side_bool, // true = YES, false = NO
        proof: proof_bytes,
        public_inputs,
        new_balance_commitment,
        encrypted_bet: vec![], // empty for now (V3 FHE)
        circuit_type: 33, // CIRCUIT_PLACE_BET_PRIVATE
    })?;

    let instruction = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(user, true),                          // 0. User
            AccountMeta::new(private_balance_pda, false),          // 1. PrivateBalance
            AccountMeta::new(market_pda, false),                   // 2. MarketV2
            AccountMeta::new(position_pda, false),                 // 3. PositionV2
            AccountMeta::new(protocol_vault_pda, false),           // 4. ProtocolVault
            AccountMeta::new(market_vault_pda, false),             // 5. MarketVault
            AccountMeta::new_readonly(zk_program_id, false),       // 6. ZK program
            AccountMeta::new_readonly(system_program::id(), false), // 7. System program
            AccountMeta::new_readonly(clock::id(), false),          // 8. Clock sysvar
        ],
        data: ix_data,
    };

    println!();
    println!("Submitting transaction...");

    let rpc_client = solana_client::rpc_client::RpcClient::new(&args.rpc_url);
    let blockhash = rpc_client.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&[instruction], Some(&user));
    tx.sign(&[&keypair], blockhash);

    let signature = rpc_client.send_and_confirm_transaction(&tx)?;

    // Update local state
    local_state.apply_bet(args.amount);
    local_state.save()?;

    println!();
    println!("{}", "Bet placed successfully!".green().bold());
    println!("TX: {}", signature.to_string().yellow());
    println!("Position PDA: {}", position_pda.to_string().cyan());
    println!("New local balance: {} lamports", local_state.balance);
    println!();
    println!("{}", "To claim winnings later, use:".cyan());
    println!("  zyb market claim-private --market-id {} --bet-witness {:?}", args.market_id, args.witness_output);

    Ok(())
}

#[tokio::main]
async fn claim_private_v2(args: ClaimPrivateArgs) -> Result<()> {
    use solana_sdk::signature::{read_keypair_file, Signer};
    use solana_sdk::transaction::Transaction;
    use solana_sdk::instruction::{AccountMeta, Instruction};
    use solana_sdk::system_program;
    use solana_sdk::sysvar::clock;

    println!("{}", "Claiming Private Winnings V2...".cyan().bold());
    println!();

    // Load witness
    let witness: BetWitness = serde_json::from_str(&fs::read_to_string(&args.bet_witness)?)?;
    let bet_commitment = hex::decode(&witness.bet_commitment)?;
    let secret = hex::decode(&witness.secret)?;

    let bet_side_bool = witness.bet_side == 1;
    let side_str = if bet_side_bool { "YES" } else { "NO" };
    println!("Market ID: {}", args.market_id);
    println!("Bet side: {}", side_str);
    println!("Bet amount: {} lamports ({:.4} SOL)", witness.bet_amount, witness.bet_amount as f64 / 1_000_000_000.0);
    println!();

    let keypair = read_keypair_file(&args.keypair)
        .map_err(|e| anyhow::anyhow!("Failed to read keypair: {}", e))?;
    let user = keypair.pubkey();
    let program_id = get_futarchy_program_id()?;
    let zk_program_id = get_zk_generator_program_id()?;

    println!("User: {}", user);

    // Load local state
    let user_str = user.to_string();
    let mut local_state = LocalPrivateBalanceState::load_or_create(&user_str)?;
    println!("Current balance: {} lamports", local_state.balance);

    // Fetch market to check resolution
    let rpc_client = solana_client::rpc_client::RpcClient::new(&args.rpc_url);
    let market_id_bytes = args.market_id.to_le_bytes();

    let (market_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[MARKET_V2_SEED, &market_id_bytes], &program_id);

    println!("Fetching market state...");
    let market_account = rpc_client.get_account(&market_pda)?;
    let data = &market_account.data;

    // Parse MarketV2 fields:
    // authority: 0..32, market_id: 32..40, oracle: 40..72, question_hash: 72..104
    // end_time: 104..112, status: 112, max_bet: 113..121, resolution: 121..123
    // bet_count_yes: 123..131, bet_count_no: 131..139
    let resolution_byte = data[121];
    let resolution = if resolution_byte == 0 {
        None
    } else {
        Some(data[122] != 0)
    };
    let bet_count_yes = u64::from_le_bytes(data[123..131].try_into().unwrap());
    let bet_count_no = u64::from_le_bytes(data[131..139].try_into().unwrap());

    let winning_side = match resolution {
        None => {
            println!("{}", "Market not yet resolved!".red());
            return Err(anyhow::anyhow!("Market not resolved"));
        }
        Some(side) => {
            let winning_str = if side { "YES" } else { "NO" };
            println!("Market resolved: {} wins", winning_str.green().bold());

            if bet_side_bool != side {
                println!("{}", "Your bet was on the losing side!".red());
                return Err(anyhow::anyhow!("Bet on losing side"));
            }
            println!("{}", "Your bet won!".green().bold());
            side
        }
    };

    // Calculate winner_count
    let winner_count = if winning_side { bet_count_yes } else { bet_count_no };
    println!("Winner count: {} bets on winning side", winner_count);

    if winner_count == 0 {
        println!("{}", "No winners recorded in market!".red());
        return Err(anyhow::anyhow!("No winners"));
    }

    // Calculate PDAs
    let bet_commitment_arr: [u8; 32] = bet_commitment.clone().try_into()
        .map_err(|_| anyhow::anyhow!("Invalid bet_commitment length"))?;

    let (private_balance_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[PRIVATE_BALANCE_SEED, user.as_ref()], &program_id);
    let (position_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[POSITION_V2_SEED, &market_id_bytes, &bet_commitment_arr], &program_id);
    let (protocol_vault_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[PROTOCOL_VAULT_SEED], &program_id);
    let (market_vault_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[MARKET_VAULT_SEED, &market_id_bytes], &program_id);

    // Fetch vault_total from MarketVault
    let market_vault_account = rpc_client.get_account(&market_vault_pda)?;
    let vault_total = market_vault_account.lamports;
    println!("Vault total: {} lamports ({:.4} SOL)", vault_total, vault_total as f64 / 1_000_000_000.0);

    // V2 Payout Model: Equitative distribution
    // payout = vault_total / winner_count
    // All winners receive equal payout regardless of bet amount.
    // V3 with FHE will implement proportional payouts.
    let payout = vault_total / winner_count;
    println!("Payout per winner: {} lamports ({:.4} SOL)", payout, payout as f64 / 1_000_000_000.0);

    // Generate nullifier = Poseidon(secret, bet_commitment)
    let secret_arr: [u8; 32] = secret.try_into()
        .map_err(|_| anyhow::anyhow!("Invalid secret length"))?;

    let nullifier_hash = {
        use light_poseidon::{Poseidon, PoseidonBytesHasher};
        let mut poseidon = Poseidon::<ark_bn254::Fr>::new_circom(2).unwrap();
        poseidon.hash_bytes_be(&[&secret_arr, &bet_commitment_arr]).unwrap()
    };

    // Nullifier PDA (to prevent double-claim)
    let (nullifier_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[NULLIFIER_SEED, &market_id_bytes, &nullifier_hash], &program_id);

    // Calculate new balance commitment = Poseidon(new_balance, new_nonce)
    let new_balance = local_state.balance + payout;
    let new_nonce = local_state.nonce + 1;

    let new_balance_commitment = {
        use light_poseidon::{Poseidon, PoseidonBytesHasher};
        let mut poseidon = Poseidon::<ark_bn254::Fr>::new_circom(2).unwrap();

        // Convert to field elements (u64 -> 32-byte big-endian)
        let mut balance_bytes = [0u8; 32];
        balance_bytes[24..32].copy_from_slice(&new_balance.to_be_bytes());
        let mut nonce_bytes = [0u8; 32];
        nonce_bytes[24..32].copy_from_slice(&new_nonce.to_be_bytes());

        poseidon.hash_bytes_be(&[&balance_bytes, &nonce_bytes]).unwrap()
    };

    // Fetch old_commitment from PrivateBalance PDA
    // Structure: user (32) + encrypted_balance Vec (4 + len) + balance_commitment (32)
    let private_balance_account = rpc_client.get_account(&private_balance_pda)?;
    let pb_data = &private_balance_account.data;
    // Read Vec length at offset 32
    let vec_len = u32::from_le_bytes(pb_data[32..36].try_into().unwrap()) as usize;
    // balance_commitment is after user (32) + vec header (4) + vec data (vec_len)
    let commitment_offset = 32 + 4 + vec_len;
    let old_commitment: [u8; 32] = pb_data[commitment_offset..commitment_offset+32].try_into()
        .map_err(|_| anyhow::anyhow!("Failed to parse old_commitment"))?;

    // Generate proof (real or mock)
    let (proof_bytes, public_inputs) = if let (Some(wasm), Some(zkey)) =
        (&args.circuit_wasm, &args.circuit_zkey)
    {
        println!("Generating real ZK proof with claim_private_v2 circuit...");

        // Build witness for circuit
        let claim_witness = ClaimPrivateWitness {
            bet_commitment: bet_commitment_arr,
            resolution: if winning_side { 1 } else { 0 },
            nullifier_hash,
            bet_amount: witness.bet_amount,
            bet_side: if bet_side_bool { 1 } else { 0 },
            old_balance_commitment: old_commitment,
            new_balance_commitment,
            secret: secret_arr,
            balance: local_state.balance,
            new_balance,
            nonce: local_state.nonce,
            new_nonce,
            payout_amount: payout,
        };

        // Generate proof
        let proof_result = generate_claim_private_proof(wasm, zkey, &claim_witness)?;
        (proof_result.proof, proof_result.public_inputs)
    } else {
        // Use mock proof for backwards compatibility
        println!("{}", "Using mock proof for claim (no circuit paths provided)...".yellow());
        let proof_bytes = vec![0u8; 256];

        // Build public inputs V2 (138 bytes):
        // [0..32]: bet_commitment
        // [32]: resolution (0=NO, 1=YES)
        // [33..65]: nullifier_hash
        // [65..73]: bet_amount (u64 le)
        // [73]: bet_side (0=NO, 1=YES)
        // [74..106]: old_commitment
        // [106..138]: new_commitment
        let resolution_byte: u8 = if winning_side { 1 } else { 0 };
        let bet_side_byte: u8 = if bet_side_bool { 1 } else { 0 };
        let mut public_inputs = Vec::with_capacity(138);
        public_inputs.extend_from_slice(&bet_commitment_arr);
        public_inputs.push(resolution_byte);
        public_inputs.extend_from_slice(&nullifier_hash);
        public_inputs.extend_from_slice(&witness.bet_amount.to_le_bytes());
        public_inputs.push(bet_side_byte);
        public_inputs.extend_from_slice(&old_commitment);
        public_inputs.extend_from_slice(&new_balance_commitment);

        (proof_bytes, public_inputs)
    };

    println!("Public inputs: {} bytes", public_inputs.len());

    // Build ClaimV2 instruction
    let ix_data = borsh::to_vec(&FutarchyInstructionV2::ClaimV2 {
        market_id: args.market_id,
        nullifier_hash,
        proof: proof_bytes,
        public_inputs,
        new_balance_commitment,
        encrypted_payout: vec![],
        bet_commitment: bet_commitment_arr,
        circuit_type: 34, // CIRCUIT_CLAIM_PRIVATE
    })?;

    let instruction = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(user, true),                          // 0. User
            AccountMeta::new(private_balance_pda, false),          // 1. PrivateBalance
            AccountMeta::new_readonly(market_pda, false),          // 2. MarketV2
            AccountMeta::new(position_pda, false),                 // 3. PositionV2
            AccountMeta::new(nullifier_pda, false),                // 4. Nullifier PDA
            AccountMeta::new(market_vault_pda, false),             // 5. MarketVault
            AccountMeta::new(protocol_vault_pda, false),           // 6. ProtocolVault
            AccountMeta::new_readonly(zk_program_id, false),       // 7. ZK program
            AccountMeta::new_readonly(system_program::id(), false), // 8. System program
            AccountMeta::new_readonly(clock::id(), false),         // 9. Clock sysvar
        ],
        data: ix_data,
    };

    println!();
    println!("Submitting claim transaction...");

    let blockhash = rpc_client.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&[instruction], Some(&user));
    tx.sign(&[&keypair], blockhash);

    let sig = rpc_client.send_and_confirm_transaction(&tx)?;

    // Update local state
    local_state.balance = new_balance;
    local_state.nonce = new_nonce;
    local_state.save()?;

    println!();
    println!("{}", "Claim successful!".green().bold());
    println!("TX: {}", sig);
    println!("Payout: {} lamports ({:.4} SOL)", payout, payout as f64 / 1_000_000_000.0);
    println!("New balance: {} lamports ({:.4} SOL)", new_balance, new_balance as f64 / 1_000_000_000.0);

    Ok(())
}

#[tokio::main]
async fn withdraw_private_v2(args: WithdrawPrivateArgs) -> Result<()> {
    use solana_sdk::signature::{read_keypair_file, Signer};
    use solana_sdk::transaction::Transaction;
    use solana_sdk::instruction::{AccountMeta, Instruction};
    use solana_sdk::system_program;

    println!("{}", "Withdrawing from Private Balance V2...".cyan().bold());
    println!();
    println!("Amount: {} lamports ({:.4} SOL)", args.amount, args.amount as f64 / 1_000_000_000.0);
    println!();

    let keypair = read_keypair_file(&args.keypair)
        .map_err(|e| anyhow::anyhow!("Failed to read keypair: {}", e))?;
    let user = keypair.pubkey();
    let program_id = get_futarchy_program_id()?;
    let zk_program_id = get_zk_generator_program_id()?;

    println!("User: {}", user);

    // Load local state
    let user_str = user.to_string();
    let mut local_state = LocalPrivateBalanceState::load_or_create(&user_str)?;
    println!("Current private balance: {} lamports ({:.4} SOL)",
             local_state.balance, local_state.balance as f64 / 1_000_000_000.0);

    if args.amount > local_state.balance {
        println!("{}", "Insufficient private balance!".red());
        return Err(anyhow::anyhow!("Insufficient balance: {} < {}", local_state.balance, args.amount));
    }

    // Calculate PDAs
    let (private_balance_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[PRIVATE_BALANCE_SEED, user.as_ref()], &program_id);
    let (protocol_vault_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[PROTOCOL_VAULT_SEED], &program_id);

    // Fetch old_commitment from chain
    let rpc_client = solana_client::rpc_client::RpcClient::new(&args.rpc_url);
    let private_balance_account = rpc_client.get_account(&private_balance_pda)?;
    let data = &private_balance_account.data;
    let vec_len = u32::from_le_bytes(data[32..36].try_into().unwrap()) as usize;
    let commitment_offset = 32 + 4 + vec_len;
    let old_commitment: [u8; 32] = data[commitment_offset..commitment_offset+32].try_into()
        .map_err(|_| anyhow::anyhow!("Failed to parse old_commitment"))?;

    // Calculate new balance and commitment using Poseidon
    let new_balance = local_state.balance - args.amount;
    let new_nonce = local_state.nonce + 1;

    let new_balance_commitment = {
        use light_poseidon::{Poseidon, PoseidonBytesHasher};
        let mut poseidon = Poseidon::<ark_bn254::Fr>::new_circom(2).unwrap();
        let mut balance_bytes = [0u8; 32];
        balance_bytes[24..32].copy_from_slice(&new_balance.to_be_bytes());
        let mut nonce_bytes = [0u8; 32];
        nonce_bytes[24..32].copy_from_slice(&new_nonce.to_be_bytes());
        poseidon.hash_bytes_be(&[&balance_bytes, &nonce_bytes]).unwrap()
    };

    // Generate proof (real or mock)
    let (proof_bytes, public_inputs) = if let (Some(wasm), Some(zkey)) =
        (&args.circuit_wasm, &args.circuit_zkey)
    {
        println!("Generating ZK proof with snarkjs...");
        let witness = WithdrawPrivateWitness {
            old_balance_commitment: old_commitment,
            new_balance_commitment,
            withdraw_amount: args.amount,
            balance: local_state.balance,
            new_balance,
            nonce: local_state.nonce,
            new_nonce,
        };
        let proof = generate_withdraw_private_proof(wasm, zkey, &witness)?;
        println!("{}", "ZK proof generated successfully!".green());
        (proof.proof, proof.public_inputs)
    } else {
        println!("{}", "Using mock proof for withdraw...".yellow());
        let mut public_inputs = Vec::with_capacity(72);
        public_inputs.extend_from_slice(&old_commitment);
        public_inputs.extend_from_slice(&new_balance_commitment);
        public_inputs.extend_from_slice(&args.amount.to_le_bytes());
        (vec![0u8; 256], public_inputs)
    };

    // Build Withdraw instruction
    let ix_data = borsh::to_vec(&FutarchyInstructionV2::Withdraw {
        amount: args.amount,
        proof: proof_bytes,
        public_inputs,
        new_balance_commitment,
    })?;

    let instruction = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(user, true),                          // 0. User
            AccountMeta::new(private_balance_pda, false),          // 1. PrivateBalance
            AccountMeta::new(protocol_vault_pda, false),           // 2. ProtocolVault
            AccountMeta::new_readonly(zk_program_id, false),       // 3. ZK program
            AccountMeta::new_readonly(system_program::id(), false), // 4. System program
        ],
        data: ix_data,
    };

    println!();
    println!("Submitting withdraw transaction...");

    let blockhash = rpc_client.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&[instruction], Some(&user));
    tx.sign(&[&keypair], blockhash);

    let sig = rpc_client.send_and_confirm_transaction(&tx)?;

    // Update local state
    local_state.balance = new_balance;
    local_state.nonce = new_nonce;
    local_state.save()?;

    println!();
    println!("{}", "Withdrawal successful!".green().bold());
    println!("TX: {}", sig);
    println!("Withdrawn: {} lamports ({:.4} SOL)", args.amount, args.amount as f64 / 1_000_000_000.0);
    println!("New private balance: {} lamports ({:.4} SOL)", new_balance, new_balance as f64 / 1_000_000_000.0);
    println!();
    println!("{}", "SOL transferred to your wallet!".green());

    Ok(())
}

#[tokio::main]
async fn settle_market_v2(args: SettleV2Args) -> Result<()> {
    use solana_sdk::signature::{read_keypair_file, Signer};
    use solana_sdk::transaction::Transaction;
    use solana_sdk::instruction::{AccountMeta, Instruction};
    use solana_sdk::sysvar::clock;

    let outcome_bool = matches!(args.outcome, BetSide::Yes);
    let outcome_str = if outcome_bool { "YES" } else { "NO" };

    println!("{}", "Settling Market V2...".cyan().bold());
    println!();
    println!("Market ID: {}", args.market_id);
    println!("Outcome: {}", outcome_str.green().bold());
    println!();

    let keypair = read_keypair_file(&args.keypair)
        .map_err(|e| anyhow::anyhow!("Failed to read keypair: {}", e))?;
    let oracle = keypair.pubkey();
    let program_id = get_futarchy_program_id()?;

    println!("Oracle: {}", oracle);

    let market_id_bytes = args.market_id.to_le_bytes();
    let (market_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[MARKET_V2_SEED, &market_id_bytes], &program_id);

    println!("Market PDA: {}", market_pda);
    println!();

    // Serialize SettleMarketV2 instruction
    let ix_data = borsh::to_vec(&FutarchyInstructionV2::SettleMarketV2 {
        market_id: args.market_id,
        outcome: outcome_bool,
    })?;

    let instruction = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(oracle, true),           // 0. Oracle (signer)
            AccountMeta::new(market_pda, false),      // 1. MarketV2 PDA
            AccountMeta::new_readonly(clock::id(), false), // 2. Clock sysvar
        ],
        data: ix_data,
    };

    println!("Submitting transaction...");

    let rpc_client = solana_client::rpc_client::RpcClient::new(&args.rpc_url);
    let blockhash = rpc_client.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&[instruction], Some(&oracle));
    tx.sign(&[&keypair], blockhash);

    let sig = rpc_client.send_and_confirm_transaction(&tx)?;
    println!();
    println!("{}", "Market settled successfully!".green().bold());
    println!("TX: {}", sig);
    println!("Winning side: {}", outcome_str.green().bold());

    Ok(())
}

// =============================================================================
// Private Balance State Management
// =============================================================================

/// Local state file for tracking private balance (stored in ~/.zyb/)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalPrivateBalanceState {
    /// Current balance in lamports
    pub balance: u64,
    /// Current nonce (incremented on each operation)
    pub nonce: u64,
    /// Associated user pubkey
    pub user_pubkey: String,
}

impl LocalPrivateBalanceState {
    pub fn new(user_pubkey: String, initial_balance: u64) -> Self {
        Self {
            balance: initial_balance,
            nonce: 0,
            user_pubkey,
        }
    }

    /// Get the state file path for a user
    pub fn state_file_path(user_pubkey: &str) -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".zyb").join(format!("private_balance_{}.json", &user_pubkey[..8]))
    }

    /// Load state from disk, or create new if not exists
    pub fn load_or_create(user_pubkey: &str) -> Result<Self> {
        let path = Self::state_file_path(user_pubkey);
        if path.exists() {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read state file: {:?}", path))?;
            serde_json::from_str(&content)
                .with_context(|| "Failed to parse state file")
        } else {
            Ok(Self::new(user_pubkey.to_string(), 0))
        }
    }

    /// Save state to disk
    pub fn save(&self) -> Result<()> {
        let path = Self::state_file_path(&self.user_pubkey);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }

    /// Update after a successful bet
    pub fn apply_bet(&mut self, bet_amount: u64) {
        self.balance = self.balance.saturating_sub(bet_amount);
        self.nonce += 1;
    }

    /// Update after a successful deposit
    pub fn apply_deposit(&mut self, deposit_amount: u64) {
        self.balance = self.balance.saturating_add(deposit_amount);
        self.nonce += 1;
    }
}

// =============================================================================
// snarkjs Proof Generation for PlaceBetPrivate
// =============================================================================

use num_bigint::BigUint;
use std::process::Command;

/// Witness for PlaceBetPrivate circuit
#[derive(Debug, Clone)]
pub struct PlaceBetPrivateWitness {
    // Public inputs
    pub old_balance_commitment: [u8; 32],
    pub new_balance_commitment: [u8; 32],
    pub bet_commitment: [u8; 32],
    pub market_id: u64,
    pub max_bet: u64,
    // Private inputs
    pub balance: u64,
    pub new_balance: u64,
    pub bet_amount: u64,
    pub side: u8,
    pub secret: [u8; 32],
    pub nonce: u64,
    pub new_nonce: u64,
}

/// Proof output from snarkjs
#[derive(Debug, Clone)]
pub struct PlaceBetPrivateProof {
    /// Groth16 proof bytes (256 bytes)
    pub proof: Vec<u8>,
    /// Public inputs for on-chain verification
    pub public_inputs: Vec<u8>,
    /// Computed new_balance_commitment
    pub new_balance_commitment: [u8; 32],
    /// Computed bet_commitment
    pub bet_commitment: [u8; 32],
}

/// Witness for ClaimPrivate circuit (claim_private_v2.circom)
#[derive(Debug, Clone)]
pub struct ClaimPrivateWitness {
    // Public inputs
    pub bet_commitment: [u8; 32],
    pub resolution: u8,  // 0=NO, 1=YES
    pub nullifier_hash: [u8; 32],
    pub bet_amount: u64,
    pub bet_side: u8,  // 0=NO, 1=YES
    pub old_balance_commitment: [u8; 32],
    pub new_balance_commitment: [u8; 32],
    // Private inputs
    pub secret: [u8; 32],
    pub balance: u64,
    pub new_balance: u64,
    pub nonce: u64,
    pub new_nonce: u64,
    pub payout_amount: u64,
}

/// Proof output from ClaimPrivate circuit
#[derive(Debug, Clone)]
pub struct ClaimPrivateProof {
    /// Groth16 proof bytes (256 bytes)
    pub proof: Vec<u8>,
    /// Public inputs for on-chain verification (138 bytes)
    pub public_inputs: Vec<u8>,
}

/// Witness for WithdrawPrivate circuit
#[derive(Debug, Clone)]
pub struct WithdrawPrivateWitness {
    // Public inputs
    pub old_balance_commitment: [u8; 32],
    pub new_balance_commitment: [u8; 32],
    pub withdraw_amount: u64,
    // Private inputs
    pub balance: u64,
    pub new_balance: u64,
    pub nonce: u64,
    pub new_nonce: u64,
}

/// Proof output from WithdrawPrivate circuit
#[derive(Debug, Clone)]
pub struct WithdrawPrivateProof {
    /// Groth16 proof bytes (256 bytes)
    pub proof: Vec<u8>,
    /// Public inputs for on-chain verification
    pub public_inputs: Vec<u8>,
}

/// Calculate Poseidon hash compatible with circomlib
/// Uses light-poseidon which matches circomlib's BN254 Poseidon
fn poseidon_hash_2(a: &BigUint, b: &BigUint) -> [u8; 32] {
    use light_poseidon::{Poseidon, PoseidonBytesHasher, parameters::bn254_x5};

    let mut poseidon = Poseidon::<ark_bn254::Fr>::new_circom(2).unwrap();
    let a_bytes = biguint_to_32be(a);
    let b_bytes = biguint_to_32be(b);

    let hash = poseidon
        .hash_bytes_be(&[&a_bytes, &b_bytes])
        .unwrap();

    hash
}

fn poseidon_hash_3(a: &BigUint, b: &BigUint, c: &BigUint) -> [u8; 32] {
    use light_poseidon::{Poseidon, PoseidonBytesHasher, parameters::bn254_x5};

    let mut poseidon = Poseidon::<ark_bn254::Fr>::new_circom(3).unwrap();
    let a_bytes = biguint_to_32be(a);
    let b_bytes = biguint_to_32be(b);
    let c_bytes = biguint_to_32be(c);

    let hash = poseidon
        .hash_bytes_be(&[&a_bytes, &b_bytes, &c_bytes])
        .unwrap();

    hash
}

fn biguint_to_32be(value: &BigUint) -> [u8; 32] {
    let mut out = [0u8; 32];
    let bytes = value.to_bytes_be();
    let start = 32usize.saturating_sub(bytes.len());
    out[start..].copy_from_slice(&bytes);
    out
}

fn bytes_to_dec(bytes: &[u8]) -> String {
    BigUint::from_bytes_be(bytes).to_str_radix(10)
}

/// Generate PlaceBetPrivate proof using snarkjs
pub fn generate_place_bet_private_proof(
    wasm_path: &PathBuf,
    zkey_path: &PathBuf,
    witness: &PlaceBetPrivateWitness,
) -> Result<PlaceBetPrivateProof> {
    use std::time::{SystemTime, UNIX_EPOCH};

    // Create temp directory
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("place_bet_proof_{}", nanos));
    fs::create_dir_all(&temp_dir)?;

    let input_path = temp_dir.join("input.json");
    let proof_path = temp_dir.join("proof.json");
    let public_path = temp_dir.join("public.json");

    // Build witness JSON
    let input_json = serde_json::json!({
        "old_balance_commitment": bytes_to_dec(&witness.old_balance_commitment),
        "new_balance_commitment": bytes_to_dec(&witness.new_balance_commitment),
        "bet_commitment": bytes_to_dec(&witness.bet_commitment),
        "market_id": witness.market_id.to_string(),
        "max_bet": witness.max_bet.to_string(),
        "balance": witness.balance.to_string(),
        "new_balance": witness.new_balance.to_string(),
        "bet_amount": witness.bet_amount.to_string(),
        "side": witness.side.to_string(),
        "secret": bytes_to_dec(&witness.secret),
        "nonce": witness.nonce.to_string(),
        "new_nonce": witness.new_nonce.to_string(),
    });

    fs::write(&input_path, serde_json::to_string_pretty(&input_json)?)?;

    println!("  Generating ZK proof with snarkjs...");

    // Run snarkjs groth16 fullprove
    let output = Command::new("npx")
        .args([
            "snarkjs",
            "groth16",
            "fullprove",
            input_path.to_str().unwrap(),
            wasm_path.to_str().unwrap(),
            zkey_path.to_str().unwrap(),
            proof_path.to_str().unwrap(),
            public_path.to_str().unwrap(),
        ])
        .output()
        .with_context(|| "Failed to run snarkjs")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        anyhow::bail!("snarkjs failed:\nstderr: {}\nstdout: {}", stderr, stdout);
    }

    // Parse proof.json
    let proof_content = fs::read_to_string(&proof_path)?;
    let proof_json: serde_json::Value = serde_json::from_str(&proof_content)?;

    // Convert proof to 256 bytes
    let proof_bytes = proof_json_to_bytes(&proof_json)?;

    // Build public_inputs for on-chain (compact format for instruction)
    // Format: old_commitment(32) + new_commitment(32) + bet_commitment(32) + max_bet(8) + bet_amount(8) = 112 bytes
    // The processor will reconstruct circuit inputs using market_id from instruction params
    let mut public_inputs = Vec::with_capacity(112);
    public_inputs.extend_from_slice(&witness.old_balance_commitment);
    public_inputs.extend_from_slice(&witness.new_balance_commitment);
    public_inputs.extend_from_slice(&witness.bet_commitment);
    public_inputs.extend_from_slice(&witness.max_bet.to_le_bytes());
    public_inputs.extend_from_slice(&witness.bet_amount.to_le_bytes());

    // Cleanup temp dir
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(PlaceBetPrivateProof {
        proof: proof_bytes,
        public_inputs,
        new_balance_commitment: witness.new_balance_commitment,
        bet_commitment: witness.bet_commitment,
    })
}

/// Generate ClaimPrivate proof using snarkjs
pub fn generate_claim_private_proof(
    wasm_path: &PathBuf,
    zkey_path: &PathBuf,
    witness: &ClaimPrivateWitness,
) -> Result<ClaimPrivateProof> {
    use std::time::{SystemTime, UNIX_EPOCH};
    use std::process::Command;

    // Create temp directory
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("claim_proof_{}", nanos));
    fs::create_dir_all(&temp_dir)?;

    let input_path = temp_dir.join("input.json");
    let proof_path = temp_dir.join("proof.json");
    let public_path = temp_dir.join("public.json");

    // Build witness JSON matching claim_private_v2.circom
    let input_json = serde_json::json!({
        // Public inputs
        "bet_commitment": bytes_to_dec(&witness.bet_commitment),
        "resolution": witness.resolution.to_string(),
        "nullifier_hash": bytes_to_dec(&witness.nullifier_hash),
        "bet_amount": witness.bet_amount.to_string(),
        "bet_side": witness.bet_side.to_string(),
        "old_balance_commitment": bytes_to_dec(&witness.old_balance_commitment),
        "new_balance_commitment": bytes_to_dec(&witness.new_balance_commitment),
        // Private inputs
        "secret": bytes_to_dec(&witness.secret),
        "balance": witness.balance.to_string(),
        "new_balance": witness.new_balance.to_string(),
        "nonce": witness.nonce.to_string(),
        "new_nonce": witness.new_nonce.to_string(),
        "payout_amount": witness.payout_amount.to_string(),
    });

    fs::write(&input_path, serde_json::to_string_pretty(&input_json)?)?;

    println!("  Generating ZK proof with snarkjs (claim_private_v2)...");

    // Run snarkjs groth16 fullprove
    let output = Command::new("npx")
        .args([
            "snarkjs",
            "groth16",
            "fullprove",
            input_path.to_str().unwrap(),
            wasm_path.to_str().unwrap(),
            zkey_path.to_str().unwrap(),
            proof_path.to_str().unwrap(),
            public_path.to_str().unwrap(),
        ])
        .output()
        .with_context(|| "Failed to run snarkjs")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        anyhow::bail!("snarkjs failed:\nstderr: {}\nstdout: {}", stderr, stdout);
    }

    // Parse proof.json
    let proof_content = fs::read_to_string(&proof_path)?;
    let proof_json: serde_json::Value = serde_json::from_str(&proof_content)?;

    // Convert proof to 256 bytes (with A negation)
    let proof_bytes = proof_json_to_bytes(&proof_json)?;

    // Build public_inputs for on-chain (138 bytes):
    // [0..32]: bet_commitment
    // [32]: resolution (0=NO, 1=YES)
    // [33..65]: nullifier_hash
    // [65..73]: bet_amount (u64 le)
    // [73]: bet_side (0=NO, 1=YES)
    // [74..106]: old_balance_commitment
    // [106..138]: new_balance_commitment
    let mut public_inputs = Vec::with_capacity(138);
    public_inputs.extend_from_slice(&witness.bet_commitment);
    public_inputs.push(witness.resolution);
    public_inputs.extend_from_slice(&witness.nullifier_hash);
    public_inputs.extend_from_slice(&witness.bet_amount.to_le_bytes());
    public_inputs.push(witness.bet_side);
    public_inputs.extend_from_slice(&witness.old_balance_commitment);
    public_inputs.extend_from_slice(&witness.new_balance_commitment);

    // Cleanup temp dir
    let _ = fs::remove_dir_all(&temp_dir);

    println!("  ZK proof generated successfully (claim_private_v2)!");

    Ok(ClaimPrivateProof {
        proof: proof_bytes,
        public_inputs,
    })
}

/// Generate WithdrawPrivate proof using snarkjs
pub fn generate_withdraw_private_proof(
    wasm_path: &PathBuf,
    zkey_path: &PathBuf,
    witness: &WithdrawPrivateWitness,
) -> Result<WithdrawPrivateProof> {
    use std::time::{SystemTime, UNIX_EPOCH};

    // Create temp directory
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("withdraw_proof_{}", nanos));
    fs::create_dir_all(&temp_dir)?;

    let input_path = temp_dir.join("input.json");
    let proof_path = temp_dir.join("proof.json");
    let public_path = temp_dir.join("public.json");

    // Build witness JSON matching withdraw_private.circom
    let input_json = serde_json::json!({
        // Public inputs
        "old_balance_commitment": bytes_to_dec(&witness.old_balance_commitment),
        "new_balance_commitment": bytes_to_dec(&witness.new_balance_commitment),
        "withdraw_amount": witness.withdraw_amount.to_string(),
        // Private inputs
        "balance": witness.balance.to_string(),
        "new_balance": witness.new_balance.to_string(),
        "nonce": witness.nonce.to_string(),
        "new_nonce": witness.new_nonce.to_string(),
    });

    fs::write(&input_path, serde_json::to_string_pretty(&input_json)?)?;

    println!("  Generating ZK proof with snarkjs (withdraw_private)...");

    // Run snarkjs groth16 fullprove
    let output = Command::new("npx")
        .args([
            "snarkjs",
            "groth16",
            "fullprove",
            input_path.to_str().unwrap(),
            wasm_path.to_str().unwrap(),
            zkey_path.to_str().unwrap(),
            proof_path.to_str().unwrap(),
            public_path.to_str().unwrap(),
        ])
        .output()
        .with_context(|| "Failed to run snarkjs")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        anyhow::bail!("snarkjs failed:\nstderr: {}\nstdout: {}", stderr, stdout);
    }

    // Parse proof.json
    let proof_content = fs::read_to_string(&proof_path)?;
    let proof_json: serde_json::Value = serde_json::from_str(&proof_content)?;

    // Convert proof to 256 bytes
    let proof_bytes = proof_json_to_bytes(&proof_json)?;

    // Build public_inputs for on-chain (compact format)
    // old_commitment(32) + new_commitment(32) + amount(8) = 72 bytes
    let mut public_inputs = Vec::with_capacity(72);
    public_inputs.extend_from_slice(&witness.old_balance_commitment);
    public_inputs.extend_from_slice(&witness.new_balance_commitment);
    public_inputs.extend_from_slice(&witness.withdraw_amount.to_le_bytes());

    // Cleanup temp dir
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(WithdrawPrivateProof {
        proof: proof_bytes,
        public_inputs,
    })
}

/// BN254 base field modulus (for G1 point negation)
fn bn254_field_modulus() -> BigUint {
    BigUint::parse_bytes(
        b"21888242871839275222246405745257275088696311157297823662689037894645226208583",
        10,
    ).unwrap()
}

/// Convert snarkjs proof.json to 256-byte format for groth16-solana
///
/// groth16-solana expects:
/// - A: 64 bytes (G1, NEGATED for pairing equation)
/// - B: 128 bytes (G2)
/// - C: 64 bytes (G1)
///
/// G1 negation: (x, y) -> (x, field_modulus - y)
fn proof_json_to_bytes(proof: &serde_json::Value) -> Result<Vec<u8>> {
    let pi_a = proof
        .get("pi_a")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("missing pi_a"))?;
    let pi_b = proof
        .get("pi_b")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("missing pi_b"))?;
    let pi_c = proof
        .get("pi_c")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("missing pi_c"))?;

    let ax = dec_value(pi_a, 0)?;
    let ay = dec_value(pi_a, 1)?;

    // Negate A.y for groth16-solana pairing equation
    let p = bn254_field_modulus();
    let ay_neg = &p - &ay;

    let bx = pi_b
        .get(0)
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("missing pi_b[0]"))?;
    let by = pi_b
        .get(1)
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("missing pi_b[1]"))?;
    let bx0 = dec_value(bx, 0)?;
    let bx1 = dec_value(bx, 1)?;
    let by0 = dec_value(by, 0)?;
    let by1 = dec_value(by, 1)?;

    let cx = dec_value(pi_c, 0)?;
    let cy = dec_value(pi_c, 1)?;

    // Format: A(64) + B(128) + C(64) = 256 bytes
    // All values in big-endian (32 bytes each coordinate)
    let mut bytes = Vec::with_capacity(256);

    // A (negated): x, -y
    bytes.extend_from_slice(&biguint_to_32be(&ax));
    bytes.extend_from_slice(&biguint_to_32be(&ay_neg));

    // B: x1, x0, y1, y0 (G2 point in Fp2, groth16-solana order)
    bytes.extend_from_slice(&biguint_to_32be(&bx1));
    bytes.extend_from_slice(&biguint_to_32be(&bx0));
    bytes.extend_from_slice(&biguint_to_32be(&by1));
    bytes.extend_from_slice(&biguint_to_32be(&by0));

    // C: x, y
    bytes.extend_from_slice(&biguint_to_32be(&cx));
    bytes.extend_from_slice(&biguint_to_32be(&cy));

    Ok(bytes)
}

fn dec_value(arr: &[serde_json::Value], idx: usize) -> Result<BigUint> {
    let s = arr
        .get(idx)
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing decimal at index {}", idx))?;
    BigUint::parse_bytes(s.as_bytes(), 10)
        .ok_or_else(|| anyhow::anyhow!("invalid decimal value: {}", s))
}

/// Fetch PrivateBalance state from chain
async fn fetch_private_balance_state(
    rpc_url: &str,
    user: &solana_sdk::pubkey::Pubkey,
) -> Result<(solana_sdk::pubkey::Pubkey, [u8; 32], u64)> {
    use borsh::BorshDeserialize;
    use solana_client::rpc_client::RpcClient;

    let program_id = get_futarchy_program_id()?;
    let (pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[PRIVATE_BALANCE_SEED, user.as_ref()],
        &program_id,
    );

    let client = RpcClient::new(rpc_url);
    let account = client.get_account(&pda)
        .with_context(|| format!("Failed to fetch PrivateBalance for {}", user))?;

    // Parse PrivateBalance (user: 32, encrypted_len: 4, encrypted: var, commitment: 32, nonce: 8, bump: 1)
    // For simplicity, we extract commitment and nonce from known offsets
    let data = &account.data;
    if data.len() < 32 + 4 {
        anyhow::bail!("PrivateBalance data too short");
    }

    // Skip user (32) + encrypted_balance vec (4 byte len + data)
    let encrypted_len = u32::from_le_bytes(data[32..36].try_into()?) as usize;
    let commitment_start = 32 + 4 + encrypted_len;

    if data.len() < commitment_start + 32 + 8 {
        anyhow::bail!("PrivateBalance data incomplete");
    }

    let mut commitment = [0u8; 32];
    commitment.copy_from_slice(&data[commitment_start..commitment_start + 32]);

    let nonce = u64::from_le_bytes(data[commitment_start + 32..commitment_start + 40].try_into()?);

    Ok((pda, commitment, nonce))
}

/// Fetch MarketV2 state to get max_bet
async fn fetch_market_v2_state(rpc_url: &str, market_id: u64) -> Result<u64> {
    use solana_client::rpc_client::RpcClient;

    let program_id = get_futarchy_program_id()?;
    let (market_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[MARKET_V2_SEED, &market_id.to_le_bytes()],
        &program_id,
    );

    let client = RpcClient::new(rpc_url);
    let account = client.get_account(&market_pda)
        .with_context(|| format!("Failed to fetch MarketV2 for id {}", market_id))?;

    // MarketV2 layout (from state/market.rs):
    // authority(32) + market_id(8) + oracle(32) + question_hash(32) +
    // end_time(8) + status(1) + max_bet(8) + resolution(2) + ...
    let data = &account.data;
    let min_len = 32 + 8 + 32 + 32 + 8 + 1 + 8; // 121 bytes to include max_bet
    if data.len() < min_len {
        anyhow::bail!("MarketV2 data too short: {} < {}", data.len(), min_len);
    }

    // max_bet offset: 32 + 8 + 32 + 32 + 8 + 1 = 113
    let max_bet_start = 32 + 8 + 32 + 32 + 8 + 1;
    let max_bet = u64::from_le_bytes(data[max_bet_start..max_bet_start + 8].try_into()?);

    Ok(max_bet)
}

/// MarketV3 state fetched from chain
#[derive(Debug)]
struct MarketV3State {
    pub pool_commitment: [u8; 32],
    pub max_bet: u64,
    pub status: u8,
    pub decrypted_pool_yes: Option<u64>,
    pub decrypted_pool_no: Option<u64>,
    pub resolution: Option<bool>,
}

/// Fetch MarketV3 state from chain
async fn fetch_market_v3_state(rpc_url: &str, market_id: u64) -> Result<MarketV3State> {
    use solana_client::rpc_client::RpcClient;

    let program_id = get_futarchy_program_id()?;
    let (market_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[MARKET_V3_SEED, &market_id.to_le_bytes()],
        &program_id,
    );

    let client = RpcClient::new(rpc_url);
    let account = client.get_account(&market_pda)
        .with_context(|| format!("Failed to fetch MarketV3 for id {}", market_id))?;

    // MarketV3 layout (from state/market.rs):
    // authority: 0-32
    // market_id: 32-40
    // oracle: 40-72
    // question_hash: 72-104
    // end_time: 104-112
    // status: 112-113
    // max_bet: 113-121
    // resolution: 121-123 (Option<bool>: 1 byte discriminant + 1 byte value)
    // pool_commitment: 123-155
    // encrypted_pool_yes_hash: 155-187
    // encrypted_pool_no_hash: 187-219
    // decrypted_pool_yes: 219-228 (Option<u64>: 1 byte discriminant + 8 bytes)
    // decrypted_pool_no: 228-237 (Option<u64>: 1 byte discriminant + 8 bytes)
    let data = &account.data;
    let min_len = 237;
    if data.len() < min_len {
        anyhow::bail!("MarketV3 data too short: {} < {}", data.len(), min_len);
    }

    let status = data[112];
    let max_bet = u64::from_le_bytes(data[113..121].try_into()?);

    // Parse resolution (Option<bool>)
    let resolution = if data[121] == 0 {
        None
    } else {
        Some(data[122] != 0)
    };

    let mut pool_commitment = [0u8; 32];
    pool_commitment.copy_from_slice(&data[123..155]);

    // Parse decrypted pools (Option<u64>)
    let decrypted_pool_yes = if data[219] == 0 {
        None
    } else {
        Some(u64::from_le_bytes(data[220..228].try_into()?))
    };

    let decrypted_pool_no = if data[228] == 0 {
        None
    } else {
        Some(u64::from_le_bytes(data[229..237].try_into()?))
    };

    Ok(MarketV3State {
        pool_commitment,
        max_bet,
        status,
        decrypted_pool_yes,
        decrypted_pool_no,
        resolution,
    })
}

/// Local pool state for V3 markets (stored per market)
#[derive(Serialize, Deserialize, Debug, Clone)]
struct LocalMarketPoolState {
    pub market_id: u64,
    pub pool_yes: u64,
    pub pool_no: u64,
    pub blinding: String, // hex encoded
    pub pool_commitment: String, // hex encoded, for verification
}

impl LocalMarketPoolState {
    /// Get the path for storing local pool state
    fn get_path(market_id: u64) -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home)
            .join(".zyb")
            .join(format!("v3_pool_{}.json", market_id))
    }

    /// Load local pool state or return None if not exists
    fn load(market_id: u64) -> Option<Self> {
        let path = Self::get_path(market_id);
        if !path.exists() {
            return None;
        }

        let content = fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Save local pool state
    fn save(&self) -> Result<()> {
        let path = Self::get_path(self.market_id);

        // Create directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }

    /// Create initial state (all zeros)
    fn initial(market_id: u64, initial_blinding: &[u8; 32]) -> Self {
        Self {
            market_id,
            pool_yes: 0,
            pool_no: 0,
            blinding: hex::encode(initial_blinding),
            pool_commitment: String::new(), // Will be set after calculation
        }
    }
}

/// Calculate Poseidon commitment using circomlibjs (via Node.js)
/// This ensures compatibility with the circom circuit
fn calculate_poseidon_commitment_js(
    pool_yes: u64,
    pool_no: u64,
    blinding_decimal: &str,
) -> Result<[u8; 32]> {
    use std::process::Command;

    let js_code = format!(
        r#"
const {{ buildPoseidon }} = require('circomlibjs');
(async () => {{
    const poseidon = await buildPoseidon();
    const hash = poseidon([BigInt('{}'), BigInt('{}'), BigInt('{}')]);
    const decimal = poseidon.F.toString(hash);
    console.log(decimal);
}})();
"#,
        pool_yes, pool_no, blinding_decimal
    );

    let output = Command::new("node")
        .arg("-e")
        .arg(&js_code)
        .current_dir("circuits")
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run node: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Poseidon calculation failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let decimal_str = stdout.trim();

    // Convert decimal to bytes (big-endian to match bytes_to_dec)
    let big = num_bigint::BigUint::parse_bytes(decimal_str.as_bytes(), 10)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse Poseidon output: {}", decimal_str))?;

    let mut bytes = [0u8; 32];
    let big_bytes = big.to_bytes_be();
    // Pad with zeros on the left (big-endian)
    let start = 32 - big_bytes.len().min(32);
    bytes[start..].copy_from_slice(&big_bytes[..big_bytes.len().min(32)]);

    Ok(bytes)
}

// =============================================================================

/// Generate ZK proof for PlaceBetBlind circuit using snarkjs
fn generate_place_bet_blind_proof(
    wasm_path: &PathBuf,
    zkey_path: &PathBuf,
    pool_commitment_before: &[u8; 32],
    pool_commitment_after: &[u8; 32],
    bet_ciphertext_hash: &[u8; 32],
    market_id: u64,
    max_bet: u64,
    bet_amount: u64,
    bet_side: u8,
    pool_yes_before: u64,
    pool_no_before: u64,
    pool_yes_after: u64,
    pool_no_after: u64,
    blinding_before: &[u8; 32],
    blinding_after: &[u8; 32],
) -> Result<(Vec<u8>, Vec<u8>)> {
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    // Create temp directory
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("place_bet_blind_{}", nanos));
    fs::create_dir_all(&temp_dir)?;

    let input_path = temp_dir.join("input.json");
    let proof_path = temp_dir.join("proof.json");
    let public_path = temp_dir.join("public.json");

    // Build witness JSON for PlaceBetBlind circuit
    let input_json = serde_json::json!({
        // Public inputs (as string decimals for circom)
        "pool_commitment_before": bytes_to_dec(pool_commitment_before),
        "pool_commitment_after": bytes_to_dec(pool_commitment_after),
        "bet_ciphertext_hash": bytes_to_dec(bet_ciphertext_hash),
        "market_id": market_id.to_string(),
        "max_bet": max_bet.to_string(),
        // Private inputs
        "bet_amount": bet_amount.to_string(),
        "bet_side": bet_side.to_string(),
        "pool_yes_before": pool_yes_before.to_string(),
        "pool_no_before": pool_no_before.to_string(),
        "pool_yes_after": pool_yes_after.to_string(),
        "pool_no_after": pool_no_after.to_string(),
        "blinding_before": bytes_to_dec(blinding_before),
        "blinding_after": bytes_to_dec(blinding_after),
    });

    fs::write(&input_path, serde_json::to_string_pretty(&input_json)?)?;

    println!("  Running snarkjs groth16 fullprove...");

    // Run snarkjs groth16 fullprove
    let output = Command::new("npx")
        .args([
            "snarkjs",
            "groth16",
            "fullprove",
            input_path.to_str().unwrap(),
            wasm_path.to_str().unwrap(),
            zkey_path.to_str().unwrap(),
            proof_path.to_str().unwrap(),
            public_path.to_str().unwrap(),
        ])
        .output()
        .with_context(|| "Failed to run snarkjs")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        anyhow::bail!("snarkjs failed:\nstderr: {}\nstdout: {}", stderr, stdout);
    }

    // Parse proof.json
    let proof_content = fs::read_to_string(&proof_path)?;
    let proof_json: serde_json::Value = serde_json::from_str(&proof_content)?;

    // Convert proof to 256 bytes (Groth16 format with A negation)
    let proof_bytes = proof_json_to_bytes(&proof_json)?;

    // Build public_inputs for on-chain verification
    // Format (120 bytes):
    // 0-32: pool_commitment_before
    // 32-64: pool_commitment_after
    // 64-96: bet_ciphertext_hash
    // 96-104: market_id (8 bytes)
    // 104-112: max_bet (8 bytes)
    // 112-120: bet_amount (8 bytes) - for on-chain transfer
    let mut public_inputs = Vec::with_capacity(120);
    public_inputs.extend_from_slice(pool_commitment_before);
    public_inputs.extend_from_slice(pool_commitment_after);
    public_inputs.extend_from_slice(bet_ciphertext_hash);
    public_inputs.extend_from_slice(&market_id.to_le_bytes());
    public_inputs.extend_from_slice(&max_bet.to_le_bytes());
    public_inputs.extend_from_slice(&bet_amount.to_le_bytes()); // bet_amount for transfer

    // Cleanup temp dir
    let _ = fs::remove_dir_all(&temp_dir);

    println!("  ZK proof generated successfully!");

    Ok((proof_bytes, public_inputs))
}
// V3 Command Implementations (Fully Blind Markets)
// =============================================================================

#[tokio::main]
async fn create_market_v3(args: CreateV3Args) -> Result<()> {
    use solana_sdk::signature::{read_keypair_file, Signer};
    use solana_sdk::transaction::Transaction;
    use solana_sdk::instruction::{AccountMeta, Instruction};
    use solana_client::rpc_client::RpcClient;
    use sha3::{Digest, Keccak256};

    println!("{}", "Creating Market V3 (fully blind with FHE)...".cyan().bold());
    println!();
    println!("Market ID: {}", args.market_id);
    println!("Question: {}", args.question);
    println!("End time: {}", args.end_time);
    println!("Max bet: {} lamports", args.max_bet);
    println!("Threshold: {}-of-{}", args.threshold_required, args.threshold_shares);
    println!();

    // Load keypair
    let keypair = read_keypair_file(&args.keypair)
        .map_err(|e| anyhow::anyhow!("Failed to load keypair from {:?}: {}", args.keypair, e))?;

    // Hash the question (32 bytes)
    let question_hash = {
        let mut hasher = Keccak256::new();
        hasher.update(args.question.as_bytes());
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    };

    // Parse threshold pubkey (32 bytes hex)
    let threshold_pubkey = {
        let decoded = hex::decode(&args.threshold_pubkey)
            .context("Invalid threshold_pubkey hex")?;
        if decoded.len() != 32 {
            anyhow::bail!("threshold_pubkey must be 32 bytes (got {} bytes)", decoded.len());
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&decoded);
        key
    };

    // Determine oracle pubkey
    let oracle_pubkey = if let Some(oracle_str) = args.oracle {
        solana_sdk::pubkey::Pubkey::from_str(&oracle_str)
            .context("Invalid oracle pubkey")?
    } else {
        // Default to the creator as oracle
        keypair.pubkey()
    };

    println!("{}", "Building CreateMarketV3 instruction...".cyan());
    println!("  Oracle: {}", oracle_pubkey);
    println!("  Question hash: {}", hex::encode(question_hash));
    println!("  Threshold pubkey: {}", hex::encode(threshold_pubkey));

    // Build CreateMarketV3 instruction data
    let instruction_data = FutarchyInstructionV3::CreateMarketV3 {
        market_id: args.market_id,
        question_hash,
        end_time: args.end_time,
        max_bet: args.max_bet,
        threshold_pubkey,
        threshold_required: args.threshold_required,
        threshold_shares: args.threshold_shares,
        has_governance: args.has_governance,
        executable_action: None,
        execution_threshold: None,
        timelock_duration: None,
    };

    let instruction_bytes = instruction_data.pack()?;

    println!("  Instruction size: {} bytes", instruction_bytes.len());

    // Derive PDAs
    println!("{}", "Deriving PDAs...".cyan());

    let program_id = get_futarchy_program_id()?;
    let authority = keypair.pubkey();
    let market_id_bytes = args.market_id.to_le_bytes();

    // MarketV3 PDA: ["market_v3", market_id]
    let (market_v3_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[MARKET_V3_SEED, &market_id_bytes],
        &program_id,
    );

    // MarketVault PDA: ["market_vault", market_id]
    let (market_vault_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[MARKET_VAULT_SEED, &market_id_bytes],
        &program_id,
    );

    println!("  Authority: {}", authority);
    println!("  MarketV3 PDA: {}", market_v3_pda);
    println!("  MarketVault PDA: {}", market_vault_pda);
    println!("  Oracle: {}", oracle_pubkey);

    // Build account metas
    // Accounts expected (from processor):
    // 0. [writable, signer] Market creator/authority
    // 1. [writable] MarketV3 PDA
    // 2. [writable] MarketVault PDA
    // 3. [] Oracle pubkey
    // 4. [] System program
    // 5. [] Clock sysvar
    // If has_governance = true:
    // 6. [writable] GovernanceConfig PDA

    let mut accounts = vec![
        AccountMeta::new(authority, true),
        AccountMeta::new(market_v3_pda, false),
        AccountMeta::new(market_vault_pda, false),
        AccountMeta::new_readonly(oracle_pubkey, false),
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        AccountMeta::new_readonly(solana_sdk::sysvar::clock::id(), false),
    ];

    // Add governance config PDA if governance is enabled
    if args.has_governance {
        let (governance_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
            &[GOVERNANCE_CONFIG_SEED, &market_id_bytes],
            &program_id,
        );
        accounts.push(AccountMeta::new(governance_pda, false));
        println!("  GovernanceConfig PDA: {}", governance_pda);
    }

    // Create instruction
    let instruction = Instruction {
        program_id,
        accounts,
        data: instruction_bytes,
    };

    // Create and send transaction
    println!("{}", "Sending transaction to Solana...".cyan());

    let rpc_client = RpcClient::new(&args.rpc_url);
    let blockhash = rpc_client.get_latest_blockhash()
        .context("Failed to get latest blockhash")?;

    let mut tx = Transaction::new_with_payer(&[instruction], Some(&authority));
    tx.sign(&[&keypair], blockhash);

    let signature = rpc_client.send_and_confirm_transaction(&tx)
        .context("Failed to send transaction")?;

    println!();
    println!("{}", "Market V3 created successfully!".green().bold());
    println!();
    println!("TX Signature: {}", signature.to_string().yellow().bold());
    println!("Market ID: {}", args.market_id);
    println!("MarketV3 PDA: {}", market_v3_pda);
    println!("MarketVault PDA: {}", market_vault_pda);
    println!("Oracle: {}", oracle_pubkey);
    println!("End time: {}", args.end_time);
    println!();

    Ok(())
}

#[tokio::main]
async fn bet_blind_v3(args: BetBlindArgs) -> Result<()> {
    use zyberlink_fhe::futarchy::{encrypt_bet_with_hash};
    use zyberlink_fhe::{generate_keys, serialize_client_key, serialize_server_key};
    use solana_sdk::signature::{read_keypair_file, Signer};
    use rand::RngCore;

    println!("{}", "Placing BLIND bet (V3)...".cyan().bold());
    println!();
    println!("Market ID: {}", args.market_id);
    println!("Side: {} (will be HIDDEN on-chain)", match args.side {
        BetSide::Yes => "YES",
        BetSide::No => "NO",
    });
    println!("Amount: {} lamports", args.amount);
    println!("Mode: Fully Blind (FHE + ZK)");
    println!();

    // Validate amount
    if args.amount == 0 {
        anyhow::bail!("Bet amount must be greater than 0");
    }

    // Load keypair
    let keypair = read_keypair_file(&args.keypair)
        .map_err(|e| anyhow::anyhow!("Failed to load keypair from {:?}: {}", args.keypair, e))?;

    // Step 1: Generate or load FHE keys
    println!("{}", "Loading FHE keys...".cyan());
    let (client_key, server_key) = if args.fhe_keys.exists() {
        println!("  Using existing FHE keys from {:?}", args.fhe_keys);
        let keys_json = fs::read_to_string(&args.fhe_keys)?;
        let keys: serde_json::Value = serde_json::from_str(&keys_json)?;

        let client_key_b64 = keys["client_key"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing client_key in FHE keys"))?;
        let server_key_b64 = keys["server_key"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing server_key in FHE keys"))?;

        use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
        let client_bytes = BASE64.decode(client_key_b64)?;
        let server_bytes = BASE64.decode(server_key_b64)?;

        let client_key = bincode::deserialize(&client_bytes)?;
        let server_key = bincode::deserialize(&server_bytes)?;

        (client_key, server_key)
    } else {
        println!("  Generating new FHE keys (this may take a minute)...");
        let keys = generate_keys()?;

        let client_bytes = serialize_client_key(&keys.0)?;
        let server_bytes = serialize_server_key(&keys.1)?;

        use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
        let keys_json = json!({
            "client_key": BASE64.encode(&client_bytes),
            "server_key": BASE64.encode(&server_bytes),
        });
        fs::write(&args.fhe_keys, serde_json::to_string_pretty(&keys_json)?)?;

        println!("  FHE keys saved to {:?}", args.fhe_keys);
        keys
    };

    // Step 2: Encrypt bet amount using FHE
    println!("{}", "Encrypting bet amount...".cyan());
    let encrypted_bet = encrypt_bet_with_hash(args.amount, &client_key)?;

    // Reduce hash to fit in BN254 field (set first byte to 0)
    let mut bet_ciphertext_hash = encrypted_bet.hash;
    bet_ciphertext_hash[0] = 0;

    println!("  Ciphertext size: {} bytes", encrypted_bet.ciphertext.len());
    println!("  Ciphertext hash: {}", hex::encode(&bet_ciphertext_hash));

    // Step 3: Generate cryptographic secrets for proof
    println!("{}", "Generating cryptographic secrets...".cyan());
    let mut rng = rand::thread_rng();

    let mut secret = [0u8; 32];
    let mut blinding_after = [0u8; 32];
    rng.fill_bytes(&mut secret);
    rng.fill_bytes(&mut blinding_after);

    // For empty pools (first bet), blinding_before must be 0 to match Poseidon(0,0,0)
    let blinding_before = [0u8; 32];

    // Bet side as u8 (0=YES, 1=NO) - note: reversed from BetSide enum
    let bet_side_u8 = match args.side {
        BetSide::Yes => 0u8,
        BetSide::No => 1u8,
    };

    // Calculate bet_secret_commitment = Poseidon(amount, side, secret)
    // Using Poseidon to match the claim_blind circom circuit
    let secret_decimal = num_bigint::BigUint::from_bytes_be(&secret).to_string();
    let bet_secret_commitment = calculate_poseidon_commitment_js(
        args.amount,
        bet_side_u8 as u64,
        &secret_decimal,
    )?;

    println!("  Bet secret commitment: {}", hex::encode(&bet_secret_commitment));

    // Step 4: Fetch current pool state from chain and synchronize with local state
    println!("{}", "Fetching market state...".cyan());

    // Try to fetch on-chain state (may fail if market doesn't exist yet or no cluster)
    let chain_state = fetch_market_v3_state(&args.rpc_url, args.market_id).await;

    // Load local pool state or create initial
    let local_pool_state = LocalMarketPoolState::load(args.market_id);

    // Determine pool_yes_before and pool_no_before
    let (pool_yes_before, pool_no_before, blinding_before) = match (&chain_state, &local_pool_state) {
        // Case 1: Both chain and local available - verify they match
        (Ok(chain), Some(local)) => {
            // Decode local blinding
            let local_blinding = hex::decode(&local.blinding)
                .map_err(|_| anyhow::anyhow!("Invalid blinding in local pool state"))?;
            let local_blinding: [u8; 32] = local_blinding.try_into()
                .map_err(|_| anyhow::anyhow!("Invalid blinding length in local pool state"))?;

            // Calculate expected commitment from local state
            let blinding_decimal = num_bigint::BigUint::from_bytes_be(&local_blinding).to_string();
            let expected_commitment = calculate_poseidon_commitment_js(
                local.pool_yes,
                local.pool_no,
                &blinding_decimal,
            )?;

            // Verify local state matches chain
            if expected_commitment != chain.pool_commitment {
                println!("{}", "Warning: Local pool state doesn't match on-chain commitment!".yellow());
                println!("  Local:   YES={}, NO={}", local.pool_yes, local.pool_no);
                println!("  On-chain commitment: {}", hex::encode(&chain.pool_commitment));
                println!("  Expected commitment: {}", hex::encode(&expected_commitment));
                println!("{}", "This means another participant placed a bet since your last bet.".yellow());
                println!("{}", "For now, using local state (multi-user sync requires prover node).".yellow());
            }

            (local.pool_yes, local.pool_no, local_blinding)
        },
        // Case 2: Only local available (chain fetch failed)
        (Err(_), Some(local)) => {
            println!("{}", "Using local pool state (chain fetch failed)".yellow());
            let local_blinding = hex::decode(&local.blinding)
                .map_err(|_| anyhow::anyhow!("Invalid blinding in local pool state"))?;
            let local_blinding: [u8; 32] = local_blinding.try_into()
                .map_err(|_| anyhow::anyhow!("Invalid blinding length in local pool state"))?;

            (local.pool_yes, local.pool_no, local_blinding)
        },
        // Case 3: Only chain available (first bet from this client)
        (Ok(chain), None) => {
            // Check if pool is empty (all zeros commitment means first bet)
            let zero_commitment = [0u8; 32];
            if chain.pool_commitment == zero_commitment {
                println!("  First bet on this market (empty pool)");
                (0u64, 0u64, blinding_before)
            } else {
                // Pool has bets but we don't have local state
                // This user is joining mid-game - they need pool state from prover
                println!("{}", "Warning: Market has existing bets but no local state.".yellow());
                println!("  On-chain pool_commitment: {}", hex::encode(&chain.pool_commitment));
                println!("{}", "For MVP, starting with empty pool (incorrect for production).".yellow());
                println!("{}", "Production requires fetching pool state from prover node.".yellow());
                (0u64, 0u64, blinding_before)
            }
        },
        // Case 4: Neither available - assume first bet
        (Err(e), None) => {
            println!("{}", format!("Chain fetch failed: {}. Assuming first bet.", e).yellow());
            (0u64, 0u64, blinding_before)
        },
    };

    // Get max_bet from chain or use default
    let max_bet = chain_state.as_ref().map(|s| s.max_bet).unwrap_or(1_000_000_000);
    println!("  Max bet: {} lamports", max_bet);

    if args.amount > max_bet {
        anyhow::bail!("Bet amount {} exceeds market max_bet {}", args.amount, max_bet);
    }

    // Calculate pools after bet
    let (pool_yes_after, pool_no_after) = if bet_side_u8 == 0 {
        (pool_yes_before + args.amount, pool_no_before)
    } else {
        (pool_yes_before, pool_no_before + args.amount)
    };

    println!("  Pool before: YES={}, NO={}", pool_yes_before, pool_no_before);
    println!("  Pool after:  YES={}, NO={}", pool_yes_after, pool_no_after);

    // Calculate pool commitments using circomlibjs Poseidon (ensures compatibility)
    // pool_commitment = Poseidon(pool_yes, pool_no, blinding)
    // We use Node.js to call circomlibjs which matches the circom circuit exactly

    // Use big-endian to match bytes_to_dec used in generate_place_bet_blind_proof
    let blinding_before_decimal = num_bigint::BigUint::from_bytes_be(&blinding_before).to_string();
    let blinding_after_decimal = num_bigint::BigUint::from_bytes_be(&blinding_after).to_string();

    let pool_commitment_before = calculate_poseidon_commitment_js(
        pool_yes_before,
        pool_no_before,
        &blinding_before_decimal,
    )?;

    let pool_commitment_after = calculate_poseidon_commitment_js(
        pool_yes_after,
        pool_no_after,
        &blinding_after_decimal,
    )?;

    // Step 5: Generate ZK proof using PlaceBetBlind circuit
    println!("{}", "Generating ZK proof...".cyan());
    println!("  Circuit: PlaceBetBlind (circuit 50)");

    // Build witness inputs for the circuit
    let _witness_input = json!({
        "pool_commitment_before": pool_commitment_before.iter().map(|b| b.to_string()).collect::<Vec<_>>(),
        "pool_commitment_after": pool_commitment_after.iter().map(|b| b.to_string()).collect::<Vec<_>>(),
        "bet_ciphertext_hash": bet_ciphertext_hash.iter().map(|b| b.to_string()).collect::<Vec<_>>(),
        "market_id": args.market_id.to_string(),
        "max_bet": max_bet.to_string(),
        "bet_amount": args.amount.to_string(),
        "bet_side": bet_side_u8.to_string(),
        "pool_yes_before": pool_yes_before.to_string(),
        "pool_no_before": pool_no_before.to_string(),
        "pool_yes_after": pool_yes_after.to_string(),
        "pool_no_after": pool_no_after.to_string(),
        "blinding_before": blinding_before.iter().map(|b| b.to_string()).collect::<Vec<_>>(),
        "blinding_after": blinding_after.iter().map(|b| b.to_string()).collect::<Vec<_>>(),
    });

    // Generate ZK proof using PlaceBetBlind circuit
    let (proof, public_inputs) = if args.circuit_wasm.exists() && args.circuit_zkey.exists() {
        println!("  Using real Groth16 proof generation");
        generate_place_bet_blind_proof(
            &args.circuit_wasm,
            &args.circuit_zkey,
            &pool_commitment_before,
            &pool_commitment_after,
            &bet_ciphertext_hash,
            args.market_id,
            max_bet,
            args.amount,
            bet_side_u8,
            pool_yes_before,
            pool_no_before,
            pool_yes_after,
            pool_no_after,
            &blinding_before,
            &blinding_after,
        )?
    } else {
        // Mock proof for testing
        println!("  WARNING: Using mock proof (circuit files not found)");
        let proof = vec![0u8; 256];
        let mut public_inputs = Vec::with_capacity(120);
        // Public inputs format (120 bytes):
        // 0-32: pool_commitment_before
        // 32-64: pool_commitment_after
        // 64-96: bet_ciphertext_hash
        // 96-104: market_id (8 bytes)
        // 104-112: max_bet (8 bytes)
        // 112-120: bet_amount (8 bytes) - for on-chain transfer
        public_inputs.extend_from_slice(&pool_commitment_before);
        public_inputs.extend_from_slice(&pool_commitment_after);
        public_inputs.extend_from_slice(&bet_ciphertext_hash);
        public_inputs.extend_from_slice(&args.market_id.to_le_bytes());
        public_inputs.extend_from_slice(&max_bet.to_le_bytes());
        public_inputs.extend_from_slice(&args.amount.to_le_bytes()); // bet_amount for transfer
        (proof, public_inputs)
    };

    println!("  Proof generated ({} bytes)", proof.len());
    println!("  Public inputs: {} bytes", public_inputs.len());
    println!("{}", "Fetching private balance state...".cyan());
    let (balance_pda, balance_commitment, _balance_nonce) =
        fetch_private_balance_state(&args.rpc_url, &keypair.pubkey()).await?;

    // For simplicity, use same balance commitment (real impl should update it)
    let new_balance_commitment = balance_commitment;

    println!("  Balance PDA: {}", balance_pda);
    println!("  Balance commitment: {}", hex::encode(&balance_commitment));

    // Step 7: Build PlaceBetBlind instruction
    println!("{}", "Building transaction...".cyan());

    let instruction_data = FutarchyInstructionV3::PlaceBetBlind {
        market_id: args.market_id,
        bet_ciphertext_hash,
        bet_secret_commitment,
        pool_commitment_after,
        proof,
        public_inputs,
        new_balance_commitment,
        circuit_type: CIRCUIT_PLACE_BET_BLIND,
    };

    let instruction_bytes = instruction_data.pack()?;

    println!("  Instruction size: {} bytes", instruction_bytes.len());
    println!("  Ready to send to chain");

    // Step 8: Derive PDAs and build transaction
    println!("{}", "Deriving PDAs...".cyan());

    let program_id = get_futarchy_program_id()?;
    let zk_program_id = get_zk_generator_program_id()?;
    let user = keypair.pubkey();
    let market_id_bytes = args.market_id.to_le_bytes();

    // MarketV3 PDA: ["market_v3", market_id]
    let (market_v3_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[MARKET_V3_SEED, &market_id_bytes],
        &program_id,
    );

    // PositionV3 PDA: ["position_v3", market_id, bet_ciphertext_hash]
    let (position_v3_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[POSITION_V3_SEED, &market_id_bytes, &bet_ciphertext_hash],
        &program_id,
    );

    // MarketVault PDA: ["market_vault", market_id]
    let (market_vault_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[MARKET_VAULT_SEED, &market_id_bytes],
        &program_id,
    );

    // ProtocolVault PDA: ["protocol_vault"]
    let (protocol_vault_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[PROTOCOL_VAULT_SEED],
        &program_id,
    );

    println!("  MarketV3 PDA: {}", market_v3_pda);
    println!("  PositionV3 PDA: {}", position_v3_pda);
    println!("  MarketVault PDA: {}", market_vault_pda);
    println!("  ProtocolVault PDA: {}", protocol_vault_pda);
    println!("  Balance PDA: {}", balance_pda);

    // Build PlaceBetBlind instruction
    println!("{}", "Building PlaceBetBlind instruction...".cyan());

    let accounts = vec![
        solana_sdk::instruction::AccountMeta::new(user, true),
        solana_sdk::instruction::AccountMeta::new(balance_pda, false),
        solana_sdk::instruction::AccountMeta::new(market_v3_pda, false),
        solana_sdk::instruction::AccountMeta::new(position_v3_pda, false),
        solana_sdk::instruction::AccountMeta::new(protocol_vault_pda, false),
        solana_sdk::instruction::AccountMeta::new(market_vault_pda, false),
        solana_sdk::instruction::AccountMeta::new_readonly(zk_program_id, false),
        solana_sdk::instruction::AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        solana_sdk::instruction::AccountMeta::new_readonly(solana_sdk::sysvar::clock::id(), false),
    ];

    let instruction = solana_sdk::instruction::Instruction {
        program_id,
        accounts,
        data: instruction_bytes,
    };

    // Create and send transaction
    use solana_sdk::transaction::Transaction;
    use solana_client::rpc_client::RpcClient;

    let rpc_client = RpcClient::new(&args.rpc_url);
    let blockhash = rpc_client.get_latest_blockhash()
        .context("Failed to get latest blockhash")?;

    let mut tx = Transaction::new_with_payer(&[instruction], Some(&user));
    tx.sign(&[&keypair], blockhash);

    println!("{}", "Sending transaction to Solana...".cyan());
    let signature = rpc_client.send_and_confirm_transaction(&tx)
        .context("Failed to send transaction")?;

    println!();
    println!("{}", "Blind bet placed successfully!".green().bold());
    println!();
    println!("TX Signature: {}", signature.to_string().yellow().bold());
    println!("Market ID: {}", args.market_id);
    println!("Position PDA: {}", position_v3_pda);
    println!("Bet ciphertext hash: {}", hex::encode(&bet_ciphertext_hash));
    println!();

    // Save bet witness for later claim (includes pool state for multi-bet sync)
    let bet_witness = json!({
        "market_id": args.market_id,
        "secret": hex::encode(secret),
        "bet_amount": args.amount,
        "bet_side": bet_side_u8,
        "bet_secret_commitment": hex::encode(bet_secret_commitment),
        "bet_ciphertext_hash": hex::encode(bet_ciphertext_hash),
        "blinding_after": hex::encode(blinding_after),
        // Pool state after this bet (for next bet sync)
        "pool_yes_after": pool_yes_after,
        "pool_no_after": pool_no_after,
        "pool_commitment_after": hex::encode(pool_commitment_after),
    });

    fs::write(&args.witness_output, serde_json::to_string_pretty(&bet_witness)?)?;
    println!("{}", "Bet witness saved:".green());
    println!("  {:?}", args.witness_output);

    // Save local pool state for subsequent bets on this market
    let new_pool_state = LocalMarketPoolState {
        market_id: args.market_id,
        pool_yes: pool_yes_after,
        pool_no: pool_no_after,
        blinding: hex::encode(blinding_after),
        pool_commitment: hex::encode(pool_commitment_after),
    };
    new_pool_state.save()?;
    println!("{}", "Local pool state saved for future bets.".green());

    Ok(())
}

/// Generate ClaimBlind proof using snarkjs
/// Circuit inputs (from claim_blind.circom):
///   Public: bet_secret_commitment, winning_pool, losing_pool, outcome, claimed_payout, market_id
///   Private: bet_amount, bet_side, bet_blinding
fn generate_claim_blind_proof(
    wasm_path: &PathBuf,
    zkey_path: &PathBuf,
    market_id: u64,
    bet_amount: u64,
    bet_side: u8,
    bet_blinding: &[u8; 32],
    payout_amount: u64,
    winning_pool: u64,
    losing_pool: u64,
    outcome: u8, // 0=YES won, 1=NO won (matches bet_side encoding)
) -> Result<(Vec<u8>, Vec<u8>)> {
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    // Create temp directory
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("claim_blind_{}", nanos));
    fs::create_dir_all(&temp_dir)?;

    let input_path = temp_dir.join("input.json");
    let proof_path = temp_dir.join("proof.json");
    let public_path = temp_dir.join("public.json");

    // Calculate bet_secret_commitment = Poseidon(bet_amount, bet_side, bet_blinding)
    // using circomlibjs via Node.js for circuit compatibility
    let blinding_decimal = num_bigint::BigUint::from_bytes_be(bet_blinding).to_string();
    let bet_secret_commitment = calculate_poseidon_commitment_js(
        bet_amount,
        bet_side as u64,
        &blinding_decimal,
    )?;
    let bet_secret_commitment_decimal = num_bigint::BigUint::from_bytes_be(&bet_secret_commitment).to_string();

    // Build witness JSON for ClaimBlind circuit
    // Names must match exactly what the circuit expects
    let input_json = serde_json::json!({
        // Public inputs (6)
        "bet_secret_commitment": bet_secret_commitment_decimal,
        "winning_pool": winning_pool.to_string(),
        "losing_pool": losing_pool.to_string(),
        "outcome": outcome.to_string(),
        "claimed_payout": payout_amount.to_string(),
        "market_id": market_id.to_string(),
        // Private inputs (3)
        "bet_amount": bet_amount.to_string(),
        "bet_side": bet_side.to_string(),
        "bet_blinding": blinding_decimal,
    });

    fs::write(&input_path, serde_json::to_string_pretty(&input_json)?)?;

    println!("  Running snarkjs groth16 fullprove (ClaimBlind)...");

    // Run snarkjs groth16 fullprove
    let output = Command::new("npx")
        .args([
            "snarkjs",
            "groth16",
            "fullprove",
            input_path.to_str().unwrap(),
            wasm_path.to_str().unwrap(),
            zkey_path.to_str().unwrap(),
            proof_path.to_str().unwrap(),
            public_path.to_str().unwrap(),
        ])
        .output()
        .with_context(|| "Failed to run snarkjs")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        anyhow::bail!("snarkjs failed:\nstderr: {}\nstdout: {}", stderr, stdout);
    }

    // Parse proof.json
    let proof_content = fs::read_to_string(&proof_path)?;
    let proof_json: serde_json::Value = serde_json::from_str(&proof_content)?;

    // Convert proof to 256 bytes (Groth16 format with A negation)
    let proof_bytes = proof_json_to_bytes(&proof_json)?;

    // Build public_inputs for on-chain verification
    // Format (72 bytes):
    // 0-32: bet_secret_commitment
    // 32-40: winning_pool (8 bytes)
    // 40-48: losing_pool (8 bytes)
    // 48-49: outcome (1 byte)
    // 49-57: claimed_payout (8 bytes)
    // 57-65: market_id (8 bytes)
    // 65-72: padding (7 bytes)
    let mut public_inputs = Vec::with_capacity(72);
    public_inputs.extend_from_slice(&bet_secret_commitment);
    public_inputs.extend_from_slice(&winning_pool.to_le_bytes());
    public_inputs.extend_from_slice(&losing_pool.to_le_bytes());
    public_inputs.push(outcome);
    public_inputs.extend_from_slice(&payout_amount.to_le_bytes());
    public_inputs.extend_from_slice(&market_id.to_le_bytes());
    public_inputs.extend_from_slice(&[0u8; 7]); // padding to 72 bytes

    // Cleanup temp dir
    let _ = fs::remove_dir_all(&temp_dir);

    println!("  ZK proof generated successfully (ClaimBlind)!");

    Ok((proof_bytes, public_inputs))
}

#[tokio::main]
async fn settle_market_v3(args: SettleV3Args) -> Result<()> {
    use solana_sdk::signature::{read_keypair_file, Signer};
    use solana_sdk::transaction::Transaction;
    use solana_client::rpc_client::RpcClient;
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

    println!("{}", "Settling market V3...".cyan().bold());
    println!();
    println!("Market ID: {}", args.market_id);
    println!("Outcome: {}", match args.outcome {
        BetSide::Yes => "YES",
        BetSide::No => "NO",
    });
    println!("Decrypted pools: YES={}, NO={}", args.decrypted_yes, args.decrypted_no);
    println!();

    // Load oracle keypair
    let keypair = read_keypair_file(&args.keypair)
        .map_err(|e| anyhow::anyhow!("Failed to load keypair from {:?}: {}", args.keypair, e))?;

    // Load threshold signatures
    println!("{}", "Loading threshold signatures...".cyan());
    let threshold_sigs: Vec<[u8; 64]> = if args.threshold_sigs.exists() {
        let sigs_json = fs::read_to_string(&args.threshold_sigs)?;
        let sigs_array: Vec<String> = serde_json::from_str(&sigs_json)?;

        sigs_array.iter().map(|sig_b64| {
            let sig_bytes = BASE64.decode(sig_b64)
                .context("Failed to decode signature from base64")?;
            if sig_bytes.len() != 64 {
                anyhow::bail!("Invalid signature length: expected 64, got {}", sig_bytes.len());
            }
            let mut sig = [0u8; 64];
            sig.copy_from_slice(&sig_bytes);
            Ok(sig)
        }).collect::<Result<Vec<_>>>()?
    } else {
        println!("  WARNING: Using mock threshold signatures (file not found)");
        vec![[0u8; 64]; 3] // Mock: 3 signatures of zeros
    };

    println!("  Loaded {} threshold signatures", threshold_sigs.len());

    // Build SettleMarketV3 instruction
    println!("{}", "Building transaction...".cyan());

    let instruction_data = FutarchyInstructionV3::SettleMarketV3 {
        market_id: args.market_id,
        outcome: args.outcome == BetSide::Yes,
        decrypted_pool_yes: args.decrypted_yes,
        decrypted_pool_no: args.decrypted_no,
        threshold_signatures: threshold_sigs,
    };

    let instruction_bytes = instruction_data.pack()?;

    // Derive PDAs
    println!("{}", "Deriving PDAs...".cyan());

    let program_id = get_futarchy_program_id()?;
    let oracle = keypair.pubkey();
    let market_id_bytes = args.market_id.to_le_bytes();

    // MarketV3 PDA: ["market_v3", market_id]
    let (market_v3_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[MARKET_V3_SEED, &market_id_bytes],
        &program_id,
    );

    println!("  MarketV3 PDA: {}", market_v3_pda);
    println!("  Oracle: {}", oracle);

    // Build instruction accounts
    let accounts = vec![
        solana_sdk::instruction::AccountMeta::new_readonly(oracle, true),
        solana_sdk::instruction::AccountMeta::new(market_v3_pda, false),
        solana_sdk::instruction::AccountMeta::new_readonly(solana_sdk::sysvar::clock::id(), false),
    ];

    let instruction = solana_sdk::instruction::Instruction {
        program_id,
        accounts,
        data: instruction_bytes,
    };

    // Send transaction
    let rpc_client = RpcClient::new(&args.rpc_url);
    let blockhash = rpc_client.get_latest_blockhash()
        .context("Failed to get latest blockhash")?;

    let mut tx = Transaction::new_with_payer(&[instruction], Some(&oracle));
    tx.sign(&[&keypair], blockhash);

    println!("{}", "Sending transaction to Solana...".cyan());
    let signature = rpc_client.send_and_confirm_transaction(&tx)
        .context("Failed to send transaction")?;

    println!();
    println!("{}", "Market settled successfully!".green().bold());
    println!();
    println!("TX Signature: {}", signature.to_string().yellow().bold());
    println!("Market ID: {}", args.market_id);
    println!("Outcome: {}", match args.outcome {
        BetSide::Yes => "YES",
        BetSide::No => "NO",
    });
    println!();

    Ok(())
}

#[tokio::main]
async fn claim_v3(args: ClaimV3Args) -> Result<()> {
    use solana_sdk::signature::{read_keypair_file, Signer};
    use solana_sdk::transaction::Transaction;
    use solana_client::rpc_client::RpcClient;

    println!("{}", "Claiming from market V3...".cyan().bold());
    println!();

    // Load keypair
    let keypair = read_keypair_file(&args.keypair)
        .map_err(|e| anyhow::anyhow!("Failed to load keypair from {:?}: {}", args.keypair, e))?;

    // Load witness file
    println!("{}", "Loading bet witness...".cyan());
    let witness_json = fs::read_to_string(&args.witness_file)
        .with_context(|| format!("Failed to read witness file {:?}", args.witness_file))?;
    let witness: serde_json::Value = serde_json::from_str(&witness_json)?;

    // Parse witness fields
    let market_id = witness["market_id"].as_u64()
        .ok_or_else(|| anyhow::anyhow!("Missing market_id in witness"))?;
    let bet_amount = witness["bet_amount"].as_u64()
        .ok_or_else(|| anyhow::anyhow!("Missing bet_amount in witness"))?;
    let bet_side_u8 = witness["bet_side"].as_u64()
        .ok_or_else(|| anyhow::anyhow!("Missing bet_side in witness"))? as u8;

    let secret_hex = witness["secret"].as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing secret in witness"))?;
    let bet_secret_commitment_hex = witness["bet_secret_commitment"].as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing bet_secret_commitment in witness"))?;
    let bet_ciphertext_hash_hex = witness["bet_ciphertext_hash"].as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing bet_ciphertext_hash in witness"))?;

    let secret = hex::decode(secret_hex)?.try_into()
        .map_err(|_| anyhow::anyhow!("Invalid secret length"))?;
    let bet_secret_commitment: [u8; 32] = hex::decode(bet_secret_commitment_hex)?.try_into()
        .map_err(|_| anyhow::anyhow!("Invalid bet_secret_commitment length"))?;
    let bet_ciphertext_hash: [u8; 32] = hex::decode(bet_ciphertext_hash_hex)?.try_into()
        .map_err(|_| anyhow::anyhow!("Invalid bet_ciphertext_hash length"))?;

    println!("  Market ID: {}", market_id);
    println!("  Bet amount: {}", bet_amount);
    println!("  Bet side: {}", if bet_side_u8 == 0 { "YES" } else { "NO" });
    println!("  Bet ciphertext hash: {}", hex::encode(&bet_ciphertext_hash));

    // Fetch market state to get decrypted pools and outcome
    println!("{}", "Fetching market state...".cyan());

    // Fetch actual market state from chain
    let market_state = fetch_market_v3_state(&args.rpc_url, market_id).await?;

    // Verify market is settled with decrypted pools
    let (decrypted_yes, decrypted_no) = match (market_state.decrypted_pool_yes, market_state.decrypted_pool_no) {
        (Some(yes), Some(no)) => (yes, no),
        _ => {
            anyhow::bail!("Market not settled or pools not yet decrypted. Wait for oracle threshold decryption.");
        }
    };

    let outcome_yes = match market_state.resolution {
        Some(outcome) => outcome,
        None => {
            anyhow::bail!("Market not yet resolved by oracle.");
        }
    };

    let (winning_pool, losing_pool) = if outcome_yes {
        (decrypted_yes, decrypted_no)
    } else {
        (decrypted_no, decrypted_yes)
    };

    println!("  Decrypted pools: YES={}, NO={}", decrypted_yes, decrypted_no);
    println!("  Outcome: {}", if outcome_yes { "YES" } else { "NO" });
    println!("  Winning pool: {}, Losing pool: {}", winning_pool, losing_pool);

    // outcome for circuit: 0=YES won, 1=NO won (matches bet_side encoding)
    let outcome_u8: u8 = if outcome_yes { 0 } else { 1 };

    // Calculate payout: bet_amount + (bet_amount * losing_pool / winning_pool)
    let payout_amount = if winning_pool > 0 {
        bet_amount + (bet_amount * losing_pool / winning_pool)
    } else {
        bet_amount // Just return bet if no winner (shouldn't happen)
    };

    println!("  Calculated payout: {} lamports", payout_amount);

    // Generate ZK proof
    println!("{}", "Generating ZK proof...".cyan());
    println!("  Circuit: ClaimBlind (circuit 51)");

    let (proof, public_inputs) = if let (Some(wasm), Some(zkey)) = (&args.circuit_wasm, &args.circuit_zkey) {
        if wasm.exists() && zkey.exists() {
            println!("  Using real Groth16 proof generation");
            generate_claim_blind_proof(
                wasm,
                zkey,
                market_id,
                bet_amount,
                bet_side_u8,
                &secret,  // bet_blinding
                payout_amount,
                winning_pool,
                losing_pool,
                outcome_u8,
            )?
        } else {
            println!("  WARNING: Circuit files not found, using mock proof");
            let proof = vec![0u8; 256];
            let mut public_inputs = Vec::with_capacity(72);
            public_inputs.extend_from_slice(&bet_secret_commitment);
            public_inputs.extend_from_slice(&winning_pool.to_le_bytes());
            public_inputs.extend_from_slice(&losing_pool.to_le_bytes());
            public_inputs.push(outcome_u8);
            public_inputs.extend_from_slice(&payout_amount.to_le_bytes());
            public_inputs.extend_from_slice(&market_id.to_le_bytes());
            public_inputs.extend_from_slice(&[0u8; 7]); // padding
            (proof, public_inputs)
        }
    } else {
        println!("  WARNING: No circuit paths provided, using mock proof");
        let proof = vec![0u8; 256];
        let mut public_inputs = Vec::with_capacity(72);
        public_inputs.extend_from_slice(&bet_secret_commitment);
        public_inputs.extend_from_slice(&winning_pool.to_le_bytes());
        public_inputs.extend_from_slice(&losing_pool.to_le_bytes());
        public_inputs.push(outcome_u8);
        public_inputs.extend_from_slice(&payout_amount.to_le_bytes());
        public_inputs.extend_from_slice(&market_id.to_le_bytes());
        public_inputs.extend_from_slice(&[0u8; 7]); // padding
        (proof, public_inputs)
    };

    println!("  Proof generated ({} bytes)", proof.len());

    // Fetch private balance state
    println!("{}", "Fetching private balance state...".cyan());
    let (balance_pda, balance_commitment, _balance_nonce) =
        fetch_private_balance_state(&args.rpc_url, &keypair.pubkey()).await?;

    // For simplicity, use same balance commitment (real impl should update it)
    let new_balance_commitment = balance_commitment;

    println!("  Balance PDA: {}", balance_pda);
    println!("  Balance commitment: {}", hex::encode(&balance_commitment));

    // Build ClaimV3 instruction
    println!("{}", "Building transaction...".cyan());

    let instruction_data = FutarchyInstructionV3::ClaimV3 {
        market_id,
        bet_ciphertext_hash,
        bet_secret_commitment,
        proof,
        public_inputs,
        new_balance_commitment,
        circuit_type: CIRCUIT_CLAIM_BLIND,
    };

    let instruction_bytes = instruction_data.pack()?;

    // Derive PDAs
    println!("{}", "Deriving PDAs...".cyan());

    let program_id = get_futarchy_program_id()?;
    let zk_program_id = get_zk_generator_program_id()?;
    let user = keypair.pubkey();
    let market_id_bytes = market_id.to_le_bytes();

    // Private balance PDA: ["private_balance", user]
    let (private_balance_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[PRIVATE_BALANCE_SEED, user.as_ref()],
        &program_id,
    );

    // MarketV3 PDA: ["market_v3", market_id]
    let (market_v3_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[MARKET_V3_SEED, &market_id_bytes],
        &program_id,
    );

    // PositionV3 PDA: ["position_v3", market_id, bet_ciphertext_hash]
    let (position_v3_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[POSITION_V3_SEED, &market_id_bytes, &bet_ciphertext_hash],
        &program_id,
    );

    // MarketVault PDA: ["market_vault", market_id]
    let (market_vault_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[MARKET_VAULT_SEED, &market_id_bytes],
        &program_id,
    );

    // ProtocolVault PDA: ["protocol_vault"]
    let (protocol_vault_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[PROTOCOL_VAULT_SEED],
        &program_id,
    );

    println!("  Private Balance PDA: {}", private_balance_pda);
    println!("  MarketV3 PDA: {}", market_v3_pda);
    println!("  PositionV3 PDA: {}", position_v3_pda);
    println!("  MarketVault PDA: {}", market_vault_pda);
    println!("  ProtocolVault PDA: {}", protocol_vault_pda);

    // Build instruction accounts
    let accounts = vec![
        solana_sdk::instruction::AccountMeta::new(user, true),
        solana_sdk::instruction::AccountMeta::new(private_balance_pda, false),
        solana_sdk::instruction::AccountMeta::new(market_v3_pda, false),
        solana_sdk::instruction::AccountMeta::new(position_v3_pda, false),
        solana_sdk::instruction::AccountMeta::new(market_vault_pda, false),
        solana_sdk::instruction::AccountMeta::new(protocol_vault_pda, false),
        solana_sdk::instruction::AccountMeta::new_readonly(zk_program_id, false),
        solana_sdk::instruction::AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        solana_sdk::instruction::AccountMeta::new_readonly(solana_sdk::sysvar::clock::id(), false),
    ];

    let instruction = solana_sdk::instruction::Instruction {
        program_id,
        accounts,
        data: instruction_bytes,
    };

    // Send transaction
    let rpc_client = RpcClient::new(&args.rpc_url);
    let blockhash = rpc_client.get_latest_blockhash()
        .context("Failed to get latest blockhash")?;

    let mut tx = Transaction::new_with_payer(&[instruction], Some(&user));
    tx.sign(&[&keypair], blockhash);

    println!("{}", "Sending transaction to Solana...".cyan());
    let signature = rpc_client.send_and_confirm_transaction(&tx)
        .context("Failed to send transaction")?;

    println!();
    println!("{}", "Claim successful!".green().bold());
    println!();
    println!("TX Signature: {}", signature.to_string().yellow().bold());
    println!("Market ID: {}", market_id);
    println!("Payout: {} lamports", payout_amount);
    println!();

    Ok(())
}
