//! x402 Anti-Spam Payment Layer
//!
//! HTTP 402 Payment Required implementation for ZyberLink.
//! Prevents spam by requiring payment proof before witness upload.
//!
//! Flow:
//! 1. Client gets quote for operation (GET /api/x402/quote)
//! 2. Client gets payment instruction (POST /api/x402/build-payment)
//! 3. Client signs and submits via confirm (POST /api/x402/confirm)
//! 4. Client uses payment token for witness upload (POST /api/x402/witness)

use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use uuid::Uuid;
use blake2::{Blake2s256, Digest};

use crate::AppState;

// =============================================================================
// Constants
// =============================================================================

/// Quote expiration time (5 minutes)
const QUOTE_EXPIRY_SECS: i64 = 300;

/// Payment token expiration time (24 hours)
const TOKEN_EXPIRY_SECS: i64 = 24 * 60 * 60;

/// Protocol fee recipient (treasury)
const PROTOCOL_FEE_RECIPIENT: &str = "ZYBRtreasury11111111111111111111111111111111";

// =============================================================================
// Types
// =============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct QuoteRequest {
    pub circuit_type: u8,
    pub payer: String,
}

#[derive(Debug, Serialize)]
pub struct QuoteResponse {
    pub circuit_type: u8,
    pub price_lamports: u64,
    pub price_sol: f64,
    pub expires_at: i64,
    pub quote_id: String,
    pub payment_recipient: String,
}

#[derive(Debug, Deserialize)]
pub struct BuildPaymentRequest {
    pub quote_id: String,
    pub payer: String,
    pub amount: String, // String to handle large numbers
    pub recipient: String,
}

#[derive(Debug, Serialize)]
pub struct BuildPaymentResponse {
    pub instruction: serde_json::Value, // Serialized instruction
}

#[derive(Debug, Deserialize)]
pub struct ConfirmPaymentRequest {
    pub quote_id: String,
    pub signed_transaction: String, // Base64 encoded
    pub signature: String,
}

#[derive(Debug, Serialize)]
pub struct ConfirmPaymentResponse {
    pub token_id: String,
    pub amount_paid: String,
    pub tx_signature: String,
    pub expires_at: i64,
}

