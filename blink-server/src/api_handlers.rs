use actix_web::{delete, get, post, web, HttpResponse, Responder};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::json;
use solana_sdk::{message::Message, transaction::Transaction};

use crate::db::{InsertJobData, JobQueries, JobStatus};
use crate::validators::{JobValidator, ValidateJobRequest};
use crate::AppState;

// ============================================================================
// Response Types
// ============================================================================

#[derive(Debug, Serialize)]
pub struct ValidateJobResponse {
    pub job_id: i64,
    pub transaction: String, // base64 serialized Transaction
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct ComputeDataResponse {
    pub job_id: i64,
    pub encrypted_data: String, // base64
    pub server_key: String,     // base64
    pub operation: String,
    pub operation_value: i16,
}

#[derive(Debug, Deserialize)]
pub struct ConfirmJobRequest {
    pub signature: String, // Transaction signature
}

#[derive(Debug, Serialize)]
pub struct JobStatusResponse {
    pub job_id: i64,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct EstimateCostRequest {
    pub operation: String,           // "add", "multiply", "sum", etc.
    pub operation_value: Option<u8>, // Constant value for operation (optional for histogram)
    pub expected_count: Option<u16>, // For operations like Sum, Average
    pub bins: Option<u8>,            // Number of bins for Histogram
    pub required_provers: u8,        // Number of provers for consensus
}

#[derive(Debug, Serialize)]
pub struct EstimateCostResponse {
    pub operation: String,
    pub complexity_tier: u8,
    pub min_payment_lamports: u64,
    pub min_payment_sol: f64,
    pub total_min_payment_lamports: u64, // min_payment × provers
    pub total_min_payment_sol: f64,
    pub timeout_seconds: i64,
    pub estimated_compute_ms: u32,
}

#[derive(Debug, Deserialize)]
pub struct ListJobsQuery {
    pub status: Option<String>, // Optional status filter: "pending_tx", "active", "completed", "failed"
}

#[derive(Debug, Serialize)]
pub struct JobListItem {
    pub job_id: i64,
    pub creator_pubkey: String,
    pub operation: String,
    pub operation_value: i16,
    pub price_lamports: i64,
    pub required_provers: i16,
    pub consensus_threshold: i16,
    pub status: String,
    pub payment_method: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ListJobsResponse {
    pub jobs: Vec<JobListItem>,
    pub count: usize,
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
    let transaction = match build_create_job_transaction(&validated, &data.sdk_builder) {
        Ok(tx) => {
            log::info!(
                "Transaction built successfully for job_id: {}",
                validated.job_id
            );
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

    log::info!(
        "Successfully created job_id: {} (db_id: {})",
        validated.job_id,
        db_id
    );

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
async fn get_compute_data(data: web::Data<AppState>, job_id: web::Path<i64>) -> impl Responder {
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
    match JobQueries::update_job_status(&data.db_pool, *job_id, JobStatus::Active).await {
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
async fn get_job_status(data: web::Data<AppState>, job_id: web::Path<i64>) -> impl Responder {
    log::info!("Fetching status for job_id: {}", *job_id);

    match JobQueries::get_job_by_id(&data.db_pool, *job_id).await {
        Ok(Some(job)) => HttpResponse::Ok().json(JobStatusResponse {
            job_id: job.job_id,
            status: job.status,
            created_at: job.created_at.to_rfc3339(),
        }),
        Ok(None) => HttpResponse::NotFound().json(json!({
            "error": "Job not found"
        })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": format!("Database error: {}", e)
        })),
    }
}

/// DELETE /api/jobs/{job_id}
///
/// Delete job data (cleanup).
/// Only allowed if job is in terminal state (completed/failed).
#[delete("/api/jobs/{job_id}")]
async fn delete_job_data(data: web::Data<AppState>, job_id: web::Path<i64>) -> impl Responder {
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

/// GET /api/jobs
///
/// List jobs with optional status filtering.
/// Query params:
///   - status (optional): Filter by job status ("pending", "claimed", "completed", "failed", "cancelled")
///
/// Examples:
///   - GET /api/jobs                  → List all jobs
///   - GET /api/jobs?status=pending   → List only pending jobs
#[get("/api/jobs")]
async fn list_jobs(data: web::Data<AppState>, query: web::Query<ListJobsQuery>) -> impl Responder {
    log::info!("Listing jobs with filter: {:?}", query.status);

    // Build SQL query with optional status filter
    let jobs_query = if let Some(ref status_str) = query.status {
        // Validate status string
        let valid_statuses = ["pending", "claimed", "completed", "failed", "cancelled"];
        if !valid_statuses.contains(&status_str.as_str()) {
            log::warn!("Invalid status filter: {}", status_str);
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Invalid status: {}. Valid values: {}", status_str, valid_statuses.join(", "))
            }));
        }

        // Query blockchain_jobs with status filter
        sqlx::query_as!(
            JobListItem,
            r#"
            SELECT
                job_id,
                creator_pubkey,
                COALESCE(fhe_operation, circuit_type) as "operation!",
                0::smallint as "operation_value!",
                price_lamports,
                COALESCE(required_provers, 1::smallint) as "required_provers!",
                COALESCE(consensus_threshold, 1::smallint) as "consensus_threshold!",
                status as "status!",
                'SOL' as "payment_method!",
                to_char(created_at, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') as "created_at!"
            FROM blockchain_jobs
            WHERE status = $1
            ORDER BY created_at DESC
            LIMIT 100
            "#,
            status_str
        )
        .fetch_all(&data.db_pool)
        .await
    } else {
        // Query all jobs without status filter
        sqlx::query_as!(
            JobListItem,
            r#"
            SELECT
                job_id,
                creator_pubkey,
                COALESCE(fhe_operation, circuit_type) as "operation!",
                0::smallint as "operation_value!",
                price_lamports,
                COALESCE(required_provers, 1::smallint) as "required_provers!",
                COALESCE(consensus_threshold, 1::smallint) as "consensus_threshold!",
                status as "status!",
                'SOL' as "payment_method!",
                to_char(created_at, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') as "created_at!"
            FROM blockchain_jobs
            ORDER BY created_at DESC
            LIMIT 100
            "#
        )
        .fetch_all(&data.db_pool)
        .await
    };

    // Handle query result
    let job_items = match jobs_query {
        Ok(jobs) => jobs,
        Err(e) => {
            log::error!("Failed to fetch jobs from blockchain_jobs: {}", e);
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Database error: {}", e)
            }));
        }
    };

    let count = job_items.len();

    log::info!("Returning {} jobs from blockchain", count);

    HttpResponse::Ok().json(ListJobsResponse {
        jobs: job_items,
        count,
    })
}

/// POST /api/estimate-cost
///
/// Estimate the cost and timeout for a given FHE operation.
/// Helps users understand pricing before creating a job.
#[post("/api/estimate-cost")]
async fn estimate_operation_cost(req: web::Json<EstimateCostRequest>) -> impl Responder {
    use cypherlink_types::fhe::{FheOperation, HistogramBin};

    log::info!("Estimating cost for operation: {}", req.operation);

    // Parse operation from request
    let op_value = req.operation_value.unwrap_or(0);
    let operation = match req.operation.as_str() {
        "add" => FheOperation::Add(op_value),
        "multiply" => FheOperation::Multiply(op_value),
        "sum" => {
            let count = req.expected_count.unwrap_or(100);
            FheOperation::Sum {
                expected_count: count,
            }
        }
        "threshold" => FheOperation::Threshold {
            threshold: op_value,
            greater_or_equal: true,
        },
        "range_check" => FheOperation::RangeCheck {
            min: 0,
            max: op_value,
        },
        "average" => {
            let count = req.expected_count.unwrap_or(100);
            FheOperation::Average {
                expected_count: count,
            }
        }
        "count_if" => {
            let count = req.expected_count.unwrap_or(100);
            FheOperation::CountIf {
                predicate: cypherlink_types::fhe::FhePredicate::GreaterThan(op_value),
                expected_count: count,
            }
        }
        "histogram" => {
            let num_bins = req.bins.unwrap_or(5) as usize;
            let bins: Vec<HistogramBin> = (0..num_bins)
                .map(|i| {
                    HistogramBin::new(
                        i as u8 * 10,
                        (i as u8 + 1) * 10 - 1,
                        format!("Bin {}", i + 1),
                    )
                })
                .collect();
            FheOperation::Histogram { bins }
        }
        _ => {
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Unknown operation: {}", req.operation)
            }));
        }
    };

