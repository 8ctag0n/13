mod health;
mod quote;
mod jobs;
mod stats;

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
}
