use anyhow::{anyhow, Result};
use sqlx::PgPool;

use super::models::{InsertJobData, JobStatus, TempJobData};

/// Database operations for job management
pub struct JobQueries;

impl JobQueries {
    /// Insert a new pending job into the database
    pub async fn insert_pending_job(pool: &PgPool, data: InsertJobData) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            r#"
            INSERT INTO temp_job_data (
                job_id,
                creator_pubkey,
                encrypted_data,
                server_key,
                operation,
                operation_value,
                price_lamports,
                required_provers,
                consensus_threshold,
                status,
                payment_method,
                payment_token_mint
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING id
            "#,
        )
        .bind(data.job_id)
        .bind(data.creator_pubkey)
        .bind(data.encrypted_data)
        .bind(data.server_key)
        .bind(data.operation)
        .bind(data.operation_value)
        .bind(data.price_lamports)
        .bind(data.required_provers)
        .bind(data.consensus_threshold)
        .bind(JobStatus::PendingTx.as_str())
        .bind(&data.payment_method)
        .bind(&data.payment_token_mint)
        .fetch_one(pool)
        .await
        .map_err(|e| anyhow!("Failed to insert pending job: {}", e))?;

        Ok(row.0)
    }

    /// Get job by job_id
    pub async fn get_job_by_id(pool: &PgPool, job_id: i64) -> Result<Option<TempJobData>> {
        let job = sqlx::query_as::<_, TempJobData>(
            r#"
            SELECT id, job_id, creator_pubkey, encrypted_data, server_key,
                   operation, operation_value, price_lamports,
                   required_provers, consensus_threshold, status,
                   payment_method, payment_token_mint,
                   created_at, updated_at, expires_at
            FROM temp_job_data
            WHERE job_id = $1
            "#,
        )
        .bind(job_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| anyhow!("Failed to fetch job: {}", e))?;

        Ok(job)
    }

    /// Update job status
    #[allow(dead_code)]
    pub async fn update_job_status(pool: &PgPool, job_id: i64, status: JobStatus) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE temp_job_data
            SET status = $1
            WHERE job_id = $2
            "#,
        )
        .bind(status.as_str())
        .bind(job_id)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to update job status: {}", e))?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("Job not found: {}", job_id));
        }

        Ok(())
    }

    /// Update job status and tx_signature (for confirmed transactions)
    pub async fn confirm_job_with_signature(
        pool: &PgPool,
        job_id: i64,
        status: JobStatus,
        tx_signature: &str,
    ) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE temp_job_data
            SET status = $1, tx_signature = $2, updated_at = NOW()
            WHERE job_id = $3
            "#,
        )
        .bind(status.as_str())
        .bind(tx_signature)
        .bind(job_id)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to confirm job: {}", e))?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("Job not found: {}", job_id));
        }

        Ok(())
    }

    /// Get all jobs by status
    #[allow(dead_code)]
    pub async fn get_jobs_by_status(pool: &PgPool, status: JobStatus) -> Result<Vec<TempJobData>> {
        let jobs = sqlx::query_as::<_, TempJobData>(
            r#"
            SELECT id, job_id, creator_pubkey, encrypted_data, server_key,
                   operation, operation_value, price_lamports,
                   required_provers, consensus_threshold, status,
                   payment_method, payment_token_mint,
                   created_at, updated_at, expires_at
            FROM temp_job_data
            WHERE status = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(status.as_str())
        .fetch_all(pool)
        .await
        .map_err(|e| anyhow!("Failed to fetch jobs by status: {}", e))?;

        Ok(jobs)
    }

    /// Delete expired jobs
    pub async fn delete_expired_jobs(pool: &PgPool) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM temp_job_data
            WHERE expires_at < NOW()
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to delete expired jobs: {}", e))?;

        Ok(result.rows_affected())
    }

    /// Delete temp jobs that have been synced to blockchain_jobs
    /// This prevents duplicates in the UNION query
    pub async fn delete_synced_jobs(pool: &PgPool) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM temp_job_data
            WHERE job_id IN (SELECT job_id FROM blockchain_jobs)
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to delete synced jobs: {}", e))?;

        Ok(result.rows_affected())
    }

    /// Delete job by job_id
    pub async fn delete_job(pool: &PgPool, job_id: i64) -> Result<()> {
        let result = sqlx::query(
            r#"
            DELETE FROM temp_job_data
            WHERE job_id = $1
            "#,
        )
        .bind(job_id)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to delete job: {}", e))?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("Job not found: {}", job_id));
        }

        Ok(())
    }
}

/// Database operations for nonce management (anti-replay)
pub struct NonceQueries;

