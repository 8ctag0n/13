//! Database queries for Futarchy prediction markets

use anyhow::{anyhow, Result};
use sqlx::PgPool;
use chrono::{DateTime, Utc};

// =============================================================================
// Data Types
// =============================================================================

/// Market data for insertion
#[derive(Debug)]
pub struct CreateMarketData {
    pub id: String,
    pub question: String,
    pub question_hash: String,
    pub creator: String,
    pub oracle: String,
    pub resolution_window_secs: i64,
    pub max_bet_lamports: i64,
    pub ends_at: Option<DateTime<Utc>>,
    pub tx_signature: Option<String>,
    pub slot: Option<i64>,
}

/// Market query result
#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct MarketRow {
    pub id: String,
    pub question: String,
    pub question_hash: String,
    pub creator: String,
    pub oracle: String,
    pub resolution_window_secs: i64,
    pub max_bet_lamports: i64,
    pub yes_pool_lamports: i64,
    pub no_pool_lamports: i64,
    pub status: String,
    pub outcome: Option<bool>,
    pub created_at: DateTime<Utc>,
    pub ends_at: Option<DateTime<Utc>>,
    pub settled_at: Option<DateTime<Utc>>,
    pub tx_signature: Option<String>,
    pub slot: Option<i64>,
}

/// Ciphertext data for insertion
#[derive(Debug)]
pub struct CreateCiphertextData {
    pub hash: String,
    pub ciphertext: Vec<u8>,
    pub ciphertext_type: String,
    pub market_id: Option<String>,
    pub side: Option<bool>,
    pub version: i32,
    pub created_by: Option<String>,
}

/// Ciphertext query result
#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct CiphertextRow {
    pub hash: String,
    #[sqlx(skip)]
    #[serde(skip_serializing)]
    pub ciphertext: Vec<u8>,
    pub ciphertext_type: String,
    pub market_id: Option<String>,
    pub side: Option<bool>,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<String>,
}

/// Position data for insertion
#[derive(Debug)]
pub struct CreatePositionData {
    pub market_id: String,
    pub bettor: String,
    pub side: bool,
    pub amount_lamports: i64,
    pub encrypted_amount_hash: Option<String>,
    pub tx_signature: Option<String>,
    pub slot: Option<i64>,
}

/// Position query result
#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct PositionRow {
    pub id: i32,
    pub market_id: String,
    pub bettor: String,
    pub side: bool,
    pub amount_lamports: i64,
    pub encrypted_amount_hash: Option<String>,
    pub tx_signature: Option<String>,
    pub slot: Option<i64>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

/// FHE job data
#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct FheJobRow {
    pub id: i32,
    pub job_id: Option<i64>,
    pub market_id: String,
    pub side: bool,
    pub pool_ciphertext_hash: String,
    pub bet_ciphertext_hash: String,
    pub result_ciphertext_hash: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
}

// =============================================================================
// Queries
// =============================================================================

pub struct FutarchyQueries;

impl FutarchyQueries {
    // =========================================================================
    // Market Queries
    // =========================================================================

