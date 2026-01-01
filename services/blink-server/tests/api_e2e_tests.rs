// E2E tests for blink-server API
// These tests require the server to be running
// Set TEST_SERVER_URL env var or defaults to http://127.0.0.1:3000

use serde_json::json;
use std::sync::LazyLock;

static BASE_URL: LazyLock<String> = LazyLock::new(|| {
    std::env::var("TEST_SERVER_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string())
});

#[tokio::test]
async fn test_health_endpoint() {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/health", *BASE_URL))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["service"], "zyberlink-blink-server");
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn test_estimate_cost_tier1_add() {
    let client = reqwest::Client::new();
    let payload = json!({
        "operation": "add",
        "operation_value": 5,
        "required_provers": 3
    });

    let response = client
        .post(format!("{}/api/estimate-cost", *BASE_URL))
        .json(&payload)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["complexity_tier"], 1);
    assert_eq!(body["min_payment_lamports"], 1_000_000);
    assert_eq!(body["total_min_payment_lamports"], 3_000_000);
    assert_eq!(body["timeout_seconds"], 60);
}

#[tokio::test]
async fn test_estimate_cost_tier1_multiply() {
    let client = reqwest::Client::new();
    let payload = json!({
        "operation": "multiply",
        "operation_value": 10,
        "required_provers": 5
    });

    let response = client
        .post(format!("{}/api/estimate-cost", *BASE_URL))
        .json(&payload)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["complexity_tier"], 1);
    assert_eq!(body["min_payment_lamports"], 1_000_000);
    assert_eq!(body["total_min_payment_lamports"], 5_000_000); // 1M × 5 provers
    assert_eq!(body["timeout_seconds"], 60);
}

#[tokio::test]
async fn test_estimate_cost_tier2_sum() {
    let client = reqwest::Client::new();
    let payload = json!({
        "operation": "sum",
        "expected_count": 1000,
        "required_provers": 3
    });

    let response = client
        .post(format!("{}/api/estimate-cost", *BASE_URL))
        .json(&payload)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["complexity_tier"], 2);

    // Tier 2: base 0.001 SOL + 1000 × 0.0001 SOL = 0.101 SOL = 101M lamports per prover
    let expected_per_prover = 101_000_000;
    assert_eq!(body["min_payment_lamports"], expected_per_prover);
    assert_eq!(body["total_min_payment_lamports"], expected_per_prover * 3);

    // Timeout: 60 + 1000 × 2 = 2060s
    assert_eq!(body["timeout_seconds"], 60 + 1000 * 2);
}

#[tokio::test]
async fn test_estimate_cost_tier3_threshold() {
    let client = reqwest::Client::new();
    let payload = json!({
        "operation": "threshold",
        "operation_value": 18,
        "required_provers": 3
    });

    let response = client
        .post(format!("{}/api/estimate-cost", *BASE_URL))
        .json(&payload)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["complexity_tier"], 3);
    assert_eq!(body["min_payment_lamports"], 5_000_000);
    assert_eq!(body["total_min_payment_lamports"], 15_000_000);
    assert_eq!(body["timeout_seconds"], 300);
}

#[tokio::test]
async fn test_estimate_cost_tier3_range_check() {
    let client = reqwest::Client::new();
    let payload = json!({
        "operation": "range_check",
        "operation_value": 100,
        "required_provers": 3
    });

    let response = client
        .post(format!("{}/api/estimate-cost", *BASE_URL))
        .json(&payload)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["complexity_tier"], 3);
    assert_eq!(body["min_payment_lamports"], 5_000_000);
    assert_eq!(body["total_min_payment_lamports"], 15_000_000);
}

#[tokio::test]
async fn test_estimate_cost_tier4_average() {
    let client = reqwest::Client::new();
    let payload = json!({
        "operation": "average",
        "expected_count": 500,
        "required_provers": 3
    });

    let response = client
        .post(format!("{}/api/estimate-cost", *BASE_URL))
        .json(&payload)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["complexity_tier"], 4);

    // Tier 4: base 0.001 SOL + 500 × 0.0002 SOL = 0.101 SOL = 101M lamports per prover
    let expected_per_prover = 101_000_000;
    assert_eq!(body["min_payment_lamports"], expected_per_prover);
    assert_eq!(body["total_min_payment_lamports"], expected_per_prover * 3);
}

#[tokio::test]
async fn test_estimate_cost_tier4_count_if() {
    let client = reqwest::Client::new();
    let payload = json!({
        "operation": "count_if",
        "operation_value": 50,
        "expected_count": 200,
        "required_provers": 3
    });

    let response = client
        .post(format!("{}/api/estimate-cost", *BASE_URL))
        .json(&payload)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["complexity_tier"], 4);
}

