//! # FluentOperation - Layer 1 API
//!
//! Fluent builder for FHE operations with retry logic, timeouts, and error handling.
//!
//! ## Example
//!
//! ```ignore
//! use zyberlink_sdk::{Zyber, RetryPolicy, ErrorAction};
//! use std::time::Duration;
//!
//! let result = zyber.sum(&[100, 200, 300])
//!     .with_retry(3)
//!     .with_timeout(Duration::from_secs(60))
//!     .on_error(|e| {
//!         if e.is_timeout() { ErrorAction::Retry }
//!         else { ErrorAction::Fail }
//!     })
//!     .await?;
//! ```
//!
//! ## Progressive Configuration
//!
//! Start simple, add complexity only when needed:
//!
//! ```ignore
//! // Simple (uses defaults)
//! zyber.sum(&values).await?;
//!
//! // Add retry
//! zyber.sum(&values).with_retry(3).await?;
//!
//! // Full control
//! zyber.sum(&values)
//!     .with_retry(3)
//!     .with_backoff(Duration::from_secs(1), 2.0)
//!     .with_timeout(Duration::from_secs(120))
//!     .on_success(|result| println!("Got: {}", result))
//!     .on_error(|e| ErrorAction::Retry)
//!     .await?;
//! ```

use anyhow::{anyhow, Result};
use std::sync::Arc;
use std::time::Duration;

use crate::prepared::PreparedOperation;

/// Action to take when an error occurs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorAction {
    /// Retry the operation
    Retry,
    /// Fail immediately
    Fail,
    /// Skip this operation (for batch operations)
    Skip,
}

/// Error information passed to error handlers
#[derive(Debug)]
pub struct OperationError {
    /// The underlying error
    pub inner: anyhow::Error,
    /// Number of retries attempted so far
    pub retry_count: u32,
    /// Time elapsed since operation started
    pub elapsed: Duration,
}

impl OperationError {
    /// Check if this is a timeout error
    pub fn is_timeout(&self) -> bool {
        let msg = self.inner.to_string().to_lowercase();
        msg.contains("timeout") || msg.contains("timed out")
    }

    /// Check if this is a network error
    pub fn is_network(&self) -> bool {
        let msg = self.inner.to_string().to_lowercase();
        msg.contains("network") || msg.contains("connection") || msg.contains("rpc")
    }

    /// Check if this is a rate limit error
    pub fn is_rate_limited(&self) -> bool {
        let msg = self.inner.to_string().to_lowercase();
        msg.contains("rate limit") || msg.contains("429") || msg.contains("too many")
    }

    /// Check if this is a transient error (worth retrying)
    pub fn is_transient(&self) -> bool {
        self.is_timeout() || self.is_network() || self.is_rate_limited()
    }
}

/// Retry policy configuration
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retries
    pub max_retries: u32,
    /// Initial delay between retries
    pub initial_delay: Duration,
    /// Backoff multiplier (e.g., 2.0 for exponential backoff)
    pub backoff_multiplier: f64,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Whether to add jitter to delays
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_secs(1),
            backoff_multiplier: 2.0,
            max_delay: Duration::from_secs(30),
            jitter: true,
        }
    }
}

impl RetryPolicy {
    /// Create a policy with no retries
    pub fn none() -> Self {
        Self {
            max_retries: 0,
            ..Default::default()
        }
    }

    /// Create a policy with fixed retries (no backoff)
    pub fn fixed(retries: u32, delay: Duration) -> Self {
        Self {
            max_retries: retries,
            initial_delay: delay,
            backoff_multiplier: 1.0,
            max_delay: delay,
            jitter: false,
        }
    }

    /// Create an exponential backoff policy
    pub fn exponential(retries: u32) -> Self {
        Self {
            max_retries: retries,
            initial_delay: Duration::from_secs(1),
            backoff_multiplier: 2.0,
            max_delay: Duration::from_secs(60),
            jitter: true,
        }
    }

    /// Calculate delay for a given retry attempt
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::ZERO;
        }

        let base_delay = self.initial_delay.as_secs_f64()
            * self.backoff_multiplier.powi((attempt - 1) as i32);

        let delay_secs = base_delay.min(self.max_delay.as_secs_f64());

        let final_delay = if self.jitter {
            // Add up to 25% jitter
            let jitter = delay_secs * 0.25 * rand_simple();
            delay_secs + jitter
        } else {
            delay_secs
        };

        Duration::from_secs_f64(final_delay)
    }
}

