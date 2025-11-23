mod actions;
mod api_handlers;
mod cleanup;
mod db;
mod tx_builder;
mod validators;

use actix_cors::Cors;
use actix_web::{get, middleware, web, App, HttpResponse, HttpServer, Responder};
use cypherlink_sdk::instructions::InstructionBuilder;
use solana_sdk::pubkey::Pubkey;
use sqlx::PgPool;
use std::env;
use std::str::FromStr;

/// Application state shared across handlers
pub struct AppState {
    pub db_pool: PgPool,
    pub sdk_builder: InstructionBuilder,
}

/// Health check endpoint
#[get("/health")]
async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": "zyberlink-blink-server",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// Serve actions.json at root (legacy blinks support)
#[get("/actions.json")]
async fn actions_json() -> impl Responder {
    let actions_json = include_str!("../static/actions.json");
    HttpResponse::Ok()
        .content_type("application/json")
        .body(actions_json)
}

/// Serve ZyberLink icon
#[get("/static/zyberlink-icon.svg")]
async fn zyberlink_icon() -> impl Responder {
    let icon_svg = include_str!("../static/zyberlink-icon.svg");
    HttpResponse::Ok()
        .content_type("image/svg+xml")
        .body(icon_svg)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logger
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    log::info!("=================================================");
    log::info!("  ZyberLink Marketplace Backend");
    log::info!("  Version: {}", env!("CARGO_PKG_VERSION"));
    log::info!("=================================================");

    // ========================================================================
    // Configuration
    // ========================================================================

    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid u16");

    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://zyberlink:dev_password@localhost:5432/zyberlink".to_string()
    });

    let rpc_url = env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "http://localhost:8899".to_string());

    let program_id_str = env::var("PROGRAM_ID")
        .unwrap_or_else(|_| "CypherLinkProgram11111111111111111111111111".to_string());
    let program_id =
        Pubkey::from_str(&program_id_str).expect("Invalid PROGRAM_ID");

    let cleanup_interval_secs = env::var("CLEANUP_INTERVAL_SECS")
        .unwrap_or_else(|_| "3600".to_string())
        .parse::<u64>()
        .unwrap_or(3600); // Default: 1 hour

    // ========================================================================
    // Database Setup
    // ========================================================================

    log::info!("Connecting to database...");
    let pool = db::create_pool(&database_url)
        .await
        .expect("Failed to connect to PostgreSQL");

    log::info!("Running database migrations...");
    if let Err(e) = db::run_migrations(&pool).await {
        log::warn!("Migration warning (might already exist): {}", e);
        // Don't fail on migration errors (tables might already exist)
    }

    // ========================================================================
    // SDK Client Setup
    // ========================================================================

    log::info!("Initializing SDK instruction builder...");
    let sdk_builder = InstructionBuilder::new(program_id);

    // ========================================================================
    // Application State
    // ========================================================================

    let app_state = web::Data::new(AppState {
        db_pool: pool.clone(),
        sdk_builder,
    });

    // ========================================================================
    // Background Tasks
    // ========================================================================

    log::info!("Starting background cleanup task...");
    cleanup::spawn_cleanup_task(pool.clone(), cleanup_interval_secs);

    // ========================================================================
    // Server Configuration
    // ========================================================================

    log::info!("");
    log::info!("Configuration:");
    log::info!("  Host: {}", host);
    log::info!("  Port: {}", port);
    log::info!("  Solana RPC: {}", rpc_url);
    log::info!("  Program ID: {}", program_id);
    log::info!("  Cleanup Interval: {}s", cleanup_interval_secs);
    log::info!("");
    log::info!("API Endpoints:");
    log::info!("  GET    /health");
    log::info!("  GET    /api/jobs                         (list jobs, optional ?status= filter)");
    log::info!("  POST   /api/jobs/validate-and-build");
    log::info!("  GET    /api/jobs/{{job_id}}/compute-data");
    log::info!("  POST   /api/jobs/{{job_id}}/confirm");
    log::info!("  GET    /api/jobs/{{job_id}}/status");
    log::info!("  DELETE /api/jobs/{{job_id}}");
    log::info!("");
    log::info!("Legacy Blinks:");
    log::info!("  GET    /actions.json");
    log::info!("  GET    /api/actions/fund-prover");
    log::info!("  POST   /api/actions/fund-prover");
    log::info!("");
    log::info!("Server starting on {}:{}", host, port);
    log::info!("=================================================");

    // ========================================================================
    // HTTP Server
    // ========================================================================

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allowed_headers(vec![
                "Content-Type",
                "Authorization",
                "Content-Encoding",
                "Accept-Encoding",
            ])
            .max_age(3600);

        App::new()
            .app_data(app_state.clone())
            // Increase JSON payload limit for large TFHE ServerKeys (up to 200 MB)
            .app_data(web::JsonConfig::default().limit(200 * 1024 * 1024))
            .wrap(middleware::Logger::default())
            .wrap(cors)
            // Core endpoints
            .service(health_check)
            .service(actions_json)
            .service(zyberlink_icon)
            // Legacy blinks
            .service(actions::get_fund_prover_action)
            .service(actions::post_fund_prover_action)
            // New marketplace API
            .configure(api_handlers::configure_routes)
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}