    // Get cost configuration
    let cost_config = operation.get_cost_config();
    let compute_time = operation.estimated_compute_time_ms();

    // Calculate total cost with multiple provers
    let total_min_payment = cost_config.min_payment_lamports * (req.required_provers as u64);

    // Convert to SOL (1 SOL = 1_000_000_000 lamports)
    let min_payment_sol = cost_config.min_payment_lamports as f64 / 1_000_000_000.0;
    let total_min_payment_sol = total_min_payment as f64 / 1_000_000_000.0;

    log::info!(
        "Cost estimate for {}: tier {}, {} lamports/prover, {} total",
        operation.name(),
        cost_config.complexity_tier,
        cost_config.min_payment_lamports,
        total_min_payment
    );

    HttpResponse::Ok().json(EstimateCostResponse {
        operation: operation.name().to_string(),
        complexity_tier: cost_config.complexity_tier,
        min_payment_lamports: cost_config.min_payment_lamports,
        min_payment_sol,
        total_min_payment_lamports: total_min_payment,
        total_min_payment_sol,
        timeout_seconds: cost_config.timeout_seconds,
        estimated_compute_ms: compute_time,
    })
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

    // Validate pricing against dynamic cost model
    let cost_config = operation.get_cost_config();
    let min_price_per_prover = cost_config.min_payment_lamports;
    let total_min_price = min_price_per_prover * (validated.required_provers as u64);