#[tokio::test]
async fn test_estimate_cost_tier5_histogram_5bins() {
    let client = reqwest::Client::new();
    let payload = json!({
        "operation": "histogram",
        "bins": 5,
        "required_provers": 3
    });

    let response = client
        .post(format!("{}/api/estimate-cost", *BASE_URL))
        .json(&payload)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["complexity_tier"], 5);

    // Tier 5: 0.1 SOL + 0.02 × bins^1.5
    // For 5 bins: 0.1 + 0.02 × 5^1.5 ≈ 0.324 SOL = 324M lamports per prover
    let min_payment = body["min_payment_lamports"].as_u64().unwrap();
    assert!(
        min_payment > 300_000_000,
        "Expected > 300M, got {}",
        min_payment
    );
    assert!(
        min_payment < 350_000_000,
        "Expected < 350M, got {}",
        min_payment
    );

    let total = body["total_min_payment_lamports"].as_u64().unwrap();
    assert_eq!(total, min_payment * 3);
}

#[tokio::test]
async fn test_estimate_cost_tier5_histogram_10bins() {
    let client = reqwest::Client::new();
    let payload = json!({
        "operation": "histogram",
        "bins": 10,
        "required_provers": 3
    });

    let response = client
        .post(format!("{}/api/estimate-cost", *BASE_URL))
        .json(&payload)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["complexity_tier"], 5);

    // For 10 bins: 0.1 + 0.02 × 10^1.5 ≈ 0.732 SOL = 732M lamports per prover
    let min_payment = body["min_payment_lamports"].as_u64().unwrap();
    assert!(
        min_payment > 700_000_000,
        "Expected > 700M, got {}",
        min_payment
    );
    assert!(
        min_payment < 800_000_000,
        "Expected < 800M, got {}",
        min_payment
    );

    let total = body["total_min_payment_lamports"].as_u64().unwrap();
    assert_eq!(total, min_payment * 3);

    // Timeout: 600 + 10 × 100 = 1600s
    assert_eq!(body["timeout_seconds"], 1600);
}

#[tokio::test]
async fn test_estimate_cost_histogram_without_operation_value() {
    // This test validates the bug fix - histogram should work without operation_value
    let client = reqwest::Client::new();
    let payload = json!({
        "operation": "histogram",
        "bins": 7,
        "required_provers": 3
    });

    let response = client
        .post(format!("{}/api/estimate-cost", *BASE_URL))
        .json(&payload)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        200,
        "Histogram should work without operation_value field"
    );

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["complexity_tier"], 5);
    assert_eq!(body["operation"], "Histogram");
}

#[tokio::test]
async fn test_estimate_cost_pricing_scales_with_provers() {
    let client = reqwest::Client::new();

    // Test with 3 provers
    let payload_3 = json!({
        "operation": "add",
        "operation_value": 5,
        "required_provers": 3
    });
    let response_3 = client
        .post(format!("{}/api/estimate-cost", *BASE_URL))
        .json(&payload_3)
        .send()
        .await
        .expect("Failed to send request");
    let body_3: serde_json::Value = response_3.json().await.unwrap();

    // Test with 7 provers
    let payload_7 = json!({
        "operation": "add",
        "operation_value": 5,
        "required_provers": 7
    });
    let response_7 = client
        .post(format!("{}/api/estimate-cost", *BASE_URL))
        .json(&payload_7)
        .send()
        .await
        .expect("Failed to send request");
    let body_7: serde_json::Value = response_7.json().await.unwrap();

    // Per-prover cost should be the same
    assert_eq!(
        body_3["min_payment_lamports"],
        body_7["min_payment_lamports"]
    );

    // Total cost should scale linearly
    let total_3 = body_3["total_min_payment_lamports"].as_u64().unwrap();
    let total_7 = body_7["total_min_payment_lamports"].as_u64().unwrap();
    assert_eq!(total_3, 3_000_000);
    assert_eq!(total_7, 7_000_000);
}

#[tokio::test]
async fn test_estimate_cost_unknown_operation() {
    let client = reqwest::Client::new();
    let payload = json!({
        "operation": "invalid_op",
        "operation_value": 5,
        "required_provers": 3
    });

    let response = client
        .post(format!("{}/api/estimate-cost", *BASE_URL))
        .json(&payload)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 400);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert!(body["error"]
        .as_str()
        .unwrap()
        .contains("Unknown operation"));
}

