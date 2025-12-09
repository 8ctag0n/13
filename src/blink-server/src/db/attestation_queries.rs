//! Attestation Database Queries
//!
//! Database operations for storing and retrieving ZK proof attestations.

use anyhow::{anyhow, Result};
use chrono::NaiveDateTime;
use sqlx::PgPool;

// =============================================================================
// Models
// =============================================================================

/// Attestation row from database
#[derive(Debug, sqlx::FromRow)]
pub struct AttestationRow {
    pub id: i64,
    pub job_id: i64,
    pub circuit_type: i16,
    pub witness: serde_json::Value,
    pub vk_hash: String,
    pub verification_result: bool,
    pub verification_time_ms: i32,
    pub created_at: NaiveDateTime,
    pub used_for_dispute: bool,
    pub dispute_tx_signature: Option<String>,
    pub proof_json: Option<serde_json::Value>,
    pub retention_expires_at: Option<NaiveDateTime>,
}

// =============================================================================
// Queries
// =============================================================================

pub struct AttestationQueries;

impl AttestationQueries {
    /// Insert a new attestation with optional proof storage
    pub async fn insert(
        pool: &PgPool,
        job_id: i64,
        circuit_type: i16,
        witness: &serde_json::Value,
        vk_hash: &str,
        verification_result: bool,
        verification_time_ms: i32,
        proof_json: Option<&serde_json::Value>,
    ) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            r#"
            INSERT INTO attestations (
                job_id,
                circuit_type,
                witness,
                vk_hash,
                verification_result,
                verification_time_ms,
                proof_json,
                retention_expires_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, CASE WHEN $7 IS NOT NULL THEN NOW() + INTERVAL '30 days' ELSE NULL END)
            RETURNING id
            "#,
        )
        .bind(job_id)
        .bind(circuit_type)
        .bind(witness)
        .bind(vk_hash)
        .bind(verification_result)
        .bind(verification_time_ms)
        .bind(proof_json)
        .fetch_one(pool)
        .await
        .map_err(|e| anyhow!("Failed to insert attestation: {}", e))?;

        Ok(row.0)
    }

    /// Get attestation by job_id
    pub async fn get_by_job_id(pool: &PgPool, job_id: i64) -> Result<Option<AttestationRow>> {
        let attestation = sqlx::query_as::<_, AttestationRow>(
            r#"
            SELECT
                id, job_id, circuit_type, witness, vk_hash,
                verification_result, verification_time_ms,
                created_at, used_for_dispute, dispute_tx_signature,
                proof_json, retention_expires_at
            FROM attestations
            WHERE job_id = $1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(job_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| anyhow!("Failed to fetch attestation: {}", e))?;

        Ok(attestation)
    }

    /// Get attestation by id
    pub async fn get_by_id(pool: &PgPool, id: i64) -> Result<Option<AttestationRow>> {
        let attestation = sqlx::query_as::<_, AttestationRow>(
            r#"
            SELECT
                id, job_id, circuit_type, witness, vk_hash,
                verification_result, verification_time_ms,
                created_at, used_for_dispute, dispute_tx_signature,
                proof_json, retention_expires_at
            FROM attestations
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| anyhow!("Failed to fetch attestation: {}", e))?;

        Ok(attestation)
    }

    /// Mark attestation as used in dispute
    pub async fn mark_used_for_dispute(
        pool: &PgPool,
        id: i64,
        tx_signature: &str,
    ) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE attestations
            SET used_for_dispute = TRUE,
                dispute_tx_signature = $1
            WHERE id = $2
            "#,
        )
        .bind(tx_signature)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to mark attestation as used: {}", e))?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("Attestation not found: {}", id));
        }

        Ok(())
    }

    /// Get all attestations for a circuit type
    pub async fn get_by_circuit_type(
        pool: &PgPool,
        circuit_type: i16,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AttestationRow>> {
        let attestations = sqlx::query_as::<_, AttestationRow>(
            r#"
            SELECT
                id, job_id, circuit_type, witness, vk_hash,
                verification_result, verification_time_ms,
                created_at, used_for_dispute, dispute_tx_signature,
                proof_json, retention_expires_at
            FROM attestations
            WHERE circuit_type = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(circuit_type)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| anyhow!("Failed to fetch attestations: {}", e))?;

        Ok(attestations)
    }

    /// Get attestations by verification result
    pub async fn get_by_result(
        pool: &PgPool,
        verification_result: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AttestationRow>> {
        let attestations = sqlx::query_as::<_, AttestationRow>(
            r#"
            SELECT
                id, job_id, circuit_type, witness, vk_hash,
                verification_result, verification_time_ms,
                created_at, used_for_dispute, dispute_tx_signature,
                proof_json, retention_expires_at
            FROM attestations
            WHERE verification_result = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(verification_result)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| anyhow!("Failed to fetch attestations: {}", e))?;

        Ok(attestations)
    }

    /// Count attestations by circuit type
    pub async fn count_by_circuit_type(pool: &PgPool, circuit_type: i16) -> Result<i64> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM attestations
            WHERE circuit_type = $1
            "#,
        )
        .bind(circuit_type)
        .fetch_one(pool)
        .await
        .map_err(|e| anyhow!("Failed to count attestations: {}", e))?;

        Ok(count.0)
    }

    /// Count valid vs invalid attestations
    pub async fn get_verification_stats(pool: &PgPool) -> Result<(i64, i64)> {
        let row: (i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE verification_result = TRUE) as valid_count,
                COUNT(*) FILTER (WHERE verification_result = FALSE) as invalid_count
            FROM attestations
            "#,
        )
        .fetch_one(pool)
        .await
        .map_err(|e| anyhow!("Failed to get verification stats: {}", e))?;

        Ok(row)
    }

    /// Get average verification time by circuit type
    pub async fn get_avg_verification_time(pool: &PgPool, circuit_type: i16) -> Result<f64> {
        let row: (Option<f64>,) = sqlx::query_as(
            r#"
            SELECT AVG(verification_time_ms)::FLOAT
            FROM attestations
            WHERE circuit_type = $1 AND verification_result = TRUE
            "#,
        )
        .bind(circuit_type)
        .fetch_one(pool)
        .await
        .map_err(|e| anyhow!("Failed to get avg verification time: {}", e))?;

        Ok(row.0.unwrap_or(0.0))
    }

    /// Delete old attestations (cleanup)
    pub async fn cleanup_old_attestations(pool: &PgPool, hours: i32) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM attestations
            WHERE created_at < NOW() - INTERVAL '1 hour' * $1
            AND used_for_dispute = FALSE
            "#,
        )
        .bind(hours)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to cleanup attestations: {}", e))?;

        Ok(result.rows_affected())
    }

    /// Clear expired proof_json data (keeps attestation, just removes proof)
    pub async fn cleanup_expired_proofs(pool: &PgPool) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE attestations
            SET proof_json = NULL
            WHERE retention_expires_at < NOW()
            AND proof_json IS NOT NULL
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to cleanup expired proofs: {}", e))?;

        Ok(result.rows_affected())
    }
}
