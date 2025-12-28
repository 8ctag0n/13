//! Gateway handlers that validate payment tokens and proxy to blink-server

use actix_web::{post, web, HttpRequest, HttpResponse, Responder};
use serde_json::json;

use crate::db::X402Queries;
use crate::rate_limit::check_rate_limit;
use crate::AppState;

fn enforce_rate_limit(data: &web::Data<AppState>) -> Option<HttpResponse> {
    if !check_rate_limit(&data.rate_limiter) {
        return Some(HttpResponse::TooManyRequests().json(json!({
            "error": "Rate limit exceeded"
        })));
    }
    None
}

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

// =============================================================================
// Futarchy Gateway (anti-spam proxy)
// =============================================================================

#[post("/gateway/futarchy/markets/validate-and-build")]
pub async fn futarchy_market_validate(
    data: web::Data<AppState>,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    if let Some(resp) = enforce_rate_limit(&data) {
        return resp;
    }

    match data.blink_client.proxy_post::<_, serde_json::Value>(
        "/api/futarchy/markets/validate-and-build",
        &body.into_inner(),
        None,
    ).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            log::error!("Blink proxy error: {}", e);
            HttpResponse::BadGateway().json(json!({"error": "Backend service unavailable"}))
        }
    }
}

#[post("/gateway/futarchy/markets/{id}/bet/validate-and-build")]
pub async fn futarchy_bet_validate(
    data: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    if let Some(resp) = enforce_rate_limit(&data) {
        return resp;
    }

    let id = path.into_inner();
    let target = format!("/api/futarchy/markets/{}/bet/validate-and-build", id);
    match data.blink_client.proxy_post::<_, serde_json::Value>(
        &target,
        &body.into_inner(),
        None,
    ).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            log::error!("Blink proxy error: {}", e);
            HttpResponse::BadGateway().json(json!({"error": "Backend service unavailable"}))
        }
    }
}

#[post("/gateway/futarchy/markets/{id}/settle/validate-and-build")]
pub async fn futarchy_settle_validate(
    data: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    if let Some(resp) = enforce_rate_limit(&data) {
        return resp;
    }

    let id = path.into_inner();
    let target = format!("/api/futarchy/markets/{}/settle/validate-and-build", id);
    match data.blink_client.proxy_post::<_, serde_json::Value>(
        &target,
        &body.into_inner(),
        None,
    ).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            log::error!("Blink proxy error: {}", e);
            HttpResponse::BadGateway().json(json!({"error": "Backend service unavailable"}))
        }
    }
}

#[post("/gateway/futarchy/markets/{id}/claim/validate-and-build")]
pub async fn futarchy_claim_validate(
    data: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    if let Some(resp) = enforce_rate_limit(&data) {
        return resp;
    }

    let id = path.into_inner();
    let target = format!("/api/futarchy/markets/{}/claim/validate-and-build", id);
    match data.blink_client.proxy_post::<_, serde_json::Value>(
        &target,
        &body.into_inner(),
        None,
    ).await {
        Ok(response) => HttpResponse::Ok().json(response),
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
        .service(futarchy_market_validate)
        .service(futarchy_bet_validate)
        .service(futarchy_settle_validate)
        .service(futarchy_claim_validate)
        .service(gateway_health);
}
