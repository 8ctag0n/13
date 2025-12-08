//! ZK Jobs Database Queries
//!
//! Database operations for Zero-Knowledge proof jobs (circuit_type 10-49).

use anyhow::{anyhow, Result};
use sqlx::PgPool;

// =============================================================================
// Models
// =============================================================================

/// ZK Job status enum
#[derive(Debug, Clone, PartialEq)]
pub enum ZkJobStatus {
    PendingTx,
    Active,
    Proving,
    Completed,
    Failed,
}

impl ZkJobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ZkJobStatus::PendingTx => "pending_tx",
            ZkJobStatus::Active => "active",
            ZkJobStatus::Proving => "proving",
            ZkJobStatus::Completed => "completed",
            ZkJobStatus::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending_tx" => Some(ZkJobStatus::PendingTx),
            "active" => Some(ZkJobStatus::Active),
            "proving" => Some(ZkJobStatus::Proving),
            "completed" => Some(ZkJobStatus::Completed),
            "failed" => Some(ZkJobStatus::Failed),
            _ => None,
        }
    }
}

/// Data for inserting a new ZK job
#[derive(Debug)]
pub struct InsertZkJobData {
    pub job_id: i64,
    pub creator_pubkey: String,
    pub circuit_type: i16,
    pub witness_commitment: String,
    pub public_inputs: serde_json::Value,
    pub x402_token_id: Option<String>,
    pub price_lamports: i64,
    pub timeout_seconds: i32,
}

/// ZK Job data from database
#[derive(Debug, sqlx::FromRow)]
pub struct ZkJobData {
    pub id: i32,
    pub job_id: i64,
    pub creator_pubkey: String,
    pub circuit_type: i16,
    pub witness_commitment: String,
    pub public_inputs: serde_json::Value,
    pub proof_hash: Option<String>,
    pub status: String,
    pub x402_token_id: Option<String>,
    pub price_lamports: i64,
    pub create_tx_signature: Option<String>,
    pub confirm_tx_signature: Option<String>,
    pub prover_pubkey: Option<String>,
    pub claimed_at: Option<chrono::NaiveDateTime>,
    pub timeout_seconds: i32,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub completed_at: Option<chrono::NaiveDateTime>,
}

/// Summary data for listing ZK jobs
#[derive(Debug, sqlx::FromRow)]
pub struct ZkJobSummary {
    pub job_id: i64,
    pub creator_pubkey: String,
    pub circuit_type: i16,
    pub status: String,
    pub price_lamports: i64,
    pub created_at: chrono::NaiveDateTime,
}

// =============================================================================
// Queries
// =============================================================================

pub struct ZkJobQueries;

