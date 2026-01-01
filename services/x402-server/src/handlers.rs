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
use crate::models::{
    BuildPaymentRequest, BuildPaymentResponse, Chain, ConfirmPaymentRequest, ConfirmPaymentResponse,
    CreateJobRequest, CreateJobResponse, CreateLoanRequest, CreateLoanResponse, EstimateRequest,
    EstimateResponse, HealthResponse, QuoteRequest, QuoteResponse, TokenStatusResponse,
    ValidateTokenRequest, ValidateTokenResponse, WitnessUploadResponse,
};
use crate::AppState;

// =============================================================================
// Constants
// =============================================================================

/// Quote expiration time (5 minutes)
const QUOTE_EXPIRY_SECS: i64 = 300;

/// Payment token expiration time (24 hours)
const TOKEN_EXPIRY_SECS: i64 = 24 * 60 * 60;

/// Protocol fee recipient (treasury)
const PROTOCOL_FEE_RECIPIENT: &str = "CfNa1QN6d933A986qd8VywkZkpNEQJoGSA1V7upbdmDM";

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
// pBTCFi Loan Creation (Multi-Chain)
// =============================================================================

/// POST /api/pbtcfi/create-loan
///
/// Create a pBTCFi loan with encrypted collateral.
/// Validates the deposit TX according to the chain, stores ciphertext,
/// and initiates the loan creation on the target chain.
#[post("/api/pbtcfi/create-loan")]
async fn create_pbtcfi_loan(
    data: web::Data<AppState>,
    req: web::Json<CreateLoanRequest>,
) -> impl Responder {
    log::info!(
        "pBTCFi loan creation request - chain: {:?}, borrower: {}",
        req.chain,
        req.borrower
    );

    // Validate borrower address format based on chain
    if let Err(e) = validate_address_for_chain(&req.borrower, &req.chain) {
        return HttpResponse::BadRequest().json(json!({
            "error": format!("Invalid borrower address: {}", e)
        }));
    }

    // Decode ciphertext from base64
    let ciphertext_bytes = match base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &req.ciphertext,
    ) {
        Ok(b) => b,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Invalid ciphertext encoding: {}", e)
            }));
        }
    };

    log::info!("Ciphertext size: {} bytes", ciphertext_bytes.len());

    // Compute witness commitment (Blake2s hash of ciphertext)
    let mut hasher = Blake2s256::new();
    hasher.update(&ciphertext_bytes);
    let hash = hasher.finalize();
    let commitment = hex::encode(hash);

    log::info!("Witness commitment: {}", commitment);

    // Validate signed deposit TX based on chain
    let deposit_tx_hash = match validate_deposit_tx(&req.signed_deposit_tx, &req.chain).await {
        Ok(hash) => hash,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Invalid deposit transaction: {}", e)
            }));
        }
    };

    log::info!("Deposit TX validated: {}", deposit_tx_hash);

    // Store ciphertext in blink-server via internal API
    match store_witness_in_blink(&data.blink_client, &commitment, &ciphertext_bytes).await {
        Ok(_) => {
            log::info!("Ciphertext stored successfully");
        }
        Err(e) => {
            log::error!("Failed to store ciphertext: {}", e);
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to store ciphertext: {}", e)
            }));
        }
    }

    // Create loan in blink-server (which will sync to Cairo/Starknet)
    let loan_id = match create_loan_in_blink(
        &data.blink_client,
        &req.chain,
        &req.borrower,
        &commitment,
        &deposit_tx_hash,
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            log::error!("Failed to create loan: {}", e);
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to create loan: {}", e)
            }));
        }
    };

    log::info!("pBTCFi loan created: {}", loan_id);

    HttpResponse::Ok().json(CreateLoanResponse {
        loan_id,
        chain: req.chain.as_str().to_string(),
        witness_commitment: commitment,
        deposit_tx_hash,
        status: "pending_fhe".to_string(),
    })
}

