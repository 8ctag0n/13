//! Quote and pricing handlers - proxy to x402

use actix_web::{post, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct QuoteRequest {
    pub circuit_type: u8,
    pub payer: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuoteResponse {
    pub circuit_type: u8,
    pub price_lamports: u64,
    pub price_sol: f64,
    pub expires_at: i64,
    pub quote_id: String,
    pub payment_recipient: String,
}

/// POST /api/quote - Get price quote from x402
#[post("/api/quote")]
pub async fn get_quote(
    data: web::Data<AppState>,
    body: web::Json<QuoteRequest>,
) -> impl Responder {
    match data.x402_client.get_quote(body.circuit_type, &body.payer).await {
        Ok(quote) => HttpResponse::Ok().json(quote),
        Err(e) => {
            log::error!("Quote request failed: {}", e);
            HttpResponse::BadGateway().json(json!({
                "error": "Payment service unavailable"
            }))
        }
    }
}
