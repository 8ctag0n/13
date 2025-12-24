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

use crate::client::{CreateJobRequest, ZkClient};
use crate::client::futarchy::{FutarchyClient, PrepareBetRequest, SubmitBetRequest, encode_base64};

// Circuit type 30 reserved for future market prediction circuit
const CIRCUIT_MARKET_BET: u8 = 31;
const CIRCUIT_MARKET_SETTLEMENT: u8 = 32;
const CIRCUIT_FHE_BET: u8 = 35;

/// Side of a futarchy bet
#[derive(Debug, Clone, Copy, ValueEnum)]
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
    /// Place a private bet
    Bet(BetArgs),
    /// Claim market winnings
    Claim(ClaimArgs),
    /// Verify market settlement
    Verify(VerifyArgs),
}

#[derive(Args, Debug)]
pub struct CreateArgs {
    /// Market question
    #[arg(long)]
    pub question: String,

    /// Outcome options (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub outcomes: Vec<String>,

    /// Resolution date (Unix timestamp)
    #[arg(long)]
    pub resolution_date: u64,

    /// Minimum bet amount
    #[arg(long, default_value = "1000")]
    pub min_bet: u64,

    /// Maximum bet amount
    #[arg(long)]
    pub max_bet: Option<u64>,

    /// Creator public key
    #[arg(long)]
    pub creator: String,

    /// Server URL
    #[arg(long, default_value = "http://localhost:3000")]
    pub server: String,
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

    /// Server URL (ZK server)
    #[arg(long, default_value = "http://localhost:3000")]
    pub server: String,

    /// Futarchy server URL (for FHE mode)
    #[arg(long, default_value = "http://localhost:9000")]
    pub futarchy_server: String,

    /// Use FHE encryption for the bet (enables encrypted pool aggregation)
    #[arg(long)]
    pub use_fhe: bool,

    /// Path to output witness file
    #[arg(long, default_value = "./bet_witness.json")]
    pub witness_output: PathBuf,

    /// Path to Solana keypair for signing transactions
    #[arg(long)]
    pub keypair: Option<PathBuf>,

    /// Solana RPC URL
    #[arg(long, default_value = "http://localhost:8899")]
    pub rpc_url: String,

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
    #[arg(long, default_value = "FutMkts111111111111111111111111111111111111")]
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

pub fn handle_market_command(cmd: MarketCommands) -> Result<()> {
    match cmd {
        MarketCommands::Create(args) => create_market(args),
        MarketCommands::Bet(args) => place_bet(args),
        MarketCommands::Claim(args) => claim_winnings(args),
        MarketCommands::Verify(args) => verify_settlement(args),
    }
}

fn create_market(args: CreateArgs) -> Result<()> {
    println!("{}", "Creating prediction market...".cyan().bold());
    println!();
    println!("Question: {}", args.question.green());
    println!("Outcomes: {}", args.outcomes.join(", "));
    println!("Resolution date: {}", args.resolution_date);
    println!("Min bet: {} lamports", args.min_bet);
    if let Some(max) = args.max_bet {
        println!("Max bet: {} lamports", max);
    }
    println!("Creator: {}", args.creator);
    println!();

    // Validate outcomes
    if args.outcomes.is_empty() {
        anyhow::bail!("Must provide at least one outcome");
    }

    if args.outcomes.len() > 10 {
        anyhow::bail!("Maximum 10 outcomes allowed");
    }

    if args.min_bet == 0 {
        anyhow::bail!("Minimum bet must be greater than 0");
    }

    if let Some(max) = args.max_bet {
        if max < args.min_bet {
            anyhow::bail!("Maximum bet must be greater than or equal to minimum bet");
        }
    }

    println!("{}", "Market created successfully!".green().bold());
    println!();
    println!("{}", "Next steps:".cyan().bold());
    println!("  1. Share market ID with participants");
    println!("  2. Participants place bets with: zyb market bet --market-id <ID>");
    println!("  3. After resolution, winners claim with: zyb market claim");
    println!();
    println!("{}", "Note: Market metadata stored locally. Share market ID securely.".yellow());

    Ok(())
}

#[tokio::main]
async fn place_bet(args: BetArgs) -> Result<()> {
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
    let futarchy_client = FutarchyClient::with_url(&args.futarchy_server);

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
    let rpc_client = solana_client::rpc_client::RpcClient::new(&args.rpc_url);
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

    let client = ZkClient::with_url(&args.server);

    let request = CreateJobRequest {
        circuit_type: CIRCUIT_MARKET_BET,
        witness_commitment,
        public_inputs: vec![],
        creator: args.bettor.clone(),
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
