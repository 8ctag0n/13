// Integration tests for Futarchy E2E queries
// Run with: cargo test -p zyberlink-blink-server --test test_futarchy_queries
// Requires: DATABASE_URL env var pointing to a running PostgreSQL instance

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    // Test helper to get pool (requires DATABASE_URL)
    async fn get_test_pool() -> Result<PgPool, sqlx::Error> {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://zyberlink:dev_password@localhost:5432/zyberlink".to_string());

        PgPool::connect(&database_url).await
    }

    #[tokio::test]
    #[ignore] // Ignore by default, run explicitly with --ignored
    async fn test_save_ciphertext_with_tx() {
        let pool = get_test_pool().await.expect("Failed to connect to DB");

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        // Test data
        let ciphertext = b"fake_ciphertext_data_here";
        let hash = "a".repeat(64);
        let server_key_hash = "b".repeat(64);
        let user_pubkey = "test_user_pubkey";

        // Save ciphertext
        let result = crate::db::futarchy_queries::FutarchyQueries::save_ciphertext_with_tx(
            &pool,
            ciphertext,
            &hash,
            None, // no tx_sig yet
            &server_key_hash,
            user_pubkey,
        ).await;

        assert!(result.is_ok(), "Failed to save ciphertext: {:?}", result);

        // Fetch it back
        let fetched = crate::db::futarchy_queries::FutarchyQueries::get_ciphertext_by_hash(&pool, &hash).await;
        assert!(fetched.is_ok());
        assert!(fetched.unwrap().is_some());
    }

    #[tokio::test]
    #[ignore]
    async fn test_mark_confirmed() {
        let pool = get_test_pool().await.expect("Failed to connect to DB");

        // Setup: create a ciphertext first
        let ciphertext = b"test_data";
        let hash = "c".repeat(64);
        let tx_sig = "test_tx_signature_123";
        let server_key_hash = "d".repeat(64);

        crate::db::futarchy_queries::FutarchyQueries::save_ciphertext_with_tx(
            &pool,
            ciphertext,
            &hash,
            Some(tx_sig),
            &server_key_hash,
            "test_user",
        ).await.unwrap();

        // Test: mark as confirmed
        let result = crate::db::futarchy_queries::FutarchyQueries::mark_confirmed(&pool, tx_sig).await;
        assert!(result.is_ok(), "Failed to mark confirmed: {:?}", result);
    }

    #[tokio::test]
    #[ignore]
    async fn test_record_validation() {
        let pool = get_test_pool().await.expect("Failed to connect to DB");

        let ciphertext_hash = "e".repeat(64);
        let user_pubkey = "test_user_123";
        let server_key_hash = "f".repeat(64);

        let validation_id = crate::db::futarchy_queries::FutarchyQueries::record_validation(
            &pool,
            &ciphertext_hash,
            user_pubkey,
            &server_key_hash,
            true,  // is_valid
            None,  // no error
        ).await;

        assert!(validation_id.is_ok(), "Failed to record validation: {:?}", validation_id);
        assert!(validation_id.unwrap() > 0);
    }
}
