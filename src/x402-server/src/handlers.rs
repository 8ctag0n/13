//! x402 HTTP Handlers
//!
//! All endpoints for the x402 anti-spam payment layer.

use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use blake2::{Blake2s256, Digest};
use serde_json::json;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use uuid::Uuid;

use crate::db::X402Queries;
use crate::models::*;
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
// Pricing Engine
// =============================================================================

/// Get price for a circuit type in lamports
fn get_circuit_price(circuit_type: u8) -> u64 {
    match circuit_type {
        // FHE circuits (0-9): Lower price
        0..=9 => 10_000_000, // 0.01 SOL

        // Core ZK primitives (10-19): Standard price
        10..=19 => 50_000_000, // 0.05 SOL

        // Voting circuits (20-29): Standard price
        20..=29 => 50_000_000, // 0.05 SOL

        // Market circuits (30-39): Higher price
        30..=39 => 75_000_000, // 0.075 SOL

        // Portfolio circuits (40-49): Premium price
        40..=49 => 100_000_000, // 0.1 SOL

        // Future circuits: Default to premium
        _ => 100_000_000, // 0.1 SOL
    }
}

// =============================================================================
// Health Check
// =============================================================================

/// GET /health
#[get("/health")]
async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(HealthResponse {
        status: "ok".to_string(),
        service: "x402-server".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

// =============================================================================
// Quote Endpoints
// =============================================================================

/// POST /api/quote
///
/// Get a price quote for a ZK/FHE operation.
#[post("/api/quote")]
async fn get_quote(data: web::Data<AppState>, req: web::Json<QuoteRequest>) -> impl Responder {
    log::info!("x402 quote request for circuit_type: {}", req.circuit_type);

    let price_lamports = get_circuit_price(req.circuit_type);
    let price_sol = price_lamports as f64 / 1_000_000_000.0;
    let quote_id = Uuid::new_v4().to_string();
    let expires_at = chrono::Utc::now().timestamp() + QUOTE_EXPIRY_SECS;

    // Store quote in database
    if let Err(e) = X402Queries::create_quote(
        &data.db_pool,
        &quote_id,
        req.circuit_type as i16,
        &req.payer,
        price_lamports as i64,
        expires_at,
    )
    .await
    {
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

/// POST /api/estimate
///
/// Get price estimate without creating a quote.
#[post("/api/estimate")]
async fn estimate_price(req: web::Json<EstimateRequest>) -> impl Responder {
    let price_lamports = get_circuit_price(req.circuit_type);
    let price_sol = price_lamports as f64 / 1_000_000_000.0;

    HttpResponse::Ok().json(EstimateResponse {
        price_lamports,
        price_sol,
    })
}

// =============================================================================
// Payment Endpoints
// =============================================================================

/// POST /api/build-payment
///
/// Build a payment instruction for a quote.
#[post("/api/build-payment")]
async fn build_payment_instruction(
    data: web::Data<AppState>,
    req: web::Json<BuildPaymentRequest>,
) -> impl Responder {
    log::info!("Building payment instruction for quote: {}", req.quote_id);

    // Validate quote exists and is not expired
    let quote_result = match X402Queries::get_quote(&data.db_pool, &req.quote_id).await {
        Ok(r) => r,
        Err(e) => {
            log::error!("Database error: {}", e);
            return HttpResponse::InternalServerError().json(json!({
                "error": "Database error"
            }));
        }
    };

    let (_circuit_type, price_lamports, _payer, expires_at) = match quote_result {
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
    let instruction =
        solana_sdk::system_instruction::transfer(&payer, &recipient, price_lamports as u64);

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

/// POST /api/confirm
///
/// Confirm payment and get a payment token.
#[post("/api/confirm")]
async fn confirm_payment(
    data: web::Data<AppState>,
    req: web::Json<ConfirmPaymentRequest>,
) -> impl Responder {
    log::info!("Confirming payment for quote: {}", req.quote_id);

    // Validate quote
    let quote_result = match X402Queries::get_quote(&data.db_pool, &req.quote_id).await {
        Ok(r) => r,
        Err(e) => {
            log::error!("Database error: {}", e);
            return HttpResponse::InternalServerError().json(json!({
                "error": "Database error"
            }));
        }
    };

    let (circuit_type, price_lamports, payer, _expires_at) = match quote_result {
        Some(q) => q,
        None => {
            return HttpResponse::NotFound().json(json!({
                "error": "Quote not found"
            }));
        }
    };

    // Generate payment token
    let token_id = Uuid::new_v4().to_string();
    let expires_at = chrono::Utc::now().timestamp() + TOKEN_EXPIRY_SECS;

    // Store token in database
    if let Err(e) = X402Queries::create_token(
        &data.db_pool,
        &token_id,
        &req.quote_id,
        &payer,
        circuit_type,
        price_lamports,
        &req.signature,
        expires_at,
    )
    .await
    {
        log::error!("Failed to store payment token: {}", e);
        return HttpResponse::InternalServerError().json(json!({
            "error": "Failed to create payment token"
        }));
    }

    // Mark quote as used
    let _ = X402Queries::mark_quote_used(&data.db_pool, &req.quote_id).await;

    log::info!("Payment confirmed, token: {}", token_id);

    HttpResponse::Ok().json(ConfirmPaymentResponse {
        token_id,
        amount_paid: price_lamports.to_string(),
        tx_signature: req.signature.clone(),
        expires_at,
    })
}

// =============================================================================
// Token Validation (Internal API for blink-server)
// =============================================================================

/// POST /api/validate
///
/// Validate a payment token. Internal endpoint for blink-server.
/// Returns token validity, circuit_type, and usage status.
#[post("/api/validate")]
async fn validate_token(
    data: web::Data<AppState>,
    req: web::Json<ValidateTokenRequest>,
) -> impl Responder {
    log::info!("Validating token: {}", req.token_id);

    let token_result = match X402Queries::validate_token(&data.db_pool, &req.token_id).await {
        Ok(r) => r,
        Err(e) => {
            log::error!("Database error: {}", e);
            return HttpResponse::InternalServerError().json(json!({
                "error": "Database error"
            }));
        }
    };

    match token_result {
        Some((used, circuit_type, expires_at)) => {
            let now = chrono::Utc::now().naive_utc();
            let expired = expires_at < now;
            let valid = !used && !expired;

            HttpResponse::Ok().json(ValidateTokenResponse {
                valid,
                circuit_type: Some(circuit_type as u8),
                used,
                expires_at: Some(expires_at.and_utc().timestamp()),
            })
        }
        None => HttpResponse::Ok().json(ValidateTokenResponse {
            valid: false,
            circuit_type: None,
            used: false,
            expires_at: None,
        }),
    }
}

/// POST /api/mark-used
///
/// Mark a token as used. Internal endpoint for blink-server.
#[post("/api/mark-used")]
async fn mark_token_used_endpoint(
    data: web::Data<AppState>,
    req: web::Json<ValidateTokenRequest>,
) -> impl Responder {
    log::info!("Marking token as used: {}", req.token_id);

    if let Err(e) = X402Queries::mark_token_used(&data.db_pool, &req.token_id).await {
        log::error!("Failed to mark token as used: {}", e);
        return HttpResponse::InternalServerError().json(json!({
            "error": "Failed to mark token as used"
        }));
    }

    HttpResponse::Ok().json(json!({
        "success": true,
        "token_id": req.token_id
    }))
}

/// GET /api/token/{token_id}/status
///
/// Check payment token status.
#[get("/api/token/{token_id}/status")]
async fn get_token_status(data: web::Data<AppState>, token_id: web::Path<String>) -> impl Responder {
    let token_result = match X402Queries::get_token_status(&data.db_pool, &*token_id).await {
        Ok(r) => r,
        Err(e) => {
            log::error!("Database error: {}", e);
            return HttpResponse::InternalServerError().json(json!({
                "error": "Database error"
            }));
        }
    };

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
// Witness Upload (with payment token)
// =============================================================================

/// POST /api/witness
///
/// Upload witness data with payment token.
#[post("/api/witness")]
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
    let token_result = match X402Queries::get_token_status(&data.db_pool, token_id).await {
        Ok(r) => r,
        Err(e) => {
            log::error!("Database error: {}", e);
            return HttpResponse::InternalServerError().json(json!({
                "error": "Database error"
            }));
        }
    };

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
    if let Err(e) = X402Queries::store_witness(&data.db_pool, &commitment, &body[..], token_id).await
    {
        log::error!("Failed to store witness: {}", e);
        return HttpResponse::InternalServerError().json(json!({
            "error": "Failed to store witness"
        }));
    }

    log::info!("Witness stored successfully");

    HttpResponse::Ok().json(WitnessUploadResponse { commitment })
}

/// POST /api/create-job
///
/// Create a ZK job with payment token.
#[post("/api/create-job")]
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
    let token_result = match X402Queries::validate_token(&data.db_pool, token_id).await {
        Ok(r) => r,
        Err(e) => {
            log::error!("Database error: {}", e);
            return HttpResponse::InternalServerError().json(json!({
                "error": "Database error"
            }));
        }
    };

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

    // Generate mock job_id
    let job_id = chrono::Utc::now().timestamp();

    // Mark token as fully used
    let _ = X402Queries::mark_token_used(&data.db_pool, token_id).await;

    log::info!(
        "Job created: {} for circuit type {}",
        job_id,
        body.circuit_type
    );

    HttpResponse::Ok().json(CreateJobResponse {
        job_id,
        unsigned_transaction: base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            "TODO: Build actual transaction".as_bytes(),
        ),
    })
}

// =============================================================================
// Route Configuration
// =============================================================================

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(health_check)
        .service(get_quote)
        .service(estimate_price)
        .service(build_payment_instruction)
        .service(confirm_payment)
        .service(validate_token)
        .service(mark_token_used_endpoint)
        .service(get_token_status);
    // Note: upload_witness and create_job removed - will be proxied to blink in Fase 2
}
