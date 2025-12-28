//! x402 Anti-Spam Payment Layer Server
//!
//! Independent service for handling payment tokens and anti-spam protection.
//! Runs on port 8081 by default.

mod db;
mod handlers;
mod models;
mod blink_client;
mod rate_limit;
mod gateway;
mod prover_auth;
mod prover_gateway;

use actix_cors::Cors;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use std::env;

/// Application state shared across handlers
pub struct AppState {
    pub db_pool: PgPool,
    pub blink_client: blink_client::BlinkClient,
    pub rate_limiter: rate_limit::GlobalRateLimiter,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Initialize logging
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    log::info!("===========================================");
    log::info!("  x402 Anti-Spam Payment Layer Server");
    log::info!("===========================================");
    log::info!("");

    // Load configuration from environment
    let host = env::var("X402_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = env::var("X402_PORT")
        .unwrap_or_else(|_| "8081".to_string())
        .parse()
        .expect("X402_PORT must be a valid port number");

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let blink_url = env::var("BLINK_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

    // Create database connection pool
    let pool = db::create_pool(&database_url)
        .await
        .expect("Failed to create database pool");

    // Initialize blink client and rate limiter
    let blink_client = blink_client::BlinkClient::new(&blink_url);
    let rate_limiter = rate_limit::create_rate_limiter(10); // 10 req/min for unauthenticated

    let app_state = web::Data::new(AppState {
        db_pool: pool,
        blink_client,
        rate_limiter,
    });

    log::info!("Configuration:");
    log::info!("  Host: {}", host);
    log::info!("  Port: {}", port);
    log::info!("  Blink URL: {}", blink_url);
    log::info!("");
    log::info!("Endpoints:");
    log::info!("  GET    /health                    (health check)");
    log::info!("");
    log::info!("  Quote & Pricing:");
    log::info!("  POST   /api/quote                 (get price quote)");
    log::info!("  POST   /api/estimate              (estimate price)");
    log::info!("");
    log::info!("  Payment:");
    log::info!("  POST   /api/build-payment         (build payment instruction)");
    log::info!("  POST   /api/confirm               (confirm payment, get token)");
    log::info!("");
    log::info!("  Token Management:");
    log::info!("  POST   /api/validate              (validate token - internal)");
    log::info!("  POST   /api/mark-used             (mark token used - internal)");
    log::info!("  GET    /api/token/{{id}}/status    (get token status)");
    log::info!("");
    log::info!("  Protected Operations:");
    log::info!("  POST   /api/witness               (upload witness with token)");
    log::info!("  POST   /api/create-job            (create job with token)");
    log::info!("");
    log::info!("  Gateway (token-validated proxy):");
    log::info!("  POST   /gateway/zk/create          (validate + proxy ZK job)");
    log::info!("  POST   /gateway/fhe/create         (validate + proxy FHE job)");
    log::info!("  POST   /gateway/futarchy/markets/validate-and-build");
    log::info!("  POST   /gateway/futarchy/markets/{id}/bet/validate-and-build");
    log::info!("  POST   /gateway/futarchy/markets/{id}/settle/validate-and-build");
    log::info!("  POST   /gateway/futarchy/markets/{id}/claim/validate-and-build");
    log::info!("  GET    /gateway/health             (gateway health check)");
    log::info!("");
    log::info!("  Prover Gateway (signature-authenticated):");
    log::info!("  GET    /gateway/prover/witness/{{hash}}      (download witness)");
    log::info!("  POST   /gateway/prover/zk/{{id}}/submit      (submit ZK proof)");
    log::info!("  POST   /gateway/prover/fhe/{{id}}/submit     (submit FHE result)");
    log::info!("");
    log::info!("Circuit Type Pricing:");
    log::info!("  0-9   (FHE):        0.01 SOL");
    log::info!("  10-19 (ZK Core):    0.05 SOL");
    log::info!("  20-29 (Voting):     0.05 SOL");
    log::info!("  30-39 (Market):     0.075 SOL");
    log::info!("  40-49 (Portfolio):  0.1 SOL");
    log::info!("");
    log::info!("Starting server at http://{}:{}...", host, port);

    HttpServer::new(move || {
        // Configure CORS to allow requests from blink-server
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(cors)
            .app_data(app_state.clone())
            .configure(handlers::configure_routes)
            .configure(gateway::configure_gateway_routes)
            .configure(prover_gateway::configure_prover_routes)
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}
