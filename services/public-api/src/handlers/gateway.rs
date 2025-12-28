//! Gateway Prover Proxy Handlers
//!
//! Proxies prover gateway requests to x402-server.
//! Maintains the 3-layer architecture: nginx -> public-api -> x402 -> blink

use actix_web::{get, post, web, HttpRequest, HttpResponse};
use actix_web::web::Bytes;

use crate::AppState;

/// Proxy headers that should be forwarded to x402
const PROVER_HEADERS: &[&str] = &[
    "x-prover-pubkey",
    "x-prover-signature",
    "x-timestamp",
    "content-type",
];

/// Build forwarded headers from request
fn build_forward_headers(req: &HttpRequest) -> Vec<(String, String)> {
    let mut headers = Vec::new();
    for header_name in PROVER_HEADERS {
        if let Some(value) = req.headers().get(*header_name) {
            if let Ok(v) = value.to_str() {
                headers.push((header_name.to_string(), v.to_string()));
            }
        }
    }
    headers
}

/// GET /gateway/prover/witness/{hash}
/// Proxy witness download to x402
#[get("/gateway/prover/witness/{hash}")]
pub async fn get_witness(
    data: web::Data<AppState>,
    req: HttpRequest,
    hash: web::Path<String>,
) -> HttpResponse {
    let witness_hash = hash.into_inner();
    let url = format!("{}/gateway/prover/witness/{}",
        data.x402_client.base_url(), witness_hash);

    let headers = build_forward_headers(&req);

    let mut request_builder = data.x402_client.client()
        .get(&url);

    for (name, value) in headers {
        request_builder = request_builder.header(&name, &value);
    }

    match request_builder.send().await {
        Ok(response) => {
            let status = response.status();
            let body = response.bytes().await.unwrap_or_default();

            HttpResponse::build(actix_web::http::StatusCode::from_u16(status.as_u16()).unwrap())
                .content_type("application/octet-stream")
                .body(body)
        }
        Err(e) => {
            log::error!("Failed to proxy witness request: {}", e);
            HttpResponse::BadGateway().json(serde_json::json!({
                "error": "Failed to contact x402 server"
            }))
        }
    }
}

/// POST /gateway/prover/zk/{job_id}/submit
/// Proxy ZK proof submission to x402
#[post("/gateway/prover/zk/{job_id}/submit")]
pub async fn submit_zk_proof(
    data: web::Data<AppState>,
    req: HttpRequest,
    job_id: web::Path<u64>,
    body: Bytes,
) -> HttpResponse {
    let url = format!("{}/gateway/prover/zk/{}/submit",
        data.x402_client.base_url(), job_id.into_inner());

    let headers = build_forward_headers(&req);

    let mut request_builder = data.x402_client.client()
        .post(&url)
        .body(body.to_vec());

    for (name, value) in headers {
        request_builder = request_builder.header(&name, &value);
    }

    match request_builder.send().await {
        Ok(response) => {
            let status = response.status();
            let body = response.bytes().await.unwrap_or_default();

            HttpResponse::build(actix_web::http::StatusCode::from_u16(status.as_u16()).unwrap())
                .content_type("application/json")
                .body(body)
        }
        Err(e) => {
            log::error!("Failed to proxy ZK submit request: {}", e);
            HttpResponse::BadGateway().json(serde_json::json!({
                "error": "Failed to contact x402 server"
            }))
        }
    }
}

/// POST /gateway/prover/fhe/{job_id}/submit
/// Proxy FHE result submission to x402
#[post("/gateway/prover/fhe/{job_id}/submit")]
pub async fn submit_fhe_result(
    data: web::Data<AppState>,
    req: HttpRequest,
    job_id: web::Path<u64>,
    body: Bytes,
) -> HttpResponse {
    let url = format!("{}/gateway/prover/fhe/{}/submit",
        data.x402_client.base_url(), job_id.into_inner());

    let headers = build_forward_headers(&req);

    let mut request_builder = data.x402_client.client()
        .post(&url)
        .body(body.to_vec());

    for (name, value) in headers {
        request_builder = request_builder.header(&name, &value);
    }

    match request_builder.send().await {
        Ok(response) => {
            let status = response.status();
            let body = response.bytes().await.unwrap_or_default();

            HttpResponse::build(actix_web::http::StatusCode::from_u16(status.as_u16()).unwrap())
                .content_type("application/json")
                .body(body)
        }
        Err(e) => {
            log::error!("Failed to proxy FHE submit request: {}", e);
            HttpResponse::BadGateway().json(serde_json::json!({
                "error": "Failed to contact x402 server"
            }))
        }
    }
}
