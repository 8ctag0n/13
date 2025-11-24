use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod storage;
pub use storage::WitnessStorage;

#[cfg(feature = "persistence")]
pub mod persistence;
#[cfg(feature = "persistence")]
pub use persistence::DiskPersistence;

#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<RwLock<WitnessStorage>>,
}

/// Create router with given state - exposed for testing
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/witness", post(upload_witness))
        .route("/witness/:commitment", get(download_witness))
        .route("/fhe-result", post(upload_fhe_result))
        .route("/fhe-result/:commitment", get(download_fhe_result))
        .route("/health", get(health_check))
        .with_state(state)
}

async fn upload_witness(
    State(state): State<AppState>,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let encrypted_witness = body.to_vec();

    // Validate witness size
    if encrypted_witness.is_empty() || encrypted_witness.len() > 10_000_000 {
        return (
            StatusCode::BAD_REQUEST,
            "Invalid witness size (max 10MB)".to_string(),
        )
            .into_response();
    }

    let commitment = {
        let mut storage = state.storage.write().await;
        storage.store(encrypted_witness.clone())
    };

    log::info!(
        "Stored witness: {} bytes, commitment: {}",
        encrypted_witness.len(),
        hex::encode(commitment)
    );

    Json(serde_json::json!({
        "commitment": hex::encode(commitment),
        "size": encrypted_witness.len(),
        "status": "stored"
    }))
    .into_response()
}

async fn download_witness(
    State(state): State<AppState>,
    Path(commitment_hex): Path<String>,
) -> impl IntoResponse {
    let commitment = match hex::decode(&commitment_hex) {
        Ok(bytes) if bytes.len() == 32 => {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            arr
        }
        _ => return (StatusCode::BAD_REQUEST, "Invalid commitment format").into_response(),
    };

    let storage = state.storage.read().await;

    match storage.retrieve(&commitment) {
        Some(witness) => {
            log::info!(
                "Retrieved witness: {} bytes, commitment: {}",
                witness.len(),
                commitment_hex
            );
            (StatusCode::OK, witness).into_response()
        }
        None => {
            log::warn!("Witness not found: {}", commitment_hex);
            (StatusCode::NOT_FOUND, "Witness not found").into_response()
        }
    }
}

async fn upload_fhe_result(
    State(state): State<AppState>,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let encrypted_result = body.to_vec();

    // Validate FHE result size
    if encrypted_result.is_empty() || encrypted_result.len() > 10_000_000 {
        return (
            StatusCode::BAD_REQUEST,
            "Invalid FHE result size (max 10MB)".to_string(),
        )
            .into_response();
    }

    let commitment = {
        let mut storage = state.storage.write().await;
        storage.store_fhe_result(encrypted_result.clone())
    };

    log::info!(
        "Stored FHE result: {} bytes, commitment: {}",
        encrypted_result.len(),
        hex::encode(commitment)
    );

    Json(serde_json::json!({
        "commitment": hex::encode(commitment),
        "size": encrypted_result.len(),
        "status": "stored"
    }))
    .into_response()
}

async fn download_fhe_result(
    State(state): State<AppState>,
    Path(commitment_hex): Path<String>,
) -> impl IntoResponse {
    let commitment = match hex::decode(&commitment_hex) {
        Ok(bytes) if bytes.len() == 32 => {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            arr
        }
        _ => return (StatusCode::BAD_REQUEST, "Invalid commitment format").into_response(),
    };

    let storage = state.storage.read().await;

    match storage.retrieve_fhe_result(&commitment) {
        Some(result) => {
            log::info!(
                "Retrieved FHE result: {} bytes, commitment: {}",
                result.len(),
                commitment_hex
            );
            (StatusCode::OK, result).into_response()
        }
        None => {
            log::warn!("FHE result not found: {}", commitment_hex);
            (StatusCode::NOT_FOUND, "FHE result not found").into_response()
        }
    }
}

async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let storage = state.storage.read().await;
    let (witness_count, witness_bytes, fhe_count, fhe_bytes) = storage.stats();

    Json(serde_json::json!({
        "status": "ok",
        "witnesses": {
            "count": witness_count,
            "bytes": witness_bytes,
        },
        "fhe_results": {
            "count": fhe_count,
            "bytes": fhe_bytes,
        },
        "total_bytes": witness_bytes + fhe_bytes,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_upload_download_roundtrip() {
        let storage = Arc::new(RwLock::new(WitnessStorage::new()));
        let state = AppState { storage };
        let app = create_router(state);

        // Upload witness
        let witness_data = b"test encrypted witness data";
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/witness")
                    .header("content-type", "application/octet-stream")
                    .body(Body::from(witness_data.to_vec()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let commitment = json["commitment"].as_str().unwrap();

        // Download witness
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/witness/{}", commitment))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(body.to_vec(), witness_data);
    }

    #[tokio::test]
    async fn test_download_nonexistent() {
        let storage = Arc::new(RwLock::new(WitnessStorage::new()));
        let state = AppState { storage };
        let app = create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/witness/{}", "0".repeat(64)))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_health_check() {
        let storage = Arc::new(RwLock::new(WitnessStorage::new()));
        let state = AppState { storage };
        let app = create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    async fn test_upload_download_fhe_result_roundtrip() {
        let storage = Arc::new(RwLock::new(WitnessStorage::new()));
        let state = AppState { storage };
        let app = create_router(state);

        // Upload FHE result
        let fhe_data = b"test encrypted FHE result";
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/fhe-result")
                    .header("content-type", "application/octet-stream")
                    .body(Body::from(fhe_data.to_vec()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let commitment = json["commitment"].as_str().unwrap();

        // Download FHE result
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/fhe-result/{}", commitment))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(body.to_vec(), fhe_data);
    }

    #[tokio::test]
    async fn test_download_nonexistent_fhe_result() {
        let storage = Arc::new(RwLock::new(WitnessStorage::new()));
        let state = AppState { storage };
        let app = create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/fhe-result/{}", "0".repeat(64)))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_health_check_with_fhe_results() {
        let storage = Arc::new(RwLock::new(WitnessStorage::new()));
        let state = AppState {
            storage: storage.clone(),
        };
        let app = create_router(state);

        // Upload a witness
        let witness_data = b"test witness";
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/witness")
                    .body(Body::from(witness_data.to_vec()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Upload an FHE result
        let fhe_data = b"test FHE result";
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/fhe-result")
                    .body(Body::from(fhe_data.to_vec()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Check health endpoint
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["status"], "ok");
        assert_eq!(json["witnesses"]["count"], 1);
        assert_eq!(json["fhe_results"]["count"], 1);
        assert!(json["total_bytes"].as_u64().unwrap() > 0);
    }
}
