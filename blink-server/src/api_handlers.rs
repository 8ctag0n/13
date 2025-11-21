use actix_web::{delete, get, post, web, HttpResponse, Responder};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::json;
use solana_sdk::{
    message::Message,
    transaction::Transaction,
};

use crate::db::{InsertJobData, JobQueries, JobStatus};
use crate::validators::{JobValidator, ValidateJobRequest};
use crate::AppState;

// ============================================================================
// Response Types
// ============================================================================

#[derive(Debug, Serialize)]
pub struct ValidateJobResponse {
    pub job_id: i64,
    pub transaction: String,  // base64 serialized Transaction
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct ComputeDataResponse {
    pub job_id: i64,
    pub encrypted_data: String,  // base64
    pub server_key: String,      // base64
    pub operation: String,
    pub operation_value: i16,
}

#[derive(Debug, Deserialize)]
pub struct ConfirmJobRequest {
    pub signature: String,  // Transaction signature
}

#[derive(Debug, Serialize)]
pub struct JobStatusResponse {
    pub job_id: i64,
    pub status: String,
    pub created_at: String,
}

// ============================================================================
// API Endpoints
// ============================================================================

/// POST /api/jobs/validate-and-build
///
/// Validates job data and builds unsigned transaction.
/// This is step 1 of the job creation flow (backend-first).
///
/// Flow:
/// 1. Validate all data (signature, sizes, format, nonce)
/// 2. Store in database with status="pending_tx"
/// 3. Build unsigned Solana transaction
/// 4. Return transaction to client for signing
#[post("/api/jobs/validate-and-build")]
async fn validate_and_build_job(
    data: web::Data<AppState>,
    req: web::Json<ValidateJobRequest>,
) -> impl Responder {
    log::info!("Received validate-and-build request");

    // Step 1: Validate all data
    let validated = match JobValidator::validate(&req, &data.db_pool).await {
        Ok(v) => {
            log::info!("Job validation successful for job_id: {}", v.job_id);
            v
        }
        Err(e) => {
            log::warn!("Validation failed: {}", e);
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Validation failed: {}", e)
            }));
        }
    };

    // Step 2: Insert into database with status="pending_tx"
    let insert_data = InsertJobData {
        job_id: validated.job_id,
        creator_pubkey: validated.creator.to_string(),
        encrypted_data: validated.encrypted_data.clone(),
        server_key: validated.server_key.clone(),
        operation: validated.operation.clone(),
        operation_value: validated.operation_value as i16,
        price_lamports: validated.price_lamports as i64,
        required_provers: validated.required_provers as i16,
        consensus_threshold: validated.consensus_threshold as i16,
        payment_method: validated.payment_method.clone(),
        payment_token_mint: validated.payment_token_mint.clone(),
    };

    let db_id = match JobQueries::insert_pending_job(&data.db_pool, insert_data).await {
        Ok(id) => {
            log::info!("Job inserted into database with id: {}", id);
            id
        }
        Err(e) => {
            log::error!("Database insertion failed: {}", e);
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Database error: {}", e)
            }));
        }
    };

    // Step 3: Build unsigned transaction
    let transaction = match build_create_job_transaction(
        &validated,
        &data.sdk_builder,
    ) {
        Ok(tx) => {
            log::info!("Transaction built successfully for job_id: {}", validated.job_id);
            tx
        }
        Err(e) => {
            log::error!("Transaction build failed: {}", e);
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Transaction build failed: {}", e)
            }));
        }
    };

    // Step 4: Serialize and encode transaction
    let tx_bytes = match bincode::serialize(&transaction) {
        Ok(bytes) => bytes,
        Err(e) => {
            log::error!("Transaction serialization failed: {}", e);
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Serialization error: {}", e)
            }));
        }
    };

    let tx_base64 = STANDARD.encode(&tx_bytes);

    log::info!("Successfully created job_id: {} (db_id: {})", validated.job_id, db_id);

    // Step 5: Return response
    HttpResponse::Ok().json(ValidateJobResponse {
        job_id: validated.job_id,
        transaction: tx_base64,
        status: "pending_signature".to_string(),
    })
}

