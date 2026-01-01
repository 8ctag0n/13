use crate::{FutarchyError, Result};
use num_bigint::BigUint;
use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

/// Prepare claim data for a user's bet on a settled market.
///
/// This calculates:
/// - Whether the user won (based on their bet side vs resolution)
/// - The payout amount using the formula: bet_amount * total_pool / winning_pool
/// - The nullifier (placeholder - actual impl needs Poseidon)
///
/// Returns None if user didn't win or market not settled.
#[derive(Debug, Clone)]
pub struct ClaimPreparation {
    /// Whether the user's bet won
    pub is_winner: bool,
    /// Payout amount in lamports (0 if lost)
    pub payout_amount: u64,
    /// Nullifier hash (32 bytes)
    pub nullifier: [u8; 32],
    /// User's bet amount
    pub bet_amount: u64,
    /// The winning side (true = YES, false = NO)
    pub winning_side: bool,
}

/// Calculate payout for a winning bet.
///
/// Formula: payout = bet_amount * total_pool / winning_pool
///
/// This ensures winners split the entire pool proportionally.
pub fn calculate_payout(
    bet_amount: u64,
    total_yes_bets: u64,
    total_no_bets: u64,
    resolution: bool, // true = YES won
    bet_side: bool,   // true = user bet YES
) -> Option<u64> {
    if bet_side != resolution {
        return None;
    }

    let total_pool = total_yes_bets.saturating_add(total_no_bets);
    let winning_pool = if resolution { total_yes_bets } else { total_no_bets };

    if winning_pool == 0 {
        return None;
    }

    let payout = (bet_amount as u128)
        .checked_mul(total_pool as u128)?
        .checked_div(winning_pool as u128)?;

    Some(payout.min(u64::MAX as u128) as u64)
}

/// Generate a nullifier from user's secret and bet commitment.
///
/// Nullifier = Poseidon(secret, bet_commitment)
///
/// For MVP, this uses a simple hash. Production should use Poseidon.
pub fn generate_nullifier(secret: &[u8; 32], bet_commitment: &[u8; 32]) -> [u8; 32] {
    use solana_program::hash::hash;

    let mut data = Vec::with_capacity(64);
    data.extend_from_slice(secret);
    data.extend_from_slice(bet_commitment);

    hash(&data).to_bytes()
}

/// Prepare all data needed to claim payout.
///
/// This is a helper that combines:
/// 1. Check if user won
/// 2. Calculate payout amount
/// 3. Generate nullifier
pub fn prepare_claim(
    bet_amount: u64,
    bet_side: bool,
    bet_commitment: &[u8; 32],
    user_secret: &[u8; 32],
    total_yes_bets: u64,
    total_no_bets: u64,
    resolution: Option<bool>,
) -> Option<ClaimPreparation> {
    let winning_side = resolution?;
    let is_winner = bet_side == winning_side;

    let payout_amount = if is_winner {
        calculate_payout(bet_amount, total_yes_bets, total_no_bets, winning_side, bet_side)?
    } else {
        0
    };

    let nullifier = generate_nullifier(user_secret, bet_commitment);

    Some(ClaimPreparation {
        is_winner,
        payout_amount,
        nullifier,
        bet_amount,
        winning_side,
    })
}

/// Configuration for invoking snarkjs.
#[derive(Debug, Clone)]
pub struct SnarkjsCommand {
    pub command: String,
    pub args_prefix: Vec<String>,
}

impl SnarkjsCommand {
    pub fn direct() -> Self {
        Self {
            command: "snarkjs".to_string(),
            args_prefix: Vec::new(),
        }
    }

    pub fn npx() -> Self {
        Self {
            command: "npx".to_string(),
            args_prefix: vec!["snarkjs".to_string()],
        }
    }
}

/// Paths for circuit artifacts needed to generate a proof.
#[derive(Debug, Clone)]
pub struct MarketClaimProofPaths {
    pub wasm_path: PathBuf,
    pub zkey_path: PathBuf,
}

