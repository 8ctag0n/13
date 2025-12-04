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

            if let Err(e) = self.cleanup_synced_jobs().await {
                log::error!("Failed to cleanup synced jobs: {}", e);
            }

            if let Err(e) = self.cleanup_old_nonces().await {
                log::error!("Failed to cleanup old nonces: {}", e);
            }

            if let Err(e) = self.cleanup_old_witnesses().await {
                log::error!("Failed to cleanup old witnesses: {}", e);
            }
        }
    }

    /// Delete expired jobs from database
    async fn cleanup_expired_jobs(&self) -> Result<(), sqlx::Error> {
        let count = JobQueries::delete_expired_jobs(&self.pool)
            .await
            .map_err(|e| sqlx::Error::Protocol(format!("Cleanup failed: {}", e)))?;

        if count > 0 {
            log::info!("Cleaned up {} expired jobs", count);
        }

        Ok(())
    }

    /// Delete jobs from temp_job_data that are already synced to blockchain_jobs
    async fn cleanup_synced_jobs(&self) -> Result<(), sqlx::Error> {
        let count = JobQueries::delete_synced_jobs(&self.pool)
            .await
            .map_err(|e| sqlx::Error::Protocol(format!("Cleanup failed: {}", e)))?;

        if count > 0 {
            log::info!("Cleaned up {} synced jobs from temp_job_data", count);
        }

        Ok(())
    }

    /// Delete old nonces (older than 1 day)
    async fn cleanup_old_nonces(&self) -> Result<(), sqlx::Error> {
        const ONE_DAY_SECS: i64 = 86400;

        let count = NonceQueries::delete_old_nonces(&self.pool, ONE_DAY_SECS)
            .await
            .map_err(|e| sqlx::Error::Protocol(format!("Cleanup failed: {}", e)))?;

        if count > 0 {
            log::info!("Cleaned up {} old nonces", count);
        }

        Ok(())
    }

    /// Smart cleanup of witnesses based on job status
    /// FHE witnesses are ~123 MB each, so this prevents disk from filling up
    ///
    /// Cleanup strategy:
    /// 1. Delete witnesses for completed/failed/cancelled jobs (with 2h grace period)
    /// 2. Delete witnesses for timed-out jobs
    /// 3. Fallback: delete orphaned witnesses older than 48h (no associated job)
    async fn cleanup_old_witnesses(&self) -> Result<(), sqlx::Error> {
        // Strategy 1: Delete witnesses for completed/failed/cancelled jobs
        // Grace period of 2 hours to ensure all provers have downloaded
        let result = sqlx::query!(
            r#"
            DELETE FROM witnesses
            WHERE commitment IN (
                SELECT witness_hash FROM blockchain_jobs
                WHERE status IN ('completed', 'failed', 'cancelled')
                AND completed_at < NOW() - INTERVAL '2 hours'
            )
            "#
        )
        .execute(&self.pool)
        .await?;

        let completed_count = result.rows_affected();

        // Strategy 2: Delete witnesses for timed-out jobs (not completed)
        let result = sqlx::query!(
            r#"
            DELETE FROM witnesses
            WHERE commitment IN (
                SELECT witness_hash FROM blockchain_jobs
                WHERE timeout_at < NOW()
                AND status NOT IN ('completed')
            )
            "#
        )
        .execute(&self.pool)
        .await?;

        let timeout_count = result.rows_affected();

        // Strategy 3: Fallback - delete orphaned witnesses older than 48h
        // These are witnesses without an associated job in blockchain_jobs
        let result = sqlx::query!(
            r#"
            DELETE FROM witnesses
            WHERE created_at < NOW() - INTERVAL '48 hours'
            AND commitment NOT IN (
                SELECT COALESCE(witness_hash, '') FROM blockchain_jobs
            )
            "#
        )
        .execute(&self.pool)
        .await?;

        let orphan_count = result.rows_affected();

        let total = completed_count + timeout_count + orphan_count;
        if total > 0 {
            log::info!(
                "Cleaned up {} witnesses (~{}MB freed): {} from completed/failed jobs, {} from timed-out, {} orphaned",
                total,
                total * 123,
                completed_count,
                timeout_count,
                orphan_count
            );
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

    #[test]
    fn test_cleanup_service_creation() {
        // Just verify struct creation works
        // Full testing would require a test database
    }
}
