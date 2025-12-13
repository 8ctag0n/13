//! Prover Gateway Endpoints
//!
//! Public endpoints for authenticated provers to interact with the system:
//! - Download witness data
//! - Submit ZK proofs
//! - Submit FHE results

use actix_web::{get, post, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::prover_auth::ProverAuth;
use crate::AppState;

// =============================================================================
// Request/Response Models
// =============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitZkProofRequest {
    pub proof: String,           // Base64 encoded proof
    pub public_inputs: Vec<String>, // Public inputs as strings
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitZkProofResponse {
    pub success: bool,
    pub job_id: i64,
    pub prover: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitFheResultResponse {
    pub success: bool,
    pub job_id: i64,
    pub prover: String,
    pub result_size: usize,
}

// =============================================================================
// Prover Endpoints
// =============================================================================

/// GET /gateway/prover/witness/{hash}
///
/// Download witness data for a claimed job.
/// Requires prover authentication via signature.
#[get("/gateway/prover/witness/{hash}")]
pub async fn get_witness_for_prover(
    data: web::Data<AppState>,
    hash: web::Path<String>,
    auth: ProverAuth,
) -> impl Responder {
    let witness_hash = hash.into_inner();
    let prover_pubkey = auth.pubkey.to_string();

    log::info!(
        "Prover {} requesting witness: {}",
        prover_pubkey,
        witness_hash
    );

    // Verify that the prover is registered and active
    let prover_status = match data.blink_client.check_prover_status(&prover_pubkey).await {
        Ok(status) => status,
        Err(e) => {
            log::error!("Failed to check prover status: {}", e);
            return HttpResponse::ServiceUnavailable().json(json!({
                "error": "Unable to verify prover registration"
            }));
        }
    };

    if !prover_status.registered {
        log::warn!("Prover {} not registered", prover_pubkey);
        return HttpResponse::Forbidden().json(json!({
            "error": "Prover not registered",
            "pubkey": prover_pubkey
        }));
    }

    if !prover_status.active.unwrap_or(false) {
        log::warn!("Prover {} is not active", prover_pubkey);
        return HttpResponse::Forbidden().json(json!({
            "error": "Prover is not active",
            "pubkey": prover_pubkey
        }));
    }

    log::info!(
        "Prover {} verified (reputation: {})",
        prover_pubkey,
        prover_status.reputation.unwrap_or(0)
    );

    // Proxy request to blink-server
    match data.blink_client.get_witness(&witness_hash).await {
        Ok(witness_data) => {
            log::info!(
                "Witness downloaded: {} bytes for hash {}",
                witness_data.len(),
                witness_hash
            );
            HttpResponse::Ok()
                .content_type("application/octet-stream")
                .body(witness_data)
        }
        Err(e) => {
            log::error!("Failed to fetch witness from blink-server: {}", e);

            // Determine error type
            if e.is_timeout() {
                HttpResponse::GatewayTimeout().json(json!({
                    "error": "Blink server timeout"
                }))
            } else if e.status().map_or(false, |s| s == 404) {
                HttpResponse::NotFound().json(json!({
                    "error": "Witness not found"
                }))
            } else {
                HttpResponse::BadGateway().json(json!({
                    "error": "Failed to fetch witness from backend"
                }))
            }
        }
    }
}

/// POST /gateway/prover/zk/{job_id}/submit
///
/// Submit a verified ZK proof for a job.
/// Requires prover authentication via signature.
#[post("/gateway/prover/zk/{job_id}/submit")]
pub async fn submit_zk_proof(
    data: web::Data<AppState>,
    job_id: web::Path<i64>,
    body: web::Json<SubmitZkProofRequest>,
    auth: ProverAuth,
) -> impl Responder {
    let job_id = job_id.into_inner();
    let prover_pubkey = auth.pubkey.to_string();

    log::info!(
        "Prover {} submitting ZK proof for job {}",
        prover_pubkey,
        job_id
    );

    // Verify that the prover is registered and active
    let prover_status = match data.blink_client.check_prover_status(&prover_pubkey).await {
        Ok(status) => status,
        Err(e) => {
            log::error!("Failed to check prover status: {}", e);
            return HttpResponse::ServiceUnavailable().json(json!({
                "error": "Unable to verify prover registration"
            }));
        }
    };

    if !prover_status.registered {
        log::warn!("Prover {} not registered", prover_pubkey);
        return HttpResponse::Forbidden().json(json!({
            "error": "Prover not registered",
            "pubkey": prover_pubkey
        }));
    }

    if !prover_status.active.unwrap_or(false) {
        log::warn!("Prover {} is not active", prover_pubkey);
        return HttpResponse::Forbidden().json(json!({
            "error": "Prover is not active",
            "pubkey": prover_pubkey
        }));
    }

    log::info!(
        "Prover {} verified (reputation: {})",
        prover_pubkey,
        prover_status.reputation.unwrap_or(0)
    );

    // Validate proof data is present
    if body.proof.is_empty() {
        return HttpResponse::BadRequest().json(json!({
            "error": "Proof data cannot be empty"
        }));
    }

    // Proxy request to blink-server
    match data
        .blink_client
        .submit_zk_proof(job_id, &prover_pubkey, &body.into_inner())
        .await
    {
        Ok(_) => {
            log::info!("ZK proof submitted successfully for job {}", job_id);
            HttpResponse::Ok().json(SubmitZkProofResponse {
                success: true,
                job_id,
                prover: prover_pubkey,
            })
        }
        Err(e) => {
            log::error!("Failed to submit proof to blink-server: {}", e);

            if e.is_timeout() {
                HttpResponse::GatewayTimeout().json(json!({
                    "error": "Blink server timeout"
                }))
            } else if e.status().map_or(false, |s| s == 404) {
                HttpResponse::NotFound().json(json!({
                    "error": "Job not found"
                }))
            } else if e.status().map_or(false, |s| s == 400) {
                HttpResponse::BadRequest().json(json!({
                    "error": "Invalid proof data"
                }))
            } else {
                HttpResponse::BadGateway().json(json!({
                    "error": "Failed to submit proof to backend"
                }))
            }
        }
    }
}

/// POST /gateway/prover/fhe/{job_id}/submit
///
/// Submit FHE computation result for a job.
/// Requires prover authentication via signature.
/// Expects binary data in request body.
#[post("/gateway/prover/fhe/{job_id}/submit")]
pub async fn submit_fhe_result(
    data: web::Data<AppState>,
    job_id: web::Path<i64>,
    body: web::Bytes,
    auth: ProverAuth,
) -> impl Responder {
    let job_id = job_id.into_inner();
    let prover_pubkey = auth.pubkey.to_string();

    log::info!(
        "Prover {} submitting FHE result for job {} ({} bytes)",
        prover_pubkey,
        job_id,
        body.len()
    );

    // Verify that the prover is registered and active
    let prover_status = match data.blink_client.check_prover_status(&prover_pubkey).await {
        Ok(status) => status,
        Err(e) => {
            log::error!("Failed to check prover status: {}", e);
            return HttpResponse::ServiceUnavailable().json(json!({
                "error": "Unable to verify prover registration"
            }));
        }
    };

    if !prover_status.registered {
        log::warn!("Prover {} not registered", prover_pubkey);
        return HttpResponse::Forbidden().json(json!({
            "error": "Prover not registered",
            "pubkey": prover_pubkey
        }));
    }

    if !prover_status.active.unwrap_or(false) {
        log::warn!("Prover {} is not active", prover_pubkey);
        return HttpResponse::Forbidden().json(json!({
            "error": "Prover is not active",
            "pubkey": prover_pubkey
        }));
    }

    log::info!(
        "Prover {} verified (reputation: {})",
        prover_pubkey,
        prover_status.reputation.unwrap_or(0)
    );

    // Validate result data is present
    if body.is_empty() {
        return HttpResponse::BadRequest().json(json!({
            "error": "FHE result data cannot be empty"
        }));
    }

    let result_size = body.len();

    // Proxy request to blink-server
    match data
        .blink_client
        .submit_fhe_result(job_id, &prover_pubkey, &body)
        .await
    {
        Ok(_) => {
            log::info!("FHE result submitted successfully for job {}", job_id);
            HttpResponse::Ok().json(SubmitFheResultResponse {
                success: true,
                job_id,
                prover: prover_pubkey,
                result_size,
            })
        }
        Err(e) => {
            log::error!("Failed to submit FHE result to blink-server: {}", e);

            if e.is_timeout() {
                HttpResponse::GatewayTimeout().json(json!({
                    "error": "Blink server timeout"
                }))
            } else if e.status().map_or(false, |s| s == 404) {
                HttpResponse::NotFound().json(json!({
                    "error": "Job not found"
                }))
            } else if e.status().map_or(false, |s| s == 400) {
                HttpResponse::BadRequest().json(json!({
                    "error": "Invalid FHE result data"
                }))
            } else {
                HttpResponse::BadGateway().json(json!({
                    "error": "Failed to submit result to backend"
                }))
            }
        }
    }
}

// =============================================================================
// Route Configuration
// =============================================================================

pub fn configure_prover_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(get_witness_for_prover)
        .service(submit_zk_proof)
        .service(submit_fhe_result);
}
