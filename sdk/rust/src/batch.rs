//! # BatchBuilder - Parallel FHE Operations
//!
//! Execute multiple FHE operations in parallel for reduced latency.
//!
//! ## Example
//!
//! ```ignore
//! use zyberlink_sdk::Zyber;
//!
//! let results = zyber.batch()
//!     .sum(&values_a)
//!     .average(&values_b)
//!     .count_if(&values_c, ">", 50)
//!     .execute_parallel().await?;
//!
//! println!("Sum: {}", results[0]);
//! println!("Avg: {}", results[1]);
//! println!("Count: {}", results[2]);
//! ```
//!
//! ## Performance
//!
//! Sequential execution: N operations × T time = N×T total
//! Parallel execution:   N operations → max(T) total ≈ T
//!
//! For 3 operations taking 3 minutes each:
//! - Sequential: ~9 minutes
//! - Parallel:   ~3 minutes

use anyhow::{anyhow, Result};
use std::time::Duration;

use crate::prepared::PreparedOperation;

/// Description of a pending batch operation
#[derive(Clone)]
pub enum BatchOp {
    Sum { values: Vec<u8> },
    Average { values: Vec<u8> },
    CountIf { values: Vec<u8>, op: String, threshold: u8 },
}

impl BatchOp {
    /// Get a human-readable description
    pub fn description(&self) -> String {
        match self {
            BatchOp::Sum { values } => format!("Sum({} values)", values.len()),
            BatchOp::Average { values } => format!("Average({} values)", values.len()),
            BatchOp::CountIf { values, op, threshold } => {
                format!("CountIf({} values {} {})", values.len(), op, threshold)
            }
        }
    }
}

/// Result of a single batch operation
#[derive(Debug)]
pub struct BatchResult {
    /// Index in the batch (0-based)
    pub index: usize,
    /// The computed result (if successful)
    pub value: Option<u64>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Time taken for this operation
    pub duration: Duration,
}

impl BatchResult {
    /// Check if this operation succeeded
    pub fn is_ok(&self) -> bool {
        self.value.is_some()
    }

    /// Get the value or panic
    pub fn unwrap(&self) -> u64 {
        self.value.expect("BatchResult was an error")
    }

    /// Get the value or return error
    pub fn get(&self) -> Result<u64> {
        self.value.ok_or_else(|| {
            anyhow!(self.error.clone().unwrap_or_else(|| "Unknown error".into()))
        })
    }
}

/// Summary of batch execution
#[derive(Debug)]
pub struct BatchSummary {
    /// All results in order
    pub results: Vec<BatchResult>,
    /// Total time for batch execution
    pub total_duration: Duration,
    /// Number of successful operations
    pub success_count: usize,
    /// Number of failed operations
    pub failure_count: usize,
}

impl BatchSummary {
    /// Check if all operations succeeded
    pub fn all_ok(&self) -> bool {
        self.failure_count == 0
    }

    /// Get all values (panics if any failed)
    pub fn unwrap_all(&self) -> Vec<u64> {
        self.results.iter().map(|r| r.unwrap()).collect()
    }

    /// Get all values, returning error if any failed
    pub fn get_all(&self) -> Result<Vec<u64>> {
        self.results.iter().map(|r| r.get()).collect()
    }

    /// Iterate over successful results only
    pub fn successes(&self) -> impl Iterator<Item = (usize, u64)> + '_ {
        self.results.iter()
            .filter(|r| r.is_ok())
            .map(|r| (r.index, r.value.unwrap()))
    }

    /// Iterate over failed results only
    pub fn failures(&self) -> impl Iterator<Item = (usize, &str)> + '_ {
        self.results.iter()
            .filter(|r| !r.is_ok())
            .map(|r| (r.index, r.error.as_deref().unwrap_or("Unknown")))
    }
}

/// Builder for batch operations
///
/// Accumulates multiple operations and executes them in parallel.
pub struct BatchBuilder<'a> {
    /// Reference to the Zyber client
    zyber: &'a mut crate::zyber::Zyber,
    /// Pending operations
    operations: Vec<BatchOp>,
    /// Whether to continue on error or fail fast
    continue_on_error: bool,
}

