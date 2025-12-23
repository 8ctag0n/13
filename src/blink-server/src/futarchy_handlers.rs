//! Futarchy Prediction Markets API Handlers
//!
//! REST API endpoints for futarchy markets with FHE-encrypted betting

use actix_web::{get, post, web, HttpResponse, Responder};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha3::{Digest, Keccak256};
use sqlx::PgPool;
use chrono::{Duration, Utc};
use solana_sdk::{
    pubkey::Pubkey,
    transaction::Transaction,
};
use solana_client::rpc_client::RpcClient;
use std::str::FromStr;
use futarchy_sdk::{find_market_pda, find_escrow_pda, FheAccounts};

use crate::db::futarchy_queries::{
    CreateCiphertextData, CreateMarketData, CreatePositionData, FutarchyQueries,
    FheJobWithCiphertexts,
};
use crate::AppState;

// =============================================================================
// DTOs - Request/Response Types
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateMarketRequest {
    /// The question/proposal for the market
    pub question: String,
    /// Creator's Solana pubkey
    pub creator: String,
    /// Oracle pubkey (who can settle)
    pub oracle: String,
    /// Resolution window in seconds (default: 86400 = 1 day)
    #[serde(default = "default_resolution_window")]
    pub resolution_window_secs: i64,
    /// Max bet amount in lamports (default: 1 SOL)
    #[serde(default = "default_max_bet")]
    pub max_bet_lamports: i64,
    /// Optional: when the market ends (ISO 8601)
    pub ends_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ValidateCreateMarketRequest {
    /// Unique market ID (u64) used for PDA derivation
    pub market_id: u64,
    /// The question/proposal for the market
    pub question: String,
    /// Creator's Solana pubkey
    pub creator: String,
    /// Oracle pubkey (who can settle)
    pub oracle: String,
    /// Resolution window in seconds (default: 86400 = 1 day)
    #[serde(default = "default_resolution_window")]
    pub resolution_window_secs: i64,
    /// Max bet amount in lamports (default: 1 SOL)
    #[serde(default = "default_max_bet")]
    pub max_bet_lamports: i64,
    /// Optional: when the market ends (ISO 8601)
    pub ends_at: Option<String>,
}

fn default_resolution_window() -> i64 { 86400 }
fn default_max_bet() -> i64 { 1_000_000_000 }

#[derive(Debug, Serialize)]
pub struct MarketResponse {
    pub id: String,
    pub question: String,
    pub question_hash: String,
    pub creator: String,
    pub oracle: String,
    pub status: String,
    pub yes_pool_lamports: i64,
    pub no_pool_lamports: i64,
    pub outcome: Option<bool>,
    pub created_at: String,
    pub ends_at: Option<String>,
    pub settled_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PlaceBetRequest {
    /// Bettor's Solana pubkey
    pub bettor: String,
    /// Side: "yes" or "no"
    pub side: String,
    /// Amount in lamports (plaintext for now)
    pub amount_lamports: i64,
    /// Optional: base64-encoded FHE ciphertext of the amount
    pub encrypted_amount: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PlaceBetResponse {
    pub position_id: i32,
    pub market_id: String,
    pub bettor: String,
    pub side: String,
    pub amount_lamports: i64,
    pub encrypted_amount_hash: Option<String>,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct SettleMarketRequest {
    /// Oracle pubkey (must match market's oracle)
    pub oracle: String,
    /// Outcome: true = YES won, false = NO won
    pub outcome: bool,
    /// Optional: signature proving oracle authorization
    pub signature: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListMarketsQuery {
    pub status: Option<String>,
    pub creator: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePoolCiphertextRequest {
    /// Base64-encoded new pool ciphertext
    pub ciphertext: String,
    /// Who is updating (prover pubkey)
    pub updated_by: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PrepareBetRequest {
    pub market_id: String,
    pub bettor: String,
    pub side: bool,
    pub amount_lamports: u64,
    pub ciphertext_hash: String,
    pub proof: String,
    pub public_inputs: String,
    pub circuit_type: u8,
}

#[derive(Debug, Serialize)]
pub struct PrepareBetResponse {
    pub unsigned_transaction: String,
    pub market_id: String,
    pub signers: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitBetRequest {
    pub signed_tx: String,
    pub ciphertext: String,
    pub server_key: Option<String>,
    pub market_id: Option<u64>,
    pub side: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum SideInput {
    Bool(bool),
    String(String),
}

#[derive(Debug, Deserialize)]
pub struct ValidateBetRequest {
    pub bettor: String,
    pub side: SideInput,
    pub amount_lamports: u64,
    pub ciphertext_hash: Option<String>,
    pub bet_commitment: Option<String>,
    pub encrypted_amount: Option<String>,
    pub proof: String,
    pub public_inputs: String,
    pub circuit_type: u8,
    pub fhe_job_id: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct ValidateClaimRequest {
    pub user: String,
    pub claim_nullifier: String,
    pub bet_commitment: String,
    pub proof: String,
    pub public_inputs: String,
    pub payout_amount: u64,
    pub include_position: Option<bool>,
}

fn get_futarchy_program_id() -> Result<Pubkey, HttpResponse> {
    let futarchy_program_id_str = std::env::var("FUTARCHY_PROGRAM_ID")
        .unwrap_or_else(|_| "AQUUuRSwDhB1eeC2Caa8GPVGV4YzZkJ1YiSvZd3BBPij".to_string());
    Pubkey::from_str(&futarchy_program_id_str).map_err(|_| {
        HttpResponse::InternalServerError().json(json!({
            "error": format!("Invalid FUTARCHY_PROGRAM_ID: {}", futarchy_program_id_str),
        }))
    })
}

fn get_zk_generator_program_id() -> Result<Pubkey, HttpResponse> {
    let zk_generator_id = std::env::var("ZK_GENERATOR_PROGRAM_ID")
        .unwrap_or_else(|_| "Dzvy1pzCBgtMw5Fte2GybpeN2PsLPW8t7zDvLfKpxnSS".to_string());
    Pubkey::from_str(&zk_generator_id).map_err(|_| {
        HttpResponse::InternalServerError().json(json!({
            "error": format!("Invalid ZK_GENERATOR_PROGRAM_ID: {}", zk_generator_id),
        }))
    })
}

fn get_fhe_generator_program_id() -> Result<Pubkey, HttpResponse> {
    let fhe_generator_id = std::env::var("FHE_GENERATOR_PROGRAM_ID")
        .unwrap_or_else(|_| "C8PpHFCKZ4F2Szbir2EMS4S4H3mwQqHNWUXK1N21nfAB".to_string());
    Pubkey::from_str(&fhe_generator_id).map_err(|_| {
        HttpResponse::InternalServerError().json(json!({
            "error": format!("Invalid FHE_GENERATOR_PROGRAM_ID: {}", fhe_generator_id),
        }))
    })
}

fn parse_side(input: &SideInput) -> Result<bool, HttpResponse> {
    match input {
        SideInput::Bool(value) => Ok(*value),
        SideInput::String(value) => match value.to_lowercase().as_str() {
            "yes" | "true" | "1" => Ok(true),
            "no" | "false" | "0" => Ok(false),
            _ => Err(HttpResponse::BadRequest().json(json!({
                "error": "Invalid side. Use 'yes' or 'no'",
            }))),
        },
    }
}

fn hex_to_32_bytes(value: &str, field: &str) -> Result<[u8; 32], HttpResponse> {
    let bytes = hex::decode(value).map_err(|e| {
        HttpResponse::BadRequest().json(json!({
            "error": format!("Invalid {} hex: {}", field, e),
        }))
    })?;
    if bytes.len() != 32 {
        return Err(HttpResponse::BadRequest().json(json!({
            "error": format!("{} must be 32 bytes (64 hex chars)", field),
        })));
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

fn derive_fhe_accounts(
    fhe_program_id: &Pubkey,
    market_pda: &Pubkey,
    job_id: u64,
) -> FheAccounts {
    let job_id_bytes = job_id.to_le_bytes();
    let (fhe_job, _) = Pubkey::find_program_address(
        &[b"fhe_job", market_pda.as_ref(), &job_id_bytes],
        fhe_program_id,
    );
    let (fhe_consensus, _) =
        Pubkey::find_program_address(&[b"fhe_consensus", &job_id_bytes], fhe_program_id);
    let (fhe_escrow, _) =
        Pubkey::find_program_address(&[b"fhe_escrow", fhe_job.as_ref()], fhe_program_id);

    FheAccounts {
        fhe_job,
        fhe_consensus,
        fhe_escrow,
        fhe_generator_program: *fhe_program_id,
    }
}

fn parse_market_id(value: &str) -> Result<u64, HttpResponse> {
    value.parse::<u64>().map_err(|_| {
        HttpResponse::BadRequest().json(json!({
            "error": "market_id must be a valid u64",
        }))
    })
}

// =============================================================================
// Market Endpoints
// =============================================================================

/// GET /api/futarchy/markets
/// List all markets with optional filtering
#[get("/api/futarchy/markets")]
pub async fn list_markets(
    pool: web::Data<PgPool>,
    query: web::Query<ListMarketsQuery>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    match FutarchyQueries::list_markets(
        &pool,
        query.status.as_deref(),
        query.creator.as_deref(),
        limit,
        offset,
    ).await {
        Ok(markets) => {
            let responses: Vec<MarketResponse> = markets
                .into_iter()
                .map(|m| MarketResponse {
                    id: m.id,
                    question: m.question,
                    question_hash: m.question_hash,
                    creator: m.creator,
                    oracle: m.oracle,
                    status: m.status,
                    yes_pool_lamports: m.yes_pool_lamports,
                    no_pool_lamports: m.no_pool_lamports,
                    outcome: m.outcome,
                    created_at: m.created_at.to_rfc3339(),
                    ends_at: m.ends_at.map(|t| t.to_rfc3339()),
                    settled_at: m.settled_at.map(|t| t.to_rfc3339()),
                })
                .collect();

            HttpResponse::Ok().json(json!({
                "markets": responses,
                "count": responses.len(),
                "limit": limit,
                "offset": offset,
            }))
        }
        Err(e) => {
            log::error!("Failed to list markets: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to list markets: {}", e),
            }))
        }
    }
}

/// GET /api/futarchy/markets/{id}
/// Get a specific market by ID
#[get("/api/futarchy/markets/{id}")]
pub async fn get_market(
    pool: web::Data<PgPool>,
    path: web::Path<String>,
) -> impl Responder {
    let market_id = path.into_inner();

    match FutarchyQueries::get_market(&pool, &market_id).await {
        Ok(Some(m)) => {
            let response = MarketResponse {
                id: m.id,
                question: m.question,
                question_hash: m.question_hash,
                creator: m.creator,
                oracle: m.oracle,
                status: m.status,
                yes_pool_lamports: m.yes_pool_lamports,
                no_pool_lamports: m.no_pool_lamports,
                outcome: m.outcome,
                created_at: m.created_at.to_rfc3339(),
                ends_at: m.ends_at.map(|t| t.to_rfc3339()),
                settled_at: m.settled_at.map(|t| t.to_rfc3339()),
            };
            HttpResponse::Ok().json(json!({ "market": response }))
        }
        Ok(None) => {
            HttpResponse::NotFound().json(json!({
                "error": "Market not found",
                "market_id": market_id,
            }))
        }
        Err(e) => {
            log::error!("Failed to get market {}: {}", market_id, e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to fetch market: {}", e),
            }))
        }
    }
}

/// POST /api/futarchy/markets
/// Create a new prediction market
#[post("/api/futarchy/markets")]
pub async fn create_market(
    pool: web::Data<PgPool>,
    body: web::Json<CreateMarketRequest>,
) -> impl Responder {
    // Hash the question for on-chain storage
    let question_hash = {
        let mut hasher = Keccak256::new();
        hasher.update(body.question.as_bytes());
        hex::encode(hasher.finalize())
    };

    // Generate market ID (in real impl, this would be PDA derivation)
    let market_id = {
        let mut hasher = Keccak256::new();
        hasher.update(&body.creator);
        hasher.update(&question_hash);
        hasher.update(&Utc::now().timestamp().to_le_bytes());
        format!("{}", &hex::encode(hasher.finalize())[..32])
    };

    // Parse ends_at if provided
    let ends_at = body.ends_at.as_ref().and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    }).or_else(|| {
        // Default: ends in resolution_window_secs
        Some(Utc::now() + Duration::seconds(body.resolution_window_secs))
    });

    let data = CreateMarketData {
        id: market_id.clone(),
        question: body.question.clone(),
        question_hash: question_hash.clone(),
        creator: body.creator.clone(),
        oracle: body.oracle.clone(),
        resolution_window_secs: body.resolution_window_secs,
        max_bet_lamports: body.max_bet_lamports,
        ends_at,
        tx_signature: None, // TODO: Build and send on-chain tx
        slot: None,
    };

    match FutarchyQueries::create_market(&pool, &data).await {
        Ok(m) => {
            log::info!("Created futarchy market: {} - {}", m.id, m.question);

            let response = MarketResponse {
                id: m.id,
                question: m.question,
                question_hash: m.question_hash,
                creator: m.creator,
                oracle: m.oracle,
                status: m.status,
                yes_pool_lamports: m.yes_pool_lamports,
                no_pool_lamports: m.no_pool_lamports,
                outcome: m.outcome,
                created_at: m.created_at.to_rfc3339(),
                ends_at: m.ends_at.map(|t| t.to_rfc3339()),
                settled_at: m.settled_at.map(|t| t.to_rfc3339()),
            };

            HttpResponse::Created().json(json!({
                "market": response,
                "message": "Market created successfully",
            }))
        }
        Err(e) => {
            log::error!("Failed to create market: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to create market: {}", e),
            }))
        }
    }
}

/// POST /api/futarchy/markets/validate-and-build
/// Build unsigned transaction for CreateMarket
#[post("/api/futarchy/markets/validate-and-build")]
pub async fn validate_create_market(
    body: web::Json<ValidateCreateMarketRequest>,
) -> impl Responder {
    let creator = match Pubkey::from_str(&body.creator) {
        Ok(pk) => pk,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Invalid creator pubkey: {}", e),
            }));
        }
    };

    let oracle = match Pubkey::from_str(&body.oracle) {
        Ok(pk) => pk,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Invalid oracle pubkey: {}", e),
            }));
        }
    };

    let question_hash_bytes = {
        let mut hasher = Keccak256::new();
        hasher.update(body.question.as_bytes());
        let hash = hasher.finalize();
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&hash);
        arr
    };

    let end_time = body
        .ends_at
        .as_ref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.timestamp())
        .unwrap_or_else(|| (Utc::now() + Duration::seconds(body.resolution_window_secs)).timestamp());

    let futarchy_program_id = match get_futarchy_program_id() {
        Ok(pk) => pk,
        Err(resp) => return resp,
    };

    let instruction = match futarchy_sdk::build_create_market_ix(
        &futarchy_program_id,
        &creator,
        body.market_id,
        question_hash_bytes,
        &oracle,
        end_time,
        body.max_bet_lamports as u64,
    ) {
        Ok(ix) => ix,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to build instruction: {}", e),
            }));
        }
    };

    let unsigned_tx = match futarchy_sdk::prepare_unsigned_transaction(&[instruction], &creator) {
        Ok(tx) => tx,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to prepare transaction: {}", e),
            }));
        }
    };

    HttpResponse::Ok().json(json!({
        "unsigned_transaction": unsigned_tx,
        "market_id": body.market_id,
        "signers": vec![body.creator.clone()],
    }))
}

