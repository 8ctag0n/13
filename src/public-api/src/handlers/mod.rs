mod health;
mod quote;
mod jobs;
mod stats;
mod futarchy;
mod gateway;

use actix_web::web;

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    // Health
    cfg.service(health::health_check);

    // Quote & Pricing
    cfg.service(quote::get_quote);

    // Jobs
    cfg.service(jobs::create_zk_job);
    cfg.service(jobs::create_fhe_job);
    cfg.service(jobs::list_zk_jobs);
    cfg.service(jobs::list_fhe_jobs);
    cfg.service(jobs::get_zk_job);
    cfg.service(jobs::get_fhe_job);

    // Stats
    cfg.service(stats::get_network_stats);
    cfg.service(stats::get_metrics);

    // Futarchy
    cfg.service(futarchy::futarchy_health);
    cfg.service(futarchy::list_markets);
    cfg.service(futarchy::get_market);
    cfg.service(futarchy::get_market_positions);
    cfg.service(futarchy::get_bettor_positions);
    cfg.service(futarchy::get_pending_fhe_jobs);
    cfg.service(futarchy::get_fhe_job_data);
    cfg.service(futarchy::create_market);
    cfg.service(futarchy::place_bet);
    cfg.service(futarchy::settle_market);
    cfg.service(futarchy::prepare_bet);
    cfg.service(futarchy::submit_bet);
    cfg.service(futarchy::complete_fhe_job);
    cfg.service(futarchy::fail_fhe_job);
    cfg.service(futarchy::submit_fhe_job_result);
    cfg.service(futarchy::validate_market);
    cfg.service(futarchy::validate_bet);
    cfg.service(futarchy::validate_settle);
    cfg.service(futarchy::validate_claim);

    // Gateway Prover (proxy to x402)
    cfg.service(gateway::get_witness);
    cfg.service(gateway::submit_zk_proof);
    cfg.service(gateway::submit_fhe_result);
}
