use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;

mod storage;
use storage::WitnessStorage;

#[derive(Clone)]
struct AppState {
    storage: Arc<RwLock<WitnessStorage>>,
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let storage = Arc::new(RwLock::new(WitnessStorage::new()));
    let state = AppState { storage };

    let app = Router::new()
        .route("/witness", post(upload_witness))
        .route("/witness/:commitment", get(download_witness))
        .route("/health", get(health_check))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3030")
        .await
        .unwrap();

    println!("Witness storage server listening on http://0.0.0.0:3030");

    axum::serve(listener, app).await.unwrap();
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
        hex::encode(&commitment)
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
        _ => {
            return (StatusCode::BAD_REQUEST, "Invalid commitment format")
                .into_response()
        }
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

async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let storage = state.storage.read().await;
    let (count, bytes) = storage.stats();

    Json(serde_json::json!({
        "status": "ok",
        "witnesses_stored": count,
        "total_bytes": bytes,
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
        let app = Router::new()
            .route("/witness", post(upload_witness))
            .route("/witness/:commitment", get(download_witness))
            .with_state(state);

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
        let app = Router::new()
            .route("/witness/:commitment", get(download_witness))
            .with_state(state);

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
        let app = Router::new()
            .route("/health", get(health_check))
            .with_state(state);

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
}