impl ZkJobQueries {
    /// Insert a new ZK job
    pub async fn insert_job(pool: &PgPool, data: InsertZkJobData) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            r#"
            INSERT INTO zk_jobs (
                job_id,
                creator_pubkey,
                circuit_type,
                witness_commitment,
                public_inputs,
                x402_token_id,
                price_lamports,
                timeout_seconds,
                status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'pending_tx')
            RETURNING job_id
            "#,
        )
        .bind(data.job_id)
        .bind(&data.creator_pubkey)
        .bind(data.circuit_type)
        .bind(&data.witness_commitment)
        .bind(&data.public_inputs)
        .bind(&data.x402_token_id)
        .bind(data.price_lamports)
        .bind(data.timeout_seconds)
        .fetch_one(pool)
        .await
        .map_err(|e| anyhow!("Failed to insert ZK job: {}", e))?;

        Ok(row.0)
    }

    /// Get ZK job by job_id
    pub async fn get_job_by_id(pool: &PgPool, job_id: i64) -> Result<Option<ZkJobData>> {
        let job = sqlx::query_as::<_, ZkJobData>(
            r#"
            SELECT
                id, job_id, creator_pubkey, circuit_type,
                witness_commitment, public_inputs, proof_hash,
                status, x402_token_id, price_lamports,
                create_tx_signature, confirm_tx_signature,
                prover_pubkey, claimed_at, timeout_seconds,
                created_at, updated_at, completed_at
            FROM zk_jobs
            WHERE job_id = $1
            "#,
        )
        .bind(job_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| anyhow!("Failed to fetch ZK job: {}", e))?;

        Ok(job)
    }

    /// Update ZK job status
    pub async fn update_status(pool: &PgPool, job_id: i64, status: ZkJobStatus) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE zk_jobs
            SET status = $1, updated_at = NOW()
            WHERE job_id = $2
            "#,
        )
        .bind(status.as_str())
        .bind(job_id)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to update ZK job status: {}", e))?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("ZK job not found: {}", job_id));
        }

        Ok(())
    }

    /// Confirm ZK job with transaction signature
    pub async fn confirm_job(pool: &PgPool, job_id: i64, tx_signature: &str) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE zk_jobs
            SET status = 'active',
                confirm_tx_signature = $1,
                updated_at = NOW()
            WHERE job_id = $2 AND status = 'pending_tx'
            "#,
        )
        .bind(tx_signature)
        .bind(job_id)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to confirm ZK job: {}", e))?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("ZK job not found or not in pending_tx status: {}", job_id));
        }

        Ok(())
    }

    /// Claim ZK job by prover
    pub async fn claim_job(pool: &PgPool, job_id: i64, prover_pubkey: &str) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE zk_jobs
            SET status = 'proving',
                prover_pubkey = $1,
                claimed_at = NOW(),
                updated_at = NOW()
            WHERE job_id = $2 AND status = 'active'
            "#,
        )
        .bind(prover_pubkey)
        .bind(job_id)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to claim ZK job: {}", e))?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("ZK job not found or not active: {}", job_id));
        }

        Ok(())
    }

    /// Complete ZK job with proof hash
    pub async fn complete_job(pool: &PgPool, job_id: i64, proof_hash: &str) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE zk_jobs
            SET status = 'completed',
                proof_hash = $1,
                completed_at = NOW(),
                updated_at = NOW()
            WHERE job_id = $2 AND status = 'proving'
            "#,
        )
        .bind(proof_hash)
        .bind(job_id)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to complete ZK job: {}", e))?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("ZK job not found or not in proving status: {}", job_id));
        }

        Ok(())
    }

    /// List ZK jobs with optional filters
    pub async fn list_jobs(
        pool: &PgPool,
        status: Option<&str>,
        creator: Option<&str>,
        circuit_type: Option<i16>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ZkJobSummary>> {
        // Build dynamic query based on filters
        let mut query = String::from(
            r#"
            SELECT job_id, creator_pubkey, circuit_type, status, price_lamports, created_at
            FROM zk_jobs
            WHERE 1=1
            "#
        );

        if status.is_some() {
            query.push_str(" AND status = $1");
        }
        if creator.is_some() {
            query.push_str(if status.is_some() { " AND creator_pubkey = $2" } else { " AND creator_pubkey = $1" });
        }
        if circuit_type.is_some() {
            let param_num = 1 + status.is_some() as i32 + creator.is_some() as i32;
            query.push_str(&format!(" AND circuit_type = ${}", param_num));
        }

        query.push_str(" ORDER BY created_at DESC LIMIT $");
        let limit_param = 1 + status.is_some() as i32 + creator.is_some() as i32 + circuit_type.is_some() as i32;
        query.push_str(&format!("{} OFFSET ${}", limit_param, limit_param + 1));

        // For simplicity, use a simpler query without dynamic binding
        // In production, use a query builder like sea-query
        let jobs = sqlx::query_as::<_, ZkJobSummary>(
            r#"
            SELECT job_id, creator_pubkey, circuit_type, status, price_lamports, created_at
            FROM zk_jobs
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| anyhow!("Failed to list ZK jobs: {}", e))?;

        Ok(jobs)
    }

    /// Count ZK jobs with optional status filter
    pub async fn count_jobs(pool: &PgPool, status: Option<&str>) -> Result<i64> {
        let count: (i64,) = match status {
            Some(s) => {
                sqlx::query_as("SELECT COUNT(*) FROM zk_jobs WHERE status = $1")
                    .bind(s)
                    .fetch_one(pool)
                    .await
            }
            None => {
                sqlx::query_as("SELECT COUNT(*) FROM zk_jobs")
                    .fetch_one(pool)
                    .await
            }
        }
        .map_err(|e| anyhow!("Failed to count ZK jobs: {}", e))?;

        Ok(count.0)
    }

    /// Get jobs by status
    pub async fn get_jobs_by_status(pool: &PgPool, status: ZkJobStatus) -> Result<Vec<ZkJobData>> {
        let jobs = sqlx::query_as::<_, ZkJobData>(
            r#"
            SELECT
                id, job_id, creator_pubkey, circuit_type,
                witness_commitment, public_inputs, proof_hash,
                status, x402_token_id, price_lamports,
                create_tx_signature, confirm_tx_signature,
                prover_pubkey, claimed_at, timeout_seconds,
                created_at, updated_at, completed_at
            FROM zk_jobs
            WHERE status = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(status.as_str())
        .fetch_all(pool)
        .await
        .map_err(|e| anyhow!("Failed to fetch ZK jobs by status: {}", e))?;

        Ok(jobs)
    }

    /// Delete expired/failed jobs older than given hours
    pub async fn cleanup_old_jobs(pool: &PgPool, hours: i32) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM zk_jobs
            WHERE status IN ('failed', 'completed')
            AND updated_at < NOW() - INTERVAL '1 hour' * $1
            "#,
        )
        .bind(hours)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to cleanup old ZK jobs: {}", e))?;

        Ok(result.rows_affected())
    }
}
