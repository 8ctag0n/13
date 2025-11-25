use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

/// Create a PostgreSQL connection pool
pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    log::info!("Connecting to database: {}", database_url);

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .connect(database_url)
        .await?;

    log::info!("Database connection pool created successfully");
    Ok(pool)
}

/// Run database migrations
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    log::info!("Running database migrations...");

    // Use runtime migration loading from the migrations directory
    log::info!("Loading migrations from ./blink-server/migrations...");
    let migrator = sqlx::migrate::Migrator::new(std::path::Path::new("./blink-server/migrations"))
        .await
        .map_err(|e| {
            log::error!(
                "Failed to load migrations from ./blink-server/migrations: {}",
                e
            );
            e
        })?;

    log::info!("Found {} migrations to apply", migrator.iter().count());
    for migration in migrator.iter() {
        log::info!(
            "  - Migration: {} ({})",
            migration.version,
            migration.description
        );
    }

    log::info!("Running migrations...");
    migrator.run(pool).await.map_err(|e| {
        log::error!("Failed to run migrations: {}", e);
        e
    })?;

    log::info!("Database migrations completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test_pool_creation() {
        // This test would require a running PostgreSQL instance
        // Skip in CI/CD environments without database
    }
}
