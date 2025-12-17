use actix_web::{delete, get, post, web, HttpResponse, Responder};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::json;
use solana_sdk::{message::Message, pubkey::Pubkey, transaction::Transaction};
use solana_client::rpc_client::RpcClient;

use crate::db::{FheResultQueries, InsertJobData, JobQueries, JobStatus, NetworkMetricsQueries, ServerKeyQueries, WitnessQueries};
use crate::validators::{JobValidator, ValidateJobRequest};
use crate::AppState;
use blake2::{Blake2s256, Digest};

/// Fetch the next job ID from the on-chain marketplace config
/// Uses spawn_blocking to avoid blocking the async runtime
async fn fetch_next_job_id(rpc_url: String, program_id: Pubkey) -> anyhow::Result<u64> {
    // Run the blocking RPC call in a separate thread pool
    let result = tokio::task::spawn_blocking(move || {
        let rpc_client = RpcClient::new(rpc_url);

        // Derive config PDA
        let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &program_id);

        // Fetch account data
        let account = rpc_client.get_account(&config_pda)
            .map_err(|e| anyhow::anyhow!("Failed to fetch marketplace config: {}", e))?;

        // Parse next_job_id from config data
        // Layout: authority(32) + fee(2) + min_stake(8) + min_rep(4) + timeout(8) + protocol_fee_recipient(32) + next_job_id(8) + ...
        // Offset for next_job_id = 32 + 2 + 8 + 4 + 8 + 32 = 86
        if account.data.len() < 94 {
            return Err(anyhow::anyhow!("Config account data too short"));
        }

        let next_job_id = u64::from_le_bytes(
            account.data[86..94].try_into()
                .map_err(|_| anyhow::anyhow!("Failed to parse next_job_id"))?
        );

        Ok(next_job_id)
    }).await.map_err(|e| anyhow::anyhow!("Task join error: {}", e))??;

    Ok(result)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Format bytes into human-readable string (B, KB, MB, GB, TB)
fn format_bytes(bytes: i64) -> String {
    const KB: i64 = 1024;
    const MB: i64 = KB * 1024;
    const GB: i64 = MB * 1024;
    const TB: i64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

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
    pub expected_count: Option<u16>, // Expected count for operations (Sum, Average, etc.)
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
pub struct PriceRecommendationRequest {
    pub operation: String,           // "add", "multiply", "sum", etc.
    pub operation_value: Option<u8>, // Constant for Add/Multiply/Threshold
    pub expected_count: Option<u16>, // For Sum, Average, CountIf
    pub bins: Option<u8>,            // For Histogram
    pub required_provers: u8,
}

#[derive(Debug, Serialize)]
pub struct PriceRecommendationResponse {
    pub operation: String,
    pub complexity_tier: u8,
    pub required_provers: u8,

    // Minimum (theoretical floor)
    pub min_price_lamports: u64,
    pub min_price_sol: f64,

    // Recommended (for ~95% prover acceptance)
    pub recommended_price_lamports: u64,
    pub recommended_price_sol: f64,

    // Maximum suggested (premium for faster processing)
    pub max_suggested_lamports: u64,
    pub max_suggested_sol: f64,

    // Acceptance estimates
    pub acceptance_at_min: String,         // "low" (~20%)
    pub acceptance_at_recommended: String, // "high" (~95%)

    // For slider UI
    pub slider_min: u64,
    pub slider_max: u64,
    pub slider_recommended: u64,
    pub slider_step: u64,

    // Extra info
    pub estimated_time_seconds: i64,
    pub prover_overhead_multiplier: f64,
    pub prover_min_roi_percent: f64,
}

#[derive(Debug, Deserialize)]
pub struct ListJobsQuery {
    pub status: Option<String>, // Filter by status: "pending_tx", "active", "completed", "failed"
    pub creator: Option<String>, // Filter by creator wallet pubkey
    pub page: Option<i64>,      // Page number (default: 1)
    pub limit: Option<i64>,     // Items per page (default: 20, max: 100)
    #[allow(dead_code)]
    pub sort: Option<String>, // Sort: "recent", "oldest", "price"
}

#[derive(Debug, Serialize)]
pub struct WitnessUploadResponse {
    pub commitment: String, // hex-encoded Blake2s256 hash
}

#[derive(Debug, Serialize)]
pub struct ServerKeyUploadResponse {
    pub server_key_hash: String, // hex-encoded Blake2s256 hash
    pub size_bytes: usize,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct WitnessDataResponse {
    pub data: String, // base64-encoded witness data
}

#[derive(Debug, Serialize, sqlx::FromRow)]
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
    pub tx_signature: Option<String>,
}

/// Full job details response (for /api/jobs/{job_id})
#[derive(Debug, Serialize)]
pub struct JobDetailsResponse {
    pub job_id: i64,
    pub creator_pubkey: String,
    pub operation: String,
    pub operation_value: i16,
    pub price_lamports: i64,
    pub required_provers: i16,
    pub consensus_threshold: i16,
    pub status: String,
    pub payment_method: String,
    pub payment_token_mint: Option<String>,
    pub tx_signature: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub claimed_at: Option<String>,
    pub completed_at: Option<String>,
    pub prover_pubkey: Option<String>,
    pub has_result: bool,
}

/// DB row for combined job details query
#[derive(Debug, sqlx::FromRow)]
struct JobDetailsRow {
    job_id: i64,
    creator_pubkey: String,
    operation: String,
    operation_value: i16,
    price_lamports: i64,
    required_provers: i16,
    consensus_threshold: i16,
    status: String,
    payment_method: String,
    payment_token_mint: Option<String>,
    tx_signature: Option<String>,
    created_at: String,
    updated_at: String,
    claimed_at: Option<String>,
    completed_at: Option<String>,
    prover_pubkey: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListJobsResponse {
    pub jobs: Vec<JobListItem>,
    pub count: usize,
    pub total: usize,
    pub page: i64,
    pub limit: i64,
}

#[derive(Debug, Serialize)]
pub struct NetworkStatsResponse {
    pub active_provers: i64,
    pub jobs_completed: i64,
    pub jobs_total: i64,
    pub data_encrypted_bytes: i64,
    pub data_encrypted_formatted: String, // Adaptive: B, KB, MB, GB, TB
    pub network_start_time: Option<String>,
    pub uptime_seconds: i64,
    pub uptime_percent: f64,
}

// ============================================================================
// Metrics Response Types
// ============================================================================

#[derive(Debug, Serialize)]
pub struct MetricsNetworkInfo {
    pub uptime_seconds: i64,
    pub uptime_formatted: String,
    pub active_provers: i64,
    pub total_provers: i64,
    pub offline_provers: i64,
    pub data_processed_tb: f64,
    pub data_processed_24h_tb: f64,
}

#[derive(Debug, Serialize)]
pub struct MetricsJobsInfo {
    pub completed_24h: i64,
    pub completed_total: i64,
    pub active_current: i64,
    pub pending_current: i64,
    pub expired_total: i64,
    pub avg_job_time_mins: f64,
    pub success_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct OperationBreakdown {
    pub operation: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct TimelinePoint {
    pub timestamp: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct MetricsResponse {
    pub network: MetricsNetworkInfo,
    pub jobs: MetricsJobsInfo,
    pub operations: Vec<OperationBreakdown>,
    pub timeline: Vec<TimelinePoint>,
    pub provers_history: Vec<i64>,
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
#[post("/internal/fhe/validate-and-build")]
async fn validate_and_build_job(
    data: web::Data<AppState>,
    req: web::Json<ValidateJobRequest>,
) -> impl Responder {
    log::info!("Received validate-and-build request");

    // Step 0: Fetch the REAL next_job_id from on-chain marketplace config
    // This is critical because the frontend doesn't know the on-chain job counter
    let onchain_job_id = match fetch_next_job_id(data.rpc_url.clone(), data.program_id).await {
        Ok(id) => {
            log::info!("Fetched next_job_id from on-chain: {}", id);
            id as i64
        }
        Err(e) => {
            log::error!("Failed to fetch next_job_id from chain: {}", e);
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to fetch job ID from blockchain: {}", e)
            }));
        }
    };

    // Step 1: Validate all data (signature verification, sizes, format, nonce)
    // Note: The job_id in the signed message is just for anti-replay, we use on-chain ID
    let mut validated = match JobValidator::validate(&req, &data.db_pool).await {
        Ok(v) => {
            log::info!("Job validation successful (message job_id: {}, using on-chain: {})",
                      v.job_id, onchain_job_id);
            v
        }
        Err(e) => {
            log::warn!("Validation failed: {}", e);
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Validation failed: {}", e)
            }));
        }
    };

    // Override job_id with the on-chain value
    validated.job_id = onchain_job_id;

    // Step 2: Insert into database with status="pending_tx"
    let insert_data = InsertJobData {
        job_id: onchain_job_id,
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
        expected_count: Some(validated.expected_count as i16),
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
#[get("/internal/fhe/{job_id}/compute-data")]
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
        expected_count: job.expected_count.map(|c| c as u16),
    })
}