/// POST /api/futarchy/markets/{id}/bet
/// Place a bet on a market
#[post("/api/futarchy/markets/{id}/bet")]
pub async fn place_bet(
    pool: web::Data<PgPool>,
    path: web::Path<String>,
    body: web::Json<PlaceBetRequest>,
) -> impl Responder {
    let market_id = path.into_inner();

    // Validate market exists and is active
    let market = match FutarchyQueries::get_market(&pool, &market_id).await {
        Ok(Some(m)) => m,
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({
                "error": "Market not found",
                "market_id": market_id,
            }));
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to fetch market: {}", e),
            }));
        }
    };

    if market.status != "active" {
        return HttpResponse::BadRequest().json(json!({
            "error": "Market is not active",
            "status": market.status,
        }));
    }

    // Validate bet amount
    if body.amount_lamports > market.max_bet_lamports {
        return HttpResponse::BadRequest().json(json!({
            "error": "Bet amount exceeds maximum",
            "max_bet_lamports": market.max_bet_lamports,
            "requested": body.amount_lamports,
        }));
    }

    // Parse side
    let side = match body.side.to_lowercase().as_str() {
        "yes" | "true" | "1" => true,
        "no" | "false" | "0" => false,
        _ => {
            return HttpResponse::BadRequest().json(json!({
                "error": "Invalid side. Use 'yes' or 'no'",
            }));
        }
    };

    // Handle encrypted amount if provided
    let encrypted_amount_hash = if let Some(ref encrypted) = body.encrypted_amount {
        match BASE64.decode(encrypted) {
            Ok(ciphertext_bytes) => {
                let hash = {
                    let mut hasher = Keccak256::new();
                    hasher.update(&ciphertext_bytes);
                    hex::encode(hasher.finalize())
                };

                // Store ciphertext
                let ct_data = CreateCiphertextData {
                    hash: hash.clone(),
                    ciphertext: ciphertext_bytes,
                    ciphertext_type: "bet".to_string(),
                    market_id: Some(market_id.clone()),
                    side: Some(side),
                    version: 0,
                    created_by: Some(body.bettor.clone()),
                };

                if let Err(e) = FutarchyQueries::store_ciphertext(&pool, &ct_data).await {
                    log::error!("Failed to store ciphertext: {}", e);
                    return HttpResponse::InternalServerError().json(json!({
                        "error": format!("Failed to store encrypted amount: {}", e),
                    }));
                }

                // Create FHE job for pool update
                let pool_hash = FutarchyQueries::get_pool_ciphertext_hash(&pool, &market_id, side)
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| "0".repeat(64)); // Zero hash for first bet

                if let Err(e) = FutarchyQueries::create_fhe_job(
                    &pool,
                    &market_id,
                    side,
                    &pool_hash,
                    &hash,
                ).await {
                    log::warn!("Failed to create FHE job: {}", e);
                    // Continue anyway, job can be created later
                }

                Some(hash)
            }
            Err(e) => {
                return HttpResponse::BadRequest().json(json!({
                    "error": format!("Invalid base64 for encrypted_amount: {}", e),
                }));
            }
        }
    } else {
        None
    };

    // Create position
    let position_data = CreatePositionData {
        market_id: market_id.clone(),
        bettor: body.bettor.clone(),
        side,
        amount_lamports: body.amount_lamports,
        encrypted_amount_hash: encrypted_amount_hash.clone(),
        tx_signature: None, // TODO: Build and send on-chain tx
        slot: None,
    };

    match FutarchyQueries::create_position(&pool, &position_data).await {
        Ok(pos) => {
            log::info!(
                "Placed bet on market {}: {} {} lamports (side: {})",
                market_id, body.bettor, body.amount_lamports, if side { "YES" } else { "NO" }
            );

            // Update pool totals (plaintext aggregate for UI)
            let (new_yes, new_no) = if side {
                (market.yes_pool_lamports + body.amount_lamports, market.no_pool_lamports)
            } else {
                (market.yes_pool_lamports, market.no_pool_lamports + body.amount_lamports)
            };

            if let Err(e) = FutarchyQueries::update_pool_totals(&pool, &market_id, new_yes, new_no).await {
                log::warn!("Failed to update pool totals: {}", e);
            }

            HttpResponse::Created().json(json!({
                "bet": PlaceBetResponse {
                    position_id: pos.id,
                    market_id: pos.market_id,
                    bettor: pos.bettor,
                    side: if side { "yes".to_string() } else { "no".to_string() },
                    amount_lamports: pos.amount_lamports,
                    encrypted_amount_hash: pos.encrypted_amount_hash,
                    status: pos.status,
                },
                "message": "Bet placed successfully",
            }))
        }
        Err(e) => {
            log::error!("Failed to place bet: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to place bet: {}", e),
            }))
        }
    }
}

