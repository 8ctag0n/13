mod actions;
mod api_handlers;
mod chain_sync;
mod cleanup;
mod db;
mod job_finalizer;
mod pbtcfi_handlers;
mod pbtcfi_sync;
mod prover_sync;
mod services;
mod tx_builder;
mod validators;
mod x402_client;
mod zk_handlers;

// NOTE: CORS removed - internal service only
// use actix_cors::Cors;
use actix_web::{get, middleware, web, App, HttpResponse, HttpServer, Responder};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use sqlx::PgPool;
use std::env;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use zyberlink_sdk::instructions::InstructionBuilder;
use zyberlink_chain_client::{SolanaClient, StarknetClient};

use services::AttestationService;

/// Application state shared across handlers
pub struct AppState {
    pub db_pool: PgPool,
    pub sdk_builder: InstructionBuilder,
    pub rpc_url: String,
    pub program_id: Pubkey,
    pub x402_url: String,
}

/// Health check endpoint
#[get("/health")]
async fn health_check(data: web::Data<AppState>) -> impl Responder {
    // Quick DB check
    let db_ok = sqlx::query("SELECT 1")
        .fetch_one(&data.db_pool)
        .await
        .is_ok();

    let status = if db_ok { "ok" } else { "degraded" };
    let body = serde_json::json!({
        "status": status,
        "service": "blink-server-internal",
        "version": env!("CARGO_PKG_VERSION"),
        "checks": {
            "database": db_ok
        }
    });

    if db_ok {
        HttpResponse::Ok().json(body)
    } else {
        HttpResponse::ServiceUnavailable().json(body)
    }
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
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid u16");

    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://zyberlink:dev_password@localhost:5432/zyberlink".to_string()
    });

    let rpc_url =
        env::var("SOLANA_RPC_URL").unwrap_or_else(|_| "http://localhost:8899".to_string());

    let program_id_str = env::var("PROGRAM_ID")
        .unwrap_or_else(|_| "ZyberLinkProgram11111111111111111111111111".to_string());
    let program_id = Pubkey::from_str(&program_id_str).expect("Invalid PROGRAM_ID");

    let x402_url =
        env::var("X402_URL").unwrap_or_else(|_| "http://localhost:8081".to_string());

    let cleanup_interval_secs = env::var("CLEANUP_INTERVAL_SECS")
        .unwrap_or_else(|_| "3600".to_string())
        .parse::<u64>()
        .unwrap_or(3600); // Default: 1 hour

    let vk_directory = env::var("VK_DIRECTORY")
        .unwrap_or_else(|_| "./verification_keys".to_string());

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
    // Attestation Service Setup
    // ========================================================================

    log::info!("Initializing AttestationService...");
    let vk_path = PathBuf::from(&vk_directory);
    let attestation_service = match AttestationService::new(&vk_path) {
        Ok(service) => {
            let circuits = service.available_circuits();
            if circuits.is_empty() {
                log::warn!("AttestationService initialized but NO verification keys loaded!");
                log::warn!("Proofs will not be verified. Set VK_DIRECTORY env var with VK files.");
            } else {
                log::info!("AttestationService ready with {} circuit types: {:?}", circuits.len(), circuits);
            }
            Some(Arc::new(service))
        }
        Err(e) => {
            log::warn!("Failed to initialize AttestationService: {}", e);
            log::warn!("ZK proof verification will be disabled. To enable, provide VKs in: {}", vk_directory);
            None
        }
    };

    // ========================================================================
    // Application State
    // ========================================================================

    let app_state = web::Data::new(AppState {
        db_pool: pool.clone(),
        sdk_builder,
        rpc_url: rpc_url.clone(),
        program_id,
        x402_url: x402_url.clone(),
    });

    // ========================================================================
    // Background Tasks
    // ========================================================================

    log::info!("Starting background cleanup task...");
    cleanup::spawn_cleanup_task(pool.clone(), cleanup_interval_secs);

    log::info!("Starting blockchain sync task...");
    log::info!("Creating SolanaClient for chain sync...");
    let chain_client = Arc::new(
        SolanaClient::new(&rpc_url)
            .expect("Failed to create SolanaClient")
    );
    chain_sync::start_chain_sync(chain_client.clone(), program_id, pool.clone());
    log::info!("Blockchain sync task started");

    log::info!("Starting prover sync task...");
    prover_sync::start_prover_sync(chain_client.clone(), program_id, pool.clone());
    log::info!("Prover sync task started");

    // pBTCFi event sync (optional - only if env vars are set)
    if let Ok(pbtcfi_contract) = env::var("PBTCFI_CONTRACT_ADDRESS") {
        log::info!("Starting pBTCFi event sync task...");
        let starknet_rpc = env::var("STARKNET_RPC_URL")
            .unwrap_or_else(|_| "http://localhost:5050".to_string());

        log::info!("  Starknet RPC: {}", starknet_rpc);
        log::info!("  pBTCFi Contract: {}", pbtcfi_contract);

        log::info!("Creating StarknetClient for pBTCFi sync...");
        match StarknetClient::new(&starknet_rpc) {
            Ok(starknet_client) => {
                let starknet_client = Arc::new(starknet_client);
                pbtcfi_sync::start_pbtcfi_sync(
                    starknet_client,
                    pbtcfi_contract,
                    pool.clone(),
                );
                log::info!("pBTCFi event sync task started");
            }
            Err(e) => {
                log::error!("Failed to create StarknetClient: {}", e);
                log::error!("pBTCFi sync will not start");
            }
        }
    } else {
        log::info!("pBTCFi sync disabled (set PBTCFI_CONTRACT_ADDRESS to enable)");
    }

    // Job finalizer (requires server keypair to sign finalize transactions)
    let server_keypair_path =
        env::var("SERVER_KEYPAIR_PATH").unwrap_or_else(|_| "~/.config/solana/id.json".to_string());

    match job_finalizer::load_server_keypair(&server_keypair_path) {
        Ok(keypair) => {
            log::info!("Starting job finalizer task...");
            log::info!("  Finalizer pubkey: {}", keypair.pubkey());
            job_finalizer::start_job_finalizer(
                chain_client.clone(),
                program_id,
                pool.clone(),
                Arc::new(keypair),
            );
            log::info!("Job finalizer task started");
        }
        Err(e) => {
            log::warn!("Job finalizer disabled: {}", e);
            log::warn!("Set SERVER_KEYPAIR_PATH to enable automatic FHE job finalization");
        }
    }

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
    log::info!("Internal FHE API:");
    log::info!("  GET    /health");
    log::info!("  GET    /internal/fhe/list                    (list FHE jobs)");
    log::info!("  POST   /internal/fhe/validate-and-build");
    log::info!("  GET    /internal/fhe/{{job_id}}/compute-data");
    log::info!("  POST   /internal/fhe/{{job_id}}/confirm");
    log::info!("  GET    /internal/fhe/{{job_id}}/status");
    log::info!("  GET    /internal/fhe/{{job_id}}              (full job details)");
    log::info!("  DELETE /internal/fhe/{{job_id}}");
    log::info!("");
    log::info!("Internal ZK API:");
    log::info!("  GET    /internal/zk                          (list ZK jobs)");
    log::info!("  POST   /internal/zk/validate-and-build");
    log::info!("  GET    /internal/zk/{{job_id}}/status");
    log::info!("  POST   /internal/zk/{{job_id}}/confirm");
    log::info!("");
    log::info!("NOTE: This is an INTERNAL service (port {})", port);
    log::info!("      Should only be accessed via public-api gateway");
    log::info!("");
    log::info!("Server starting on {}:{}", host, port);
    log::info!("=================================================");

    // ========================================================================
    // HTTP Server
    // ========================================================================

    HttpServer::new(move || {
        // NOTE: CORS removed - this is an internal service
        // All external requests go through public-api gateway

        let mut app = App::new()
            .app_data(app_state.clone())
            .app_data(web::Data::new(pool.clone())) // For pBTCFi handlers
            // Increase JSON payload limit for large TFHE ServerKeys (up to 200 MB)
            .app_data(web::JsonConfig::default().limit(200 * 1024 * 1024))
            // Increase raw payload limit for witness data (up to 200 MB)
            .app_data(web::PayloadConfig::default().limit(200 * 1024 * 1024))
            .wrap(middleware::Logger::default())
            // Core endpoints
            .service(health_check)
            .service(actions_json)
            .service(zyberlink_icon)
            // Legacy blinks
            .service(actions::get_fund_prover_action)
            .service(actions::post_fund_prover_action)
            // FHE Jobs API
            .configure(api_handlers::configure_routes)
            // ZK Jobs API
            .configure(zk_handlers::configure_routes)
            // pBTCFi API
            .configure(pbtcfi_handlers::configure_routes);

        // Add AttestationService if available
        if let Some(service) = attestation_service.clone() {
            app = app.app_data(web::Data::from(service));
        }

        app
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}