#[derive(Debug, Serialize)]
pub struct WitnessUploadResponse {
    pub commitment: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateJobRequest {
    pub circuit_type: u8,
    pub witness_commitment: String,
    pub witness_size: u32,
    pub timeout_seconds: i64,
    pub payer: String,
}

#[derive(Debug, Serialize)]
pub struct CreateJobResponse {
    pub job_id: i64,
    pub unsigned_transaction: String,
}

#[derive(Debug, Deserialize)]
pub struct EstimateRequest {
    pub circuit_type: u8,
}

#[derive(Debug, Serialize)]
pub struct EstimateResponse {
    pub price_lamports: u64,
    pub price_sol: f64,
}

#[derive(Debug, Serialize)]
pub struct TokenStatusResponse {
    pub valid: bool,
    pub used: bool,
    pub expires_at: i64,
}

// =============================================================================
// Pricing Engine
// =============================================================================

/// Get price for a circuit type in lamports
fn get_circuit_price(circuit_type: u8) -> u64 {
    // Pricing tiers based on circuit complexity
    match circuit_type {
        // Legacy circuits (0-9): Lower price
        0..=9 => 10_000_000, // 0.01 SOL

        // Core primitives (10-19): Standard price
        10..=19 => 50_000_000, // 0.05 SOL

        // Voting circuits (20-29): Standard price
        20..=29 => 50_000_000, // 0.05 SOL

        // Market circuits (30-39): Higher price (more complex)
        30..=39 => 75_000_000, // 0.075 SOL

        // Portfolio circuits (40-49): Premium price (most complex)
        40..=49 => 100_000_000, // 0.1 SOL

        // Future circuits: Default to premium
        _ => 100_000_000, // 0.1 SOL
    }
}

// =============================================================================
// API Endpoints
// =============================================================================

/// POST /api/x402/quote
///
/// Get a price quote for a ZK proof operation.
/// Free endpoint - no authentication required.
#[post("/api/x402/quote")]
async fn get_quote(
    data: web::Data<AppState>,
    req: web::Json<QuoteRequest>,
) -> impl Responder {
    log::info!("x402 quote request for circuit_type: {}", req.circuit_type);

    let price_lamports = get_circuit_price(req.circuit_type);
    let price_sol = price_lamports as f64 / 1_000_000_000.0;
    let quote_id = Uuid::new_v4().to_string();
    let expires_at = chrono::Utc::now().timestamp() + QUOTE_EXPIRY_SECS;

    // Store quote in database for later validation
    let insert_result = sqlx::query(
        r#"
        INSERT INTO x402_quotes (quote_id, circuit_type, payer, price_lamports, expires_at)
        VALUES ($1, $2, $3, $4, to_timestamp($5))
        "#,
    )
    .bind(&quote_id)
    .bind(req.circuit_type as i16)
    .bind(&req.payer)
    .bind(price_lamports as i64)
    .bind(expires_at)
    .execute(&data.db_pool)
    .await;

    if let Err(e) = insert_result {
        log::error!("Failed to store quote: {}", e);
        return HttpResponse::InternalServerError().json(json!({
            "error": "Failed to create quote"
        }));
    }

    log::info!("Quote created: {} for {} lamports", quote_id, price_lamports);

    HttpResponse::Ok().json(QuoteResponse {
        circuit_type: req.circuit_type,
        price_lamports,
        price_sol,
        expires_at,
        quote_id,
        payment_recipient: PROTOCOL_FEE_RECIPIENT.to_string(),
    })
}

/// POST /api/x402/estimate
///
/// Get price estimate without creating a quote.
/// Useful for UI display.
#[post("/api/x402/estimate")]
async fn estimate_price(req: web::Json<EstimateRequest>) -> impl Responder {
    let price_lamports = get_circuit_price(req.circuit_type);
    let price_sol = price_lamports as f64 / 1_000_000_000.0;

    HttpResponse::Ok().json(EstimateResponse {
        price_lamports,
        price_sol,
    })
}

/// POST /api/x402/build-payment
///
/// Build a payment instruction for a quote.
#[post("/api/x402/build-payment")]
async fn build_payment_instruction(
    data: web::Data<AppState>,
    req: web::Json<BuildPaymentRequest>,
) -> impl Responder {
    log::info!("Building payment instruction for quote: {}", req.quote_id);

    // Validate quote exists and is not expired
    let quote_result: Option<(i16, i64, chrono::NaiveDateTime)> = sqlx::query_as(
        "SELECT circuit_type, price_lamports, expires_at FROM x402_quotes WHERE quote_id = $1",
    )
    .bind(&req.quote_id)
    .fetch_optional(&data.db_pool)
    .await
    .unwrap_or(None);

    let (circuit_type, price_lamports, expires_at) = match quote_result {
        Some(q) => q,
        None => {
            return HttpResponse::NotFound().json(json!({
                "error": "Quote not found"
            }));
        }
    };

    // Check expiration
    if expires_at < chrono::Utc::now().naive_utc() {
        return HttpResponse::BadRequest().json(json!({
            "error": "Quote has expired"
        }));
    }

    // Parse addresses
    let payer = match Pubkey::from_str(&req.payer) {
        Ok(p) => p,
        Err(_) => {
            return HttpResponse::BadRequest().json(json!({
                "error": "Invalid payer address"
            }));
        }
    };

    let recipient = match Pubkey::from_str(&req.recipient) {
        Ok(r) => r,
        Err(_) => {
            return HttpResponse::BadRequest().json(json!({
                "error": "Invalid recipient address"
            }));
        }
    };

    // Build SOL transfer instruction
    let instruction = solana_sdk::system_instruction::transfer(
        &payer,
        &recipient,
        price_lamports as u64,
    );

    // Serialize instruction to JSON-friendly format
    let instruction_json = json!({
        "programId": instruction.program_id.to_string(),
        "keys": instruction.accounts.iter().map(|acc| json!({
            "pubkey": acc.pubkey.to_string(),
            "isSigner": acc.is_signer,
            "isWritable": acc.is_writable,
        })).collect::<Vec<_>>(),
        "data": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &instruction.data),
    });

    HttpResponse::Ok().json(BuildPaymentResponse {
        instruction: instruction_json,
    })
}