/// POST /api/futarchy/markets/{id}/bet/validate-and-build
/// Build unsigned transaction for PlaceBet
#[post("/api/futarchy/markets/{id}/bet/validate-and-build")]
pub async fn validate_place_bet(
    path: web::Path<String>,
    body: web::Json<ValidateBetRequest>,
) -> impl Responder {
    let market_ok = parse_market_id(&path.into_inner());
    let market_id = match market_ok {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let bettor = match Pubkey::from_str(&body.bettor) {
        Ok(pk) => pk,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Invalid bettor pubkey: {}", e),
            }));
        }
    };

    let side = match parse_side(&body.side) {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    let proof_bytes = match BASE64.decode(&body.proof) {
        Ok(bytes) => bytes,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Invalid proof base64: {}", e),
            }));
        }
    };

    let public_inputs_bytes = match BASE64.decode(&body.public_inputs) {
        Ok(bytes) => bytes,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Invalid public_inputs base64: {}", e),
            }));
        }
    };

    let encrypted_bet_amount = match &body.encrypted_amount {
        Some(value) => match BASE64.decode(value) {
            Ok(bytes) => Some(bytes),
            Err(e) => {
                return HttpResponse::BadRequest().json(json!({
                    "error": format!("Invalid encrypted_amount base64: {}", e),
                }));
            }
        },
        None => None,
    };

    let ciphertext_hash_bytes = if let Some(hash) = &body.ciphertext_hash {
        match hex_to_32_bytes(hash, "ciphertext_hash") {
            Ok(arr) => Some(arr),
            Err(resp) => return resp,
        }
    } else if let Some(ref encrypted) = encrypted_bet_amount {
        let mut hasher = Keccak256::new();
        hasher.update(encrypted);
        let hash = hasher.finalize();
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&hash);
        Some(arr)
    } else {
        None
    };

    let bet_commitment = if let Some(commitment) = &body.bet_commitment {
        match hex_to_32_bytes(commitment, "bet_commitment") {
            Ok(arr) => arr,
            Err(resp) => return resp,
        }
    } else if let Some(cipher_hash) = ciphertext_hash_bytes {
        let mut hasher = Keccak256::new();
        hasher.update(&cipher_hash);
        hasher.update(&body.amount_lamports.to_le_bytes());
        hasher.update(&[side as u8]);
        let hash = hasher.finalize();
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&hash);
        arr
    } else {
        return HttpResponse::BadRequest().json(json!({
            "error": "bet_commitment or ciphertext_hash is required",
        }));
    };

    let futarchy_program_id = match get_futarchy_program_id() {
        Ok(pk) => pk,
        Err(resp) => return resp,
    };

    let zk_generator_program = match get_zk_generator_program_id() {
        Ok(pk) => pk,
        Err(resp) => return resp,
    };

    let fhe_accounts = if encrypted_bet_amount.is_some() {
        let job_id = match body.fhe_job_id {
            Some(id) => id,
            None => {
                return HttpResponse::BadRequest().json(json!({
                    "error": "fhe_job_id is required when encrypted_amount is provided",
                }));
            }
        };
        let market_pda = find_market_pda(&futarchy_program_id, market_id);
        let fhe_program_id = match get_fhe_generator_program_id() {
            Ok(pk) => pk,
            Err(resp) => return resp,
        };
        Some(derive_fhe_accounts(&fhe_program_id, &market_pda.address, job_id))
    } else {
        None
    };

    let instruction = match futarchy_sdk::build_place_bet_ix(
        &futarchy_program_id,
        &bettor,
        market_id,
        bet_commitment,
        proof_bytes,
        public_inputs_bytes,
        body.amount_lamports,
        body.circuit_type,
        ciphertext_hash_bytes,
        encrypted_bet_amount,
        Some(side),
        &zk_generator_program,
        fhe_accounts,
    ) {
        Ok(ix) => ix,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to build instruction: {}", e),
            }));
        }
    };

    let unsigned_tx = match futarchy_sdk::prepare_unsigned_transaction(&[instruction], &bettor) {
        Ok(tx) => tx,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to prepare transaction: {}", e),
            }));
        }
    };

    HttpResponse::Ok().json(json!({
        "unsigned_transaction": unsigned_tx,
        "market_id": market_id,
        "signers": vec![body.bettor.clone()],
    }))
}