    if validated.price_lamports < total_min_price {
        return Err(anyhow::anyhow!(
            "Price too low for operation '{}' (tier {}): {} < {} lamports",
            operation.name(),
            cost_config.complexity_tier,
            validated.price_lamports,
            total_min_price
        ));
    }

    log::info!(
        "Price validated - Op: {}, Tier: {}, Price: {}, Min: {}",
        operation.name(),
        cost_config.complexity_tier,
        validated.price_lamports,
        total_min_price
    );

    // Use dynamic timeout from cost config
    let dynamic_timeout = cost_config.timeout_seconds;

    // Create FHE consensus config
    let fhe_config = FheConsensusConfig {
        required_provers: validated.required_provers,
        consensus_threshold: validated.consensus_threshold,
        submission_timeout_secs: dynamic_timeout,
        operation,
    };

    // Build instruction based on payment method
    let instruction = match validated.payment_method.as_str() {
        "SOL" => {
            // Use existing create_fhe_job for SOL payments with dynamic timeout
            builder.create_fhe_job(
                validated.creator,
                validated.job_id as u64,
                &validated.encrypted_data,
                fhe_config,
                validated.price_lamports,
                dynamic_timeout,
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
                dynamic_timeout,
                token_mint_pubkey,
                creator_token_account,
            )?
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Invalid payment method: {}",
                validated.payment_method
            ))
        }
    };

    // Get recent blockhash (in production, fetch from RPC)
    let recent_blockhash = solana_sdk::hash::Hash::default();

    // Build message
    let message =
        Message::new_with_blockhash(&[instruction], Some(&validated.creator), &recent_blockhash);

    // Create unsigned transaction
    let transaction = Transaction::new_unsigned(message);

    Ok(transaction)
}

// ============================================================================
// Route Configuration
// ============================================================================

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_jobs)
        .service(validate_and_build_job)
        .service(estimate_operation_cost)
        .service(get_compute_data)
        .service(confirm_job_transaction)
        .service(get_job_status)
        .service(delete_job_data);
}
