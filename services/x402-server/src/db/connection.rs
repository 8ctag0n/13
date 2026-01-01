//! Database connection pool

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// Create a new PostgreSQL connection pool
pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    log::info!("Connecting to database...");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;

    log::info!("Database connection established");
    Ok(pool)
}