/// POST /api/futarchy/markets/{id}/settle
/// Settle a market (oracle only)
#[post("/api/futarchy/markets/{id}/settle")]
pub async fn settle_market(
    pool: web::Data<PgPool>,
    path: web::Path<String>,
    body: web::Json<SettleMarketRequest>,
) -> impl Responder {
    let market_id = path.into_inner();

    // Validate market exists
    let market = match FutarchyQueries::get_market(&pool, &market_id).await {
        Ok(Some(m)) => m,
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({
                "error": "Market not found",
                "market_id": market_id,
            }));
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to fetch market: {}", e),
            }));
        }
    };

    // Verify oracle
    if market.oracle != body.oracle {
        return HttpResponse::Forbidden().json(json!({
            "error": "Only the designated oracle can settle this market",
            "expected_oracle": market.oracle,
        }));
    }

    // TODO: Verify signature in production
    // For MVP, we skip signature verification

    match FutarchyQueries::settle_market(&pool, &market_id, body.outcome).await {
        Ok(_) => {
            log::info!(
                "Settled market {}: outcome = {}",
                market_id,
                if body.outcome { "YES" } else { "NO" }
            );

            HttpResponse::Ok().json(json!({
                "message": "Market settled successfully",
                "market_id": market_id,
                "outcome": if body.outcome { "yes" } else { "no" },
            }))
        }
        Err(e) => {
            log::error!("Failed to settle market {}: {}", market_id, e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to settle market: {}", e),
            }))
        }
    }
}

/// POST /api/futarchy/markets/{id}/settle/validate-and-build
/// Build unsigned transaction for SettleMarket
#[post("/api/futarchy/markets/{id}/settle/validate-and-build")]
pub async fn validate_settle_market(
    path: web::Path<String>,
    body: web::Json<SettleMarketRequest>,
) -> impl Responder {
    let market_ok = parse_market_id(&path.into_inner());
    let market_id = match market_ok {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let oracle = match Pubkey::from_str(&body.oracle) {
        Ok(pk) => pk,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Invalid oracle pubkey: {}", e),
            }));
        }
    };

    let futarchy_program_id = match get_futarchy_program_id() {
        Ok(pk) => pk,
        Err(resp) => return resp,
    };

    let instruction = match futarchy_sdk::build_settle_market_ix(
        &futarchy_program_id,
        &oracle,
        market_id,
        body.outcome,
    ) {
        Ok(ix) => ix,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to build instruction: {}", e),
            }));
        }
    };

    let unsigned_tx = match futarchy_sdk::prepare_unsigned_transaction(&[instruction], &oracle) {
        Ok(tx) => tx,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to prepare transaction: {}", e),
            }));
        }
    };

    HttpResponse::Ok().json(json!({
        "unsigned_transaction": unsigned_tx,
        "market_id": market_id,
        "signers": vec![body.oracle.clone()],
    }))
}

// =============================================================================
// FHE Ciphertext Endpoints
// =============================================================================

/// GET /api/fhe/ciphertext/{hash}
/// Fetch a ciphertext by its hash
#[get("/api/fhe/ciphertext/{hash}")]
pub async fn get_ciphertext(
    pool: web::Data<PgPool>,
    path: web::Path<String>,
) -> impl Responder {
    let hash = path.into_inner();

    match FutarchyQueries::get_ciphertext(&pool, &hash).await {
        Ok(Some(ciphertext)) => {
            // Return as base64
            HttpResponse::Ok().json(json!({
                "hash": hash,
                "ciphertext": BASE64.encode(&ciphertext),
                "size_bytes": ciphertext.len(),
            }))
        }
        Ok(None) => {
            HttpResponse::NotFound().json(json!({
                "error": "Ciphertext not found",
                "hash": hash,
            }))
        }
        Err(e) => {
            log::error!("Failed to fetch ciphertext {}: {}", hash, e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to fetch ciphertext: {}", e),
            }))
        }
    }
}

/// GET /api/fhe/markets/{id}/pool/{side}
/// Get the current pool ciphertext for a market side
#[get("/api/fhe/markets/{id}/pool/{side}")]
pub async fn get_pool_ciphertext(
    pool: web::Data<PgPool>,
    path: web::Path<(String, String)>,
) -> impl Responder {
    let (market_id, side_str) = path.into_inner();

    let side = match side_str.to_lowercase().as_str() {
        "yes" | "true" | "1" => true,
        "no" | "false" | "0" => false,
        _ => {
            return HttpResponse::BadRequest().json(json!({
                "error": "Invalid side. Use 'yes' or 'no'",
            }));
        }
    };

    match FutarchyQueries::get_pool_ciphertext(&pool, &market_id, side).await {
        Ok(Some(ciphertext)) => {
            let hash = {
                let mut hasher = Keccak256::new();
                hasher.update(&ciphertext);
                hex::encode(hasher.finalize())
            };

            HttpResponse::Ok().json(json!({
                "market_id": market_id,
                "side": if side { "yes" } else { "no" },
                "hash": hash,
                "ciphertext": BASE64.encode(&ciphertext),
                "size_bytes": ciphertext.len(),
            }))
        }
        Ok(None) => {
            // Return "zero" ciphertext placeholder for new pools
            HttpResponse::Ok().json(json!({
                "market_id": market_id,
                "side": if side { "yes" } else { "no" },
                "hash": null,
                "ciphertext": null,
                "message": "Pool is empty (no bets yet)",
            }))
        }
        Err(e) => {
            log::error!("Failed to fetch pool ciphertext for {}/{}: {}", market_id, side_str, e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to fetch pool ciphertext: {}", e),
            }))
        }
    }
}

