//! Public API Gateway
//!
//! Single entry point for all external requests.
//! Routes to x402 (anti-spam) and blink-server (backend).

mod handlers;
mod clients;
mod middleware;
mod solana;

use actix_cors::Cors;
use actix_web::{web, App, HttpServer, middleware as actix_middleware};
use std::env;

use clients::{BlinkClient, X402Client};
use middleware::rate_limit::{create_rate_limiter, GlobalRateLimiter};
use solana::SolanaVerifier;

/// Application state shared across handlers
pub struct AppState {
    pub x402_client: X402Client,
    pub blink_client: BlinkClient,
    pub rate_limiter: GlobalRateLimiter,
    pub solana_verifier: SolanaVerifier,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    log::info!("===========================================");
    log::info!("  ZyberLink Public API Gateway");
    log::info!("  Version: {}", env!("CARGO_PKG_VERSION"));
    log::info!("===========================================");

    // Configuration
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()
        .expect("PORT must be a valid port number");

    let x402_url = env::var("X402_URL").unwrap_or_else(|_| "http://localhost:8081".to_string());
    let blink_url = env::var("BLINK_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let solana_rpc_url = env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());

    let rate_limit_rpm: u32 = env::var("RATE_LIMIT_RPM")
        .unwrap_or_else(|_| "100".to_string())
        .parse()
        .unwrap_or(100);

    // Initialize clients
    let x402_client = X402Client::new(&x402_url);
    let blink_client = BlinkClient::new(&blink_url);
    let rate_limiter = create_rate_limiter(rate_limit_rpm);
    let solana_verifier = SolanaVerifier::new(&solana_rpc_url);

    let app_state = web::Data::new(AppState {
        x402_client,
        blink_client,
        rate_limiter,
        solana_verifier,
    });

    log::info!("");
    log::info!("Configuration:");
    log::info!("  Host: {}", host);
    log::info!("  Port: {}", port);
    log::info!("  X402 URL: {}", x402_url);
    log::info!("  Blink URL: {}", blink_url);
    log::info!("  Rate Limit: {} req/min", rate_limit_rpm);
    log::info!("");
    log::info!("Public Endpoints:");
    log::info!("  GET  /health");
    log::info!("");
    log::info!("  Pricing:");
    log::info!("  POST /api/quote                   (get price quote)");
    log::info!("");
    log::info!("  ZK Jobs:");
    log::info!("  POST /api/jobs/zk/create          (create ZK job)");
    log::info!("  GET  /api/jobs/zk                 (list ZK jobs)");
    log::info!("  GET  /api/jobs/zk/{{job_id}}        (get ZK job status)");
    log::info!("");
    log::info!("  FHE Jobs:");
    log::info!("  POST /api/jobs/fhe/create         (create FHE job)");
    log::info!("  GET  /api/jobs/fhe                (list FHE jobs)");
    log::info!("  GET  /api/jobs/fhe/{{job_id}}       (get FHE job details)");
    log::info!("");
    log::info!("  Stats:");
    log::info!("  GET  /api/stats/network           (network statistics)");
    log::info!("  GET  /api/metrics                 (prometheus metrics)");
    log::info!("");
    log::info!("Starting server at http://{}:{}...", host, port);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .app_data(app_state.clone())
            .wrap(actix_middleware::Logger::default())
            .wrap(cors)
            .configure(handlers::configure_routes)
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}
