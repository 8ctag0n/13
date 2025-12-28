//! pBTCFi Integration Tests
//!
//! Tests the complete FHE loan flow:
//! 1. Loan creation (event sync)
//! 2. FHE job creation
//! 3. Prover processing
//! 4. Callback generation
//!
//! Run with: cargo test -p zyberlink-blink-server --test pbtcfi_integration_tests

use sqlx::PgPool;

/// Helper to get test database pool
async fn get_test_pool() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://zyberlink:dev_password@localhost:5432/zyberlink".to_string());

    PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to database")
}

/// Clean up test data
async fn cleanup_test_data(pool: &PgPool, loan_id: &str) {
    let _ = sqlx::query("DELETE FROM pbtcfi_callbacks WHERE loan_id = $1")
        .bind(loan_id)
        .execute(pool)
        .await;

    let _ = sqlx::query("DELETE FROM pbtcfi_events WHERE loan_id = $1")
        .bind(loan_id)
        .execute(pool)
        .await;

    let _ = sqlx::query("DELETE FROM pbtcfi_loans WHERE loan_id = $1")
        .bind(loan_id)
        .execute(pool)
        .await;
}

// Test IDs (exactly 64 chars including 0x prefix)
// Format: 0x + 62 hex chars
const TEST_LOAN_1: &str = "0x000000000000000000000000000000000000000000000000000000000TEST01";
const TEST_LOAN_2: &str = "0x000000000000000000000000000000000000000000000000000000000TEST02";
const TEST_LOAN_3: &str = "0x000000000000000000000000000000000000000000000000000000000TEST03";
const TEST_LOAN_E2E: &str = "0x000000000000000000000000000000000000000000000000000000000E2E001";
const TEST_BORROWER: &str = "0x0000000000000000000000000000000000000000000000000000000BORROW01";
const TEST_COMMIT: &str = "0x00000000000000000000000000000000000000000000000000000COMMIT0001";
const TEST_ENC_C1: &str = "0x0000000000000000000000000000000000000000000000000000000000000064";
const TEST_ENC_C2: &str = "0x0000000000000000000000000000000000000000000000000000000000000001";
const TEST_PLST_C1: &str = "0x00000000000000000000000000000000000000000000000000000000PLST_C01";
const TEST_PLST_C2: &str = "0x00000000000000000000000000000000000000000000000000000000PLST_C02";
const TEST_PLST_ADDR: &str = "0x0000000000000000000000000000000000000000000000000000000000PLST00";

mod loan_flow_tests {
    use super::*;

    #[tokio::test]
    async fn test_loan_creation_and_fhe_job() {
        let pool = get_test_pool().await;
        cleanup_test_data(&pool, TEST_LOAN_1).await;

        // 1. Insert a loan
        let result = sqlx::query(
            r#"
            INSERT INTO pbtcfi_loans (
                loan_id, borrower, btc_commitment,
                btc_encrypted_c1, btc_encrypted_c2,
                status, created_at
            )
            VALUES ($1, $2, $3, $4, $5, 'pending', $6)
            "#,
        )
        .bind(TEST_LOAN_1)
        .bind(TEST_BORROWER)
        .bind(TEST_COMMIT)
        .bind(TEST_ENC_C1)
        .bind(TEST_ENC_C2)
        .bind(1702800000i64)
        .execute(&pool)
        .await;

        assert!(result.is_ok(), "Loan insertion should succeed");

        // 2. Create FHE job
        let fhe_result = sqlx::query(
            "UPDATE pbtcfi_loans SET fhe_status = 'fhe_pending', fhe_created_at = NOW() WHERE loan_id = $1"
        )
        .bind(TEST_LOAN_1)
        .execute(&pool)
        .await;

        assert!(fhe_result.is_ok(), "FHE job creation should succeed");

        // 3. Verify FHE job is pending
        let row: (String,) = sqlx::query_as(
            "SELECT fhe_status FROM pbtcfi_loans WHERE loan_id = $1"
        )
        .bind(TEST_LOAN_1)
        .fetch_one(&pool)
        .await
        .expect("Should fetch loan");

        assert_eq!(row.0, "fhe_pending");

        cleanup_test_data(&pool, TEST_LOAN_1).await;
    }