/// POST /api/fhe/markets/{id}/pool/{side}/update
/// Update the pool ciphertext (called by prover after FHE computation)
#[post("/api/fhe/markets/{id}/pool/{side}/update")]
pub async fn update_pool_ciphertext(
    pool: web::Data<PgPool>,
    path: web::Path<(String, String)>,
    body: web::Json<UpdatePoolCiphertextRequest>,
) -> impl Responder {
    let (market_id, side_str) = path.into_inner();

    let side = match side_str.to_lowercase().as_str() {
        "yes" | "true" | "1" => true,
        "no" | "false" | "0" => false,
        _ => {
            return HttpResponse::BadRequest().json(json!({
                "error": "Invalid side. Use 'yes' or 'no'",
            }));
        }
    };

    let ciphertext_bytes = match BASE64.decode(&body.ciphertext) {
        Ok(bytes) => bytes,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Invalid base64: {}", e),
            }));
        }
    };

    // Get next version number
    let version = match FutarchyQueries::get_next_pool_version(&pool, &market_id, side).await {
        Ok(v) => v,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to get version: {}", e),
            }));
        }
    };

    let hash = {
        let mut hasher = Keccak256::new();
        hasher.update(&ciphertext_bytes);
        hex::encode(hasher.finalize())
    };

    let ct_data = CreateCiphertextData {
        hash: hash.clone(),
        ciphertext: ciphertext_bytes.clone(),
        ciphertext_type: "pool".to_string(),
        market_id: Some(market_id.clone()),
        side: Some(side),
        version,
        created_by: body.updated_by.clone(),
    };

    match FutarchyQueries::store_ciphertext(&pool, &ct_data).await {
        Ok(_) => {
            log::info!(
                "Updated pool ciphertext for market {} side {} (version {})",
                market_id,
                if side { "YES" } else { "NO" },
                version
            );

            HttpResponse::Ok().json(json!({
                "message": "Pool ciphertext updated",
                "market_id": market_id,
                "side": if side { "yes" } else { "no" },
                "hash": hash,
                "version": version,
                "size_bytes": ciphertext_bytes.len(),
            }))
        }
        Err(e) => {
            log::error!("Failed to update pool ciphertext: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to update pool ciphertext: {}", e),
            }))
        }
    }
}

// =============================================================================
// Position Endpoints
// =============================================================================

/// GET /api/futarchy/markets/{id}/positions
/// Get all positions for a market
#[get("/api/futarchy/markets/{id}/positions")]
pub async fn get_market_positions(
    pool: web::Data<PgPool>,
    path: web::Path<String>,
) -> impl Responder {
    let market_id = path.into_inner();

    match FutarchyQueries::get_market_positions(&pool, &market_id).await {
        Ok(positions) => {
            let responses: Vec<_> = positions
                .into_iter()
                .map(|p| json!({
                    "id": p.id,
                    "bettor": p.bettor,
                    "side": if p.side { "yes" } else { "no" },
                    "amount_lamports": p.amount_lamports,
                    "encrypted": p.encrypted_amount_hash.is_some(),
                    "status": p.status,
                    "created_at": p.created_at.to_rfc3339(),
                }))
                .collect();

            HttpResponse::Ok().json(json!({
                "market_id": market_id,
                "positions": responses,
                "count": responses.len(),
            }))
        }
        Err(e) => {
            log::error!("Failed to get positions for market {}: {}", market_id, e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to fetch positions: {}", e),
            }))
        }
    }
}

/// GET /api/futarchy/positions/{bettor}
/// Get all positions for a bettor
#[get("/api/futarchy/positions/{bettor}")]
pub async fn get_bettor_positions(
    pool: web::Data<PgPool>,
    path: web::Path<String>,
) -> impl Responder {
    let bettor = path.into_inner();

    match FutarchyQueries::get_bettor_positions(&pool, &bettor).await {
        Ok(positions) => {
            let responses: Vec<_> = positions
                .into_iter()
                .map(|p| json!({
                    "id": p.id,
                    "market_id": p.market_id,
                    "side": if p.side { "yes" } else { "no" },
                    "amount_lamports": p.amount_lamports,
                    "encrypted": p.encrypted_amount_hash.is_some(),
                    "status": p.status,
                    "created_at": p.created_at.to_rfc3339(),
                }))
                .collect();

            HttpResponse::Ok().json(json!({
                "bettor": bettor,
                "positions": responses,
                "count": responses.len(),
            }))
        }
        Err(e) => {
            log::error!("Failed to get positions for bettor {}: {}", bettor, e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to fetch positions: {}", e),
            }))
        }
    }
}

/// POST /api/futarchy/markets/{id}/claim/validate-and-build
/// Build unsigned transaction for ClaimPayout
#[post("/api/futarchy/markets/{id}/claim/validate-and-build")]
pub async fn validate_claim_payout(
    path: web::Path<String>,
    body: web::Json<ValidateClaimRequest>,
) -> impl Responder {
    let market_ok = parse_market_id(&path.into_inner());
    let market_id = match market_ok {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let user = match Pubkey::from_str(&body.user) {
        Ok(pk) => pk,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Invalid user pubkey: {}", e),
            }));
        }
    };

    let claim_nullifier = match hex_to_32_bytes(&body.claim_nullifier, "claim_nullifier") {
        Ok(arr) => arr,
        Err(resp) => return resp,
    };

    let bet_commitment = match hex_to_32_bytes(&body.bet_commitment, "bet_commitment") {
        Ok(arr) => arr,
        Err(resp) => return resp,
    };

    let proof_bytes = match BASE64.decode(&body.proof) {
        Ok(bytes) => bytes,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Invalid proof base64: {}", e),
            }));
        }
    };

    let public_inputs_bytes = match BASE64.decode(&body.public_inputs) {
        Ok(bytes) => bytes,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Invalid public_inputs base64: {}", e),
            }));
        }
    };

    let futarchy_program_id = match get_futarchy_program_id() {
        Ok(pk) => pk,
        Err(resp) => return resp,
    };

    let zk_generator_program = match get_zk_generator_program_id() {
        Ok(pk) => pk,
        Err(resp) => return resp,
    };

    let instruction = match futarchy_sdk::build_claim_payout_ix(
        &futarchy_program_id,
        &user,
        market_id,
        claim_nullifier,
        bet_commitment,
        proof_bytes,
        public_inputs_bytes,
        body.payout_amount,
        &zk_generator_program,
        body.include_position.unwrap_or(true),
    ) {
        Ok(ix) => ix,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to build instruction: {}", e),
            }));
        }
    };

    let unsigned_tx = match futarchy_sdk::prepare_unsigned_transaction(&[instruction], &user) {
        Ok(tx) => tx,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to prepare transaction: {}", e),
            }));
        }
    };

    HttpResponse::Ok().json(json!({
        "unsigned_transaction": unsigned_tx,
        "market_id": market_id,
        "signers": vec![body.user.clone()],
    }))
}