/// GET /api/jobs/{job_id}/compute-data
///
/// Returns compute data for provers.
/// Only returns data if job status is "active" (on-chain confirmed).
#[get("/api/jobs/{job_id}/compute-data")]
async fn get_compute_data(
    data: web::Data<AppState>,
    job_id: web::Path<i64>,
) -> impl Responder {
    log::info!("Fetching compute data for job_id: {}", *job_id);

    // Query database
    let job = match JobQueries::get_job_by_id(&data.db_pool, *job_id).await {
        Ok(Some(job)) => job,
        Ok(None) => {
            log::warn!("Job not found: {}", *job_id);
            return HttpResponse::NotFound().json(json!({
                "error": "Job not found"
            }));
        }
        Err(e) => {
            log::error!("Database query failed: {}", e);
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Database error: {}", e)
            }));
        }
    };

    // Check job status
    if job.status != JobStatus::Active.as_str() {
        log::warn!("Job {} not active, current status: {}", *job_id, job.status);
        return HttpResponse::BadRequest().json(json!({
            "error": format!("Job not ready, status: {}", job.status)
        }));
    }

    log::info!("Returning compute data for job_id: {}", *job_id);

    // Return compute data
    HttpResponse::Ok().json(ComputeDataResponse {
        job_id: job.job_id,
        encrypted_data: STANDARD.encode(&job.encrypted_data),
        server_key: STANDARD.encode(&job.server_key),
        operation: job.operation,
        operation_value: job.operation_value,
    })
}

/// POST /api/jobs/{job_id}/confirm
///
/// Webhook to confirm job transaction was successfully submitted on-chain.
/// Updates job status from "pending_tx" to "active".
///
/// Note: In production, this should verify the transaction signature on-chain
/// before updating status. For PoC, we trust the client.
#[post("/api/jobs/{job_id}/confirm")]
async fn confirm_job_transaction(
    data: web::Data<AppState>,
    job_id: web::Path<i64>,
    req: web::Json<ConfirmJobRequest>,
) -> impl Responder {
    log::info!("Confirming transaction for job_id: {}", *job_id);

    // TODO: Verify transaction signature on-chain
    // For now, we trust the client
    let _signature = &req.signature;

    // Update job status to "active"
    match JobQueries::update_job_status(
        &data.db_pool,
        *job_id,
        JobStatus::Active,
    ).await {
        Ok(_) => {
            log::info!("Job {} confirmed and activated", *job_id);
            HttpResponse::Ok().json(json!({
                "job_id": *job_id,
                "status": "active",
                "message": "Job confirmed and ready for provers"
            }))
        }
        Err(e) => {
            log::error!("Failed to update job status: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to confirm job: {}", e)
            }))
        }
    }
}