impl<'a> BatchBuilder<'a> {
    /// Create a new batch builder
    pub fn new(zyber: &'a mut crate::zyber::Zyber) -> Self {
        Self {
            zyber,
            operations: Vec::new(),
            continue_on_error: true,
        }
    }

    /// Add a sum operation to the batch
    pub fn sum(mut self, values: &[u8]) -> Self {
        self.operations.push(BatchOp::Sum {
            values: values.to_vec(),
        });
        self
    }

    /// Add an average operation to the batch
    pub fn average(mut self, values: &[u8]) -> Self {
        self.operations.push(BatchOp::Average {
            values: values.to_vec(),
        });
        self
    }

    /// Add a count_if operation to the batch
    pub fn count_if(mut self, values: &[u8], op: &str, threshold: u8) -> Self {
        self.operations.push(BatchOp::CountIf {
            values: values.to_vec(),
            op: op.to_string(),
            threshold,
        });
        self
    }

    /// Set whether to continue on error (default: true)
    ///
    /// If true, all operations are attempted even if some fail.
    /// If false, stops at first error.
    pub fn continue_on_error(mut self, value: bool) -> Self {
        self.continue_on_error = value;
        self
    }

    /// Fail fast on first error
    pub fn fail_fast(self) -> Self {
        self.continue_on_error(false)
    }

    /// Get the number of pending operations
    pub fn len(&self) -> usize {
        self.operations.len()
    }

    /// Check if the batch is empty
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    /// Get descriptions of all pending operations
    pub fn describe(&self) -> Vec<String> {
        self.operations.iter().map(|op| op.description()).collect()
    }

    /// Estimate total cost for all operations
    pub fn estimate_total_cost(&self) -> crate::prepared::CostEstimate {
        // Estimate based on number of operations
        // Each operation has roughly the same base cost
        let n = self.operations.len() as u64;
        let per_job = crate::prepared::CostEstimate {
            job_creation_lamports: 15_000_000,
            prover_payment_lamports: 10_000_000, // Default price
            tx_fee_lamports: 10_000,
            total_lamports: 25_010_000,
        };

        crate::prepared::CostEstimate {
            job_creation_lamports: per_job.job_creation_lamports * n,
            prover_payment_lamports: per_job.prover_payment_lamports * n,
            tx_fee_lamports: per_job.tx_fee_lamports * n,
            total_lamports: per_job.total_lamports * n,
        }
    }

    /// Prepare a single operation
    async fn prepare_op(&mut self, op: BatchOp) -> Result<PreparedOperation> {
        match op {
            BatchOp::Sum { values } => self.zyber.prepare_sum(&values).await,
            BatchOp::Average { values } => self.zyber.prepare_average(&values).await,
            BatchOp::CountIf { values, op, threshold } => {
                self.zyber.prepare_count_if(&values, &op, threshold).await
            }
        }
    }