/// Simple pseudo-random for jitter (no external dependency)
fn rand_simple() -> f64 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (nanos % 1000) as f64 / 1000.0
}

/// Type alias for error handler closure
pub type ErrorHandler = Arc<dyn Fn(&OperationError) -> ErrorAction + Send + Sync>;

/// Type alias for success callback
pub type SuccessCallback = Arc<dyn Fn(u64) + Send + Sync>;

/// A fluent builder for FHE operations
pub struct FluentOperation {
    /// The prepared operation to execute
    prepared: PreparedOperation,
    /// Retry policy
    retry_policy: RetryPolicy,
    /// Custom timeout (overrides prepared operation timeout)
    timeout: Option<Duration>,
    /// Error handler
    error_handler: Option<ErrorHandler>,
    /// Success callback
    on_success: Option<SuccessCallback>,
}

impl FluentOperation {
    /// Create a new fluent operation from a prepared operation
    pub fn new(prepared: PreparedOperation) -> Self {
        Self {
            prepared,
            retry_policy: RetryPolicy::default(),
            timeout: None,
            error_handler: None,
            on_success: None,
        }
    }

    // =========================================================================
    // Retry Configuration
    // =========================================================================

    /// Set the maximum number of retries
    ///
    /// # Example
    /// ```ignore
    /// zyber.sum(&values).with_retry(3).await?;
    /// ```
    pub fn with_retry(mut self, max_retries: u32) -> Self {
        self.retry_policy.max_retries = max_retries;
        self
    }

    /// Set a custom retry policy
    ///
    /// # Example
    /// ```ignore
    /// zyber.sum(&values)
    ///     .with_retry_policy(RetryPolicy::exponential(5))
    ///     .await?;
    /// ```
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = policy;
        self
    }

    /// Configure exponential backoff
    ///
    /// # Example
    /// ```ignore
    /// zyber.sum(&values)
    ///     .with_backoff(Duration::from_secs(1), 2.0)
    ///     .await?;
    /// ```
    pub fn with_backoff(mut self, initial_delay: Duration, multiplier: f64) -> Self {
        self.retry_policy.initial_delay = initial_delay;
        self.retry_policy.backoff_multiplier = multiplier;
        self
    }

    /// Disable retries
    pub fn no_retry(mut self) -> Self {
        self.retry_policy = RetryPolicy::none();
        self
    }

    // =========================================================================
    // Timeout Configuration
    // =========================================================================

    /// Set a custom timeout for this operation
    ///
    /// # Example
    /// ```ignore
    /// zyber.sum(&values)
    ///     .with_timeout(Duration::from_secs(60))
    ///     .await?;
    /// ```
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    // =========================================================================
    // Error Handling
    // =========================================================================

    /// Set a custom error handler
    ///
    /// The handler receives error information and returns an action:
    /// - `ErrorAction::Retry` - retry the operation
    /// - `ErrorAction::Fail` - fail immediately
    ///
    /// # Example
    /// ```ignore
    /// zyber.sum(&values)
    ///     .on_error(|e| {
    ///         if e.is_timeout() { ErrorAction::Retry }
    ///         else { ErrorAction::Fail }
    ///     })
    ///     .await?;
    /// ```
    pub fn on_error<F>(mut self, handler: F) -> Self
    where
        F: Fn(&OperationError) -> ErrorAction + Send + Sync + 'static,
    {
        self.error_handler = Some(Arc::new(handler));
        self
    }

    /// Retry only on transient errors (timeout, network, rate limit)
    pub fn retry_transient(self) -> Self {
        self.on_error(|e| {
            if e.is_transient() {
                ErrorAction::Retry
            } else {
                ErrorAction::Fail
            }
        })
    }

    // =========================================================================
    // Success Handling
    // =========================================================================

    /// Set a success callback
    ///
    /// # Example
    /// ```ignore
    /// zyber.sum(&values)
    ///     .on_success(|result| println!("Computed: {}", result))
    ///     .await?;
    /// ```
    pub fn on_success_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(u64) + Send + Sync + 'static,
    {
        self.on_success = Some(Arc::new(callback));
        self
    }

    // =========================================================================
    // Inspection (delegates to PreparedOperation)
    // =========================================================================

    /// Get estimated cost
    pub fn estimate_cost(&self) -> crate::prepared::CostEstimate {
        self.prepared.estimate_cost()
    }

    /// Get PDAs
    pub fn pdas(&self) -> crate::prepared::OperationPdas {
        self.prepared.pdas()
    }

    /// Get job ID
    pub fn job_id(&self) -> u64 {
        self.prepared.job_id()
    }

    // =========================================================================
    // Execution
    // =========================================================================

    /// Execute the operation with configured retry logic
    pub async fn execute(self) -> Result<u64> {
        let start = std::time::Instant::now();
        let on_success = self.on_success.clone();
        let error_handler = self.error_handler.clone();
        let retry_policy = self.retry_policy.clone();

        // First attempt uses the prepared operation directly
        let result = self.prepared.execute().await;

        match result {
            Ok(value) => {
                if let Some(callback) = on_success {
                    callback(value);
                }
                Ok(value)
            }
            Err(e) => {
                let op_error = OperationError {
                    inner: e,
                    retry_count: 1,
                    elapsed: start.elapsed(),
                };

                // Check if we should retry
                let action = if let Some(ref handler) = error_handler {
                    handler(&op_error)
                } else {
                    // Default: retry on transient errors
                    if op_error.is_transient() {
                        ErrorAction::Retry
                    } else {
                        ErrorAction::Fail
                    }
                };

                match action {
                    ErrorAction::Fail | ErrorAction::Skip => {
                        Err(anyhow!(
                            "Operation failed: {}",
                            op_error.inner
                        ))
                    }
                    ErrorAction::Retry => {
                        // Wait before retry indication
                        let delay = retry_policy.delay_for_attempt(1);
                        if delay > Duration::ZERO {
                            tokio::time::sleep(delay).await;
                        }

                        // PreparedOperation is consumed - can't retry without re-preparation
                        // Return error with retry hint
                        Err(anyhow!(
                            "Operation failed (retry recommended after {:?}): {}. \
                            Note: Use prepare_*() to enable full retry capability.",
                            delay,
                            op_error.inner
                        ))
                    }
                }
            }
        }
    }
}