/// GET /api/jobs/{job_id}/status
///
/// Get current status of a job
#[get("/api/jobs/{job_id}/status")]
async fn get_job_status(
    data: web::Data<AppState>,
    job_id: web::Path<i64>,
) -> impl Responder {
    log::info!("Fetching status for job_id: {}", *job_id);

    match JobQueries::get_job_by_id(&data.db_pool, *job_id).await {
        Ok(Some(job)) => {
            HttpResponse::Ok().json(JobStatusResponse {
                job_id: job.job_id,
                status: job.status,
                created_at: job.created_at.to_rfc3339(),
            })
        }
        Ok(None) => {
            HttpResponse::NotFound().json(json!({
                "error": "Job not found"
            }))
        }
        Err(e) => {
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

/// DELETE /api/jobs/{job_id}
///
/// Delete job data (cleanup).
/// Only allowed if job is in terminal state (completed/failed).
#[delete("/api/jobs/{job_id}")]
async fn delete_job_data(
    data: web::Data<AppState>,
    job_id: web::Path<i64>,
) -> impl Responder {
    log::info!("Deleting job_id: {}", *job_id);

    // Check current status
    let job = match JobQueries::get_job_by_id(&data.db_pool, *job_id).await {
        Ok(Some(job)) => job,
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({
                "error": "Job not found"
            }))
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Database error: {}", e)
            }))
        }
    };

    // Only allow deletion if in terminal state
    let status = JobStatus::from_str(&job.status);
    if status != Some(JobStatus::Completed) && status != Some(JobStatus::Failed) {
        return HttpResponse::BadRequest().json(json!({
            "error": format!("Cannot delete job in status: {}", job.status)
        }));
    }

    // Delete from database
    match JobQueries::delete_job(&data.db_pool, *job_id).await {
        Ok(_) => {
            log::info!("Job {} deleted successfully", *job_id);
            HttpResponse::Ok().json(json!({
                "message": "Job deleted successfully"
            }))
        }
        Err(e) => {
            log::error!("Failed to delete job: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to delete job: {}", e)
            }))
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Build unsigned create_fhe_job transaction (supports SOL and wZEC payments)
fn build_create_job_transaction(
    validated: &crate::validators::ValidatedJob,
    builder: &cypherlink_sdk::instructions::InstructionBuilder,
) -> anyhow::Result<Transaction> {
    use cypherlink_types::{FheConsensusConfig, FheOperation};
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    // Parse operation
    let operation = match validated.operation.as_str() {
        "add" => FheOperation::Add(validated.operation_value),
        "multiply" => FheOperation::Multiply(validated.operation_value),
        _ => return Err(anyhow::anyhow!("Invalid operation")),
    };

    // Create FHE consensus config
    let fhe_config = FheConsensusConfig {
        required_provers: validated.required_provers,
        consensus_threshold: validated.consensus_threshold,
        submission_timeout_secs: 300,
        operation,
    };

    // Build instruction based on payment method
    let instruction = match validated.payment_method.as_str() {
        "SOL" => {
            // Use existing create_fhe_job for SOL payments
            builder.create_fhe_job(
                validated.creator,
                validated.job_id as u64,
                &validated.encrypted_data,
                fhe_config,
                validated.price_lamports,
                300,
            )?
        }
        "wZEC" => {
            // Use create_fhe_job_with_token for wZEC payments
            let token_mint = validated
                .payment_token_mint
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Missing payment_token_mint for wZEC payment"))?;

            let token_mint_pubkey = Pubkey::from_str(token_mint)
                .map_err(|e| anyhow::anyhow!("Invalid token mint pubkey: {}", e))?;

            // Derive creator's associated token account (using PDA derivation)
            // ATA Program ID: ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL
            let ata_program_id = Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")
                .expect("Valid ATA program ID");

            let (creator_token_account, _) = Pubkey::find_program_address(
                &[
                    validated.creator.as_ref(),
                    &Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
                        .expect("Valid token program ID")
                        .to_bytes(),
                    token_mint_pubkey.as_ref(),
                ],
                &ata_program_id,
            );

            builder.create_fhe_job_with_token(
                validated.creator,
                validated.job_id as u64,
                &validated.encrypted_data,
                fhe_config,
                validated.price_lamports, // In wZEC, this is in zatoshis
                300,
                token_mint_pubkey,
                creator_token_account,
            )?
        }
        _ => return Err(anyhow::anyhow!("Invalid payment method: {}", validated.payment_method)),
    };

    // Get recent blockhash (in production, fetch from RPC)
    let recent_blockhash = solana_sdk::hash::Hash::default();

    // Build message
    let message = Message::new_with_blockhash(
        &[instruction],
        Some(&validated.creator),
        &recent_blockhash,
    );

    // Create unsigned transaction
    let transaction = Transaction::new_unsigned(message);

    Ok(transaction)
}

// ============================================================================
// Route Configuration
// ============================================================================

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(validate_and_build_job)
        .service(get_compute_data)
        .service(confirm_job_transaction)
        .service(get_job_status)
        .service(delete_job_data);
}
