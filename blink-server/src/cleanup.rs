use sqlx::PgPool;
use tokio::time::{interval, Duration};

use crate::db::{JobQueries, NonceQueries};

/// Background cleanup service
///
/// Runs periodic cleanup tasks:
/// - Delete expired jobs (24h+ old)
/// - Delete old nonces (1 day+ old)
pub struct CleanupService {
    pool: PgPool,
    interval_secs: u64,
}

impl CleanupService {
    /// Create a new cleanup service
    pub fn new(pool: PgPool, interval_secs: u64) -> Self {
        Self {
            pool,
            interval_secs,
        }
    }

    /// Start the cleanup service (runs forever)
    pub async fn start(self) {
        log::info!(
            "Starting cleanup service (interval: {}s)",
            self.interval_secs
        );

        let mut interval = interval(Duration::from_secs(self.interval_secs));

        loop {
            interval.tick().await;

            if let Err(e) = self.cleanup_expired_jobs().await {
                log::error!("Failed to cleanup expired jobs: {}", e);
            }

            if let Err(e) = self.cleanup_old_nonces().await {
                log::error!("Failed to cleanup old nonces: {}", e);
            }
        }
    }

    /// Delete expired jobs from database
    async fn cleanup_expired_jobs(&self) -> Result<(), sqlx::Error> {
        let count = JobQueries::delete_expired_jobs(&self.pool).await
            .map_err(|e| sqlx::Error::Protocol(format!("Cleanup failed: {}", e)))?;

        if count > 0 {
            log::info!("Cleaned up {} expired jobs", count);
        }

        Ok(())
    }

    /// Delete old nonces (older than 1 day)
    async fn cleanup_old_nonces(&self) -> Result<(), sqlx::Error> {
        const ONE_DAY_SECS: i64 = 86400;

        let count = NonceQueries::delete_old_nonces(&self.pool, ONE_DAY_SECS).await
            .map_err(|e| sqlx::Error::Protocol(format!("Cleanup failed: {}", e)))?;

        if count > 0 {
            log::info!("Cleaned up {} old nonces", count);
        }

        Ok(())
    }
}

/// Start cleanup job in background
pub fn spawn_cleanup_task(pool: PgPool, interval_secs: u64) {
    tokio::spawn(async move {
        let service = CleanupService::new(pool, interval_secs);
        service.start().await;
    });

    log::info!("Cleanup task spawned in background");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_service_creation() {
        // Just verify struct creation works
        // Full testing would require a test database
    }
}