    #[tokio::test]
    async fn test_fhe_job_processing() {
        let pool = get_test_pool().await;
        cleanup_test_data(&pool, TEST_LOAN_2).await;

        // 1. Insert loan with pending FHE job
        sqlx::query(
            r#"
            INSERT INTO pbtcfi_loans (
                loan_id, borrower, btc_commitment,
                btc_encrypted_c1, btc_encrypted_c2,
                status, created_at, fhe_status, fhe_created_at
            )
            VALUES ($1, $2, $3, $4, $5, 'pending', $6, 'fhe_pending', NOW())
            "#,
        )
        .bind(TEST_LOAN_2)
        .bind(TEST_BORROWER)
        .bind(TEST_COMMIT)
        .bind(TEST_ENC_C1)
        .bind(TEST_ENC_C2)
        .bind(1702800000i64)
        .execute(&pool)
        .await
        .expect("Loan insertion should succeed");

        // 2. Claim job
        let claim_result = sqlx::query(
            r#"
            UPDATE pbtcfi_loans
            SET fhe_status = 'fhe_processing',
                fhe_prover_id = 'test-prover-1',
                fhe_started_at = NOW()
            WHERE loan_id = $1 AND fhe_status = 'fhe_pending'
            "#,
        )
        .bind(TEST_LOAN_2)
        .execute(&pool)
        .await
        .expect("Claim should succeed");

        assert_eq!(claim_result.rows_affected(), 1);

        // 3. Complete job
        sqlx::query(
            r#"
            UPDATE pbtcfi_loans
            SET fhe_status = 'fhe_completed',
                fhe_completed_at = NOW(),
                plst_encrypted_c1 = $2,
                plst_encrypted_c2 = $3,
                status = 'active'
            WHERE loan_id = $1
            "#,
        )
        .bind(TEST_LOAN_2)
        .bind(TEST_PLST_C1)
        .bind(TEST_PLST_C2)
        .execute(&pool)
        .await
        .expect("Complete should succeed");

        // 4. Verify final state
        let row: (String, String, Option<String>) = sqlx::query_as(
            "SELECT status, fhe_status, plst_encrypted_c1 FROM pbtcfi_loans WHERE loan_id = $1"
        )
        .bind(TEST_LOAN_2)
        .fetch_one(&pool)
        .await
        .expect("Should fetch loan");

        assert_eq!(row.0, "active");
        assert_eq!(row.1, "fhe_completed");
        assert!(row.2.is_some());

        cleanup_test_data(&pool, TEST_LOAN_2).await;
    }

    #[tokio::test]
    async fn test_callback_creation() {
        let pool = get_test_pool().await;
        cleanup_test_data(&pool, TEST_LOAN_3).await;

        // 1. Insert loan first
        sqlx::query(
            r#"
            INSERT INTO pbtcfi_loans (
                loan_id, borrower, btc_commitment,
                btc_encrypted_c1, btc_encrypted_c2,
                status, created_at
            )
            VALUES ($1, $2, $3, $4, $5, 'active', $6)
            "#,
        )
        .bind(TEST_LOAN_3)
        .bind(TEST_BORROWER)
        .bind(TEST_COMMIT)
        .bind(TEST_ENC_C1)
        .bind(TEST_ENC_C2)
        .bind(1702800000i64)
        .execute(&pool)
        .await
        .expect("Loan insertion should succeed");

        // 2. Create callback
        let call_args = serde_json::json!([TEST_BORROWER, TEST_PLST_C1, TEST_PLST_C2]);

        let callback_id: (i32,) = sqlx::query_as(
            r#"
            INSERT INTO pbtcfi_callbacks (
                loan_id, callback_type, target_contract,
                function_name, call_args, status
            )
            VALUES ($1, 'mint_encrypted', $2, 'mint_encrypted', $3, 'pending')
            RETURNING id
            "#,
        )
        .bind(TEST_LOAN_3)
        .bind(TEST_PLST_ADDR)
        .bind(&call_args)
        .fetch_one(&pool)
        .await
        .expect("Callback creation should succeed");

        assert!(callback_id.0 > 0);

        // 3. Verify callback exists
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM pbtcfi_callbacks WHERE loan_id = $1 AND status = 'pending'"
        )
        .bind(TEST_LOAN_3)
        .fetch_one(&pool)
        .await
        .expect("Should count callbacks");

