//! x402 Database Queries

use sqlx::PgPool;

pub struct X402Queries;

impl X402Queries {
    /// Create a new quote in the database
    pub async fn create_quote(
        pool: &PgPool,
        quote_id: &str,
        circuit_type: i16,
        payer: &str,
        price_lamports: i64,
        expires_at: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO x402_quotes (quote_id, circuit_type, payer, price_lamports, expires_at)
            VALUES ($1, $2, $3, $4, to_timestamp($5))
            "#,
        )
        .bind(quote_id)
        .bind(circuit_type)
        .bind(payer)
        .bind(price_lamports)
        .bind(expires_at)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Get quote by ID
    pub async fn get_quote(
        pool: &PgPool,
        quote_id: &str,
    ) -> Result<Option<(i16, i64, String, chrono::NaiveDateTime)>, sqlx::Error> {
        let result: Option<(i16, i64, String, chrono::NaiveDateTime)> = sqlx::query_as(
            "SELECT circuit_type, price_lamports, payer, expires_at FROM x402_quotes WHERE quote_id = $1",
        )
        .bind(quote_id)
        .fetch_optional(pool)
        .await?;

        Ok(result)
    }

    /// Mark quote as used
    pub async fn mark_quote_used(pool: &PgPool, quote_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE x402_quotes SET used = true WHERE quote_id = $1")
            .bind(quote_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Create a new payment token
    pub async fn create_token(
        pool: &PgPool,
        token_id: &str,
        quote_id: &str,
        payer: &str,
        circuit_type: i16,
        amount_paid: i64,
        tx_signature: &str,
        expires_at: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO x402_tokens (token_id, quote_id, payer, circuit_type, amount_paid, tx_signature, expires_at, used)
            VALUES ($1, $2, $3, $4, $5, $6, to_timestamp($7), false)
            "#,
        )
        .bind(token_id)
        .bind(quote_id)
        .bind(payer)
        .bind(circuit_type)
        .bind(amount_paid)
        .bind(tx_signature)
        .bind(expires_at)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Validate token and return its status
    /// Returns: (used, circuit_type, expires_at)
    pub async fn validate_token(
        pool: &PgPool,
        token_id: &str,
    ) -> Result<Option<(bool, i16, chrono::NaiveDateTime)>, sqlx::Error> {
        let result: Option<(bool, i16, chrono::NaiveDateTime)> = sqlx::query_as(
            "SELECT used, circuit_type, expires_at FROM x402_tokens WHERE token_id = $1",
        )
        .bind(token_id)
        .fetch_optional(pool)
        .await?;

        Ok(result)
    }

    /// Get token status (used, expires_at)
    pub async fn get_token_status(
        pool: &PgPool,
        token_id: &str,
    ) -> Result<Option<(bool, chrono::NaiveDateTime)>, sqlx::Error> {
        let result: Option<(bool, chrono::NaiveDateTime)> = sqlx::query_as(
            "SELECT used, expires_at FROM x402_tokens WHERE token_id = $1",
        )
        .bind(token_id)
        .fetch_optional(pool)
        .await?;

        Ok(result)
    }

    /// Mark token as used
    pub async fn mark_token_used(pool: &PgPool, token_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE x402_tokens SET used = true WHERE token_id = $1")
            .bind(token_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Store witness data
    pub async fn store_witness(
        pool: &PgPool,
        commitment: &str,
        data: &[u8],
        token_id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO witnesses (commitment, data, token_id) VALUES ($1, $2, $3) ON CONFLICT (commitment) DO NOTHING",
        )
        .bind(commitment)
        .bind(data)
        .bind(token_id)
        .execute(pool)
        .await?;

        Ok(())
    }
}
