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

use crate::db::{InsertZkJobData, ZkJobQueries, ZkJobStatus};
use crate::x402_client::X402Client;
use crate::AppState;

// =============================================================================
// Constants
// =============================================================================

/// Default price for ZK operations (lamports)
fn get_zk_price(circuit_type: u8) -> i64 {
    match circuit_type {
        10..=19 => 50_000_000,  // 0.05 SOL - Core
        20..=29 => 50_000_000,  // 0.05 SOL - Voting
        30..=39 => 75_000_000,  // 0.075 SOL - Market
        40..=49 => 100_000_000, // 0.1 SOL - Portfolio
        _ => 100_000_000,
    }
}

// =============================================================================
// Request/Response Types
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct ValidateZkJobRequest {
    pub circuit_type: u8,
    pub witness_commitment: String,
    pub public_inputs: Vec<String>,
    pub creator: String,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: i32,
}

fn default_timeout() -> i32 {
    3600
}

#[derive(Debug, Serialize)]
pub struct ValidateZkJobResponse {
    pub job_id: i64,
    pub circuit_type: u8,
    pub transaction: String,
    pub status: String,
    pub price_lamports: i64,
}

#[derive(Debug, Serialize)]
pub struct ZkJobStatusResponse {
    pub job_id: i64,
    pub status: String,
    pub circuit_type: i16,
    pub witness_commitment: String,
    pub proof_hash: Option<String>,
    pub prover_pubkey: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ConfirmZkJobRequest {
    pub signature: String,
}

#[derive(Debug, Deserialize)]
pub struct ListZkJobsQuery {
    pub status: Option<String>,
    pub creator: Option<String>,
    pub circuit_type: Option<i16>,
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

/// Generate next job_id (timestamp-based for now)
fn generate_job_id() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

// =============================================================================
// API Endpoints
// =============================================================================

/// POST /api/jobs/zk/validate-and-build
///
/// Validates ZK job data and stores in database.
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
    if body.witness_commitment.len() != 64 {
        log::warn!(
            "Invalid witness_commitment length: {} (expected 64 hex chars)",
            body.witness_commitment.len()
        );
        return HttpResponse::BadRequest().json(json!({
            "error": "Invalid witness_commitment: must be 64 hex characters (32 bytes)"
        }));
    }

    // Validate hex format
    if hex::decode(&body.witness_commitment).is_err() {
        return HttpResponse::BadRequest().json(json!({
            "error": "Invalid witness_commitment: must be valid hex string"
        }));
    }

    // Step 3: Validate public_inputs (non-empty array)
    if body.public_inputs.is_empty() {
        log::warn!("Empty public_inputs array");
        return HttpResponse::BadRequest().json(json!({
            "error": "public_inputs cannot be empty"
        }));
    }

    // Step 4: Extract x402 token if provided
    let x402_token_id = req
        .headers()
        .get("X-Payment-Token")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty());

    // Step 5: Validate x402 payment token if provided
    if let Some(ref token_id) = x402_token_id {
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
                log::warn!("Failed to validate x402 token (continuing without): {}", e);
                // Don't fail - x402-server might be down, allow job creation anyway
            }
        }
    }

    // Step 6: Generate job_id and calculate price
    let job_id = generate_job_id();
    let price_lamports = get_zk_price(body.circuit_type);

    // Step 7: Insert into database
    let insert_data = InsertZkJobData {
        job_id,
        creator_pubkey: body.creator.clone(),
        circuit_type: body.circuit_type as i16,
        witness_commitment: body.witness_commitment.clone(),
        public_inputs: serde_json::json!(body.public_inputs),
        x402_token_id,
        price_lamports,
        timeout_seconds: body.timeout_seconds,
    };

    match ZkJobQueries::insert_job(&data.db_pool, insert_data).await {
        Ok(inserted_job_id) => {
            log::info!(
                "ZK job created - job_id: {}, circuit_type: {} ({})",
                inserted_job_id,
                body.circuit_type,
                get_circuit_category(body.circuit_type)
            );

            HttpResponse::Ok().json(ValidateZkJobResponse {
                job_id: inserted_job_id,
                circuit_type: body.circuit_type,
                transaction: "TODO_ZK_TRANSACTION_BASE64".to_string(), // TODO: Build actual tx
                status: ZkJobStatus::PendingTx.as_str().to_string(),
                price_lamports,
            })
        }
        Err(e) => {
            log::error!("Failed to insert ZK job: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to create ZK job: {}", e)
            }))
        }
    }
}

