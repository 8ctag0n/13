//! Gateway handlers that validate payment tokens and proxy to blink-server

use actix_web::{post, web, HttpRequest, HttpResponse, Responder};
use serde_json::json;

use crate::db::X402Queries;
use crate::AppState;

/// Validate token and proxy to blink for ZK job creation
#[post("/gateway/zk/create")]
pub async fn gateway_zk_create(
    data: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    // Extract token from header
    let token_id = match req.headers().get("X-Payment-Token") {
        Some(t) => match t.to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return HttpResponse::BadRequest().json(json!({"error": "Invalid token header"})),
        },
        None => return HttpResponse::PaymentRequired().json(json!({"error": "X-Payment-Token header required"})),
    };

    // Validate token
    let validation = match X402Queries::validate_token(&data.db_pool, &token_id).await {
        Ok(Some((used, circuit_type, expires_at))) => {
            if used {
                return HttpResponse::PaymentRequired().json(json!({"error": "Token already used"}));
            }
            if expires_at < chrono::Utc::now().naive_utc() {
                return HttpResponse::PaymentRequired().json(json!({"error": "Token expired"}));
            }
            (circuit_type, token_id.clone())
        }
        Ok(None) => return HttpResponse::PaymentRequired().json(json!({"error": "Invalid token"})),
        Err(e) => {
            log::error!("Token validation error: {}", e);
            return HttpResponse::InternalServerError().json(json!({"error": "Database error"}));
        }
    };

    // Proxy to blink-server
    match data.blink_client.proxy_post::<_, serde_json::Value>(
        "/internal/zk/validate-and-build",
        &body.into_inner(),
        Some(&validation.1),
    ).await {
        Ok(response) => {
            // Mark token as used on success
            let _ = X402Queries::mark_token_used(&data.db_pool, &validation.1).await;
            HttpResponse::Ok().json(response)
        }
        Err(e) => {
            log::error!("Blink proxy error: {}", e);
            HttpResponse::BadGateway().json(json!({"error": "Backend service unavailable"}))
        }
    }
}

/// Validate token and proxy to blink for FHE job creation
#[post("/gateway/fhe/create")]
pub async fn gateway_fhe_create(
    data: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    // Extract token from header
    let token_id = match req.headers().get("X-Payment-Token") {
        Some(t) => match t.to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return HttpResponse::BadRequest().json(json!({"error": "Invalid token header"})),
        },
        None => return HttpResponse::PaymentRequired().json(json!({"error": "X-Payment-Token header required"})),
    };

    // Validate token
    let validation = match X402Queries::validate_token(&data.db_pool, &token_id).await {
        Ok(Some((used, _circuit_type, expires_at))) => {
            if used {
                return HttpResponse::PaymentRequired().json(json!({"error": "Token already used"}));
            }
            if expires_at < chrono::Utc::now().naive_utc() {
                return HttpResponse::PaymentRequired().json(json!({"error": "Token expired"}));
            }
            token_id.clone()
        }
        Ok(None) => return HttpResponse::PaymentRequired().json(json!({"error": "Invalid token"})),
        Err(e) => {
            log::error!("Token validation error: {}", e);
            return HttpResponse::InternalServerError().json(json!({"error": "Database error"}));
        }
    };

    // Proxy to blink-server
    match data.blink_client.proxy_post::<_, serde_json::Value>(
        "/internal/fhe/validate-and-build",
        &body.into_inner(),
        Some(&validation),
    ).await {
        Ok(response) => {
            // Mark token as used on success
            let _ = X402Queries::mark_token_used(&data.db_pool, &validation).await;
            HttpResponse::Ok().json(response)
        }
        Err(e) => {
            log::error!("Blink proxy error: {}", e);
            HttpResponse::BadGateway().json(json!({"error": "Backend service unavailable"}))
        }
    }
}

/// Gateway health check - verifies blink connectivity
#[actix_web::get("/gateway/health")]
pub async fn gateway_health(data: web::Data<AppState>) -> impl Responder {
    let blink_ok = data.blink_client.health_check().await.unwrap_or(false);

    let status = if blink_ok { "ok" } else { "degraded" };
    let body = json!({
        "status": status,
        "service": "x402-gateway",
        "checks": {
            "blink_server": blink_ok
        }
    });

    if blink_ok {
        HttpResponse::Ok().json(body)
    } else {
        HttpResponse::ServiceUnavailable().json(body)
    }
}

pub fn configure_gateway_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(gateway_zk_create)
        .service(gateway_fhe_create)
        .service(gateway_health);
}
