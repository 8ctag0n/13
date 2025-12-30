//! Job handlers - coordinate x402 validation and blink execution

use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use serde_json::json;

use crate::AppState;

/// POST /api/jobs/zk/create - Create ZK job (requires payment token or signature)
#[post("/api/jobs/zk/create")]
pub async fn create_zk_job(
    data: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    // Check for X-Payment-Signature header (new flow)
    if let Some(signature) = req.headers().get("X-Payment-Signature") {
        return handle_payment_signature(data, signature, body).await;
    }

    // Check for X-Payment-Token header (legacy flow)
    let token = match req.headers().get("X-Payment-Token") {
        Some(t) => match t.to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return HttpResponse::BadRequest().json(json!({"error": "Invalid token"})),
        },
        None => return HttpResponse::PaymentRequired().json(json!({
            "error": "Payment required. Provide either X-Payment-Token or X-Payment-Signature header"
        })),
    };

    // Proxy to x402 gateway (which validates and forwards to blink)
    let client = reqwest::Client::new();
    let x402_url = format!("{}/gateway/zk/create",
        std::env::var("X402_URL").unwrap_or_else(|_| "http://localhost:8081".to_string()));

    match client.post(&x402_url)
        .header("X-Payment-Token", &token)
        .json(&body.into_inner())
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status();
            match resp.json::<serde_json::Value>().await {
                Ok(json) => HttpResponse::build(status).json(json),
                Err(_) => HttpResponse::BadGateway().json(json!({"error": "Invalid response from gateway"})),
            }
        }
        Err(e) => {
            log::error!("Gateway request failed: {}", e);
            HttpResponse::BadGateway().json(json!({"error": "Gateway unavailable"}))
        }
    }
}

/// Handle payment via Solana transaction signature
async fn handle_payment_signature(
    data: web::Data<AppState>,
    signature_header: &actix_web::http::header::HeaderValue,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    // Extract signature string
    let signature_str = match signature_header.to_str() {
        Ok(s) => s,
        Err(_) => return HttpResponse::BadRequest().json(json!({"error": "Invalid signature header"})),
    };

    // Extract circuit_type from body to get pricing
    let circuit_type = match body.get("circuit_type") {
        Some(serde_json::Value::Number(n)) => match n.as_u64() {
            Some(ct) if ct >= 10 && ct <= 49 => ct as u8,
            _ => return HttpResponse::BadRequest().json(json!({"error": "Invalid circuit_type"})),
        },
        _ => return HttpResponse::BadRequest().json(json!({"error": "circuit_type is required"})),
    };

    // Extract creator/payer from body
    let payer = match body.get("creator") {
        Some(serde_json::Value::String(s)) => s.clone(),
        _ => return HttpResponse::BadRequest().json(json!({"error": "creator is required"})),
    };

    // Get quote to determine expected payment amount and recipient
    let quote = match data.x402_client.get_quote(circuit_type, &payer).await {
        Ok(q) => q,
        Err(e) => {
            log::error!("Failed to get quote: {}", e);
            return HttpResponse::BadGateway().json(json!({"error": "Failed to get pricing"}));
        }
    };

    // Verify payment transaction on-chain
    match data.solana_verifier.verify_payment_transaction(
        signature_str,
        quote.price_lamports,
        &quote.payment_recipient,
    ).await {
        Ok(true) => {
            log::info!("Payment verified for signature: {}", signature_str);
        }
        Ok(false) => {
            return HttpResponse::PaymentRequired().json(json!({
                "error": "Payment verification failed"
            }));
        }
        Err(e) => {
            log::error!("Payment verification error: {}", e);
            return HttpResponse::PaymentRequired().json(json!({
                "error": format!("Payment verification failed: {}", e)
            }));
        }
    }

    // Payment verified - proxy directly to blink-server (skip x402 gateway)
    let blink_url = std::env::var("BLINK_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());
    let client = reqwest::Client::new();
    let url = format!("{}/internal/zk/validate-and-build", blink_url);

    match client.post(&url)
        .json(&body.into_inner())
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status();
            match resp.json::<serde_json::Value>().await {
                Ok(json) => HttpResponse::build(status).json(json),
                Err(_) => HttpResponse::BadGateway().json(json!({"error": "Invalid response from backend"})),
            }
        }
        Err(e) => {
            log::error!("Backend request failed: {}", e);
            HttpResponse::BadGateway().json(json!({"error": "Backend unavailable"}))
        }
    }
}