impl NonceQueries {
    /// Check if nonce has been used
    pub async fn is_nonce_used(pool: &PgPool, nonce: &str) -> Result<bool> {
        let result: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT nonce FROM used_nonces
            WHERE nonce = $1
            "#,
        )
        .bind(nonce)
        .fetch_optional(pool)
        .await
        .map_err(|e| anyhow!("Failed to check nonce: {}", e))?;

        Ok(result.is_some())
    }

    /// Mark nonce as used
    pub async fn mark_nonce_used(pool: &PgPool, nonce: &str) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO used_nonces (nonce)
            VALUES ($1)
            ON CONFLICT (nonce) DO NOTHING
            "#,
        )
        .bind(nonce)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to mark nonce as used: {}", e))?;

        Ok(())
    }

    /// Delete old nonces (older than specified seconds)
    pub async fn delete_old_nonces(pool: &PgPool, older_than_secs: i64) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM used_nonces
            WHERE used_at < NOW() - $1 * INTERVAL '1 second'
            "#,
        )
        .bind(older_than_secs)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to delete old nonces: {}", e))?;

        Ok(result.rows_affected())
    }
}

/// Database operations for witness storage
pub struct WitnessQueries;

impl WitnessQueries {
    /// Store witness data and return its commitment hash
    pub async fn store_witness(pool: &PgPool, commitment: &str, data: &[u8]) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO witnesses (commitment, data)
            VALUES ($1, $2)
            ON CONFLICT (commitment) DO NOTHING
            "#,
        )
        .bind(commitment)
        .bind(data)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to store witness: {}", e))?;

        Ok(())
    }

    /// Get witness data by commitment hash
    pub async fn get_witness(pool: &PgPool, commitment: &str) -> Result<Option<Vec<u8>>> {
        let result: Option<(Vec<u8>,)> = sqlx::query_as(
            r#"
            SELECT data FROM witnesses
            WHERE commitment = $1
            "#,
        )
        .bind(commitment)
        .fetch_optional(pool)
        .await
        .map_err(|e| anyhow!("Failed to fetch witness: {}", e))?;

        Ok(result.map(|(data,)| data))
    }
}

/// Database operations for FHE result storage
pub struct FheResultQueries;

impl FheResultQueries {
    /// Store FHE result data with job_id and optional prover info
    pub async fn store_result_with_job(
        pool: &PgPool,
        commitment: &str,
        data: &[u8],
        job_id: Option<i64>,
        prover_pubkey: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO fhe_results (commitment, data, job_id, prover_pubkey)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (commitment) DO UPDATE SET
                job_id = COALESCE(EXCLUDED.job_id, fhe_results.job_id),
                prover_pubkey = COALESCE(EXCLUDED.prover_pubkey, fhe_results.prover_pubkey)
            "#,
        )
        .bind(commitment)
        .bind(data)
        .bind(job_id)
        .bind(prover_pubkey)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to store FHE result: {}", e))?;

        Ok(())
    }

    /// Store FHE result data by commitment hash (legacy, without job_id)
    pub async fn store_result(pool: &PgPool, commitment: &str, data: &[u8]) -> Result<()> {
        Self::store_result_with_job(pool, commitment, data, None, None).await
    }

    /// Get FHE result data by commitment hash
    pub async fn get_result(pool: &PgPool, commitment: &str) -> Result<Option<Vec<u8>>> {
        let result: Option<(Vec<u8>,)> = sqlx::query_as(
            r#"
            SELECT data FROM fhe_results
            WHERE commitment = $1
            "#,
        )
        .bind(commitment)
        .fetch_optional(pool)
        .await
        .map_err(|e| anyhow!("Failed to fetch FHE result: {}", e))?;

        Ok(result.map(|(data,)| data))
    }

    /// Get FHE results by job_id (returns all results for multi-prover consensus)
    pub async fn get_results_by_job_id(pool: &PgPool, job_id: i64) -> Result<Vec<(String, Vec<u8>)>> {
        let results: Vec<(String, Vec<u8>)> = sqlx::query_as(
            r#"
            SELECT commitment, data FROM fhe_results
            WHERE job_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(job_id)
        .fetch_all(pool)
        .await
        .map_err(|e| anyhow!("Failed to fetch FHE results by job_id: {}", e))?;

        Ok(results)
    }

    /// Get first FHE result by job_id (for simple retrieval)
    pub async fn get_first_result_by_job_id(pool: &PgPool, job_id: i64) -> Result<Option<Vec<u8>>> {
        let result: Option<(Vec<u8>,)> = sqlx::query_as(
            r#"
            SELECT data FROM fhe_results
            WHERE job_id = $1
            ORDER BY created_at ASC
            LIMIT 1
            "#,
        )
        .bind(job_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| anyhow!("Failed to fetch FHE result by job_id: {}", e))?;

        Ok(result.map(|(data,)| data))
    }
}

#[cfg(test)]
mod tests {

    // Tests would require a test database instance
    // Implement integration tests separately
}