// =============================================================================
// FHE Job Endpoints (for prover polling)
// =============================================================================

/// Response for pending FHE job
#[derive(Debug, Serialize)]
pub struct FheJobResponse {
    pub id: i32,
    pub market_id: String,
    pub side: String,
    pub pool_ciphertext_hash: String,
    pub bet_ciphertext_hash: String,
    pub status: String,
    pub created_at: String,
}

/// GET /api/futarchy/fhe-jobs/pending
/// Get pending FHE jobs for provers to process
#[get("/api/futarchy/fhe-jobs/pending")]
pub async fn get_pending_fhe_jobs(
    pool: web::Data<PgPool>,
    query: web::Query<ListMarketsQuery>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(10).min(50);

    match FutarchyQueries::get_pending_fhe_jobs(&pool, limit).await {
        Ok(jobs) => {
            let responses: Vec<FheJobResponse> = jobs
                .into_iter()
                .map(|j| FheJobResponse {
                    id: j.id,
                    market_id: j.market_id,
                    side: if j.side { "yes".to_string() } else { "no".to_string() },
                    pool_ciphertext_hash: j.pool_ciphertext_hash,
                    bet_ciphertext_hash: j.bet_ciphertext_hash,
                    status: j.status,
                    created_at: j.created_at.to_rfc3339(),
                })
                .collect();

            HttpResponse::Ok().json(json!({
                "jobs": responses,
                "count": responses.len(),
            }))
        }
        Err(e) => {
            log::error!("Failed to get pending FHE jobs: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to get pending FHE jobs: {}", e),
            }))
        }
    }
}

/// Request to complete an FHE job
#[derive(Debug, Deserialize)]
pub struct CompleteFheJobRequest {
    /// Hash of the result ciphertext
    pub result_hash: String,
    /// Prover pubkey (for verification)
    pub prover: Option<String>,
}

/// POST /api/futarchy/fhe-jobs/{id}/complete
/// Mark an FHE job as completed
#[post("/api/futarchy/fhe-jobs/{id}/complete")]
pub async fn complete_fhe_job(
    pool: web::Data<PgPool>,
    path: web::Path<i32>,
    body: web::Json<CompleteFheJobRequest>,
) -> impl Responder {
    let job_id = path.into_inner();

    match FutarchyQueries::complete_fhe_job(&pool, job_id, &body.result_hash).await {
        Ok(_) => {
            log::info!("Completed FHE job {} with result hash: {}", job_id, &body.result_hash[..16]);
            HttpResponse::Ok().json(json!({
                "message": "FHE job completed",
                "job_id": job_id,
                "result_hash": body.result_hash,
            }))
        }
        Err(e) => {
            log::error!("Failed to complete FHE job {}: {}", job_id, e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to complete FHE job: {}", e),
            }))
        }
    }
}

/// POST /api/futarchy/fhe-jobs/{id}/fail
/// Mark an FHE job as failed
#[post("/api/futarchy/fhe-jobs/{id}/fail")]
pub async fn fail_fhe_job(
    pool: web::Data<PgPool>,
    path: web::Path<i32>,
) -> impl Responder {
    let job_id = path.into_inner();

    match FutarchyQueries::fail_fhe_job(&pool, job_id).await {
        Ok(_) => {
            log::warn!("Marked FHE job {} as failed", job_id);
            HttpResponse::Ok().json(json!({
                "message": "FHE job marked as failed",
                "job_id": job_id,
            }))
        }
        Err(e) => {
            log::error!("Failed to mark FHE job {} as failed: {}", job_id, e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to mark FHE job as failed: {}", e),
            }))
        }
    }
}

/// GET /api/futarchy/fhe-jobs/{id}/data
/// Get ciphertexts for an FHE job (pool + bet) - for prover hybrid flow
#[get("/api/futarchy/fhe-jobs/{id}/data")]
pub async fn get_fhe_job_data(
    pool: web::Data<PgPool>,
    path: web::Path<i32>,
) -> impl Responder {
    let job_id = path.into_inner();

    match FutarchyQueries::get_fhe_job_with_ciphertexts(&pool, job_id).await {
        Ok(Some(job)) => {
            // Si pool_ciphertext_hash es todo ceros, pool esta vacio (primera bet)
            let is_pool_empty = job.pool_ciphertext_hash.chars().all(|c| c == '0');

            let pool_ciphertext_b64 = if is_pool_empty {
                None
            } else {
                job.pool_ciphertext.as_ref().map(|ct| BASE64.encode(ct))
            };

            let bet_ciphertext_b64 = job.bet_ciphertext.as_ref().map(|ct| BASE64.encode(ct));

            HttpResponse::Ok().json(json!({
                "job_id": job.id,
                "market_id": job.market_id,
                "side": if job.side { "yes" } else { "no" },
                "pool_ciphertext_hash": if is_pool_empty { serde_json::Value::Null } else { json!(job.pool_ciphertext_hash) },
                "pool_ciphertext": pool_ciphertext_b64,
                "bet_ciphertext_hash": job.bet_ciphertext_hash,
                "bet_ciphertext": bet_ciphertext_b64,
                "status": job.status,
                "created_at": job.created_at.to_rfc3339(),
            }))
        }
        Ok(None) => {
            HttpResponse::NotFound().json(json!({
                "error": "FHE job not found",
                "job_id": job_id,
            }))
        }
        Err(e) => {
            log::error!("Failed to fetch FHE job data {}: {}", job_id, e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to fetch FHE job data: {}", e),
            }))
        }
    }
}

/// POST /api/futarchy/fhe-jobs/{id}/result
/// Submit result ciphertext for completed FHE job - for prover hybrid flow
#[derive(Debug, Deserialize)]
pub struct SubmitFheResultRequest {
    /// Base64-encoded result ciphertext (new pool)
    pub result_ciphertext: String,
    /// Prover pubkey (for tracking)
    pub prover_pubkey: Option<String>,
}