    /// Create a new market
    pub async fn create_market(pool: &PgPool, data: &CreateMarketData) -> Result<MarketRow> {
        let market = sqlx::query_as::<_, MarketRow>(
            r#"
            INSERT INTO futarchy_markets (
                id, question, question_hash, creator, oracle,
                resolution_window_secs, max_bet_lamports, ends_at,
                tx_signature, slot
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, question, question_hash, creator, oracle,
                      resolution_window_secs, max_bet_lamports,
                      yes_pool_lamports, no_pool_lamports,
                      status, outcome, created_at, ends_at, settled_at,
                      tx_signature, slot
            "#,
        )
        .bind(&data.id)
        .bind(&data.question)
        .bind(&data.question_hash)
        .bind(&data.creator)
        .bind(&data.oracle)
        .bind(data.resolution_window_secs)
        .bind(data.max_bet_lamports)
        .bind(data.ends_at)
        .bind(&data.tx_signature)
        .bind(data.slot)
        .fetch_one(pool)
        .await
        .map_err(|e| anyhow!("Failed to create market: {}", e))?;

        Ok(market)
    }

    /// Get a market by ID
    pub async fn get_market(pool: &PgPool, market_id: &str) -> Result<Option<MarketRow>> {
        let market = sqlx::query_as::<_, MarketRow>(
            r#"
            SELECT id, question, question_hash, creator, oracle,
                   resolution_window_secs, max_bet_lamports,
                   yes_pool_lamports, no_pool_lamports,
                   status, outcome, created_at, ends_at, settled_at,
                   tx_signature, slot
            FROM futarchy_markets
            WHERE id = $1
            "#,
        )
        .bind(market_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| anyhow!("Failed to fetch market: {}", e))?;

        Ok(market)
    }

    /// List markets with optional filtering
    pub async fn list_markets(
        pool: &PgPool,
        status: Option<&str>,
        creator: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<MarketRow>> {
        let markets = match (status, creator) {
            (Some(s), Some(c)) => {
                sqlx::query_as::<_, MarketRow>(
                    r#"
                    SELECT id, question, question_hash, creator, oracle,
                           resolution_window_secs, max_bet_lamports,
                           yes_pool_lamports, no_pool_lamports,
                           status, outcome, created_at, ends_at, settled_at,
                           tx_signature, slot
                    FROM futarchy_markets
                    WHERE status = $1 AND creator = $2
                    ORDER BY created_at DESC
                    LIMIT $3 OFFSET $4
                    "#,
                )
                .bind(s)
                .bind(c)
                .bind(limit)
                .bind(offset)
                .fetch_all(pool)
                .await
            }
            (Some(s), None) => {
                sqlx::query_as::<_, MarketRow>(
                    r#"
                    SELECT id, question, question_hash, creator, oracle,
                           resolution_window_secs, max_bet_lamports,
                           yes_pool_lamports, no_pool_lamports,
                           status, outcome, created_at, ends_at, settled_at,
                           tx_signature, slot
                    FROM futarchy_markets
                    WHERE status = $1
                    ORDER BY created_at DESC
                    LIMIT $2 OFFSET $3
                    "#,
                )
                .bind(s)
                .bind(limit)
                .bind(offset)
                .fetch_all(pool)
                .await
            }
            (None, Some(c)) => {
                sqlx::query_as::<_, MarketRow>(
                    r#"
                    SELECT id, question, question_hash, creator, oracle,
                           resolution_window_secs, max_bet_lamports,
                           yes_pool_lamports, no_pool_lamports,
                           status, outcome, created_at, ends_at, settled_at,
                           tx_signature, slot
                    FROM futarchy_markets
                    WHERE creator = $1
                    ORDER BY created_at DESC
                    LIMIT $2 OFFSET $3
                    "#,
                )
                .bind(c)
                .bind(limit)
                .bind(offset)
                .fetch_all(pool)
                .await
            }
            (None, None) => {
                sqlx::query_as::<_, MarketRow>(
                    r#"
                    SELECT id, question, question_hash, creator, oracle,
                           resolution_window_secs, max_bet_lamports,
                           yes_pool_lamports, no_pool_lamports,
                           status, outcome, created_at, ends_at, settled_at,
                           tx_signature, slot
                    FROM futarchy_markets
                    ORDER BY created_at DESC
                    LIMIT $1 OFFSET $2
                    "#,
                )
                .bind(limit)
                .bind(offset)
                .fetch_all(pool)
                .await
            }
        };

        markets.map_err(|e| anyhow!("Failed to list markets: {}", e))
    }

    /// Update market pool totals
    pub async fn update_pool_totals(
        pool: &PgPool,
        market_id: &str,
        yes_pool: i64,
        no_pool: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE futarchy_markets
            SET yes_pool_lamports = $2, no_pool_lamports = $3
            WHERE id = $1
            "#,
        )
        .bind(market_id)
        .bind(yes_pool)
        .bind(no_pool)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to update pool totals: {}", e))?;

        Ok(())
    }

    /// Settle a market
    pub async fn settle_market(
        pool: &PgPool,
        market_id: &str,
        outcome: bool,
    ) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE futarchy_markets
            SET status = 'resolved',
                outcome = $2,
                settled_at = NOW()
            WHERE id = $1 AND status = 'active'
            "#,
        )
        .bind(market_id)
        .bind(outcome)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to settle market: {}", e))?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("Market not found or not active: {}", market_id));
        }

        Ok(())
    }

    // =========================================================================
    // Ciphertext Queries
    // =========================================================================

    /// Store a ciphertext
    pub async fn store_ciphertext(pool: &PgPool, data: &CreateCiphertextData) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO futarchy_ciphertexts (
                hash, ciphertext, ciphertext_type, market_id,
                side, version, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (hash) DO NOTHING
            "#,
        )
        .bind(&data.hash)
        .bind(&data.ciphertext)
        .bind(&data.ciphertext_type)
        .bind(&data.market_id)
        .bind(data.side)
        .bind(data.version)
        .bind(&data.created_by)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to store ciphertext: {}", e))?;

        Ok(())
    }

    /// Get ciphertext by hash
    pub async fn get_ciphertext(pool: &PgPool, hash: &str) -> Result<Option<Vec<u8>>> {
        let row: Option<(Vec<u8>,)> = sqlx::query_as(
            "SELECT ciphertext FROM futarchy_ciphertexts WHERE hash = $1",
        )
        .bind(hash)
        .fetch_optional(pool)
        .await
        .map_err(|e| anyhow!("Failed to fetch ciphertext: {}", e))?;

        Ok(row.map(|(c,)| c))
    }

    /// Get ciphertext metadata (without the actual bytes)
    pub async fn get_ciphertext_metadata(pool: &PgPool, hash: &str) -> Result<Option<CiphertextRow>> {
        // We need a different struct without the ciphertext field or handle it differently
        let row = sqlx::query_as::<_, CiphertextMetadata>(
            r#"
            SELECT hash, ciphertext_type, market_id, side, version, created_at, created_by
            FROM futarchy_ciphertexts
            WHERE hash = $1
            "#,
        )
        .bind(hash)
        .fetch_optional(pool)
        .await
        .map_err(|e| anyhow!("Failed to fetch ciphertext metadata: {}", e))?;

        Ok(row.map(|m| CiphertextRow {
            hash: m.hash,
            ciphertext: vec![], // Empty, not fetched
            ciphertext_type: m.ciphertext_type,
            market_id: m.market_id,
            side: m.side,
            version: m.version,
            created_at: m.created_at,
            created_by: m.created_by,
        }))
    }

    /// Get latest pool ciphertext for a market side
    pub async fn get_pool_ciphertext(
        pool: &PgPool,
        market_id: &str,
        side: bool,
    ) -> Result<Option<Vec<u8>>> {
        let row: Option<(Vec<u8>,)> = sqlx::query_as(
            r#"
            SELECT ciphertext
            FROM futarchy_ciphertexts
            WHERE market_id = $1 AND side = $2 AND ciphertext_type = 'pool'
            ORDER BY version DESC
            LIMIT 1
            "#,
        )
        .bind(market_id)
        .bind(side)
        .fetch_optional(pool)
        .await
        .map_err(|e| anyhow!("Failed to fetch pool ciphertext: {}", e))?;

        Ok(row.map(|(c,)| c))
    }

    /// Get latest pool ciphertext hash for a market side
    pub async fn get_pool_ciphertext_hash(
        pool: &PgPool,
        market_id: &str,
        side: bool,
    ) -> Result<Option<String>> {
        let row: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT hash
            FROM futarchy_ciphertexts
            WHERE market_id = $1 AND side = $2 AND ciphertext_type = 'pool'
            ORDER BY version DESC
            LIMIT 1
            "#,
        )
        .bind(market_id)
        .bind(side)
        .fetch_optional(pool)
        .await
        .map_err(|e| anyhow!("Failed to fetch pool ciphertext hash: {}", e))?;

        Ok(row.map(|(h,)| h))
    }

    /// Get next version number for pool ciphertext
    pub async fn get_next_pool_version(
        pool: &PgPool,
        market_id: &str,
        side: bool,
    ) -> Result<i32> {
        let row: Option<(i32,)> = sqlx::query_as(
            r#"
            SELECT MAX(version)
            FROM futarchy_ciphertexts
            WHERE market_id = $1 AND side = $2 AND ciphertext_type = 'pool'
            "#,
        )
        .bind(market_id)
        .bind(side)
        .fetch_optional(pool)
        .await
        .map_err(|e| anyhow!("Failed to get pool version: {}", e))?;

        Ok(row.and_then(|(v,)| Some(v + 1)).unwrap_or(0))
    }

    // =========================================================================
    // Position Queries
    // =========================================================================

    /// Create a position (bet)
    pub async fn create_position(pool: &PgPool, data: &CreatePositionData) -> Result<PositionRow> {
        let position = sqlx::query_as::<_, PositionRow>(
            r#"
            INSERT INTO futarchy_positions (
                market_id, bettor, side, amount_lamports,
                encrypted_amount_hash, tx_signature, slot
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (market_id, bettor, side) DO UPDATE SET
                amount_lamports = futarchy_positions.amount_lamports + EXCLUDED.amount_lamports,
                tx_signature = EXCLUDED.tx_signature,
                slot = EXCLUDED.slot
            RETURNING id, market_id, bettor, side, amount_lamports,
                      encrypted_amount_hash, tx_signature, slot, status, created_at
            "#,
        )
        .bind(&data.market_id)
        .bind(&data.bettor)
        .bind(data.side)
        .bind(data.amount_lamports)
        .bind(&data.encrypted_amount_hash)
        .bind(&data.tx_signature)
        .bind(data.slot)
        .fetch_one(pool)
        .await
        .map_err(|e| anyhow!("Failed to create position: {}", e))?;

        Ok(position)
    }

    /// Get positions for a market
    pub async fn get_market_positions(
        pool: &PgPool,
        market_id: &str,
    ) -> Result<Vec<PositionRow>> {
        let positions = sqlx::query_as::<_, PositionRow>(
            r#"
            SELECT id, market_id, bettor, side, amount_lamports,
                   encrypted_amount_hash, tx_signature, slot, status, created_at
            FROM futarchy_positions
            WHERE market_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(market_id)
        .fetch_all(pool)
        .await
        .map_err(|e| anyhow!("Failed to get market positions: {}", e))?;

        Ok(positions)
    }

    /// Get positions for a bettor
    pub async fn get_bettor_positions(
        pool: &PgPool,
        bettor: &str,
    ) -> Result<Vec<PositionRow>> {
        let positions = sqlx::query_as::<_, PositionRow>(
            r#"
            SELECT id, market_id, bettor, side, amount_lamports,
                   encrypted_amount_hash, tx_signature, slot, status, created_at
            FROM futarchy_positions
            WHERE bettor = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(bettor)
        .fetch_all(pool)
        .await
        .map_err(|e| anyhow!("Failed to get bettor positions: {}", e))?;

        Ok(positions)
    }

    /// Update position status
    pub async fn update_position_status(
        pool: &PgPool,
        position_id: i32,
        status: &str,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE futarchy_positions SET status = $2 WHERE id = $1",
        )
        .bind(position_id)
        .bind(status)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to update position status: {}", e))?;

        Ok(())
    }

    // =========================================================================
    // FHE Job Queries
    // =========================================================================

    /// Create an FHE job for pool update
    pub async fn create_fhe_job(
        pool: &PgPool,
        market_id: &str,
        side: bool,
        pool_ciphertext_hash: &str,
        bet_ciphertext_hash: &str,
    ) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            r#"
            INSERT INTO futarchy_fhe_jobs (
                market_id, side, pool_ciphertext_hash, bet_ciphertext_hash
            )
            VALUES ($1, $2, $3, $4)
            RETURNING id
            "#,
        )
        .bind(market_id)
        .bind(side)
        .bind(pool_ciphertext_hash)
        .bind(bet_ciphertext_hash)
        .fetch_one(pool)
        .await
        .map_err(|e| anyhow!("Failed to create FHE job: {}", e))?;

        Ok(row.0)
    }

    /// Get pending FHE jobs
    pub async fn get_pending_fhe_jobs(pool: &PgPool, limit: i64) -> Result<Vec<FheJobRow>> {
        let jobs = sqlx::query_as::<_, FheJobRow>(
            r#"
            SELECT id, job_id, market_id, side, pool_ciphertext_hash,
                   bet_ciphertext_hash, result_ciphertext_hash, status,
                   created_at, processed_at
            FROM futarchy_fhe_jobs
            WHERE status = 'pending'
            ORDER BY created_at ASC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| anyhow!("Failed to get pending FHE jobs: {}", e))?;

        Ok(jobs)
    }

    /// Complete an FHE job
    pub async fn complete_fhe_job(
        pool: &PgPool,
        job_id: i32,
        result_hash: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE futarchy_fhe_jobs
            SET status = 'completed',
                result_ciphertext_hash = $2,
                processed_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(job_id)
        .bind(result_hash)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to complete FHE job: {}", e))?;

        Ok(())
    }

    /// Fail an FHE job
    pub async fn fail_fhe_job(pool: &PgPool, job_id: i32) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE futarchy_fhe_jobs
            SET status = 'failed', processed_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(job_id)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to mark FHE job as failed: {}", e))?;

        Ok(())
    }

    // =========================================================================
    // E2E Anti-Spam Support Queries
    // =========================================================================

    /// Save ciphertext with transaction info (anti-spam flow)
    /// Returns the hash of the stored ciphertext
    pub async fn save_ciphertext_with_tx(
        pool: &PgPool,
        ciphertext: &[u8],
        hash: &str,
        tx_sig: Option<&str>,
        server_key_hash: &str,
        user_pubkey: &str,
    ) -> Result<String> {
        sqlx::query(
            r#"
            INSERT INTO futarchy_ciphertexts (
                hash, ciphertext, ciphertext_type, server_key_hash,
                tx_signature, status, created_by
            )
            VALUES ($1, $2, 'bet', $3, $4, 'pending', $5)
            ON CONFLICT (hash) DO UPDATE SET
                tx_signature = EXCLUDED.tx_signature,
                server_key_hash = EXCLUDED.server_key_hash,
                status = EXCLUDED.status
            "#,
        )
        .bind(hash)
        .bind(ciphertext)
        .bind(server_key_hash)
        .bind(tx_sig)
        .bind(user_pubkey)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to save ciphertext with TX: {}", e))?;

        Ok(hash.to_string())
    }

    /// Get ciphertext by hash (for prover to fetch validated data)
    pub async fn get_ciphertext_by_hash(pool: &PgPool, hash: &str) -> Result<Option<Vec<u8>>> {
        let row: Option<(Vec<u8>,)> = sqlx::query_as(
            "SELECT ciphertext FROM futarchy_ciphertexts WHERE hash = $1",
        )
        .bind(hash)
        .fetch_optional(pool)
        .await
        .map_err(|e| anyhow!("Failed to fetch ciphertext by hash: {}", e))?;

        Ok(row.map(|(c,)| c))
    }

    /// Mark ciphertext as confirmed after TX confirmation
    pub async fn mark_confirmed(pool: &PgPool, tx_sig: &str) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE futarchy_ciphertexts
            SET status = 'confirmed'
            WHERE tx_signature = $1
            "#,
        )
        .bind(tx_sig)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to mark ciphertext as confirmed: {}", e))?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("No ciphertext found with tx_signature: {}", tx_sig));
        }

        Ok(())
    }

    /// Mark ciphertext as failed and cleanup if needed
    pub async fn mark_failed_and_cleanup(pool: &PgPool, tx_sig: &str) -> Result<()> {
        // Mark as failed first
        sqlx::query(
            r#"
            UPDATE futarchy_ciphertexts
            SET status = 'failed'
            WHERE tx_signature = $1
            "#,
        )
        .bind(tx_sig)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to mark ciphertext as failed: {}", e))?;

        // Optionally delete failed ciphertexts (to save space)
        // Uncomment if you want aggressive cleanup:
        // sqlx::query("DELETE FROM futarchy_ciphertexts WHERE status = 'failed' AND created_at < NOW() - INTERVAL '1 hour'")
        //     .execute(pool)
        //     .await?;

        Ok(())
    }

    /// Get ciphertext with server_key_hash (for validation flow)
    pub async fn get_ciphertext_with_server_key(
        pool: &PgPool,
        hash: &str,
    ) -> Result<Option<CiphertextWithServerKey>> {
        let row = sqlx::query_as::<_, CiphertextWithServerKey>(
            r#"
            SELECT hash, ciphertext, server_key_hash, status, tx_signature, created_by
            FROM futarchy_ciphertexts
            WHERE hash = $1
            "#,
        )
        .bind(hash)
        .fetch_optional(pool)
        .await
        .map_err(|e| anyhow!("Failed to fetch ciphertext with server key: {}", e))?;

        Ok(row)
    }

    /// Update position with PDA info after on-chain creation
    pub async fn update_position_pda(
        pool: &PgPool,
        position_id: i32,
        position_pda: &str,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE futarchy_positions SET position_pda = $2 WHERE id = $1",
        )
        .bind(position_id)
        .bind(position_pda)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to update position PDA: {}", e))?;

        Ok(())
    }

    /// Get position by PDA
    pub async fn get_position_by_pda(
        pool: &PgPool,
        position_pda: &str,
    ) -> Result<Option<PositionRow>> {
        let pos = sqlx::query_as::<_, PositionRow>(
            r#"
            SELECT id, market_id, bettor, side, amount_lamports,
                   encrypted_amount_hash, tx_signature, slot, status, created_at
            FROM futarchy_positions
            WHERE position_pda = $1
            "#,
        )
        .bind(position_pda)
        .fetch_optional(pool)
        .await
        .map_err(|e| anyhow!("Failed to fetch position by PDA: {}", e))?;

        Ok(pos)
    }

    // =========================================================================
    // Anti-Spam Validation Tracking (optional audit trail)
    // =========================================================================

    /// Record a validation attempt
    pub async fn record_validation(
        pool: &PgPool,
        ciphertext_hash: &str,
        user_pubkey: &str,
        server_key_hash: &str,
        is_valid: bool,
        validation_error: Option<&str>,
    ) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            r#"
            INSERT INTO futarchy_anti_spam_validations (
                ciphertext_hash, user_pubkey, server_key_hash,
                is_valid, validation_error, status
            )
            VALUES ($1, $2, $3, $4, $5, CASE WHEN $4 THEN 'validated' ELSE 'rejected' END)
            RETURNING id
            "#,
        )
        .bind(ciphertext_hash)
        .bind(user_pubkey)
        .bind(server_key_hash)
        .bind(is_valid)
        .bind(validation_error)
        .fetch_one(pool)
        .await
        .map_err(|e| anyhow!("Failed to record validation: {}", e))?;

        Ok(row.0)
    }

    /// Update validation with TX info
    pub async fn update_validation_tx(
        pool: &PgPool,
        validation_id: i32,
        tx_signature: &str,
        position_pda: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE futarchy_anti_spam_validations
            SET tx_signature = $2,
                position_pda = $3,
                submitted_at = NOW(),
                status = 'tx_submitted'
            WHERE id = $1
            "#,
        )
        .bind(validation_id)
        .bind(tx_signature)
        .bind(position_pda)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to update validation TX: {}", e))?;

        Ok(())
    }

    /// Mark validation as confirmed (after TX confirmation)
    pub async fn mark_validation_confirmed(pool: &PgPool, tx_signature: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE futarchy_anti_spam_validations
            SET status = 'tx_confirmed', confirmed_at = NOW()
            WHERE tx_signature = $1
            "#,
        )
        .bind(tx_signature)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to mark validation as confirmed: {}", e))?;

        Ok(())
    }

    /// Mark validation as failed (after TX failure)
    pub async fn mark_validation_failed(pool: &PgPool, tx_signature: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE futarchy_anti_spam_validations
            SET status = 'tx_failed'
            WHERE tx_signature = $1
            "#,
        )
        .bind(tx_signature)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to mark validation as failed: {}", e))?;

        Ok(())
    }
}

/// Ciphertext metadata (without actual bytes)
#[derive(Debug, sqlx::FromRow)]
struct CiphertextMetadata {
    pub hash: String,
    pub ciphertext_type: String,
    pub market_id: Option<String>,
    pub side: Option<bool>,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<String>,
}

/// Ciphertext with server key info (for E2E anti-spam flow)
#[derive(Debug, sqlx::FromRow)]
pub struct CiphertextWithServerKey {
    pub hash: String,
    pub ciphertext: Vec<u8>,
    pub server_key_hash: Option<String>,
    pub status: String,
    pub tx_signature: Option<String>,
    pub created_by: Option<String>,
}
