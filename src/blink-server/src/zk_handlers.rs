//! ZK Proof Job Handlers
//!
//! Endpoints for Zero-Knowledge proof operations.
//! Circuit types 10-49:
//!   10-19: Core primitives (PoI, PoR)
//!   20-29: Voting circuits
//!   30-39: Market data circuits
//!   40-49: Portfolio circuits

use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::x402_client::X402Client;
use crate::AppState;

// =============================================================================
// Request/Response Types
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct ValidateZkJobRequest {
    pub circuit_type: u8,
    pub witness_commitment: String,
    pub public_inputs: Vec<String>,
    pub creator: String,
}

#[derive(Debug, Serialize)]
pub struct ValidateZkJobResponse {
    pub job_id: i64,
    pub circuit_type: u8,
    pub transaction: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct ZkJobStatusResponse {
    pub job_id: i64,
    pub status: String,
    pub circuit_type: Option<u8>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ConfirmZkJobRequest {
    pub signature: String,
}

#[derive(Debug, Deserialize)]
pub struct ListZkJobsQuery {
    pub status: Option<String>,
    pub creator: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Validate ZK circuit_type (must be 10-49)
fn validate_zk_circuit_type(circuit_type: u8) -> Result<(), String> {
    if circuit_type < 10 || circuit_type > 49 {
        return Err(format!(
            "Invalid ZK circuit_type: {}. Must be 10-49 (10-19: Core, 20-29: Voting, 30-39: Market, 40-49: Portfolio)",
            circuit_type
        ));
    }
    Ok(())
}

/// Get circuit category name
fn get_circuit_category(circuit_type: u8) -> &'static str {
    match circuit_type {
        10..=19 => "Core (PoI, PoR)",
        20..=29 => "Voting",
        30..=39 => "Market",
        40..=49 => "Portfolio",
        _ => "Unknown",
    }
}

// =============================================================================
// API Endpoints
// =============================================================================

/// POST /api/jobs/zk/validate-and-build
///
/// Validates ZK job data and builds unsigned transaction.
/// Requires X-Payment-Token header from x402-server.
#[post("/api/jobs/zk/validate-and-build")]
async fn validate_and_build_zk_job(
    data: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<ValidateZkJobRequest>,
) -> impl Responder {
    log::info!(
        "ZK job validation request for circuit_type: {} ({}) from creator: {}",
        body.circuit_type,
        get_circuit_category(body.circuit_type),
        body.creator
    );

    // Step 1: Validate circuit_type
    if let Err(e) = validate_zk_circuit_type(body.circuit_type) {
        log::warn!("Invalid circuit_type: {}", e);
        return HttpResponse::BadRequest().json(json!({"error": e}));
    }

    // Step 2: Validate witness_commitment format (hex-encoded hash, 64 chars = 32 bytes)
    if body.witness_commitment.is_empty() || body.witness_commitment.len() != 64 {
        log::warn!(
            "Invalid witness_commitment length: {} (expected 64 hex chars)",
            body.witness_commitment.len()
        );
        return HttpResponse::BadRequest().json(json!({
            "error": "Invalid witness_commitment: must be 64 hex characters (32 bytes)"
        }));
    }

    // Step 3: Validate public_inputs (non-empty array)
    if body.public_inputs.is_empty() {
        log::warn!("Empty public_inputs array");
        return HttpResponse::BadRequest().json(json!({
            "error": "public_inputs cannot be empty"
        }));
    }

    // Step 4: Validate x402 payment token if provided
    if let Some(token_header) = req.headers().get("X-Payment-Token") {
        let token_id = token_header.to_str().unwrap_or("");
        if !token_id.is_empty() {
            let x402_client = X402Client::new(&data.x402_url);
            match x402_client.validate_token(token_id).await {
                Ok(response) => {
                    if !response.valid {
                        return HttpResponse::PaymentRequired().json(json!({
                            "error": "Payment token is invalid or already used"
                        }));
                    }
                    // Verify circuit type matches
                    if let Some(token_circuit_type) = response.circuit_type {
                        if token_circuit_type != body.circuit_type {
                            return HttpResponse::BadRequest().json(json!({
                                "error": format!(
                                    "Circuit type mismatch: token is for {}, request is for {}",
                                    token_circuit_type, body.circuit_type
                                )
                            }));
                        }
                    }
                    log::info!("x402 token validated for ZK job");
                }
                Err(e) => {
                    log::error!("Failed to validate x402 token: {}", e);
                    return HttpResponse::ServiceUnavailable().json(json!({
                        "error": "Failed to validate payment token",
                        "details": "x402-server may be unavailable"
                    }));
                }
            }
        }
    }

    // TODO: Store in database with status="pending_tx"
    // TODO: Build unsigned create_zk_job transaction

    // For now, return mock response
    let job_id = chrono::Utc::now().timestamp();

    log::info!(
        "ZK job validation successful - job_id: {}, circuit_type: {} ({})",
        job_id,
        body.circuit_type,
        get_circuit_category(body.circuit_type)
    );

    HttpResponse::Ok().json(ValidateZkJobResponse {
        job_id,
        circuit_type: body.circuit_type,
        transaction: "TODO_ZK_TRANSACTION_BASE64".to_string(),
        status: "pending_signature".to_string(),
    })
}

/// GET /api/jobs/zk/{job_id}/status
///
/// Get current status of a ZK job.
#[get("/api/jobs/zk/{job_id}/status")]
async fn get_zk_job_status(
    _data: web::Data<AppState>,
    job_id: web::Path<i64>,
) -> impl Responder {
    log::info!("Fetching ZK job status for job_id: {}", *job_id);

    // TODO: Query zk_jobs table from database
    // For now, return mock response

    HttpResponse::Ok().json(ZkJobStatusResponse {
        job_id: *job_id,
        status: "pending".to_string(),
        circuit_type: Some(10),
        created_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// POST /api/jobs/zk/{job_id}/confirm
///
/// Confirm ZK job transaction was successfully submitted on-chain.
/// Updates job status from "pending_tx" to "active".
#[post("/api/jobs/zk/{job_id}/confirm")]
async fn confirm_zk_job(
    data: web::Data<AppState>,
    job_id: web::Path<i64>,
    req: HttpRequest,
    body: web::Json<ConfirmZkJobRequest>,
) -> impl Responder {
    log::info!(
        "Confirming ZK job transaction for job_id: {} with signature: {}",
        *job_id,
        &body.signature
    );

    // Mark x402 token as used if provided
    if let Some(token_header) = req.headers().get("X-Payment-Token") {
        let token_id = token_header.to_str().unwrap_or("");
        if !token_id.is_empty() {
            let x402_client = X402Client::new(&data.x402_url);
            if let Err(e) = x402_client.mark_token_used(token_id).await {
                log::warn!("Failed to mark token as used: {}", e);
            }
        }
    }

    // TODO: Verify transaction signature on-chain
    // TODO: Update job status to "active" in database

    log::info!("ZK job {} confirmed with tx: {}", *job_id, &body.signature);

    HttpResponse::Ok().json(json!({
        "job_id": *job_id,
        "status": "active",
        "tx_signature": &body.signature,
        "message": "ZK job confirmed and ready for provers"
    }))
}

/// GET /api/jobs/zk
///
/// List ZK jobs with optional filtering.
#[get("/api/jobs/zk")]
async fn list_zk_jobs(
    _data: web::Data<AppState>,
    query: web::Query<ListZkJobsQuery>,
) -> impl Responder {
    log::info!("Listing ZK jobs with filter: {:?}", query.status);

    // TODO: Query zk_jobs table from database with filters
    // For now, return empty list

    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).clamp(1, 100);

    HttpResponse::Ok().json(json!({
        "jobs": [],
        "count": 0,
        "total": 0,
        "page": page,
        "limit": limit,
        "type": "zk"
    }))
}

// =============================================================================
// Route Configuration
// =============================================================================

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(validate_and_build_zk_job)
        .service(get_zk_job_status)
        .service(confirm_zk_job)
        .service(list_zk_jobs);
}