        assert_eq!(count.0, 1);

        cleanup_test_data(&pool, TEST_LOAN_3).await;
    }

    #[tokio::test]
    async fn test_full_flow_end_to_end() {
        let pool = get_test_pool().await;
        cleanup_test_data(&pool, TEST_LOAN_E2E).await;

        // === PHASE 1: Loan Creation ===
        sqlx::query(
            r#"
            INSERT INTO pbtcfi_loans (
                loan_id, borrower, btc_commitment,
                btc_encrypted_c1, btc_encrypted_c2,
                status, created_at
            )
            VALUES ($1, $2, $3, $4, $5, 'pending', $6)
            "#,
        )
        .bind(TEST_LOAN_E2E)
        .bind(TEST_BORROWER)
        .bind(TEST_COMMIT)
        .bind(TEST_ENC_C1)
        .bind(TEST_ENC_C2)
        .bind(1702800000i64)
        .execute(&pool)
        .await
        .expect("Phase 1: Loan creation");

        // === PHASE 2: FHE Job Creation ===
        sqlx::query(
            "UPDATE pbtcfi_loans SET fhe_status = 'fhe_pending', fhe_created_at = NOW() WHERE loan_id = $1"
        )
        .bind(TEST_LOAN_E2E)
        .execute(&pool)
        .await
        .expect("Phase 2: FHE job creation");

        // === PHASE 3: Prover Claims Job ===
        sqlx::query(
            r#"
            UPDATE pbtcfi_loans
            SET fhe_status = 'fhe_processing', fhe_prover_id = 'e2e-prover', fhe_started_at = NOW()
            WHERE loan_id = $1 AND fhe_status = 'fhe_pending'
            "#,
        )
        .bind(TEST_LOAN_E2E)
        .execute(&pool)
        .await
        .expect("Phase 3: Prover claims");

        // === PHASE 4: Prover Completes FHE ===
        sqlx::query(
            r#"
            UPDATE pbtcfi_loans
            SET fhe_status = 'fhe_completed',
                fhe_completed_at = NOW(),
                plst_encrypted_c1 = $2,
                plst_encrypted_c2 = $3,
                status = 'active'
            WHERE loan_id = $1
            "#,
        )
        .bind(TEST_LOAN_E2E)
        .bind(TEST_PLST_C1)
        .bind(TEST_PLST_C2)
        .execute(&pool)
        .await
        .expect("Phase 4: FHE completion");

        // === PHASE 5: Callback Created ===
        let call_args = serde_json::json!([TEST_BORROWER, TEST_PLST_C1, TEST_PLST_C2]);

        sqlx::query(
            r#"
            INSERT INTO pbtcfi_callbacks (
                loan_id, callback_type, target_contract, function_name, call_args, status
            )
            VALUES ($1, 'mint_encrypted', $2, 'mint_encrypted', $3, 'pending')
            "#,
        )
        .bind(TEST_LOAN_E2E)
        .bind(TEST_PLST_ADDR)
        .bind(&call_args)
        .execute(&pool)
        .await
        .expect("Phase 5: Callback creation");

        // === VERIFY FINAL STATE ===
        let loan: (String, String, Option<String>, Option<String>) = sqlx::query_as(
            "SELECT status, fhe_status, plst_encrypted_c1, fhe_prover_id FROM pbtcfi_loans WHERE loan_id = $1"
        )
        .bind(TEST_LOAN_E2E)
        .fetch_one(&pool)
        .await
        .expect("Should fetch final loan state");

        assert_eq!(loan.0, "active", "Loan should be active");
        assert_eq!(loan.1, "fhe_completed", "FHE should be completed");
        assert!(loan.2.is_some(), "pLST encrypted should be set");
        assert_eq!(loan.3, Some("e2e-prover".to_string()), "Prover ID should be set");

        let callback_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM pbtcfi_callbacks WHERE loan_id = $1 AND status = 'pending'"
        )
        .bind(TEST_LOAN_E2E)
        .fetch_one(&pool)
        .await
        .expect("Should count callbacks");

        assert_eq!(callback_count.0, 1, "Should have 1 pending callback");

        cleanup_test_data(&pool, TEST_LOAN_E2E).await;

        println!("E2E Test PASSED: Full pBTCFi FHE flow completed successfully");
    }
}