/// Validate address format for a specific chain
fn validate_address_for_chain(address: &str, chain: &Chain) -> Result<(), String> {
    match chain {
        Chain::Solana => {
            // Solana addresses are base58 encoded, 32-44 chars
            if address.len() < 32 || address.len() > 44 {
                return Err("Solana address must be 32-44 characters".to_string());
            }
            // Try to parse as Pubkey
            Pubkey::from_str(address)
                .map_err(|e| format!("Invalid Solana address: {}", e))?;
            Ok(())
        }
        Chain::Starknet => {
            // Starknet addresses are 0x prefixed hex, 64+ chars
            if !address.starts_with("0x") {
                return Err("Starknet address must start with 0x".to_string());
            }
            let hex_part = address.trim_start_matches("0x");
            if hex_part.len() < 60 || hex_part.len() > 66 {
                return Err("Starknet address must be 64-66 hex chars after 0x".to_string());
            }
            // Validate hex
            hex::decode(hex_part).map_err(|e| format!("Invalid hex: {}", e))?;
            Ok(())
        }
        Chain::Aptos => {
            // Aptos addresses are 0x prefixed hex, 64 chars
            if !address.starts_with("0x") {
                return Err("Aptos address must start with 0x".to_string());
            }
            let hex_part = address.trim_start_matches("0x");
            if hex_part.len() != 64 {
                return Err("Aptos address must be 64 hex chars after 0x".to_string());
            }
            hex::decode(hex_part).map_err(|e| format!("Invalid hex: {}", e))?;
            Ok(())
        }
    }
}

/// Validate deposit transaction based on chain
/// Returns the transaction hash if valid
async fn validate_deposit_tx(signed_tx: &str, chain: &Chain) -> Result<String, String> {
    match chain {
        Chain::Solana => {
            // For Solana, signed_tx is base64 encoded transaction
            // In production: decode, verify signature, check accounts
            // For MVP: just validate format and return a mock hash
            let _tx_bytes = base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                signed_tx,
            )
            .map_err(|e| format!("Invalid base64: {}", e))?;

            // TODO: Actually verify Solana transaction
            // - Deserialize transaction
            // - Verify signatures
            // - Check it's a transfer to the pool

            // Return mock hash for now
            let mut hasher = Blake2s256::new();
            hasher.update(signed_tx.as_bytes());
            Ok(hex::encode(hasher.finalize()))
        }
        Chain::Starknet => {
            // For Starknet, signed_tx is hex encoded
            // In production: verify ECDSA/Pedersen signature
            if !signed_tx.starts_with("0x") {
                return Err("Starknet TX must start with 0x".to_string());
            }

            // TODO: Actually verify Starknet transaction
            // - Parse invoke transaction
            // - Verify signature
            // - Check it calls the deposit function

            // Return mock hash for now
            let mut hasher = Blake2s256::new();
            hasher.update(signed_tx.as_bytes());
            Ok(format!("0x{}", hex::encode(hasher.finalize())))
        }
        Chain::Aptos => {
            // For Aptos, similar to Starknet
            if !signed_tx.starts_with("0x") {
                return Err("Aptos TX must start with 0x".to_string());
            }

            // TODO: Actually verify Aptos transaction

            let mut hasher = Blake2s256::new();
            hasher.update(signed_tx.as_bytes());
            Ok(format!("0x{}", hex::encode(hasher.finalize())))
        }
    }
}

/// Store witness/ciphertext in blink-server
async fn store_witness_in_blink(
    blink_client: &crate::blink_client::BlinkClient,
    commitment: &str,
    ciphertext: &[u8],
) -> Result<(), String> {
    blink_client
        .store_witness(commitment, ciphertext)
        .await
        .map_err(|e| e.to_string())
}

/// Create loan in blink-server
async fn create_loan_in_blink(
    blink_client: &crate::blink_client::BlinkClient,
    chain: &Chain,
    borrower: &str,
    commitment: &str,
    deposit_tx_hash: &str,
) -> Result<String, String> {
    blink_client
        .create_pbtcfi_loan(chain.as_str(), borrower, commitment, deposit_tx_hash)
        .await
        .map_err(|e| e.to_string())
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
        .service(get_token_status)
        // pBTCFi endpoints
        .service(create_pbtcfi_loan);
}