#[tokio::test]
async fn test_estimate_cost_all_operations() {
    // Test that all documented operations work
    let client = reqwest::Client::new();
    let operations = vec![
        ("add", Some(5), None, None),
        ("multiply", Some(10), None, None),
        ("sum", None, Some(100), None),
        ("threshold", Some(18), None, None),
        ("range_check", Some(100), None, None),
        ("average", None, Some(200), None),
        ("count_if", Some(50), Some(150), None),
        ("histogram", None, None, Some(8)),
    ];

    for (op, value, count, bins) in operations {
        let mut payload = json!({
            "operation": op,
            "required_provers": 3
        });

        if let Some(v) = value {
            payload["operation_value"] = json!(v);
        }
        if let Some(c) = count {
            payload["expected_count"] = json!(c);
        }
        if let Some(b) = bins {
            payload["bins"] = json!(b);
        }

        let response = client
            .post(format!("{}/api/estimate-cost", *BASE_URL))
            .json(&payload)
            .send()
            .await
            .unwrap_or_else(|_| panic!("Failed to send request for {}", op));

        assert_eq!(
            response.status(),
            200,
            "Operation '{}' failed with status {}",
            op,
            response.status()
        );

        let body: serde_json::Value = response
            .json()
            .await
            .unwrap_or_else(|_| panic!("Failed to parse JSON for {}", op));
        assert!(
            body["complexity_tier"].as_u64().unwrap() >= 1
                && body["complexity_tier"].as_u64().unwrap() <= 5,
            "Invalid tier for operation '{}'",
            op
        );
        assert!(
            body["min_payment_lamports"].as_u64().unwrap() > 0,
            "Zero payment for operation '{}'",
            op
        );
    }
}

// ============================================================================
// Network Stats Tests
// ============================================================================

#[tokio::test]
async fn test_network_stats_endpoint_returns_200() {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/stats/network", *BASE_URL))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_network_stats_has_required_fields() {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/stats/network", *BASE_URL))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");

    // Verify all required fields exist
    assert!(
        body.get("active_provers").is_some(),
        "Missing active_provers"
    );
    assert!(
        body.get("jobs_completed").is_some(),
        "Missing jobs_completed"
    );
    assert!(body.get("jobs_total").is_some(), "Missing jobs_total");
    assert!(
        body.get("data_encrypted_bytes").is_some(),
        "Missing data_encrypted_bytes"
    );
    assert!(
        body.get("data_encrypted_formatted").is_some(),
        "Missing data_encrypted_formatted"
    );
    assert!(
        body.get("uptime_seconds").is_some(),
        "Missing uptime_seconds"
    );
    assert!(
        body.get("uptime_percent").is_some(),
        "Missing uptime_percent"
    );
}

#[tokio::test]
async fn test_network_stats_values_are_valid() {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/stats/network", *BASE_URL))
        .send()
        .await
        .expect("Failed to send request");

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");

    // active_provers should be >= 0
    let active_provers = body["active_provers"].as_i64().unwrap();
    assert!(active_provers >= 0, "active_provers should be >= 0");

    // jobs_completed should be >= 0
    let jobs_completed = body["jobs_completed"].as_i64().unwrap();
    assert!(jobs_completed >= 0, "jobs_completed should be >= 0");

    // jobs_total should be >= jobs_completed
    let jobs_total = body["jobs_total"].as_i64().unwrap();
    assert!(
        jobs_total >= jobs_completed,
        "jobs_total should be >= jobs_completed"
    );

    // uptime_percent should be between 0 and 100
    let uptime_percent = body["uptime_percent"].as_f64().unwrap();
    assert!(
        (0.0..=100.0).contains(&uptime_percent),
        "uptime_percent should be 0-100"
    );

    // data_encrypted_bytes should match jobs_total * 1024
    let data_bytes = body["data_encrypted_bytes"].as_i64().unwrap();
    assert_eq!(
        data_bytes,
        jobs_total * 1024,
        "data_encrypted_bytes should be jobs_total * 1024"
    );
}

#[tokio::test]
async fn test_network_stats_data_formatted() {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/stats/network", *BASE_URL))
        .send()
        .await
        .expect("Failed to send request");

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");

    let data_bytes = body["data_encrypted_bytes"].as_i64().unwrap();
    let data_formatted = body["data_encrypted_formatted"].as_str().unwrap();

    // Verify formatted string is present and reasonable
    assert!(
        !data_formatted.is_empty(),
        "data_encrypted_formatted should not be empty"
    );

    // If bytes is 0, formatted should be "0 B"
    if data_bytes == 0 {
        assert_eq!(data_formatted, "0 B", "Zero bytes should format as '0 B'");
    }
}