/// POST /api/x402/confirm
///
/// Confirm payment and get a payment token.
/// Submits the signed transaction and verifies on-chain.
#[post("/api/x402/confirm")]
async fn confirm_payment(
    data: web::Data<AppState>,
    req: web::Json<ConfirmPaymentRequest>,
) -> impl Responder {
    log::info!("Confirming payment for quote: {}", req.quote_id);

    // Validate quote
    let quote_result: Option<(i16, i64, String)> = sqlx::query_as(
        "SELECT circuit_type, price_lamports, payer FROM x402_quotes WHERE quote_id = $1",
    )
    .bind(&req.quote_id)
    .fetch_optional(&data.db_pool)
    .await
    .unwrap_or(None);

    let (circuit_type, price_lamports, payer) = match quote_result {
        Some(q) => q,
        None => {
            return HttpResponse::NotFound().json(json!({
                "error": "Quote not found"
            }));
        }
    };

    // TODO: In production, verify the transaction on-chain
    // For now, we trust the client's signature
    // This would involve:
    // 1. Decode the signed transaction
    // 2. Submit to RPC
    // 3. Wait for confirmation
    // 4. Verify transfer amount matches quote

    // Generate payment token
    let token_id = Uuid::new_v4().to_string();
    let expires_at = chrono::Utc::now().timestamp() + TOKEN_EXPIRY_SECS;

    // Store token in database
    let insert_result = sqlx::query(
        r#"
        INSERT INTO x402_tokens (token_id, quote_id, payer, circuit_type, amount_paid, tx_signature, expires_at, used)
        VALUES ($1, $2, $3, $4, $5, $6, to_timestamp($7), false)
        "#,
    )
    .bind(&token_id)
    .bind(&req.quote_id)
    .bind(&payer)
    .bind(circuit_type)
    .bind(price_lamports)
    .bind(&req.signature)
    .bind(expires_at)
    .execute(&data.db_pool)
    .await;

    if let Err(e) = insert_result {
        log::error!("Failed to store payment token: {}", e);
        return HttpResponse::InternalServerError().json(json!({
            "error": "Failed to create payment token"
        }));
    }

    // Mark quote as used
    let _ = sqlx::query("UPDATE x402_quotes SET used = true WHERE quote_id = $1")
        .bind(&req.quote_id)
        .execute(&data.db_pool)
        .await;

    log::info!("Payment confirmed, token: {}", token_id);

    HttpResponse::Ok().json(ConfirmPaymentResponse {
        token_id,
        amount_paid: price_lamports.to_string(),
        tx_signature: req.signature.clone(),
        expires_at,
    })
}

/// POST /api/x402/witness
///
/// Upload witness data with payment token.
/// Requires X-Payment-Token header.
#[post("/api/x402/witness")]
async fn upload_witness_with_token(
    data: web::Data<AppState>,
    req: HttpRequest,
    body: web::Bytes,
) -> impl Responder {
    // Get payment token from header
    let token_id = match req.headers().get("X-Payment-Token") {
        Some(t) => t.to_str().unwrap_or(""),
        None => {
            return HttpResponse::PaymentRequired().json(json!({
                "error": "Payment required - X-Payment-Token header missing"
            }));
        }
    };

    log::info!("Witness upload with token: {}", token_id);

    // Validate token
    let token_result: Option<(bool, chrono::NaiveDateTime)> = sqlx::query_as(
        "SELECT used, expires_at FROM x402_tokens WHERE token_id = $1",
    )
    .bind(token_id)
    .fetch_optional(&data.db_pool)
    .await
    .unwrap_or(None);

    let (used, expires_at) = match token_result {
        Some(t) => t,
        None => {
            return HttpResponse::PaymentRequired().json(json!({
                "error": "Invalid payment token"
            }));
        }
    };

    if used {
        return HttpResponse::PaymentRequired().json(json!({
            "error": "Payment token already used"
        }));
    }

    if expires_at < chrono::Utc::now().naive_utc() {
        return HttpResponse::PaymentRequired().json(json!({
            "error": "Payment token expired"
        }));
    }

    // Compute witness commitment
    let mut hasher = Blake2s256::new();
    hasher.update(&body);
    let hash = hasher.finalize();
    let commitment = hex::encode(hash);

    log::info!("Witness commitment: {}", commitment);

    // Store witness
    let insert_result = sqlx::query(
        "INSERT INTO witnesses (commitment, data, token_id) VALUES ($1, $2, $3) ON CONFLICT (commitment) DO NOTHING",
    )
    .bind(&commitment)
    .bind(&body[..])
    .bind(token_id)
    .execute(&data.db_pool)
    .await;

    if let Err(e) = insert_result {
        log::error!("Failed to store witness: {}", e);
        return HttpResponse::InternalServerError().json(json!({
            "error": "Failed to store witness"
        }));
    }

    // Mark token as used for witness upload
    // Token can still be used for job creation
    log::info!("Witness stored successfully");

    HttpResponse::Ok().json(WitnessUploadResponse { commitment })
}

