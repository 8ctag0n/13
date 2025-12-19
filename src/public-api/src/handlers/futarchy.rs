//! Futarchy handlers - proxy to x402 and blink

use actix_web::{get, post, web, HttpResponse, Responder};
use serde_json::json;

use crate::AppState;

fn x402_base_url() -> String {
    std::env::var("X402_URL").unwrap_or_else(|_| "http://localhost:8081".to_string())
}

async fn proxy_x402_post(path: &str, body: serde_json::Value) -> HttpResponse {
    let client = reqwest::Client::new();
    let url = format!("{}{}", x402_base_url(), path);

    match client.post(&url).json(&body).send().await {
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

async fn proxy_blink_get(data: web::Data<AppState>, path: &str) -> HttpResponse {
    match data.blink_client.proxy_get::<serde_json::Value>(path).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            log::error!("Blink proxy failed: {}", e);
            HttpResponse::BadGateway().json(json!({"error": "Backend unavailable"}))
        }
    }
}

async fn proxy_blink_post(data: web::Data<AppState>, path: &str, body: serde_json::Value) -> HttpResponse {
    match data.blink_client.proxy_post::<_, serde_json::Value>(path, &body).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            log::error!("Blink proxy failed: {}", e);
            HttpResponse::BadGateway().json(json!({"error": "Backend unavailable"}))
        }
    }
}

// =============================================================================
// Futarchy GET routes (proxy to blink)
// =============================================================================

#[get("/api/futarchy/health")]
pub async fn futarchy_health(data: web::Data<AppState>) -> impl Responder {
    proxy_blink_get(data, "/api/futarchy/health").await
}

#[get("/api/futarchy/markets")]
pub async fn list_markets(data: web::Data<AppState>) -> impl Responder {
    proxy_blink_get(data, "/api/futarchy/markets").await
}

#[get("/api/futarchy/markets/{id}")]
pub async fn get_market(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let id = path.into_inner();
    proxy_blink_get(data, &format!("/api/futarchy/markets/{}", id)).await
}

#[get("/api/futarchy/markets/{id}/positions")]
pub async fn get_market_positions(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let id = path.into_inner();
    proxy_blink_get(data, &format!("/api/futarchy/markets/{}/positions", id)).await
}

#[get("/api/futarchy/positions/{bettor}")]
pub async fn get_bettor_positions(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let bettor = path.into_inner();
    proxy_blink_get(data, &format!("/api/futarchy/positions/{}", bettor)).await
}

#[get("/api/futarchy/fhe-jobs/pending")]
pub async fn get_pending_fhe_jobs(data: web::Data<AppState>) -> impl Responder {
    proxy_blink_get(data, "/api/futarchy/fhe-jobs/pending").await
}

// =============================================================================
// Futarchy POST routes (legacy proxy to blink)
// =============================================================================

#[post("/api/futarchy/markets")]
pub async fn create_market(
    data: web::Data<AppState>,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    proxy_blink_post(data, "/api/futarchy/markets", body.into_inner()).await
}

#[post("/api/futarchy/markets/{id}/bet")]
pub async fn place_bet(
    data: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    let id = path.into_inner();
    proxy_blink_post(data, &format!("/api/futarchy/markets/{}/bet", id), body.into_inner()).await
}

#[post("/api/futarchy/markets/{id}/settle")]
pub async fn settle_market(
    data: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    let id = path.into_inner();
    proxy_blink_post(data, &format!("/api/futarchy/markets/{}/settle", id), body.into_inner()).await
}

#[post("/api/futarchy/bet/prepare")]
pub async fn prepare_bet(
    data: web::Data<AppState>,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    proxy_blink_post(data, "/api/futarchy/bet/prepare", body.into_inner()).await
}

#[post("/api/futarchy/bet/submit")]
pub async fn submit_bet(
    data: web::Data<AppState>,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    proxy_blink_post(data, "/api/futarchy/bet/submit", body.into_inner()).await
}

#[post("/api/futarchy/fhe-jobs/{id}/complete")]
pub async fn complete_fhe_job(
    data: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    let id = path.into_inner();
    proxy_blink_post(data, &format!("/api/futarchy/fhe-jobs/{}/complete", id), body.into_inner()).await
}

#[post("/api/futarchy/fhe-jobs/{id}/fail")]
pub async fn fail_fhe_job(
    data: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    let id = path.into_inner();
    proxy_blink_post(data, &format!("/api/futarchy/fhe-jobs/{}/fail", id), body.into_inner()).await
}

// =============================================================================
// Futarchy validate-and-build routes (proxy to x402)
// =============================================================================

#[post("/api/futarchy/markets/validate-and-build")]
pub async fn validate_market(
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    proxy_x402_post("/gateway/futarchy/markets/validate-and-build", body.into_inner()).await
}

#[post("/api/futarchy/markets/{id}/bet/validate-and-build")]
pub async fn validate_bet(
    path: web::Path<String>,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    let id = path.into_inner();
    proxy_x402_post(
        &format!("/gateway/futarchy/markets/{}/bet/validate-and-build", id),
        body.into_inner(),
    ).await
}

#[post("/api/futarchy/markets/{id}/settle/validate-and-build")]
pub async fn validate_settle(
    path: web::Path<String>,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    let id = path.into_inner();
    proxy_x402_post(
        &format!("/gateway/futarchy/markets/{}/settle/validate-and-build", id),
        body.into_inner(),
    ).await
}

#[post("/api/futarchy/markets/{id}/claim/validate-and-build")]
pub async fn validate_claim(
    path: web::Path<String>,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    let id = path.into_inner();
    proxy_x402_post(
        &format!("/gateway/futarchy/markets/{}/claim/validate-and-build", id),
        body.into_inner(),
    ).await
}
