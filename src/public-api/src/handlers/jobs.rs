//! Job handlers - coordinate x402 validation and blink execution

use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use serde_json::json;

use crate::AppState;

/// POST /api/jobs/zk/create - Create ZK job (requires payment token)
#[post("/api/jobs/zk/create")]
pub async fn create_zk_job(
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
