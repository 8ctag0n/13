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

    // Try multiple paths for migrations (local dev vs container)
    let paths = [
        "./src/blink-server/migrations", // Local development
        "./blink-server/migrations",     // Container
    ];

    let mut migrator = None;
    for path in &paths {
        log::info!("Trying migrations from {}...", path);
        match sqlx::migrate::Migrator::new(std::path::Path::new(path)).await {
            Ok(m) => {
                log::info!("Found migrations at {}", path);
                migrator = Some(m);
                break;
            }
            Err(_) => continue,
        }
    }

    let migrator = migrator.ok_or_else(|| {
        log::error!("Failed to load migrations from any path: {:?}", paths);
        sqlx::Error::Configuration("No migrations directory found".into())
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