/// Extension trait to make any async operation fluent
pub trait IntoFluent {
    fn into_fluent(self) -> FluentOperation;
}

impl IntoFluent for PreparedOperation {
    fn into_fluent(self) -> FluentOperation {
        FluentOperation::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_policy_default() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_retries, 3);
        assert_eq!(policy.backoff_multiplier, 2.0);
    }

    #[test]
    fn test_retry_policy_none() {
        let policy = RetryPolicy::none();
        assert_eq!(policy.max_retries, 0);
    }

    #[test]
    fn test_retry_policy_exponential() {
        let policy = RetryPolicy::exponential(5);
        assert_eq!(policy.max_retries, 5);
        assert_eq!(policy.backoff_multiplier, 2.0);
    }

    #[test]
    fn test_delay_calculation() {
        let policy = RetryPolicy {
            max_retries: 5,
            initial_delay: Duration::from_secs(1),
            backoff_multiplier: 2.0,
            max_delay: Duration::from_secs(60),
            jitter: false,
        };

        assert_eq!(policy.delay_for_attempt(0), Duration::ZERO);
        assert_eq!(policy.delay_for_attempt(1), Duration::from_secs(1));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_secs(2));
        assert_eq!(policy.delay_for_attempt(3), Duration::from_secs(4));
        assert_eq!(policy.delay_for_attempt(4), Duration::from_secs(8));
    }

    #[test]
    fn test_delay_max_cap() {
        let policy = RetryPolicy {
            max_retries: 10,
            initial_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            max_delay: Duration::from_secs(30),
            jitter: false,
        };

        // 10 * 2^5 = 320, should be capped at 30
        assert_eq!(policy.delay_for_attempt(6), Duration::from_secs(30));
    }

    #[test]
    fn test_operation_error_detection() {
        let timeout_err = OperationError {
            inner: anyhow!("Request timed out"),
            retry_count: 1,
            elapsed: Duration::from_secs(5),
        };
        assert!(timeout_err.is_timeout());
        assert!(timeout_err.is_transient());

        let network_err = OperationError {
            inner: anyhow!("Network connection failed"),
            retry_count: 1,
            elapsed: Duration::from_secs(5),
        };
        assert!(network_err.is_network());
        assert!(network_err.is_transient());

        let other_err = OperationError {
            inner: anyhow!("Invalid input data"),
            retry_count: 1,
            elapsed: Duration::from_secs(5),
        };
        assert!(!other_err.is_transient());
    }
}