/// Inputs required to generate a MarketClaim proof.
#[derive(Debug, Clone)]
pub struct MarketClaimWitness {
    pub market_id: u64,
    pub nullifier: [u8; 32],
    pub payout_amount: u64,
    pub resolution: u8,
    pub total_pool: u64,
    pub winning_pool: u64,
    pub bet_commitment: [u8; 32],
    pub timestamp: i64,
    pub secret: [u8; 32],
    pub bet_amount: u64,
    pub bet_side: u8,
    pub blinding: [u8; 32],
}

/// Proof output for MarketClaim.
#[derive(Debug, Clone)]
pub struct MarketClaimProof {
    /// Groth16 proof bytes (256 bytes)
    pub proof: Vec<u8>,
    /// Public inputs bytes (105 bytes)
    pub public_inputs: Vec<u8>,
    /// Public signals from snarkjs (decimal strings)
    pub public_signals: Vec<String>,
}

/// Encode public inputs for Circuit 32 (MarketClaim).
pub fn encode_market_claim_public_inputs(
    market_id: u64,
    nullifier: [u8; 32],
    payout_amount: u64,
    resolution: u8,
    total_pool: u64,
    winning_pool: u64,
    bet_commitment: [u8; 32],
    timestamp: i64,
) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(105);
    bytes.extend_from_slice(&market_id.to_le_bytes());
    bytes.extend_from_slice(&nullifier);
    bytes.extend_from_slice(&payout_amount.to_le_bytes());
    bytes.push(resolution);
    bytes.extend_from_slice(&total_pool.to_le_bytes());
    bytes.extend_from_slice(&winning_pool.to_le_bytes());
    bytes.extend_from_slice(&bet_commitment);
    bytes.extend_from_slice(&timestamp.to_le_bytes());
    bytes
}

/// Generate a MarketClaim proof using snarkjs (Groth16 fullprove).
pub fn generate_market_claim_proof(
    snarkjs: &SnarkjsCommand,
    paths: &MarketClaimProofPaths,
    witness: &MarketClaimWitness,
) -> Result<MarketClaimProof> {
    let temp_dir = make_temp_dir("market_claim_proof")?;
    let input_path = temp_dir.join("input.json");
    let proof_path = temp_dir.join("proof.json");
    let public_path = temp_dir.join("public.json");

    let input_json = build_market_claim_witness_json(witness)?;
    fs::write(&input_path, serde_json::to_string_pretty(&input_json).map_err(map_json_err)?)?;

    let mut cmd = Command::new(&snarkjs.command);
    cmd.args(&snarkjs.args_prefix)
        .arg("groth16")
        .arg("fullprove")
        .arg(&input_path)
        .arg(&paths.wasm_path)
        .arg(&paths.zkey_path)
        .arg(&proof_path)
        .arg(&public_path);

    let output = cmd.output().map_err(map_io_err)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(FutarchyError::Custom(format!(
            "snarkjs failed: {stderr}\n{stdout}"
        )));
    }

    let proof_json: Value =
        serde_json::from_slice(&fs::read(&proof_path).map_err(map_io_err)?).map_err(map_json_err)?;
    let public_signals: Vec<String> =
        serde_json::from_slice(&fs::read(&public_path).map_err(map_io_err)?).map_err(map_json_err)?;

    let proof = proof_json_to_bytes(&proof_json)?;
    let public_inputs = encode_market_claim_public_inputs(
        witness.market_id,
        witness.nullifier,
        witness.payout_amount,
        witness.resolution,
        witness.total_pool,
        witness.winning_pool,
        witness.bet_commitment,
        witness.timestamp,
    );

    Ok(MarketClaimProof {
        proof,
        public_inputs,
        public_signals,
    })
}

