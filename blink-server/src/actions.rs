use actix_web::{get, post, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use crate::tx_builder;

// ============================================================================
// Solana Actions API Data Structures
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ActionGetResponse {
    #[serde(rename = "type")]
    pub action_type: String,
    pub icon: String,
    pub title: String,
    pub description: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<ActionLinks>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActionLinks {
    pub actions: Vec<LinkedAction>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LinkedAction {
    pub label: String,
    pub href: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Vec<ActionParameter>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActionParameter {
    pub name: String,
    pub label: String,
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActionPostRequest {
    pub account: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActionPostResponse {
    pub transaction: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

// ============================================================================
// Query Parameters
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct FundProverQuery {
    #[serde(default)]
    pub amount: Option<String>,
    #[serde(default)]
    pub pubkey: Option<String>,
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /api/actions/fund-prover
/// Returns metadata about the fund prover action
#[get("/api/actions/fund-prover")]
pub async fn get_fund_prover_action(query: web::Query<FundProverQuery>) -> impl Responder {
    log::info!("GET /api/actions/fund-prover - query: {:?}", query);

    // Build action links
    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

    let mut actions = vec![
        LinkedAction {
            label: "Stake 0.1 SOL".to_string(),
            href: format!(
                "{}/api/actions/fund-prover?amount=0.1&pubkey={{pubkey}}",
                base_url
            ),
            parameters: Some(vec![ActionParameter {
                name: "pubkey".to_string(),
                label: "Prover Public Key".to_string(),
                param_type: Some("text".to_string()),
                required: Some(true),
            }]),
        },
        LinkedAction {
            label: "Stake 0.5 SOL".to_string(),
            href: format!(
                "{}/api/actions/fund-prover?amount=0.5&pubkey={{pubkey}}",
                base_url
            ),
            parameters: Some(vec![ActionParameter {
                name: "pubkey".to_string(),
                label: "Prover Public Key".to_string(),
                param_type: Some("text".to_string()),
                required: Some(true),
            }]),
        },
        LinkedAction {
            label: "Stake 1 SOL".to_string(),
            href: format!(
                "{}/api/actions/fund-prover?amount=1&pubkey={{pubkey}}",
                base_url
            ),
            parameters: Some(vec![ActionParameter {
                name: "pubkey".to_string(),
                label: "Prover Public Key".to_string(),
                param_type: Some("text".to_string()),
                required: Some(true),
            }]),
        },
    ];

    // Add custom amount option
    actions.push(LinkedAction {
        label: "Custom Amount".to_string(),
        href: format!(
            "{}/api/actions/fund-prover?amount={{amount}}&pubkey={{pubkey}}",
            base_url
        ),
        parameters: Some(vec![
            ActionParameter {
                name: "amount".to_string(),
                label: "SOL Amount".to_string(),
                param_type: Some("number".to_string()),
                required: Some(true),
            },
            ActionParameter {
                name: "pubkey".to_string(),
                label: "Prover Public Key".to_string(),
                param_type: Some("text".to_string()),
                required: Some(true),
            },
        ]),
    });

    let response = ActionGetResponse {
        action_type: "action".to_string(),
        icon: format!("{}/static/zyberlink-icon.svg", base_url),
        title: "Fund ZyberLink Prover".to_string(),
        description: "Stake SOL to register and fund your ZyberLink prover node".to_string(),
        label: "Fund Prover".to_string(),
        disabled: None,
        error: None,
        links: Some(ActionLinks { actions }),
    };

    HttpResponse::Ok().json(response)
}

/// POST /api/actions/fund-prover
/// Returns a transaction to fund the prover
#[post("/api/actions/fund-prover")]
pub async fn post_fund_prover_action(
    query: web::Query<FundProverQuery>,
    body: web::Json<ActionPostRequest>,
) -> impl Responder {
    log::info!(
        "POST /api/actions/fund-prover - query: {:?}, account: {}",
        query,
        body.account
    );

    // Validate inputs
    let amount_str = match &query.amount {
        Some(amt) => amt,
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Missing 'amount' parameter"
            }));
        }
    };

    let recipient_str = match &query.pubkey {
        Some(pk) => pk,
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Missing 'pubkey' parameter"
            }));
        }
    };

    // Parse amount (SOL to lamports)
    let amount_sol: f64 = match amount_str.parse() {
        Ok(amt) => amt,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid amount format"
            }));
        }
    };

    let lamports = (amount_sol * 1_000_000_000.0) as u64;

    // Parse public keys
    let sender_pubkey = match Pubkey::from_str(&body.account) {
        Ok(pk) => pk,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid sender public key"
            }));
        }
    };

    let recipient_pubkey = match Pubkey::from_str(recipient_str) {
        Ok(pk) => pk,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid recipient public key"
            }));
        }
    };

    // Build transaction
    let transaction =
        match tx_builder::build_transfer_transaction(&sender_pubkey, &recipient_pubkey, lamports) {
            Ok(tx) => tx,
            Err(e) => {
                log::error!("Failed to build transaction: {}", e);
                return HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "Failed to build transaction"
                }));
            }
        };

    // Serialize transaction
    let encoded_tx = match tx_builder::serialize_transaction(&transaction) {
        Ok(tx) => tx,
        Err(e) => {
            log::error!("Failed to serialize transaction: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to serialize transaction"
            }));
        }
    };

    let response = ActionPostResponse {
        transaction: encoded_tx,
        message: Some(format!(
            "Transfer {} SOL to prover {}",
            amount_sol, recipient_str
        )),
    };

    HttpResponse::Ok().json(response)
}