/// POST /api/jobs/fhe/create - Create FHE job (requires payment token)
#[post("/api/jobs/fhe/create")]
pub async fn create_fhe_job(
    data: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    // Get payment token
    let token = match req.headers().get("X-Payment-Token") {
        Some(t) => match t.to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return HttpResponse::BadRequest().json(json!({"error": "Invalid token"})),
        },
        None => return HttpResponse::PaymentRequired().json(json!({"error": "Payment token required"})),
    };

    // Proxy to x402 gateway
    let client = reqwest::Client::new();
    let x402_url = format!("{}/gateway/fhe/create",
        std::env::var("X402_URL").unwrap_or_else(|_| "http://localhost:8081".to_string()));

    match client.post(&x402_url)
        .header("X-Payment-Token", &token)
        .json(&body.into_inner())
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status();
            match resp.json::<serde_json::Value>().await {
                Ok(json) => HttpResponse::build(status).json(json),
                Err(_) => HttpResponse::BadGateway().json(json!({"error": "Invalid response"})),
            }
        }
        Err(e) => {
            log::error!("Gateway request failed: {}", e);
            HttpResponse::BadGateway().json(json!({"error": "Gateway unavailable"}))
        }
    }
}

/// GET /api/jobs/zk - List ZK jobs (proxy to blink)
#[get("/api/jobs/zk")]
pub async fn list_zk_jobs(data: web::Data<AppState>) -> impl Responder {
    match data.blink_client.proxy_get::<serde_json::Value>("/internal/zk").await {
        Ok(jobs) => HttpResponse::Ok().json(jobs),
        Err(e) => {
            log::error!("Failed to list ZK jobs: {}", e);
            HttpResponse::BadGateway().json(json!({"error": "Backend unavailable"}))
        }
    }
}

/// GET /api/jobs/fhe - List FHE jobs (proxy to blink)
#[get("/api/jobs/fhe")]
pub async fn list_fhe_jobs(data: web::Data<AppState>) -> impl Responder {
    match data.blink_client.proxy_get::<serde_json::Value>("/internal/fhe").await {
        Ok(jobs) => HttpResponse::Ok().json(jobs),
        Err(e) => {
            log::error!("Failed to list FHE jobs: {}", e);
            HttpResponse::BadGateway().json(json!({"error": "Backend unavailable"}))
        }
    }
}

/// GET /api/jobs/zk/{job_id} - Get ZK job status (proxy to blink)
#[get("/api/jobs/zk/{job_id}")]
pub async fn get_zk_job(
    data: web::Data<AppState>,
    job_id: web::Path<String>,
) -> impl Responder {
    let path = format!("/internal/zk/{}/status", job_id);
    match data.blink_client.proxy_get::<serde_json::Value>(&path).await {
        Ok(job) => HttpResponse::Ok().json(job),
        Err(e) => {
            log::error!("Failed to get ZK job: {}", e);
            HttpResponse::BadGateway().json(json!({"error": "Backend unavailable"}))
        }
    }
}

/// GET /api/jobs/fhe/{job_id} - Get FHE job details (proxy to blink)
#[get("/api/jobs/fhe/{job_id}")]
pub async fn get_fhe_job(
    data: web::Data<AppState>,
    job_id: web::Path<String>,
) -> impl Responder {
    let path = format!("/internal/fhe/{}", job_id);
    match data.blink_client.proxy_get::<serde_json::Value>(&path).await {
        Ok(job) => HttpResponse::Ok().json(job),
        Err(e) => {
            log::error!("Failed to get FHE job: {}", e);
            HttpResponse::BadGateway().json(json!({"error": "Backend unavailable"}))
        }
    }
}