#[post("/api/futarchy/fhe-jobs/{id}/result")]
pub async fn submit_fhe_result(
    pool: web::Data<PgPool>,
    path: web::Path<i32>,
    body: web::Json<SubmitFheResultRequest>,
) -> impl Responder {
    let job_id = path.into_inner();

    // Decode result ciphertext
    let result_bytes = match BASE64.decode(&body.result_ciphertext) {
        Ok(bytes) => bytes,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Invalid result_ciphertext base64: {}", e),
            }));
        }
    };

    // Compute hash
    let result_hash = {
        let mut hasher = Keccak256::new();
        hasher.update(&result_bytes);
        hex::encode(hasher.finalize())
    };

    // Fetch job data to get market_id and side
    let job_data = match FutarchyQueries::get_fhe_job_with_ciphertexts(&pool, job_id).await {
        Ok(Some(job)) => job,
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({
                "error": "FHE job not found",
                "job_id": job_id,
            }));
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to fetch job: {}", e),
            }));
        }
    };

    // Get next version for pool ciphertext
    let version = match FutarchyQueries::get_next_pool_version(
        &pool,
        &job_data.market_id,
        job_data.side,
    ).await {
        Ok(v) => v,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to get version: {}", e),
            }));
        }
    };

    // Store result as new pool ciphertext
    let ct_data = CreateCiphertextData {
        hash: result_hash.clone(),
        ciphertext: result_bytes,
        ciphertext_type: "pool".to_string(),
        market_id: Some(job_data.market_id.clone()),
        side: Some(job_data.side),
        version,
        created_by: body.prover_pubkey.clone(),
    };

    if let Err(e) = FutarchyQueries::store_ciphertext(&pool, &ct_data).await {
        return HttpResponse::InternalServerError().json(json!({
            "error": format!("Failed to store result ciphertext: {}", e),
        }));
    }

    // Mark job as completed
    if let Err(e) = FutarchyQueries::complete_fhe_job_with_prover(
        &pool,
        job_id,
        &result_hash,
        body.prover_pubkey.as_deref(),
    ).await {
        return HttpResponse::InternalServerError().json(json!({
            "error": format!("Failed to complete FHE job: {}", e),
        }));
    }

    log::info!(
        "FHE job {} completed: result_hash={} version={}",
        job_id,
        &result_hash[..16],
        version
    );

    HttpResponse::Ok().json(json!({
        "message": "FHE job result submitted successfully",
        "job_id": job_id,
        "result_hash": result_hash,
        "market_id": job_data.market_id,
        "side": if job_data.side { "yes" } else { "no" },
        "version": version,
    }))
}

// =============================================================================
// E2E Bet Flow Endpoints (prepare + submit)
// =============================================================================

/// POST /api/futarchy/bet/prepare
/// Build unsigned transaction for placing a bet
#[post("/api/futarchy/bet/prepare")]
pub async fn prepare_bet(
    app_state: web::Data<AppState>,
    body: web::Json<PrepareBetRequest>,
) -> impl Responder {
    let bettor = match Pubkey::from_str(&body.bettor) {
        Ok(pk) => pk,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Invalid bettor pubkey: {}", e),
            }));
        }
    };

    let ciphertext_hash_bytes = match hex::decode(&body.ciphertext_hash) {
        Ok(bytes) if bytes.len() == 32 => {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            arr
        }
        Ok(_) => {
            return HttpResponse::BadRequest().json(json!({
                "error": "ciphertext_hash must be exactly 32 bytes (64 hex chars)",
            }));
        }
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Invalid ciphertext_hash hex: {}", e),
            }));
        }
    };

    let proof_bytes = match BASE64.decode(&body.proof) {
        Ok(bytes) => bytes,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Invalid proof base64: {}", e),
            }));
        }
    };

    let public_inputs_bytes = match BASE64.decode(&body.public_inputs) {
        Ok(bytes) => bytes,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Invalid public_inputs base64: {}", e),
            }));
        }
    };

    let bet_commitment = {
        let mut hasher = Keccak256::new();
        hasher.update(&ciphertext_hash_bytes);
        hasher.update(&body.amount_lamports.to_le_bytes());
        hasher.update(&[body.side as u8]);
        let hash = hasher.finalize();
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&hash);
        arr
    };

    let zk_generator_id = std::env::var("ZK_GENERATOR_PROGRAM_ID")
        .unwrap_or_else(|_| "Dzvy1pzCBgtMw5Fte2GybpeN2PsLPW8t7zDvLfKpxnSS".to_string());
    let zk_generator_program = match Pubkey::from_str(&zk_generator_id) {
        Ok(pk) => pk,
        Err(_) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Invalid ZK_GENERATOR_PROGRAM_ID: {}", zk_generator_id),
            }));
        }
    };

    // Use FUTARCHY_PROGRAM_ID for futarchy handlers, NOT the bedrock program_id
    let futarchy_program_id_str = std::env::var("FUTARCHY_PROGRAM_ID")
        .unwrap_or_else(|_| "AQUUuRSwDhB1eeC2Caa8GPVGV4YzZkJ1YiSvZd3BBPij".to_string());
    let futarchy_program_id = match Pubkey::from_str(&futarchy_program_id_str) {
        Ok(pk) => pk,
        Err(_) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Invalid FUTARCHY_PROGRAM_ID: {}", futarchy_program_id_str),
            }));
        }
    };

    // Convert market_id string to u64 (hash first 8 bytes of hex-decoded string)
    let market_id_u64: u64 = {
        let bytes = match hex::decode(&body.market_id) {
            Ok(b) if b.len() >= 8 => {
                let mut arr = [0u8; 8];
                arr.copy_from_slice(&b[..8]);
                u64::from_le_bytes(arr)
            }
            Ok(_) => {
                // Short hex, try parsing as number
                body.market_id.parse::<u64>().unwrap_or(0)
            }
            Err(_) => {
                // Not hex, try parsing as number directly
                body.market_id.parse::<u64>().unwrap_or(0)
            }
        };
        bytes
    };

    let instruction = match futarchy_sdk::build_place_bet_ix(
        &futarchy_program_id,
        &bettor,
        market_id_u64,
        bet_commitment,
        proof_bytes,
        public_inputs_bytes,
        body.amount_lamports,
        body.circuit_type,
        Some(ciphertext_hash_bytes),
        None,
        Some(body.side),
        &zk_generator_program,
        None,
    ) {
        Ok(ix) => ix,
        Err(e) => {
            log::error!("Failed to build place bet instruction: {}", e);
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to build instruction: {}", e),
            }));
        }
    };

    let unsigned_tx = match futarchy_sdk::prepare_unsigned_transaction(&[instruction], &bettor) {
        Ok(tx) => tx,
        Err(e) => {
            log::error!("Failed to prepare unsigned transaction: {}", e);
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to prepare transaction: {}", e),
            }));
        }
    };

    log::info!(
        "Prepared unsigned bet transaction for market {} bettor {} side {}",
        body.market_id,
        body.bettor,
        if body.side { "YES" } else { "NO" }
    );

    HttpResponse::Ok().json(json!({
        "unsigned_transaction": unsigned_tx,
        "market_id": body.market_id,
        "signers": vec![body.bettor.clone()],
    }))
}