fn build_market_claim_witness_json(witness: &MarketClaimWitness) -> Result<Value> {
    let json = serde_json::json!({
        "market_id": witness.market_id.to_string(),
        "nullifier": bytes_to_dec(&witness.nullifier),
        "payout_amount": witness.payout_amount.to_string(),
        "resolution": witness.resolution.to_string(),
        "total_pool": witness.total_pool.to_string(),
        "winning_pool": witness.winning_pool.to_string(),
        "bet_commitment": bytes_to_dec(&witness.bet_commitment),
        "timestamp": witness.timestamp.to_string(),
        "secret": bytes_to_dec(&witness.secret),
        "bet_amount": witness.bet_amount.to_string(),
        "bet_side": witness.bet_side.to_string(),
        "blinding": bytes_to_dec(&witness.blinding),
    });
    Ok(json)
}

fn proof_json_to_bytes(proof: &Value) -> Result<Vec<u8>> {
    let pi_a = proof
        .get("pi_a")
        .and_then(|v| v.as_array())
        .ok_or_else(|| FutarchyError::Custom("missing pi_a".to_string()))?;
    let pi_b = proof
        .get("pi_b")
        .and_then(|v| v.as_array())
        .ok_or_else(|| FutarchyError::Custom("missing pi_b".to_string()))?;
    let pi_c = proof
        .get("pi_c")
        .and_then(|v| v.as_array())
        .ok_or_else(|| FutarchyError::Custom("missing pi_c".to_string()))?;

    let ax = dec_value(&pi_a, 0)?;
    let ay = dec_value(&pi_a, 1)?;

    let bx = pi_b
        .get(0)
        .and_then(|v| v.as_array())
        .ok_or_else(|| FutarchyError::Custom("missing pi_b[0]".to_string()))?;
    let by = pi_b
        .get(1)
        .and_then(|v| v.as_array())
        .ok_or_else(|| FutarchyError::Custom("missing pi_b[1]".to_string()))?;
    let bx0 = dec_value(bx, 0)?;
    let bx1 = dec_value(bx, 1)?;
    let by0 = dec_value(by, 0)?;
    let by1 = dec_value(by, 1)?;

    let cx = dec_value(&pi_c, 0)?;
    let cy = dec_value(&pi_c, 1)?;

    let mut bytes = Vec::with_capacity(256);
    bytes.extend_from_slice(&biguint_to_32be(&ax));
    bytes.extend_from_slice(&biguint_to_32be(&ay));
    bytes.extend_from_slice(&biguint_to_32be(&bx0));
    bytes.extend_from_slice(&biguint_to_32be(&bx1));
    bytes.extend_from_slice(&biguint_to_32be(&by0));
    bytes.extend_from_slice(&biguint_to_32be(&by1));
    bytes.extend_from_slice(&biguint_to_32be(&cx));
    bytes.extend_from_slice(&biguint_to_32be(&cy));
    Ok(bytes)
}

fn dec_value(arr: &[Value], idx: usize) -> Result<BigUint> {
    let s = arr
        .get(idx)
        .and_then(|v| v.as_str())
        .ok_or_else(|| FutarchyError::Custom(format!("missing decimal at index {idx}")))?;
    BigUint::parse_bytes(s.as_bytes(), 10)
        .ok_or_else(|| FutarchyError::Custom(format!("invalid decimal value: {s}")))
}

fn biguint_to_32be(value: &BigUint) -> [u8; 32] {
    let mut out = [0u8; 32];
    let bytes = value.to_bytes_be();
    let start = 32usize.saturating_sub(bytes.len());
    out[start..].copy_from_slice(&bytes);
    out
}

fn bytes_to_dec(bytes: &[u8; 32]) -> String {
    BigUint::from_bytes_be(bytes).to_str_radix(10)
}

fn make_temp_dir(prefix: &str) -> Result<PathBuf> {
    let mut dir = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(map_time_err)?
        .as_nanos();
    dir.push(format!("{prefix}_{nanos}"));
    fs::create_dir_all(&dir).map_err(map_io_err)?;
    Ok(dir)
}

fn map_io_err(err: std::io::Error) -> FutarchyError {
    FutarchyError::SerializationError(err)
}

fn map_json_err(err: serde_json::Error) -> FutarchyError {
    FutarchyError::Custom(err.to_string())
}

fn map_time_err(err: std::time::SystemTimeError) -> FutarchyError {
    FutarchyError::Custom(err.to_string())
}