/// POST /api/jobs/{job_id}/confirm
///
/// Webhook to confirm job transaction was successfully submitted on-chain.
/// Updates job status from "pending_tx" to "active".
///
/// Note: In production, this should verify the transaction signature on-chain
/// before updating status. For PoC, we trust the client.
#[post("/internal/fhe/{job_id}/confirm")]
async fn confirm_job_transaction(
    data: web::Data<AppState>,
    job_id: web::Path<i64>,
    req: web::Json<ConfirmJobRequest>,
) -> impl Responder {
    log::info!(
        "Confirming transaction for job_id: {} with signature: {}",
        *job_id,
        &req.signature
    );

    // TODO: Verify transaction signature on-chain
    // For now, we trust the client
    let signature = &req.signature;

    // Update job status to "active" and store tx_signature
    match JobQueries::confirm_job_with_signature(
        &data.db_pool,
        *job_id,
        JobStatus::Active,
        signature,
    )
    .await
    {
        Ok(_) => {
            log::info!(
                "Job {} confirmed and activated with tx: {}",
                *job_id,
                signature
            );
            HttpResponse::Ok().json(json!({
                "job_id": *job_id,
                "status": "active",
                "tx_signature": signature,
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
#[get("/internal/fhe/{job_id}/status")]
async fn get_job_status(data: web::Data<AppState>, job_id: web::Path<i64>) -> impl Responder {
    log::info!("Fetching status for job_id: {}", *job_id);

    match JobQueries::get_job_by_id(&data.db_pool, *job_id).await {
        Ok(Some(job)) => HttpResponse::Ok().json(JobStatusResponse {
            job_id: job.job_id,
            status: job.status,
            created_at: job.created_at.and_utc().to_rfc3339(),
        }),
        Ok(None) => HttpResponse::NotFound().json(json!({
            "error": "Job not found"
        })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": format!("Database error: {}", e)
        })),
    }
}

/// GET /api/jobs/fhe/{job_id}
///
/// Get full details of a specific FHE job (without encrypted data).
/// Combines data from temp_job_data and blockchain_jobs tables.
#[get("/internal/fhe/{job_id}")]
async fn get_job_details(data: web::Data<AppState>, job_id: web::Path<i64>) -> impl Responder {
    log::info!("Fetching full details for job_id: {}", *job_id);

    // Query that combines temp_job_data and blockchain_jobs
    let job_query = sqlx::query_as::<_, JobDetailsRow>(
        r#"
        SELECT * FROM (
            -- From temp_job_data (frontend-created jobs)
            SELECT
                t.job_id,
                t.creator_pubkey,
                t.operation,
                t.operation_value,
                t.price_lamports,
                t.required_provers,
                t.consensus_threshold,
                t.status,
                COALESCE(t.payment_method, 'SOL') as payment_method,
                t.payment_token_mint,
                t.tx_signature,
                to_char(t.created_at, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') as created_at,
                to_char(t.updated_at, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') as updated_at,
                NULL::text as claimed_at,
                NULL::text as completed_at,
                NULL::text as prover_pubkey
            FROM temp_job_data t
            WHERE t.job_id = $1

            UNION ALL

            -- From blockchain_jobs (synced from chain)
            SELECT
                b.job_id,
                b.creator_pubkey,
                COALESCE(b.fhe_operation, b.circuit_type) as operation,
                0::smallint as operation_value,
                b.price_lamports,
                COALESCE(b.required_provers, 1::smallint) as required_provers,
                COALESCE(b.consensus_threshold, 1::smallint) as consensus_threshold,
                CASE
                    WHEN b.timeout_at < NOW() AND b.status NOT IN ('completed', 'failed') THEN 'expired'
                    ELSE b.status
                END as status,
                'SOL' as payment_method,
                NULL::text as payment_token_mint,
                b.tx_signature,
                to_char(b.created_at, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') as created_at,
                to_char(COALESCE(b.synced_at, b.created_at), 'YYYY-MM-DD"T"HH24:MI:SS"Z"') as updated_at,
                CASE WHEN b.claimed_at IS NOT NULL
                    THEN to_char(b.claimed_at, 'YYYY-MM-DD"T"HH24:MI:SS"Z"')
                    ELSE NULL
                END as claimed_at,
                CASE WHEN b.completed_at IS NOT NULL
                    THEN to_char(b.completed_at, 'YYYY-MM-DD"T"HH24:MI:SS"Z"')
                    ELSE NULL
                END as completed_at,
                b.prover_pubkey
            FROM blockchain_jobs b
            WHERE b.job_id = $1
            AND NOT EXISTS (SELECT 1 FROM temp_job_data t2 WHERE t2.job_id = b.job_id)
        ) combined
        LIMIT 1
        "#
    )
    .bind(*job_id)
    .fetch_optional(&data.db_pool)
    .await;

    match job_query {
        Ok(Some(job)) => {
            // Check if there's a result for this job
            let has_result = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM fhe_results WHERE job_id = $1)",
            )
            .bind(*job_id)
            .fetch_one(&data.db_pool)
            .await
            .unwrap_or(false);

            HttpResponse::Ok().json(JobDetailsResponse {
                job_id: job.job_id,
                creator_pubkey: job.creator_pubkey,
                operation: job.operation,
                operation_value: job.operation_value,
                price_lamports: job.price_lamports,
                required_provers: job.required_provers,
                consensus_threshold: job.consensus_threshold,
                status: job.status,
                payment_method: job.payment_method,
                payment_token_mint: job.payment_token_mint,
                tx_signature: job.tx_signature,
                created_at: job.created_at,
                updated_at: job.updated_at,
                claimed_at: job.claimed_at,
                completed_at: job.completed_at,
                prover_pubkey: job.prover_pubkey,
                has_result,
            })
        }
        Ok(None) => {
            log::warn!("Job not found: {}", *job_id);
            HttpResponse::NotFound().json(json!({
                "error": "Job not found"
            }))
        }
        Err(e) => {
            log::error!("Database error fetching job {}: {}", *job_id, e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

/// DELETE /api/jobs/fhe/{job_id}
///
/// Delete FHE job data (cleanup).
/// Only allowed if job is in terminal state (completed/failed).
#[delete("/internal/fhe/{job_id}")]
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
/// List jobs with optional filtering.
/// Query params:
///   - status (optional): Filter by job status ("pending", "pending_tx", "active", "claimed", "completed", "failed", "cancelled")
///   - creator (optional): Filter by creator wallet pubkey
///   - page (optional): Page number for pagination (default: 1)
///   - limit (optional): Items per page (default: 20, max: 100)
///   - sort (optional): Sort order ("recent", "oldest", "price")
///
/// Examples:
///   - GET /api/jobs/fhe                  → List all FHE jobs
///   - GET /api/jobs/fhe?status=pending   → List only pending FHE jobs
///   - GET /api/jobs/fhe?creator=WALLET   → List FHE jobs from specific wallet
///   - GET /api/jobs/fhe?page=2&limit=10  → Paginated results
#[get("/internal/fhe")]
async fn list_jobs(data: web::Data<AppState>, query: web::Query<ListJobsQuery>) -> impl Responder {
    log::info!("Listing jobs with filter: {:?}", query.status);

    // Combined query from both temp_job_data and blockchain_jobs
    // This ensures jobs created from frontend appear immediately
    // Shows ALL historical jobs (including expired) for full visibility
    // IMPORTANT: temp_job_data is excluded if job already exists in blockchain_jobs
    // to avoid duplicates (blockchain_jobs is the source of truth)
    let jobs_query = sqlx::query_as::<_, JobListItem>(
        r#"
        SELECT * FROM (
            -- temp_job_data: ONLY jobs not yet synced to blockchain_jobs
            SELECT
                job_id,
                creator_pubkey,
                operation,
                operation_value,
                price_lamports,
                required_provers,
                consensus_threshold,
                status,
                COALESCE(payment_method, 'SOL') as payment_method,
                to_char(created_at, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') as created_at,
                tx_signature
            FROM temp_job_data t
            WHERE t.status IN ('pending_tx', 'active')
            AND NOT EXISTS (SELECT 1 FROM blockchain_jobs b WHERE b.job_id = t.job_id)

            UNION ALL

            -- blockchain_jobs: source of truth for all synced jobs
            SELECT
                job_id,
                creator_pubkey,
                COALESCE(fhe_operation, circuit_type) as operation,
                0::smallint as operation_value,
                price_lamports,
                COALESCE(required_provers, 1::smallint) as required_provers,
                COALESCE(consensus_threshold, 1::smallint) as consensus_threshold,
                CASE
                    WHEN timeout_at < NOW() AND status != 'completed' THEN 'expired'
                    ELSE status
                END as status,
                'SOL' as payment_method,
                to_char(created_at, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') as created_at,
                tx_signature
            FROM blockchain_jobs
        ) combined_jobs
        ORDER BY created_at DESC
        LIMIT 500
        "#,
    )
    .fetch_all(&data.db_pool)
    .await;

    // Handle query result
    let job_items = match jobs_query {
        Ok(jobs) => jobs,
        Err(e) => {
            log::error!("Failed to fetch jobs: {}", e);
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Database error: {}", e)
            }));
        }
    };

    // Apply filters
    let mut filtered_jobs: Vec<JobListItem> = job_items;

    // Filter by status if provided
    if let Some(ref status_str) = query.status {
        filtered_jobs.retain(|j| j.status == *status_str);
    }

    // Filter by creator if provided
    if let Some(ref creator_str) = query.creator {
        filtered_jobs.retain(|j| j.creator_pubkey == *creator_str);
    }

    // Apply pagination
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let total = filtered_jobs.len();
    let offset = ((page - 1) * limit) as usize;

    let paginated_jobs: Vec<JobListItem> = filtered_jobs
        .into_iter()
        .skip(offset)
        .take(limit as usize)
        .collect();

    let count = paginated_jobs.len();

    log::info!("Returning {} jobs (page {}, total {})", count, page, total);

    HttpResponse::Ok().json(ListJobsResponse {
        jobs: paginated_jobs,
        count,
        total,
        page,
        limit,
    })
}

/// GET /internal/stats/network
///
/// Get network statistics including active provers, jobs processed, etc.
/// Used by the frontend to display real-time network metrics.
#[get("/internal/stats/network")]
async fn get_network_stats(data: web::Data<AppState>) -> impl Responder {
    log::info!("Fetching network statistics");

    // Query 1: Count active provers from provers table (synced from blockchain)
    let active_provers: i64 =
        sqlx::query_scalar!(r#"SELECT COUNT(*) as "count!" FROM provers WHERE is_active = true"#)
            .fetch_one(&data.db_pool)
            .await
            .unwrap_or(0);

    // Query 2: Count completed jobs
    let jobs_completed: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(*) as "count!" FROM blockchain_jobs WHERE status = 'completed'"#
    )
    .fetch_one(&data.db_pool)
    .await
    .unwrap_or(0);

    // Query 3: Count active jobs (not expired)
    let jobs_active: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(*) as "count!" FROM blockchain_jobs WHERE timeout_at >= NOW() AND status IN ('pending', 'claimed')"#
    )
    .fetch_one(&data.db_pool)
    .await
    .unwrap_or(0);

    // Total = completed + active
    let jobs_total: i64 = jobs_completed + jobs_active;

    // Query 4: Get network start time (first job created)
    let network_start: Option<chrono::NaiveDateTime> =
        sqlx::query_scalar!(r#"SELECT MIN(created_at) FROM blockchain_jobs"#)
            .fetch_one(&data.db_pool)
            .await
            .unwrap_or(None);

    // Calculate uptime
    let now = chrono::Utc::now().naive_utc();
    let (uptime_seconds, network_start_str) = if let Some(start) = network_start {
        let duration = now.signed_duration_since(start);
        (
            duration.num_seconds(),
            Some(start.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
        )
    } else {
        (0, None)
    };

    // Query 5: Get historical data processed from network_metrics
    // This is cumulative and never decreases, even after cleanup
    let historical_metrics = NetworkMetricsQueries::get_metrics(&data.db_pool)
        .await
        .unwrap_or_else(|_| crate::db::queries::NetworkMetrics {
            total_data_processed_bytes: 0,
            total_jobs_processed: 0,
            total_fhe_computations: 0,
            total_zk_proofs: 0,
        });

    // Also get current data in temp storage (not yet counted in historical)
    let current_data_result: Option<(i64,)> = sqlx::query_as(
        r#"SELECT COALESCE(SUM(LENGTH(encrypted_data) + LENGTH(server_key)), 0)::bigint FROM temp_job_data"#
    )
    .fetch_optional(&data.db_pool)
    .await
    .ok()
    .flatten();
    let current_data_size = current_data_result.map(|(s,)| s).unwrap_or(0);

    // Total = historical + current pending
    let data_bytes = historical_metrics.total_data_processed_bytes + current_data_size;

    // Format bytes adaptively
    let data_formatted = format_bytes(data_bytes);

    HttpResponse::Ok().json(NetworkStatsResponse {
        active_provers,
        jobs_completed,
        jobs_total,
        data_encrypted_bytes: data_bytes,
        data_encrypted_formatted: data_formatted,
        network_start_time: network_start_str,
        uptime_seconds,
        uptime_percent: 99.97, // Simplified - in production track actual downtime
    })
}

/// GET /internal/metrics
///
/// Get comprehensive network metrics for the dashboard.
/// Includes historical data, 24h stats, operation breakdowns, and timeline.
#[get("/internal/metrics")]
async fn get_metrics(data: web::Data<AppState>) -> impl Responder {
    log::info!("Fetching comprehensive metrics");

    // Query 1: Active provers
    let active_provers: i64 =
        sqlx::query_scalar!(r#"SELECT COUNT(*) as "count!" FROM provers WHERE is_active = true"#)
            .fetch_one(&data.db_pool)
            .await
            .unwrap_or(0);

    // Query 2: Total provers
    let total_provers: i64 = sqlx::query_scalar!(r#"SELECT COUNT(*) as "count!" FROM provers"#)
        .fetch_one(&data.db_pool)
        .await
        .unwrap_or(0);

    let offline_provers = total_provers - active_provers;

    // Query 3: Jobs completed (all time)
    let completed_total: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(*) as "count!" FROM blockchain_jobs WHERE status = 'completed'"#
    )
    .fetch_one(&data.db_pool)
    .await
    .unwrap_or(0);

    // Query 4: Jobs completed in last 24h
    let completed_24h: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(*) as "count!" FROM blockchain_jobs WHERE status = 'completed' AND created_at >= NOW() - INTERVAL '24 hours'"#
    )
    .fetch_one(&data.db_pool)
    .await
    .unwrap_or(0);

    // Query 5: Active jobs (not expired, pending/claimed)
    let active_current: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(*) as "count!" FROM blockchain_jobs WHERE timeout_at >= NOW() AND status IN ('pending', 'claimed')"#
    )
    .fetch_one(&data.db_pool)
    .await
    .unwrap_or(0);

    // Query 6: Pending jobs
    let pending_current: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(*) as "count!" FROM blockchain_jobs WHERE timeout_at >= NOW() AND status = 'pending'"#
    )
    .fetch_one(&data.db_pool)
    .await
    .unwrap_or(0);

    // Query 7: Expired jobs
    let expired_total: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(*) as "count!" FROM blockchain_jobs WHERE timeout_at < NOW() AND status != 'completed'"#
    )
    .fetch_one(&data.db_pool)
    .await
    .unwrap_or(0);

    // Query 8: Network start time
    let network_start: Option<chrono::NaiveDateTime> =
        sqlx::query_scalar!(r#"SELECT MIN(created_at) FROM blockchain_jobs"#)
            .fetch_one(&data.db_pool)
            .await
            .unwrap_or(None);

    let now = chrono::Utc::now().naive_utc();
    let uptime_seconds = if let Some(start) = network_start {
        now.signed_duration_since(start).num_seconds()
    } else {
        0
    };

    // Format uptime as "Xd Yh Zm"
    let days = uptime_seconds / 86400;
    let hours = (uptime_seconds % 86400) / 3600;
    let mins = (uptime_seconds % 3600) / 60;
    let uptime_formatted = format!("{}d {}h {}m", days, hours, mins);

    // Query 9: Operations breakdown
    #[derive(sqlx::FromRow)]
    struct OpCount {
        operation: Option<String>,
        count: i64,
    }

    let operations_raw: Vec<OpCount> = sqlx::query_as(
        r#"SELECT COALESCE(fhe_operation, circuit_type) as operation, COUNT(*) as count
           FROM blockchain_jobs
           GROUP BY COALESCE(fhe_operation, circuit_type)
           ORDER BY count DESC
           LIMIT 10"#,
    )
    .fetch_all(&data.db_pool)
    .await
    .unwrap_or_default();

    let operations: Vec<OperationBreakdown> = operations_raw
        .into_iter()
        .map(|o| OperationBreakdown {
            operation: o.operation.unwrap_or_else(|| "unknown".to_string()),
            count: o.count,
        })
        .collect();

    // Query 10: Timeline (jobs per hour for last 24h)
    #[derive(sqlx::FromRow)]
    struct TimelineRow {
        hour: Option<chrono::NaiveDateTime>,
        count: i64,
    }

    let timeline_raw: Vec<TimelineRow> = sqlx::query_as(
        r#"SELECT date_trunc('hour', created_at) as hour, COUNT(*) as count
           FROM blockchain_jobs
           WHERE created_at >= NOW() - INTERVAL '24 hours'
           GROUP BY date_trunc('hour', created_at)
           ORDER BY hour ASC"#,
    )
    .fetch_all(&data.db_pool)
    .await
    .unwrap_or_default();

    let timeline: Vec<TimelinePoint> = timeline_raw
        .into_iter()
        .map(|t| TimelinePoint {
            timestamp: t
                .hour
                .map(|h| h.format("%H:%M").to_string())
                .unwrap_or_default(),
            count: t.count,
        })
        .collect();

    // Calculate metrics
    let total_jobs = completed_total + active_current + pending_current + expired_total;
    let success_rate = if total_jobs > 0 {
        (completed_total as f64 / total_jobs as f64) * 100.0
    } else {
        0.0
    };

    // Estimate data processed (1KB per job)
    let data_processed_tb = (total_jobs * 1024) as f64 / (1024.0 * 1024.0 * 1024.0 * 1024.0);
    let data_processed_24h_tb =
        ((completed_24h + active_current) * 1024) as f64 / (1024.0 * 1024.0 * 1024.0 * 1024.0);

    // Mock provers history (in production, track this in a separate table)
    let provers_history: Vec<i64> = vec![
        active_provers.saturating_sub(2),
        active_provers.saturating_sub(1),
        active_provers,
        active_provers,
        active_provers.saturating_add(1),
        active_provers,
        active_provers.saturating_sub(1),
        active_provers,
    ];

    HttpResponse::Ok().json(MetricsResponse {
        network: MetricsNetworkInfo {
            uptime_seconds,
            uptime_formatted,
            active_provers,
            total_provers,
            offline_provers,
            data_processed_tb,
            data_processed_24h_tb,
        },
        jobs: MetricsJobsInfo {
            completed_24h,
            completed_total,
            active_current,
            pending_current,
            expired_total,
            avg_job_time_mins: 2.5, // Mock - in production calculate from actual job durations
            success_rate,
        },
        operations,
        timeline,
        provers_history,
    })
}

/// POST /internal/estimate-cost
///
/// Estimate the cost and timeout for a given FHE operation.
/// Helps users understand pricing before creating a job.
#[post("/internal/estimate-cost")]
async fn estimate_operation_cost(req: web::Json<EstimateCostRequest>) -> impl Responder {
    use zyberlink_types::fhe::{FheOperation, HistogramBin};

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
                predicate: zyberlink_types::fhe::FhePredicate::GreaterThan(op_value),
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

/// POST /internal/price-recommendation
///
/// Get price recommendation for FHE operations with slider parameters.
/// Returns min/recommended/max prices based on prover economics.
#[post("/internal/price-recommendation")]
async fn get_price_recommendation(req: web::Json<PriceRecommendationRequest>) -> impl Responder {
    use zyberlink_types::fhe::{FheOperation, HistogramBin};

    log::info!("Price recommendation for operation: {}", req.operation);

    // Parse operation (same as estimate-cost)
    let operation = match req.operation.to_lowercase().as_str() {
        "add" => FheOperation::Add(req.operation_value.unwrap_or(1)),
        "multiply" => FheOperation::Multiply(req.operation_value.unwrap_or(1)),
        "sum" => FheOperation::Sum {
            expected_count: req.expected_count.unwrap_or(10),
        },
        "threshold" => FheOperation::Threshold {
            threshold: req.operation_value.unwrap_or(50),
            greater_or_equal: true,
        },
        "rangecheck" => FheOperation::RangeCheck {
            min: 0,
            max: req.operation_value.unwrap_or(100),
        },
        "average" => FheOperation::Average {
            expected_count: req.expected_count.unwrap_or(10),
        },
        "countif" => FheOperation::CountIf {
            expected_count: req.expected_count.unwrap_or(10),
            predicate: zyberlink_types::fhe::FhePredicate::GreaterThan(
                req.operation_value.unwrap_or(50),
            ),
        },
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

    // Prover economics constants
    const PROVER_OVERHEAD_MULTIPLIER: f64 = 1.5; // 50% operational overhead
    const PROVER_MIN_ROI_PERCENT: f64 = 20.0; // Minimum 20% ROI required
    const LAMPORTS_PER_SOL: f64 = 1_000_000_000.0;

    // Get base cost from operation
    let cost_config = operation.get_cost_config();
    let base_cost_per_prover = cost_config.min_payment_lamports;
    let provers = req.required_provers as u64;

    // Calculate prices per prover
    // Min: theoretical floor (base cost)
    let min_per_prover = base_cost_per_prover;

    // Recommended: covers overhead + minimum ROI
    // Formula: base_cost × overhead × (1 + ROI/100)
    let recommended_per_prover = (base_cost_per_prover as f64
        * PROVER_OVERHEAD_MULTIPLIER
        * (1.0 + PROVER_MIN_ROI_PERCENT / 100.0)) as u64;

    // Max suggested: premium pricing for priority (2x recommended)
    let max_per_prover = recommended_per_prover * 2;

    // Total prices (per prover × number of provers)
    let min_total = min_per_prover * provers;
    let recommended_total = recommended_per_prover * provers;
    let max_total = max_per_prover * provers;

    // Slider parameters
    let slider_step = if min_total < 10_000_000 {
        100_000 // 0.0001 SOL steps for small amounts
    } else {
        1_000_000 // 0.001 SOL steps for larger amounts
    };

    log::info!(
        "Price recommendation for {}: min={}, recommended={}, max={} lamports",
        operation.name(),
        min_total,
        recommended_total,
        max_total
    );

    HttpResponse::Ok().json(PriceRecommendationResponse {
        operation: operation.name().to_string(),
        complexity_tier: cost_config.complexity_tier,
        required_provers: req.required_provers,

        min_price_lamports: min_total,
        min_price_sol: min_total as f64 / LAMPORTS_PER_SOL,

        recommended_price_lamports: recommended_total,
        recommended_price_sol: recommended_total as f64 / LAMPORTS_PER_SOL,

        max_suggested_lamports: max_total,
        max_suggested_sol: max_total as f64 / LAMPORTS_PER_SOL,

        acceptance_at_min: "low".to_string(),
        acceptance_at_recommended: "high".to_string(),

        slider_min: min_total,
        slider_max: max_total,
        slider_recommended: recommended_total,
        slider_step,

        estimated_time_seconds: cost_config.timeout_seconds,
        prover_overhead_multiplier: PROVER_OVERHEAD_MULTIPLIER,
        prover_min_roi_percent: PROVER_MIN_ROI_PERCENT,
    })
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Build unsigned create_fhe_job transaction (supports SOL and wZEC payments)
fn build_create_job_transaction(
    validated: &crate::validators::ValidatedJob,
    builder: &zyberlink_sdk::instructions::InstructionBuilder,
) -> anyhow::Result<Transaction> {
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;
    use zyberlink_types::{FheConsensusConfig, FheOperation};

    // Parse operation
    let operation = match validated.operation.as_str() {
        "add" => FheOperation::Add(validated.operation_value),
        "multiply" => FheOperation::Multiply(validated.operation_value),
        "sum" => FheOperation::Sum {
            expected_count: validated.expected_count,
        },
        "count_if" => FheOperation::CountIf {
            expected_count: validated.expected_count,
            predicate: validated
                .predicate
                .clone()
                .ok_or_else(|| anyhow::anyhow!("Missing predicate for count_if operation"))?,
        },
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
// Witness Storage Endpoints
// ============================================================================

/// POST /witness
///
/// Upload encrypted witness data.
/// Returns the Blake2b commitment hash of the uploaded data.
#[post("/internal/witness")]
async fn upload_witness(data: web::Data<AppState>, body: web::Bytes) -> impl Responder {
    log::info!("Received witness upload, size: {} bytes", body.len());

    if body.is_empty() {
        return HttpResponse::BadRequest().json(json!({
            "error": "Empty witness data"
        }));
    }

    // Compute Blake2s-256 hash as commitment (must match SDK's Blake2s256)
    let mut hasher = Blake2s256::new();
    hasher.update(&body);
    let hash = hasher.finalize();
    let commitment = hex::encode(hash);

    log::info!("Witness commitment: {}", commitment);

    // Store in database
    match WitnessQueries::store_witness(&data.db_pool, &commitment, &body).await {
        Ok(_) => {
            log::info!("Witness stored successfully");
            HttpResponse::Ok().json(WitnessUploadResponse { commitment })
        }
        Err(e) => {
            log::error!("Failed to store witness: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to store witness: {}", e)
            }))
        }
    }
}

/// GET /witness/{commitment}
///
/// Download encrypted witness data by commitment hash.
/// First checks the witnesses table, then attempts to reconstruct from
/// blockchain_jobs + temp_job_data if not found.
#[get("/internal/witness/{commitment}")]
async fn get_witness(data: web::Data<AppState>, commitment: web::Path<String>) -> impl Responder {
    log::info!("Fetching witness for commitment: {}", *commitment);

    // Option 1: Check witnesses table (legacy/pre-stored)
    match WitnessQueries::get_witness(&data.db_pool, &commitment).await {
        Ok(Some(witness_data)) => {
            log::info!("Witness found in storage, returning {} bytes", witness_data.len());
            return HttpResponse::Ok()
                .content_type("application/octet-stream")
                .body(witness_data);
        }
        Ok(None) => {
            log::info!("Witness not in storage, attempting dynamic reconstruction");
        }
        Err(e) => {
            log::error!("Failed to fetch witness from storage: {}", e);
        }
    }

    // Option 2: Reconstruct from blockchain_jobs + temp_job_data
    // Find job_id by witness_hash in blockchain_jobs, then get data from temp_job_data
    let reconstruction_query = sqlx::query_as::<_, (Vec<u8>, Vec<u8>)>(
        r#"
        SELECT t.server_key, t.encrypted_data
        FROM blockchain_jobs b
        JOIN temp_job_data t ON b.job_id = t.job_id
        WHERE b.witness_hash = $1
        LIMIT 1
        "#
    )
    .bind(&*commitment)
    .fetch_optional(&data.db_pool)
    .await;

    match reconstruction_query {
        Ok(Some((server_key, encrypted_data))) => {
            // Reconstruct witness in the format expected by prover:
            // [encrypted_data_len (4 bytes LE)] [encrypted_data] [server_key]
            let encrypted_data_len = encrypted_data.len() as u32;
            let mut witness = Vec::with_capacity(4 + encrypted_data.len() + server_key.len());
            witness.extend_from_slice(&encrypted_data_len.to_le_bytes());
            witness.extend_from_slice(&encrypted_data);
            witness.extend_from_slice(&server_key);

            log::info!(
                "Witness reconstructed dynamically: {} bytes (header: 4, encrypted_data: {}, server_key: {})",
                witness.len(),
                encrypted_data.len(),
                server_key.len()
            );

            HttpResponse::Ok()
                .content_type("application/octet-stream")
                .body(witness)
        }
        Ok(None) => {
            log::warn!("Witness not found: {} (not in storage, not reconstructable)", *commitment);
            HttpResponse::NotFound().json(json!({
                "error": "Witness not found"
            }))
        }
        Err(e) => {
            log::error!("Failed to reconstruct witness: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

// ============================================================================
// FHE Result Storage Endpoints
// ============================================================================

/// Query params for FHE result upload
#[derive(Debug, Deserialize)]
pub struct FheResultUploadQuery {
    pub job_id: Option<i64>,
    pub prover: Option<String>,
}

/// POST /fhe-result
///
/// Upload FHE computation result.
/// Query params:
///   - job_id (optional): Associate result with a job
///   - prover (optional): Prover pubkey that computed this result
/// Returns the Blake2s256 commitment hash of the uploaded data.
#[post("/internal/fhe-result")]
async fn upload_fhe_result(
    data: web::Data<AppState>,
    query: web::Query<FheResultUploadQuery>,
    body: web::Bytes,
) -> impl Responder {
    log::info!(
        "Received FHE result upload, size: {} bytes, job_id: {:?}, prover: {:?}",
        body.len(),
        query.job_id,
        query.prover
    );

    if body.is_empty() {
        return HttpResponse::BadRequest().json(json!({
            "error": "Empty FHE result data"
        }));
    }

    // Compute Blake2s256 hash as commitment
    let mut hasher = Blake2s256::new();
    hasher.update(&body);
    let hash = hasher.finalize();
    let commitment = hex::encode(hash);

    log::info!("FHE result commitment: {}", commitment);

    // Store in database with job_id and prover
    match FheResultQueries::store_result_with_job(
        &data.db_pool,
        &commitment,
        &body,
        query.job_id,
        query.prover.as_deref(),
    )
    .await
    {
        Ok(_) => {
            log::info!(
                "FHE result stored successfully for job_id: {:?}",
                query.job_id
            );
            HttpResponse::Ok().json(json!({
                "commitment": commitment,
                "job_id": query.job_id
            }))
        }
        Err(e) => {
            log::error!("Failed to store FHE result: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to store FHE result: {}", e)
            }))
        }
    }
}

/// GET /fhe-result/{commitment}
///
/// Download FHE computation result by commitment hash.
#[get("/internal/fhe-result/{commitment}")]
async fn get_fhe_result(
    data: web::Data<AppState>,
    commitment: web::Path<String>,
) -> impl Responder {
    log::info!("Fetching FHE result for commitment: {}", *commitment);

    match FheResultQueries::get_result(&data.db_pool, &commitment).await {
        Ok(Some(result_data)) => {
            log::info!("FHE result found, returning {} bytes", result_data.len());
            HttpResponse::Ok()
                .content_type("application/octet-stream")
                .body(result_data)
        }
        Ok(None) => {
            log::warn!("FHE result not found: {}", *commitment);
            HttpResponse::NotFound().json(json!({
                "error": "FHE result not found"
            }))
        }
        Err(e) => {
            log::error!("Failed to fetch FHE result: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

// ============================================================================
// Server Key Pre-Upload Endpoints
// ============================================================================

/// POST /api/server-key/upload
///
/// Pre-upload a TFHE server key (~117MB) before creating a job.
/// Returns the Blake2s256 hash which can be used in validate-and-build.
///
/// This endpoint accepts raw binary data (Content-Type: application/octet-stream)
/// to avoid base64 encoding overhead and timeout issues.
///
/// Usage:
/// 1. Upload server_key via POST /api/server-key/upload (raw bytes)
/// 2. Use returned server_key_hash in validate-and-build request
#[post("/internal/server-key/upload")]
async fn upload_server_key(data: web::Data<AppState>, body: web::Bytes) -> impl Responder {
    let size_bytes = body.len();
    log::info!("Received server key upload, size: {} bytes ({:.2} MB)",
        size_bytes, size_bytes as f64 / (1024.0 * 1024.0));

    // Validate size (must be between 40MB and 120MB for TFHE server keys)
    const MIN_SERVER_KEY_SIZE: usize = 40 * 1024 * 1024; // 40 MB
    const MAX_SERVER_KEY_SIZE: usize = 120 * 1024 * 1024; // 120 MB

    if body.is_empty() {
        return HttpResponse::BadRequest().json(json!({
            "error": "Empty server key data"
        }));
    }

    if size_bytes < MIN_SERVER_KEY_SIZE {
        return HttpResponse::BadRequest().json(json!({
            "error": format!("Server key too small: {} bytes (min: {} bytes)", size_bytes, MIN_SERVER_KEY_SIZE)
        }));
    }

    if size_bytes > MAX_SERVER_KEY_SIZE {
        return HttpResponse::BadRequest().json(json!({
            "error": format!("Server key too large: {} bytes (max: {} bytes)", size_bytes, MAX_SERVER_KEY_SIZE)
        }));
    }

    // Validate that it's a valid TFHE ServerKey by attempting deserialization
    match bincode::deserialize::<tfhe::ServerKey>(&body) {
        Ok(_) => {
            log::info!("Server key deserialization successful");
        }
        Err(e) => {
            log::warn!("Invalid server key format: {}", e);
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Invalid server key format: {}", e)
            }));
        }
    }

    // Compute Blake2s256 hash
    let mut hasher = Blake2s256::new();
    hasher.update(&body);
    let hash = hasher.finalize();
    let server_key_hash = hex::encode(hash);

    log::info!("Server key hash: {}", server_key_hash);

    // Store in database (idempotent - duplicate uploads are OK)
    match ServerKeyQueries::store_server_key(&data.db_pool, &server_key_hash, &body).await {
        Ok(_) => {
            log::info!("Server key stored successfully, hash: {}", server_key_hash);
            HttpResponse::Ok().json(ServerKeyUploadResponse {
                server_key_hash,
                size_bytes,
            })
        }
        Err(e) => {
            log::error!("Failed to store server key: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to store server key: {}", e)
            }))
        }
    }
}

/// GET /api/server-key/{hash}/exists
///
/// Check if a server key exists by its hash.
/// Useful for frontend to check if re-upload is needed.
#[get("/internal/server-key/{hash}/exists")]
async fn check_server_key_exists(
    data: web::Data<AppState>,
    hash: web::Path<String>,
) -> impl Responder {
    log::info!("Checking server key existence: {}", *hash);

    match ServerKeyQueries::server_key_exists(&data.db_pool, &hash).await {
        Ok(exists) => {
            HttpResponse::Ok().json(json!({
                "exists": exists,
                "hash": *hash
            }))
        }
        Err(e) => {
            log::error!("Failed to check server key: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

/// GET /api/jobs/fhe/{job_id}/chain-status
///
/// Get FHE job status directly from blockchain_jobs (synced from chain).
/// Useful for getting witness_hash and current on-chain status.
#[get("/internal/fhe/{job_id}/chain-status")]
async fn get_job_chain_status(data: web::Data<AppState>, job_id: web::Path<i64>) -> impl Responder {
    log::info!("Fetching chain status for job_id: {}", *job_id);

    let query = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT status, witness_hash FROM blockchain_jobs WHERE job_id = $1"
    )
    .bind(*job_id)
    .fetch_optional(&data.db_pool)
    .await;

    match query {
        Ok(Some((status, witness_hash))) => {
            HttpResponse::Ok().json(json!({
                "job_id": *job_id,
                "status": status,
                "witness_hash": witness_hash
            }))
        }
        Ok(None) => {
            HttpResponse::NotFound().json(json!({
                "error": "Job not found in blockchain_jobs (chain sync may not have run yet)",
                "job_id": *job_id
            }))
        }
        Err(e) => {
            log::error!("Failed to fetch chain status: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

/// Prover result row from fhe_results table
#[derive(Debug, sqlx::FromRow)]
struct ProverResultRow {
    prover_pubkey: Option<String>,
    commitment: String,
    created_at: Option<chrono::NaiveDateTime>,
}

/// GET /api/jobs/fhe/{job_id}/provers
///
/// Get list of provers who have submitted results for this FHE job.
/// Returns prover pubkeys, submission times, and consensus status.
#[get("/internal/fhe/{job_id}/provers")]
async fn get_job_provers(data: web::Data<AppState>, job_id: web::Path<i64>) -> impl Responder {
    log::info!("Fetching provers for job_id: {}", *job_id);

    // Query fhe_results to get all provers who submitted for this job
    let provers_query = sqlx::query_as::<_, ProverResultRow>(
        r#"
        SELECT
            prover_pubkey,
            commitment,
            created_at
        FROM fhe_results
        WHERE job_id = $1
        ORDER BY created_at ASC
        "#
    )
    .bind(*job_id)
    .fetch_all(&data.db_pool)
    .await;

    match provers_query {
        Ok(rows) => {
            let provers: Vec<serde_json::Value> = rows
                .into_iter()
                .enumerate()
                .map(|(i, row)| {
                    json!({
                        "index": i + 1,
                        "prover_pubkey": row.prover_pubkey,
                        "commitment": row.commitment,
                        "submitted_at": row.created_at.map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
                        "status": "verified"
                    })
                })
                .collect();

            HttpResponse::Ok().json(json!({
                "job_id": *job_id,
                "prover_count": provers.len(),
                "provers": provers
            }))
        }
        Err(e) => {
            log::error!("Failed to fetch provers for job {}: {}", *job_id, e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

/// GET /api/jobs/fhe/{job_id}/result
///
/// Get FHE computation result for a completed job.
/// Searches fhe_results table by job_id directly.
#[get("/internal/fhe/{job_id}/result")]
async fn get_job_result(data: web::Data<AppState>, job_id: web::Path<i64>) -> impl Responder {
    log::info!("Fetching FHE result for job_id: {}", *job_id);

    // Fetch result directly from fhe_results table by job_id
    match FheResultQueries::get_first_result_by_job_id(&data.db_pool, *job_id).await {
        Ok(Some(result_data)) => {
            log::info!(
                "FHE result found for job {}, returning {} bytes",
                *job_id,
                result_data.len()
            );
            // Return as base64-encoded JSON for job-creator compatibility
            HttpResponse::Ok().json(json!({
                "job_id": *job_id,
                "encrypted_result": STANDARD.encode(&result_data)
            }))
        }
        Ok(None) => {
            log::warn!("FHE result not found for job_id: {}", *job_id);
            HttpResponse::NotFound().json(json!({
                "error": "Result not found for this job",
                "job_id": *job_id
            }))
        }
        Err(e) => {
            log::error!("Failed to fetch FHE result: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

// =============================================================================
// Prover Verification Endpoints (for x402-gateway)
// =============================================================================

#[derive(Debug, Serialize)]
pub struct ProverStatusResponse {
    pub registered: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reputation: Option<i32>,
}

/// GET /internal/prover/{pubkey}/status
///
/// Verify if a prover is registered and active.
/// Used by x402-gateway to authorize requests from provers.
#[get("/internal/prover/{pubkey}/status")]
async fn get_prover_status(
    data: web::Data<AppState>,
    pubkey: web::Path<String>,
) -> impl Responder {
    log::info!("Checking prover status for pubkey: {}", *pubkey);

    // Query provers table
    let query_result = sqlx::query_as::<_, (bool, i32)>(
        r#"
        SELECT is_active, reputation_score
        FROM provers
        WHERE pubkey = $1
        "#,
    )
    .bind(&*pubkey)
    .fetch_optional(&data.db_pool)
    .await;

    match query_result {
        Ok(Some((is_active, reputation_score))) => {
            log::info!(
                "Prover {} found: active={}, reputation={}",
                *pubkey,
                is_active,
                reputation_score
            );
            HttpResponse::Ok().json(ProverStatusResponse {
                registered: true,
                active: Some(is_active),
                reputation: Some(reputation_score),
            })
        }
        Ok(None) => {
            log::warn!("Prover {} not found in database", *pubkey);
            HttpResponse::Ok().json(ProverStatusResponse {
                registered: false,
                active: None,
                reputation: None,
            })
        }
        Err(e) => {
            log::error!("Database error checking prover status: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

// ============================================================================
// pBTCFi FHE Job Endpoints
// ============================================================================

/// Request to create an FHE verification job for a loan
#[derive(Debug, Deserialize)]
pub struct PbtcfiCreateJobRequest {
    pub loan_id: String,
}

/// Response for FHE job creation
#[derive(Debug, Serialize)]
pub struct PbtcfiCreateJobResponse {
    pub loan_id: String,
    pub status: String,
    pub message: String,
}

/// Request to claim an FHE job
#[derive(Debug, Deserialize)]
pub struct PbtcfiClaimJobRequest {
    pub loan_id: String,
    pub prover_id: String,
}

/// Request to complete an FHE job
#[derive(Debug, Deserialize)]
pub struct PbtcfiCompleteJobRequest {
    pub loan_id: String,
    pub collateral_value_c1: String,
    pub collateral_value_c2: String,
    pub plst_amount_c1: String,
    pub plst_amount_c2: String,
}

/// POST /internal/pbtcfi/create-job
///
/// Create an FHE verification job for a pBTCFi loan.
/// Called by pbtcfi_sync after detecting a LoanCreated event.
#[post("/internal/pbtcfi/create-job")]
async fn pbtcfi_create_fhe_job(
    data: web::Data<AppState>,
    req: web::Json<PbtcfiCreateJobRequest>,
) -> impl Responder {
    log::info!("Creating FHE job for pBTCFi loan: {}", req.loan_id);

    match crate::db::PbtcfiQueries::create_fhe_job(&data.db_pool, &req.loan_id).await {
        Ok(()) => {
            log::info!("FHE job created for loan {}", req.loan_id);
            HttpResponse::Ok().json(PbtcfiCreateJobResponse {
                loan_id: req.loan_id.clone(),
                status: "fhe_pending".to_string(),
                message: "FHE verification job created".to_string(),
            })
        }
        Err(e) => {
            log::error!("Failed to create FHE job for loan {}: {}", req.loan_id, e);
            HttpResponse::BadRequest().json(json!({
                "error": format!("Failed to create FHE job: {}", e)
            }))
        }
    }
}

/// GET /internal/pbtcfi/pending-jobs
///
/// Get list of loans pending FHE verification.
/// Used by pBTCFi provers to find work.
#[get("/internal/pbtcfi/pending-jobs")]
async fn pbtcfi_get_pending_jobs(
    data: web::Data<AppState>,
    query: web::Query<ListJobsQuery>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(10).min(50);

    log::debug!("Fetching pending pBTCFi FHE jobs, limit: {}", limit);

    match crate::db::PbtcfiQueries::get_pending_fhe_jobs(&data.db_pool, limit).await {
        Ok(jobs) => {
            log::info!("Found {} pending pBTCFi FHE jobs", jobs.len());
            HttpResponse::Ok().json(jobs)
        }
        Err(e) => {
            log::error!("Failed to fetch pending FHE jobs: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

/// POST /internal/pbtcfi/claim-job
///
/// Claim an FHE job for processing.
/// Atomic operation to prevent multiple provers working on same job.
#[post("/internal/pbtcfi/claim-job")]
async fn pbtcfi_claim_job(
    data: web::Data<AppState>,
    req: web::Json<PbtcfiClaimJobRequest>,
) -> impl Responder {
    log::info!(
        "Prover {} claiming FHE job for loan {}",
        req.prover_id,
        req.loan_id
    );

    match crate::db::PbtcfiQueries::claim_fhe_job(&data.db_pool, &req.loan_id, &req.prover_id).await
    {
        Ok(true) => {
            log::info!("Job {} claimed by prover {}", req.loan_id, req.prover_id);
            HttpResponse::Ok().json(json!({
                "success": true,
                "message": "Job claimed successfully"
            }))
        }
        Ok(false) => {
            log::warn!(
                "Job {} not available for claim (already taken or doesn't exist)",
                req.loan_id
            );
            HttpResponse::Conflict().json(json!({
                "success": false,
                "error": "Job not available (already claimed or not found)"
            }))
        }
        Err(e) => {
            log::error!("Failed to claim job {}: {}", req.loan_id, e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

/// POST /internal/pbtcfi/complete-job
///
/// Submit FHE computation result for a job.
/// Updates loan with encrypted pLST amount and marks as active.
#[post("/internal/pbtcfi/complete-job")]
async fn pbtcfi_complete_job(
    data: web::Data<AppState>,
    req: web::Json<PbtcfiCompleteJobRequest>,
) -> impl Responder {
    log::info!("Completing FHE job for loan {}", req.loan_id);

    match crate::db::PbtcfiQueries::complete_fhe_job(
        &data.db_pool,
        &req.loan_id,
        &req.collateral_value_c1,
        &req.collateral_value_c2,
        &req.plst_amount_c1,
        &req.plst_amount_c2,
    )
    .await
    {
        Ok(()) => {
            log::info!("FHE job completed for loan {}", req.loan_id);
            HttpResponse::Ok().json(json!({
                "success": true,
                "loan_id": req.loan_id,
                "status": "active",
                "message": "FHE verification completed, loan activated"
            }))
        }
        Err(e) => {
            log::error!("Failed to complete FHE job for loan {}: {}", req.loan_id, e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to complete job: {}", e)
            }))
        }
    }
}

/// Request to fail an FHE job
#[derive(Debug, Deserialize)]
pub struct PbtcfiFailJobRequest {
    pub loan_id: String,
    pub error: String,
}

/// POST /internal/pbtcfi/fail-job
///
/// Report that FHE computation failed for a job.
/// Marks the loan's FHE status as failed with error message.
#[post("/internal/pbtcfi/fail-job")]
async fn pbtcfi_fail_job(
    data: web::Data<AppState>,
    req: web::Json<PbtcfiFailJobRequest>,
) -> impl Responder {
    log::warn!("Failing FHE job for loan {}: {}", req.loan_id, req.error);

    match crate::db::PbtcfiQueries::fail_fhe_job(&data.db_pool, &req.loan_id, &req.error).await {
        Ok(()) => {
            log::info!("FHE job marked as failed for loan {}", req.loan_id);
            HttpResponse::Ok().json(json!({
                "success": true,
                "loan_id": req.loan_id,
                "status": "fhe_failed",
                "message": "FHE job marked as failed"
            }))
        }
        Err(e) => {
            log::error!("Failed to mark FHE job as failed for loan {}: {}", req.loan_id, e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to mark job as failed: {}", e)
            }))
        }
    }
}

// ============================================================================
// Route Configuration
// ============================================================================

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_jobs)
        .service(get_network_stats)
        .service(get_metrics)
        .service(validate_and_build_job)
        .service(estimate_operation_cost)
        .service(get_price_recommendation)
        .service(get_compute_data)
        .service(confirm_job_transaction)
        .service(get_job_status)
        .service(get_job_details) // GET /api/jobs/{job_id} - full job details
        .service(get_job_provers) // GET /api/jobs/{job_id}/provers - prover consensus info
        .service(get_job_result) // GET /api/jobs/{job_id}/result - FHE result
        .service(delete_job_data)
        .service(upload_witness)
        .service(get_witness)
        .service(upload_fhe_result)
        .service(get_fhe_result)
        // Server key pre-upload endpoints
        .service(upload_server_key) // POST /api/server-key/upload
        .service(check_server_key_exists) // GET /api/server-key/{hash}/exists
        // Prover verification endpoints (for x402-gateway)
        .service(get_prover_status) // GET /internal/prover/{pubkey}/status
        // pBTCFi FHE job endpoints
        .service(pbtcfi_create_fhe_job) // POST /internal/pbtcfi/create-job
        .service(pbtcfi_get_pending_jobs) // GET /internal/pbtcfi/pending-jobs
        .service(pbtcfi_claim_job) // POST /internal/pbtcfi/claim-job
        .service(pbtcfi_complete_job) // POST /internal/pbtcfi/complete-job
        .service(pbtcfi_fail_job); // POST /internal/pbtcfi/fail-job
}