/// POST /api/futarchy/bet/submit
/// Submit signed transaction and store ciphertext
#[post("/api/futarchy/bet/submit")]
pub async fn submit_bet(
    app_state: web::Data<AppState>,
    body: web::Json<SubmitBetRequest>,
) -> impl Responder {
    let signed_tx_bytes = match BASE64.decode(&body.signed_tx) {
        Ok(bytes) => bytes,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Invalid signed_tx base64: {}", e),
            }));
        }
    };

    // Deserialize transaction - try bincode first (legacy), then versioned wire format
    let transaction: Transaction = match bincode::deserialize::<Transaction>(&signed_tx_bytes) {
        Ok(tx) => tx,
        Err(bincode_err) => {
            // Try as VersionedTransaction (wire format from @solana/web3.js)
            match bincode::deserialize::<solana_sdk::transaction::VersionedTransaction>(&signed_tx_bytes) {
                Ok(versioned_tx) => {
                    // Convert VersionedTransaction to legacy Transaction
                    match versioned_tx.into_legacy_transaction() {
                        Some(tx) => tx,
                        None => {
                            return HttpResponse::BadRequest().json(json!({
                                "error": "VersionedTransaction cannot be converted to legacy Transaction",
                            }));
                        }
                    }
                }
                Err(versioned_err) => {
                    log::error!("Failed to deserialize transaction. Legacy bincode: {}, Versioned: {}. Bytes len: {}",
                        bincode_err, versioned_err, signed_tx_bytes.len());
                    return HttpResponse::BadRequest().json(json!({
                        "error": format!("Invalid transaction format"),
                    }));
                }
            }
        }
    };

    let ciphertext_bytes = match BASE64.decode(&body.ciphertext) {
        Ok(bytes) => bytes,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Invalid ciphertext base64: {}", e),
            }));
        }
    };

    let ciphertext_hash = {
        let mut hasher = Keccak256::new();
        hasher.update(&ciphertext_bytes);
        hex::encode(hasher.finalize())
    };

    let server_key_hash = if let Some(ref sk) = body.server_key {
        match BASE64.decode(sk) {
            Ok(sk_bytes) => {
                let mut hasher = Keccak256::new();
                hasher.update(&sk_bytes);
                hex::encode(hasher.finalize())
            }
            Err(e) => {
                return HttpResponse::BadRequest().json(json!({
                    "error": format!("Invalid server_key base64: {}", e),
                }));
            }
        }
    } else {
        "0".repeat(64)
    };

    let bettor_pubkey = if !transaction.message.account_keys.is_empty() {
        transaction.message.account_keys[0].to_string()
    } else {
        return HttpResponse::BadRequest().json(json!({
            "error": "Transaction has no account keys",
        }));
    };

    let rpc_url = app_state.rpc_url.clone();

    // Use spawn_blocking for the synchronous RPC call
    let tx_result = tokio::task::spawn_blocking(move || {
        let rpc_client = RpcClient::new(&rpc_url);
        rpc_client.send_and_confirm_transaction(&transaction)
    })
    .await;

    let tx_signature = match tx_result {
        Ok(Ok(sig)) => {
            log::info!("Transaction confirmed: {}", sig);
            sig.to_string()
        }
        Ok(Err(e)) => {
            log::error!("Transaction failed: {}", e);

            if let Err(cleanup_err) = FutarchyQueries::mark_failed_and_cleanup(
                &app_state.db_pool,
                &format!("pending_{}", &ciphertext_hash[..16]),
            )
            .await
            {
                log::warn!("Failed to cleanup after TX failure: {}", cleanup_err);
            }

            return HttpResponse::BadRequest().json(json!({
                "error": format!("Transaction failed: {}", e),
                "status": "failed",
            }));
        }
        Err(e) => {
            log::error!("Task join error: {}", e);
            return HttpResponse::InternalServerError().json(json!({
                "error": "Internal server error during transaction submission",
            }));
        }
    };

    if let Err(e) = FutarchyQueries::save_ciphertext_with_tx(
        &app_state.db_pool,
        &ciphertext_bytes,
        &ciphertext_hash,
        Some(&tx_signature),
        &server_key_hash,
        &bettor_pubkey,
    )
    .await
    {
        log::error!("Failed to save ciphertext after TX confirm: {}", e);
        return HttpResponse::InternalServerError().json(json!({
            "error": "Transaction confirmed but failed to store ciphertext",
            "tx_signature": tx_signature,
        }));
    }

    if let Err(e) = FutarchyQueries::mark_confirmed(&app_state.db_pool, &tx_signature).await {
        log::warn!("Failed to mark ciphertext as confirmed: {}", e);
    }

    // Create FHE job for prover to process (if market_id and side provided)
    if let (Some(market_id), Some(side)) = (body.market_id, body.side) {
        let market_id_str = market_id.to_string();
        let pool_hash = FutarchyQueries::get_pool_ciphertext_hash(&app_state.db_pool, &market_id_str, side)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "0".repeat(64));

        // Try to get on-chain job_id from Market.pending_pool_update_job
        let onchain_job_id: Option<u64> = {
            let rpc_url = app_state.rpc_url.clone();
            let futarchy_program_id = get_futarchy_program_id().ok();

            if let Some(program_id) = futarchy_program_id {
                // Read Market on-chain to get pending_pool_update_job
                // Use spawn_blocking with internal tokio runtime for async SDK call
                tokio::task::spawn_blocking(move || {
                    let rpc_client = RpcClient::new(&rpc_url);
                    let rt = tokio::runtime::Runtime::new().ok()?;
                    rt.block_on(async {
                        match futarchy_sdk::query_market(&rpc_client, &program_id, market_id).await {
                            Ok(market) => market.pending_pool_update_job,
                            Err(e) => {
                                log::warn!("Failed to read Market on-chain for job_id: {}", e);
                                None
                            }
                        }
                    })
                })
                .await
                .unwrap_or(None)
            } else {
                None
            }
        };

        if let Some(job_id) = onchain_job_id {
            log::info!("Found on-chain FHE job_id {} for market {}", job_id, market_id);
        }

        if let Err(e) = FutarchyQueries::create_fhe_job_with_onchain_id(
            &app_state.db_pool,
            &market_id_str,
            side,
            &pool_hash,
            &ciphertext_hash,
            onchain_job_id,
        ).await {
            log::warn!("Failed to create FHE job for market {} side {}: {}", market_id, side, e);
            // Continue anyway - bet is confirmed, job can be created later
        } else {
            log::info!("Created FHE job for market {} side {} bet_hash={} onchain_job_id={:?}",
                market_id, side, &ciphertext_hash[..16], onchain_job_id);
        }
    }

    log::info!(
        "Bet submitted successfully: tx={} ciphertext_hash={}",
        tx_signature,
        &ciphertext_hash[..16]
    );

    HttpResponse::Ok().json(json!({
        "tx_signature": tx_signature,
        "status": "confirmed",
        "ciphertext_hash": ciphertext_hash,
    }))
}

// =============================================================================
// Health Check
// =============================================================================

/// GET /api/futarchy/health
/// Health check for futarchy subsystem
#[get("/api/futarchy/health")]
pub async fn futarchy_health(pool: web::Data<PgPool>) -> impl Responder {
    // Quick check: count markets
    let count_result: Result<(i64,), _> = sqlx::query_as(
        "SELECT COUNT(*) FROM futarchy_markets"
    )
    .fetch_one(pool.get_ref())
    .await;

    match count_result {
        Ok((count,)) => {
            HttpResponse::Ok().json(json!({
                "status": "ok",
                "service": "futarchy",
                "markets_count": count,
            }))
        }
        Err(e) => {
            log::error!("Futarchy health check failed: {}", e);
            HttpResponse::ServiceUnavailable().json(json!({
                "status": "degraded",
                "error": format!("Database query failed: {}", e),
            }))
        }
    }
}

// =============================================================================
// Route Configuration
// =============================================================================

/// Configure all futarchy routes
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg
        // Health
        .service(futarchy_health)
        // E2E Bet Flow
        .service(prepare_bet)
        .service(submit_bet)
        // Markets
        .service(list_markets)
        .service(get_market)
        .service(create_market)
        .service(validate_create_market)
        .service(place_bet)
        .service(validate_place_bet)
        .service(settle_market)
        .service(validate_settle_market)
        .service(get_market_positions)
        // FHE ciphertexts
        .service(get_ciphertext)
        .service(get_pool_ciphertext)
        .service(update_pool_ciphertext)
        // FHE jobs (for prover polling)
        .service(get_pending_fhe_jobs)
        .service(complete_fhe_job)
        .service(fail_fhe_job)
        // FHE jobs data endpoints (for prover hybrid flow)
        .service(get_fhe_job_data)
        .service(submit_fhe_result)
        // Positions
        .service(get_bettor_positions)
        .service(validate_claim_payout);
}