/// GET /api/jobs/zk/{job_id}/status
///
/// Get current status of a ZK job.
#[get("/api/jobs/zk/{job_id}/status")]
async fn get_zk_job_status(
    data: web::Data<AppState>,
    job_id: web::Path<i64>,
) -> impl Responder {
    log::info!("Fetching ZK job status for job_id: {}", *job_id);

    match ZkJobQueries::get_job_by_id(&data.db_pool, *job_id).await {
        Ok(Some(job)) => {
            HttpResponse::Ok().json(ZkJobStatusResponse {
                job_id: job.job_id,
                status: job.status,
                circuit_type: job.circuit_type,
                witness_commitment: job.witness_commitment,
                proof_hash: job.proof_hash,
                prover_pubkey: job.prover_pubkey,
                created_at: job.created_at.to_string(),
                updated_at: job.updated_at.to_string(),
            })
        }
        Ok(None) => {
            HttpResponse::NotFound().json(json!({
                "error": "ZK job not found",
                "job_id": *job_id
            }))
        }
        Err(e) => {
            log::error!("Failed to fetch ZK job: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
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

    // Confirm job in database
    match ZkJobQueries::confirm_job(&data.db_pool, *job_id, &body.signature).await {
        Ok(()) => {
            // Mark x402 token as used if provided
            if let Some(token_header) = req.headers().get("X-Payment-Token") {
                if let Ok(token_id) = token_header.to_str() {
                    if !token_id.is_empty() {
                        let x402_client = X402Client::new(&data.x402_url);
                        if let Err(e) = x402_client.mark_token_used(token_id).await {
                            log::warn!("Failed to mark x402 token as used: {}", e);
                        }
                    }
                }
            }

            log::info!("ZK job {} confirmed with tx: {}", *job_id, &body.signature);

            HttpResponse::Ok().json(json!({
                "job_id": *job_id,
                "status": ZkJobStatus::Active.as_str(),
                "tx_signature": &body.signature,
                "message": "ZK job confirmed and ready for provers"
            }))
        }
        Err(e) => {
            log::error!("Failed to confirm ZK job: {}", e);
            HttpResponse::BadRequest().json(json!({
                "error": format!("Failed to confirm job: {}", e)
            }))
        }
    }
}

/// GET /api/jobs/zk
///
/// List ZK jobs with optional filtering.
#[get("/api/jobs/zk")]
async fn list_zk_jobs(
    data: web::Data<AppState>,
    query: web::Query<ListZkJobsQuery>,
) -> impl Responder {
    log::info!("Listing ZK jobs with filter: {:?}", query.status);

    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * limit;

    // Get jobs
    let jobs_result = ZkJobQueries::list_jobs(
        &data.db_pool,
        query.status.as_deref(),
        query.creator.as_deref(),
        query.circuit_type,
        limit,
        offset,
    )
    .await;

    // Get total count
    let count_result = ZkJobQueries::count_jobs(&data.db_pool, query.status.as_deref()).await;

    match (jobs_result, count_result) {
        (Ok(jobs), Ok(total)) => {
            let jobs_json: Vec<serde_json::Value> = jobs
                .iter()
                .map(|j| {
                    json!({
                        "job_id": j.job_id,
                        "creator_pubkey": j.creator_pubkey,
                        "circuit_type": j.circuit_type,
                        "status": j.status,
                        "price_lamports": j.price_lamports,
                        "created_at": j.created_at.to_string()
                    })
                })
                .collect();

            HttpResponse::Ok().json(json!({
                "jobs": jobs_json,
                "count": jobs.len(),
                "total": total,
                "page": page,
                "limit": limit,
                "type": "zk"
            }))
        }
        (Err(e), _) | (_, Err(e)) => {
            log::error!("Failed to list ZK jobs: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

/// GET /api/jobs/zk/{job_id}
///
/// Get full details of a ZK job.
#[get("/api/jobs/zk/{job_id}")]
async fn get_zk_job_details(
    data: web::Data<AppState>,
    job_id: web::Path<i64>,
) -> impl Responder {
    log::info!("Fetching ZK job details for job_id: {}", *job_id);

    match ZkJobQueries::get_job_by_id(&data.db_pool, *job_id).await {
        Ok(Some(job)) => {
            HttpResponse::Ok().json(json!({
                "job_id": job.job_id,
                "creator_pubkey": job.creator_pubkey,
                "circuit_type": job.circuit_type,
                "circuit_category": get_circuit_category(job.circuit_type as u8),
                "witness_commitment": job.witness_commitment,
                "public_inputs": job.public_inputs,
                "proof_hash": job.proof_hash,
                "status": job.status,
                "price_lamports": job.price_lamports,
                "x402_token_id": job.x402_token_id,
                "create_tx_signature": job.create_tx_signature,
                "confirm_tx_signature": job.confirm_tx_signature,
                "prover_pubkey": job.prover_pubkey,
                "claimed_at": job.claimed_at.map(|t| t.to_string()),
                "timeout_seconds": job.timeout_seconds,
                "created_at": job.created_at.to_string(),
                "updated_at": job.updated_at.to_string(),
                "completed_at": job.completed_at.map(|t| t.to_string())
            }))
        }
        Ok(None) => {
            HttpResponse::NotFound().json(json!({
                "error": "ZK job not found",
                "job_id": *job_id
            }))
        }
        Err(e) => {
            log::error!("Failed to fetch ZK job: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

// =============================================================================
// Route Configuration
// =============================================================================

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(validate_and_build_zk_job)
        .service(get_zk_job_status)
        .service(confirm_zk_job)
        .service(list_zk_jobs)
        .service(get_zk_job_details);
}
