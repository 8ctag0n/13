mod actions;
mod tx_builder;

use actix_cors::Cors;
use actix_web::{get, middleware, web, App, HttpResponse, HttpServer, Responder};
use actix_web::http::header::ContentType;
use std::env;

/// Health check endpoint
#[get("/health")]
async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": "zyberlink-blink-server"
    }))
}

/// Serve actions.json at root
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

    // Get configuration from environment
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid u16");

    let base_url = env::var("BASE_URL")
        .unwrap_or_else(|_| format!("http://{}:{}", host, port));

    log::info!("🚀 Starting ZyberLink Blink Server");
    log::info!("   Base URL: {}", base_url);
    log::info!("   Listening on: {}:{}", host, port);
    log::info!("");
    log::info!("📡 Endpoints:");
    log::info!("   GET  /health");
    log::info!("   GET  /actions.json");
    log::info!("   GET  /static/zyberlink-icon.svg");
    log::info!("   GET  /api/actions/fund-prover");
    log::info!("   POST /api/actions/fund-prover");
    log::info!("");

    HttpServer::new(move || {
        // Configure CORS for Solana Actions
        let cors = Cors::default()
            .allow_any_origin()
            .allowed_methods(vec!["GET", "POST", "PUT", "OPTIONS"])
            .allowed_headers(vec![
                "Content-Type",
                "Authorization",
                "Content-Encoding",
                "Accept-Encoding",
            ])
            .max_age(3600);

        App::new()
            .wrap(middleware::Logger::default())
            .wrap(cors)
            .service(health_check)
            .service(actions_json)
            .service(zyberlink_icon)
            .service(actions::get_fund_prover_action)
            .service(actions::post_fund_prover_action)
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}