    /// Execute all operations in parallel
    ///
    /// Returns a summary with all results, even if some failed.
    pub async fn execute_parallel(mut self) -> Result<BatchSummary> {
        if self.operations.is_empty() {
            return Ok(BatchSummary {
                results: Vec::new(),
                total_duration: Duration::ZERO,
                success_count: 0,
                failure_count: 0,
            });
        }

        let start = std::time::Instant::now();
        let ops = std::mem::take(&mut self.operations);
        let n = ops.len();

        // Prepare all operations first (sequential, needs mutable access)
        let mut prepared: Vec<(usize, Result<PreparedOperation>)> = Vec::with_capacity(n);
        for (idx, op) in ops.into_iter().enumerate() {
            let result = self.prepare_op(op).await;
            prepared.push((idx, result));
        }

        // Execute all in parallel using tokio spawn
        let mut handles = Vec::with_capacity(n);

        for (idx, prep_result) in prepared {
            let handle = tokio::spawn(async move {
                let op_start = std::time::Instant::now();

                let result = match prep_result {
                    Ok(prepared) => prepared.execute().await,
                    Err(e) => Err(e),
                };

                let duration = op_start.elapsed();

                match result {
                    Ok(value) => BatchResult {
                        index: idx,
                        value: Some(value),
                        error: None,
                        duration,
                    },
                    Err(e) => BatchResult {
                        index: idx,
                        value: None,
                        error: Some(e.to_string()),
                        duration,
                    },
                }
            });
            handles.push(handle);
        }

        // Wait for all
        let mut results = Vec::with_capacity(n);
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(BatchResult {
                    index: results.len(),
                    value: None,
                    error: Some(format!("Task panicked: {}", e)),
                    duration: Duration::ZERO,
                }),
            }
        }

        // Sort by index (they may complete out of order)
        let mut results = results;
        results.sort_by_key(|r| r.index);

        let success_count = results.iter().filter(|r| r.is_ok()).count();
        let failure_count = results.len() - success_count;

        Ok(BatchSummary {
            results,
            total_duration: start.elapsed(),
            success_count,
            failure_count,
        })
    }

    /// Execute all operations sequentially
    ///
    /// Slower but uses less resources. Stops on first error if fail_fast is set.
    pub async fn execute_sequential(mut self) -> Result<BatchSummary> {
        let start = std::time::Instant::now();
        let ops = std::mem::take(&mut self.operations);
        let mut results = Vec::with_capacity(ops.len());

        for (idx, op) in ops.into_iter().enumerate() {
            let op_start = std::time::Instant::now();

            let prep_result = self.prepare_op(op).await;
            let result = match prep_result {
                Ok(prepared) => prepared.execute().await,
                Err(e) => Err(e),
            };

            let duration = op_start.elapsed();

            let batch_result = match result {
                Ok(value) => BatchResult {
                    index: idx,
                    value: Some(value),
                    error: None,
                    duration,
                },
                Err(e) => {
                    let br = BatchResult {
                        index: idx,
                        value: None,
                        error: Some(e.to_string()),
                        duration,
                    };

                    if !self.continue_on_error {
                        results.push(br);
                        let success_count = results.iter().filter(|r| r.is_ok()).count();
                        return Ok(BatchSummary {
                            failure_count: results.len() - success_count,
                            success_count,
                            results,
                            total_duration: start.elapsed(),
                        });
                    }

                    br
                }
            };

            results.push(batch_result);
        }

        let success_count = results.iter().filter(|r| r.is_ok()).count();
        let failure_count = results.len() - success_count;

        Ok(BatchSummary {
            results,
            total_duration: start.elapsed(),
            success_count,
            failure_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_op_description() {
        let sum = BatchOp::Sum { values: vec![1, 2, 3] };
        assert_eq!(sum.description(), "Sum(3 values)");

        let avg = BatchOp::Average { values: vec![1, 2] };
        assert_eq!(avg.description(), "Average(2 values)");

        let count = BatchOp::CountIf {
            values: vec![1, 2, 3, 4],
            op: ">".to_string(),
            threshold: 2,
        };
        assert_eq!(count.description(), "CountIf(4 values > 2)");
    }

    #[test]
    fn test_batch_result() {
        let success = BatchResult {
            index: 0,
            value: Some(42),
            error: None,
            duration: Duration::from_secs(1),
        };
        assert!(success.is_ok());
        assert_eq!(success.unwrap(), 42);

        let failure = BatchResult {
            index: 1,
            value: None,
            error: Some("Failed".into()),
            duration: Duration::from_secs(1),
        };
        assert!(!failure.is_ok());
        assert!(failure.get().is_err());
    }

    #[test]
    fn test_batch_summary() {
        let summary = BatchSummary {
            results: vec![
                BatchResult {
                    index: 0,
                    value: Some(10),
                    error: None,
                    duration: Duration::from_secs(1),
                },
                BatchResult {
                    index: 1,
                    value: Some(20),
                    error: None,
                    duration: Duration::from_secs(2),
                },
            ],
            total_duration: Duration::from_secs(2),
            success_count: 2,
            failure_count: 0,
        };

        assert!(summary.all_ok());
        assert_eq!(summary.unwrap_all(), vec![10, 20]);
    }
}