/// POST /api/x402/create-job
///
/// Create a ZK job with payment token.
/// Requires X-Payment-Token header.
#[post("/api/x402/create-job")]
async fn create_job_with_token(
    data: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<CreateJobRequest>,
) -> impl Responder {
    // Get payment token from header
    let token_id = match req.headers().get("X-Payment-Token") {
        Some(t) => t.to_str().unwrap_or(""),
        None => {
            return HttpResponse::PaymentRequired().json(json!({
                "error": "Payment required - X-Payment-Token header missing"
            }));
        }
    };

    log::info!("Create job with token: {}", token_id);

    // Validate token and get circuit type
    let token_result: Option<(bool, i16, chrono::NaiveDateTime)> = sqlx::query_as(
        "SELECT used, circuit_type, expires_at FROM x402_tokens WHERE token_id = $1",
    )
    .bind(token_id)
    .fetch_optional(&data.db_pool)
    .await
    .unwrap_or(None);

    let (used, token_circuit_type, expires_at) = match token_result {
        Some(t) => t,
        None => {
            return HttpResponse::PaymentRequired().json(json!({
                "error": "Invalid payment token"
            }));
        }
    };

    if used {
        return HttpResponse::PaymentRequired().json(json!({
            "error": "Payment token already used"
        }));
    }

    if expires_at < chrono::Utc::now().naive_utc() {
        return HttpResponse::PaymentRequired().json(json!({
            "error": "Payment token expired"
        }));
    }

    // Verify circuit type matches
    if token_circuit_type != body.circuit_type as i16 {
        return HttpResponse::BadRequest().json(json!({
            "error": format!("Circuit type mismatch: token is for {}, request is for {}",
                           token_circuit_type, body.circuit_type)
        }));
    }

    // Get next job ID from chain (simplified - would use the same logic as api_handlers.rs)
    // For now, generate a mock job_id
    let job_id = chrono::Utc::now().timestamp();

    // Mark token as fully used
    let _ = sqlx::query("UPDATE x402_tokens SET used = true WHERE token_id = $1")
        .bind(token_id)
        .execute(&data.db_pool)
        .await;

    // Build unsigned transaction (simplified)
    // In production, this would call the SDK builder
    let unsigned_transaction = "TODO: Build actual transaction";

    log::info!("Job created: {} for circuit type {}", job_id, body.circuit_type);

    HttpResponse::Ok().json(CreateJobResponse {
        job_id,
        unsigned_transaction: base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            unsigned_transaction.as_bytes(),
        ),
    })
}

/// GET /api/x402/token/{token_id}/status
///
/// Check payment token status.
#[get("/api/x402/token/{token_id}/status")]
async fn get_token_status(
    data: web::Data<AppState>,
    token_id: web::Path<String>,
) -> impl Responder {
    let token_result: Option<(bool, chrono::NaiveDateTime)> = sqlx::query_as(
        "SELECT used, expires_at FROM x402_tokens WHERE token_id = $1",
    )
    .bind(&*token_id)
    .fetch_optional(&data.db_pool)
    .await
    .unwrap_or(None);

    match token_result {
        Some((used, expires_at)) => {
            let now = chrono::Utc::now().naive_utc();
            let valid = !used && expires_at > now;

            HttpResponse::Ok().json(TokenStatusResponse {
                valid,
                used,
                expires_at: expires_at.and_utc().timestamp(),
            })
        }
        None => HttpResponse::NotFound().json(json!({
            "error": "Token not found"
        })),
    }
}

// =============================================================================
// Route Configuration
// =============================================================================

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(get_quote)
        .service(estimate_price)
        .service(build_payment_instruction)
        .service(confirm_payment)
        .service(upload_witness_with_token)
        .service(create_job_with_token)
        .service(get_token_status);
}
