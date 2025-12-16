//! Database queries for pBTCFi loans and events

use anyhow::{anyhow, Result};
use sqlx::PgPool;
use serde_json::json;

// Re-export types from pbtcfi
// Note: This will require adding pbtcfi types as a dependency to blink-server
// For now, we'll define minimal inline types

/// Minimal loan data for insertion
pub struct LoanData {
    pub loan_id: String,
    pub borrower: String,
    pub btc_commitment: String,
    pub btc_encrypted_c1: String,
    pub btc_encrypted_c2: String,
    pub created_at: i64,
}

/// Loan query result
#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct LoanRow {
    pub loan_id: String,
    pub borrower: String,
    pub status: String,
    pub btc_commitment: String,
    pub plst_commitment: Option<String>,
    pub ltv_commitment: Option<String>,
    pub btc_encrypted_c1: String,
    pub btc_encrypted_c2: String,
    pub plst_encrypted_c1: Option<String>,
    pub plst_encrypted_c2: Option<String>,
    pub collateral_hash: Option<String>,
    pub created_at: i64,
    pub activated_at: Option<i64>,
    pub repaid_at: Option<i64>,
    pub liquidated_at: Option<i64>,
}

/// pBTCFi database query operations
pub struct PbtcfiQueries;

impl PbtcfiQueries {
    /// Insert or update a loan (upsert)
    pub async fn upsert_loan(pool: &PgPool, loan: &LoanData) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO pbtcfi_loans (
                loan_id, borrower, btc_commitment,
                btc_encrypted_c1, btc_encrypted_c2,
                status, created_at
            )
            VALUES ($1, $2, $3, $4, $5, 'pending', $6)
            ON CONFLICT (loan_id) DO UPDATE SET
                synced_at = NOW()
            "#,
        )
        .bind(&loan.loan_id)
        .bind(&loan.borrower)
        .bind(&loan.btc_commitment)
        .bind(&loan.btc_encrypted_c1)
        .bind(&loan.btc_encrypted_c2)
        .bind(loan.created_at)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to upsert loan: {}", e))?;

        Ok(())
    }

    /// Get a specific loan by ID
    pub async fn get_loan(pool: &PgPool, loan_id: &str) -> Result<Option<LoanRow>> {
        let loan = sqlx::query_as::<_, LoanRow>(
            r#"
            SELECT loan_id, borrower, status,
                   btc_commitment, plst_commitment, ltv_commitment,
                   btc_encrypted_c1, btc_encrypted_c2,
                   plst_encrypted_c1, plst_encrypted_c2,
                   collateral_hash, created_at, activated_at,
                   repaid_at, liquidated_at
            FROM pbtcfi_loans
            WHERE loan_id = $1
            "#,
        )
        .bind(loan_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| anyhow!("Failed to fetch loan: {}", e))?;

        Ok(loan)
    }

    /// List loans with optional filtering
    pub async fn list_loans(
        pool: &PgPool,
        status: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<LoanRow>> {
        let loans = if let Some(status_filter) = status {
            sqlx::query_as::<_, LoanRow>(
                r#"
                SELECT loan_id, borrower, status,
                       btc_commitment, plst_commitment, ltv_commitment,
                       btc_encrypted_c1, btc_encrypted_c2,
                       plst_encrypted_c1, plst_encrypted_c2,
                       collateral_hash, created_at, activated_at,
                       repaid_at, liquidated_at
                FROM pbtcfi_loans
                WHERE status = $1
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(status_filter)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
        } else {
            sqlx::query_as::<_, LoanRow>(
                r#"
                SELECT loan_id, borrower, status,
                       btc_commitment, plst_commitment, ltv_commitment,
                       btc_encrypted_c1, btc_encrypted_c2,
                       plst_encrypted_c1, plst_encrypted_c2,
                       collateral_hash, created_at, activated_at,
                       repaid_at, liquidated_at
                FROM pbtcfi_loans
                ORDER BY created_at DESC
                LIMIT $1 OFFSET $2
                "#,
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
        };

        loans.map_err(|e| anyhow!("Failed to list loans: {}", e))
    }

    /// Get loans by borrower address
    pub async fn get_borrower_loans(
        pool: &PgPool,
        borrower: &str,
        limit: i64,
    ) -> Result<Vec<LoanRow>> {
        let loans = sqlx::query_as::<_, LoanRow>(
            r#"
            SELECT loan_id, borrower, status,
                   btc_commitment, plst_commitment, ltv_commitment,
                   btc_encrypted_c1, btc_encrypted_c2,
                   plst_encrypted_c1, plst_encrypted_c2,
                   collateral_hash, created_at, activated_at,
                   repaid_at, liquidated_at
            FROM pbtcfi_loans
            WHERE borrower = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(borrower)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| anyhow!("Failed to fetch borrower loans: {}", e))?;

        Ok(loans)
    }

    /// Update loan status
    pub async fn update_loan_status(
        pool: &PgPool,
        loan_id: &str,
        status: &str,
    ) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE pbtcfi_loans
            SET status = $1, synced_at = NOW()
            WHERE loan_id = $2
            "#,
        )
        .bind(status)
        .bind(loan_id)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to update loan status: {}", e))?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("Loan not found: {}", loan_id));
        }

        Ok(())
    }

    /// Update collateral hash for a loan
    pub async fn update_collateral(
        pool: &PgPool,
        loan_id: &str,
        collateral_hash: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE pbtcfi_loans
            SET collateral_hash = $1,
                status = CASE
                    WHEN status = 'pending' THEN 'collateral_registered'
                    ELSE status
                END,
                synced_at = NOW()
            WHERE loan_id = $2
            "#,
        )
        .bind(collateral_hash)
        .bind(loan_id)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to update collateral: {}", e))?;

        Ok(())
    }

    /// Get last synced block number
    pub async fn get_last_synced_block(pool: &PgPool) -> Result<u64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT last_synced_block FROM pbtcfi_sync_state WHERE id = 1"
        )
        .fetch_one(pool)
        .await
        .map_err(|e| anyhow!("Failed to get last synced block: {}", e))?;

        Ok(row.0 as u64)
    }

    /// Update last synced block number
    pub async fn update_last_synced_block(pool: &PgPool, block: u64) -> Result<()> {
        sqlx::query(
            "UPDATE pbtcfi_sync_state SET last_synced_block = $1, updated_at = NOW() WHERE id = 1",
        )
        .bind(block as i64)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to update last synced block: {}", e))?;

        Ok(())
    }

    /// Insert an event record (audit log)
    pub async fn insert_event(
        pool: &PgPool,
        event_type: &str,
        loan_id: &str,
        block_number: u64,
        transaction_hash: &str,
        event_index: i32,
        event_data: &serde_json::Value,
        timestamp: i64,
    ) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            r#"
            INSERT INTO pbtcfi_events (
                event_type, loan_id, block_number,
                transaction_hash, event_index, event_data, timestamp
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (transaction_hash, event_index) DO UPDATE SET
                synced_at = NOW()
            RETURNING id
            "#,
        )
        .bind(event_type)
        .bind(loan_id)
        .bind(block_number as i64)
        .bind(transaction_hash)
        .bind(event_index)
        .bind(event_data)
        .bind(timestamp)
        .fetch_one(pool)
        .await
        .map_err(|e| anyhow!("Failed to insert event: {}", e))?;

        Ok(row.0)
    }

    /// Get recent events
    pub async fn get_recent_events(pool: &PgPool, limit: i64) -> Result<Vec<EventRow>> {
        let events = sqlx::query_as::<_, EventRow>(
            r#"
            SELECT id, event_type, loan_id, block_number,
                   transaction_hash, event_index, event_data,
                   timestamp
            FROM pbtcfi_events
            ORDER BY block_number DESC, event_index DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| anyhow!("Failed to fetch events: {}", e))?;

        Ok(events)
    }
}

/// Event query result
#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct EventRow {
    pub id: i64,
    pub event_type: String,
    pub loan_id: String,
    pub block_number: i64,
    pub transaction_hash: String,
    pub event_index: i32,
    pub event_data: serde_json::Value,
    pub timestamp: i64,
}
