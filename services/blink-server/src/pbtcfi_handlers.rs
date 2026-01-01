//! pBTCFi API Handlers
//!
//! REST API endpoints for querying pBTCFi loan data

use actix_web::{get, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

use crate::db::PbtcfiQueries;

// =============================================================================
// Query Parameters
// =============================================================================

#[derive(Deserialize)]
pub struct ListLoansQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// =============================================================================
// API Handlers
// =============================================================================

/// GET /internal/pbtcfi/health
/// Health check for pBTCFi sync status
#[get("/internal/pbtcfi/health")]
pub async fn pbtcfi_health(pool: web::Data<PgPool>) -> impl Responder {
    match PbtcfiQueries::get_last_synced_block(&pool).await {
        Ok(last_block) => {
            HttpResponse::Ok().json(json!({
                "status": "ok",
                "service": "pbtcfi-sync",
                "last_synced_block": last_block,
            }))
        }
        Err(e) => {
            log::error!("Failed to get pBTCFi sync status: {}", e);
            HttpResponse::ServiceUnavailable().json(json!({
                "status": "degraded",
                "error": format!("Failed to query sync state: {}", e),
            }))
        }
    }
}

/// GET /internal/pbtcfi/loans
/// List loans with optional filtering
#[get("/internal/pbtcfi/loans")]
pub async fn list_loans(
    pool: web::Data<PgPool>,
    query: web::Query<ListLoansQuery>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(50).min(500); // Max 500
    let offset = query.offset.unwrap_or(0);

    match PbtcfiQueries::list_loans(
        &pool,
        query.status.as_deref(),
        limit,
        offset,
    ).await {
        Ok(loans) => {
            HttpResponse::Ok().json(json!({
                "loans": loans,
                "count": loans.len(),
                "limit": limit,
                "offset": offset,
            }))
        }
        Err(e) => {
            log::error!("Failed to list loans: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to list loans: {}", e),
            }))
        }
    }
}

/// GET /internal/pbtcfi/loans/{loan_id}
/// Get a specific loan by ID
#[get("/internal/pbtcfi/loans/{loan_id}")]
pub async fn get_loan(
    pool: web::Data<PgPool>,
    path: web::Path<String>,
) -> impl Responder {
    let loan_id = path.into_inner();

    match PbtcfiQueries::get_loan(&pool, &loan_id).await {
        Ok(Some(loan)) => {
            HttpResponse::Ok().json(json!({
                "loan": loan,
            }))
        }
        Ok(None) => {
            HttpResponse::NotFound().json(json!({
                "error": "Loan not found",
                "loan_id": loan_id,
            }))
        }
        Err(e) => {
            log::error!("Failed to get loan {}: {}", loan_id, e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to fetch loan: {}", e),
            }))
        }
    }
}

/// GET /internal/pbtcfi/loans/borrower/{address}
/// Get loans by borrower address
#[get("/internal/pbtcfi/loans/borrower/{address}")]
pub async fn get_borrower_loans(
    pool: web::Data<PgPool>,
    path: web::Path<String>,
) -> impl Responder {
    let borrower = path.into_inner();
    let limit = 100; // Default limit for borrower queries

    match PbtcfiQueries::get_borrower_loans(&pool, &borrower, limit).await {
        Ok(loans) => {
            HttpResponse::Ok().json(json!({
                "borrower": borrower,
                "loans": loans,
                "count": loans.len(),
            }))
        }
        Err(e) => {
            log::error!("Failed to get loans for borrower {}: {}", borrower, e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to fetch borrower loans: {}", e),
            }))
        }
    }
}

/// GET /internal/pbtcfi/events
/// Get recent events (for debugging)
#[get("/internal/pbtcfi/events")]
pub async fn list_events(
    pool: web::Data<PgPool>,
    query: web::Query<ListEventsQuery>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(50).min(500);

    match PbtcfiQueries::get_recent_events(&pool, limit).await {
        Ok(events) => {
            HttpResponse::Ok().json(json!({
                "events": events,
                "count": events.len(),
            }))
        }
        Err(e) => {
            log::error!("Failed to list events: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to list events: {}", e),
            }))
        }
    }
}

#[derive(Deserialize)]
pub struct ListEventsQuery {
    pub limit: Option<i64>,
}

// =============================================================================
// Route Configuration
// =============================================================================

/// Configure pBTCFi routes
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg
        .service(pbtcfi_health)
        .service(list_loans)
        .service(get_loan)
        .service(get_borrower_loans)
        .service(list_events);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_limit_capping() {
        let query = ListLoansQuery {
            status: None,
            limit: Some(1000),
            offset: None,
        };

        let capped_limit = query.limit.unwrap_or(50).min(500);
        assert_eq!(capped_limit, 500);
    }
}
