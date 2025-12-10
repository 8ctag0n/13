//! Stats and metrics handlers - proxy to blink

use actix_web::{get, web, HttpResponse, Responder};
use serde_json::json;

use crate::AppState;

/// GET /api/stats/network - Network statistics (proxy to blink)
#[get("/api/stats/network")]
pub async fn get_network_stats(data: web::Data<AppState>) -> impl Responder {
    match data.blink_client.proxy_get::<serde_json::Value>("/internal/stats/network").await {
        Ok(stats) => HttpResponse::Ok().json(stats),
        Err(e) => {
            log::error!("Failed to get network stats: {}", e);
            HttpResponse::BadGateway().json(json!({"error": "Backend unavailable"}))
        }
    }
}

/// GET /api/metrics - Prometheus metrics (proxy to blink)
#[get("/api/metrics")]
pub async fn get_metrics(data: web::Data<AppState>) -> impl Responder {
    match data.blink_client.proxy_get::<serde_json::Value>("/internal/metrics").await {
        Ok(metrics) => HttpResponse::Ok().json(metrics),
        Err(e) => {
            log::error!("Failed to get metrics: {}", e);
            HttpResponse::BadGateway().json(json!({"error": "Backend unavailable"}))
        }
    }
}
