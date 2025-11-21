use cypherlink_types::{CircuitType, fhe::FheOperation};
use log::{debug, info, warn};

/// ROI calculation result for a job
#[derive(Debug, Clone)]
pub struct JobROI {
    /// Expected revenue per prover (lamports)
    pub revenue_per_prover: u64,

    /// Estimated computational cost (lamports)
    pub estimated_cost: u64,

    /// Expected profit (revenue - cost, in lamports)
    pub profit: i64,

    /// Return on Investment as percentage (profit / cost * 100)
    pub roi_percentage: f64,

    /// Complexity tier of the operation
    pub complexity_tier: u8,

    /// Expected timeout for the operation (seconds)
    pub timeout_seconds: i64,

    /// Whether this job is profitable
    pub is_profitable: bool,
}

/// ROI Calculator for evaluating job profitability
pub struct ROICalculator {
    /// Minimum ROI percentage required to accept a job (default: 20%)
    min_roi_percentage: f64,

    /// Operational cost multiplier (infrastructure, electricity, etc.)
    /// Default: 1.5 (50% overhead on computational cost)
    operational_cost_multiplier: f64,
}

impl Default for ROICalculator {
    fn default() -> Self {
        Self::new(20.0, 1.5)
    }
}

impl ROICalculator {
    /// Create a new ROI calculator
    ///
    /// # Arguments
    /// * `min_roi_percentage` - Minimum ROI % to accept job (e.g., 20.0 = 20%)
    /// * `operational_cost_multiplier` - Cost multiplier for overhead (e.g., 1.5 = 50% overhead)
    pub fn new(min_roi_percentage: f64, operational_cost_multiplier: f64) -> Self {
        Self {
            min_roi_percentage,
            operational_cost_multiplier,
        }
    }

    /// Evaluate if a job is profitable based on its parameters
    ///
    /// # Arguments
    /// * `circuit_type` - Type of circuit/operation
    /// * `price_lamports` - Total price offered for the job
    /// * `required_provers` - Number of provers needed for consensus
    ///
    /// # Returns
    /// `JobROI` struct with detailed profitability analysis
    pub fn evaluate_job(
        &self,
        circuit_type: &CircuitType,
        price_lamports: u64,
        required_provers: u8,
    ) -> JobROI {
        // Extract FHE operation from circuit type
        let fhe_operation = match circuit_type {
            CircuitType::FheComputation(op) => op,
            _ => {
                // For non-FHE jobs, use simple flat pricing model
                return self.evaluate_simple_job(price_lamports, required_provers);
            }
        };

        // Get cost configuration from operation
        let cost_config = fhe_operation.get_cost_config();

        // Calculate revenue per prover
        let revenue_per_prover = price_lamports / (required_provers as u64);

        // Calculate total cost including operational overhead
        let base_cost = cost_config.min_payment_lamports;
        let total_cost = (base_cost as f64 * self.operational_cost_multiplier) as u64;

        // Calculate profit and ROI
        let profit = revenue_per_prover as i64 - total_cost as i64;
        let roi_percentage = if total_cost > 0 {
            (profit as f64 / total_cost as f64) * 100.0
        } else {
            0.0
        };

        let is_profitable = roi_percentage >= self.min_roi_percentage;

        let roi = JobROI {
            revenue_per_prover,
            estimated_cost: total_cost,
            profit,
            roi_percentage,
            complexity_tier: cost_config.complexity_tier,
            timeout_seconds: cost_config.timeout_seconds,
            is_profitable,
        };

        // Log evaluation
        if is_profitable {
            info!(
                "Job is PROFITABLE - Op: {}, Tier: {}, Revenue: {} lamports, Cost: {} lamports, Profit: {} lamports, ROI: {:.1}%",
                fhe_operation.name(),
                roi.complexity_tier,
                revenue_per_prover,
                total_cost,
                profit,
                roi_percentage
            );
        } else {
            warn!(
                "Job is NOT PROFITABLE - Op: {}, Tier: {}, Revenue: {} lamports, Cost: {} lamports, Loss: {} lamports, ROI: {:.1}% (min: {:.1}%)",
                fhe_operation.name(),
                roi.complexity_tier,
                revenue_per_prover,
                total_cost,
                profit,
                roi_percentage,
                self.min_roi_percentage
            );
        }

        roi
    }

    /// Evaluate simple job (non-FHE) with flat pricing
    fn evaluate_simple_job(&self, price_lamports: u64, required_provers: u8) -> JobROI {
        let revenue_per_prover = price_lamports / (required_provers as u64);

        // Use a flat minimum cost for non-FHE jobs (e.g., 0.001 SOL)
        let base_cost = 1_000_000; // 0.001 SOL
        let total_cost = (base_cost as f64 * self.operational_cost_multiplier) as u64;

        let profit = revenue_per_prover as i64 - total_cost as i64;
        let roi_percentage = if total_cost > 0 {
            (profit as f64 / total_cost as f64) * 100.0
        } else {
            0.0
        };

        JobROI {
            revenue_per_prover,
            estimated_cost: total_cost,
            profit,
            roi_percentage,
            complexity_tier: 1,
            timeout_seconds: 60,
            is_profitable: roi_percentage >= self.min_roi_percentage,
        }
    }

    /// Get minimum acceptable price for a given operation
    ///
    /// Useful for setting dynamic minimum prices based on operation complexity
    pub fn get_minimum_price(
        &self,
        circuit_type: &CircuitType,
        required_provers: u8,
    ) -> u64 {
        let fhe_operation = match circuit_type {
            CircuitType::FheComputation(op) => op,
            _ => return 1_000_000 * (required_provers as u64), // 0.001 SOL per prover
        };

        let cost_config = fhe_operation.get_cost_config();
        let base_cost = cost_config.min_payment_lamports;
        let total_cost_per_prover = (base_cost as f64 * self.operational_cost_multiplier) as u64;

        // Calculate minimum price with desired ROI
        let min_revenue = (total_cost_per_prover as f64 * (1.0 + self.min_roi_percentage / 100.0)) as u64;

        min_revenue * (required_provers as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cypherlink_types::fhe::{FheOperation, HistogramBin};

    #[test]
    fn test_profitable_tier1_job() {
        let calculator = ROICalculator::default();
        let circuit = CircuitType::FheComputation(FheOperation::Add(5));

        // 0.003 SOL for 3 provers = 0.001 SOL per prover
        let roi = calculator.evaluate_job(&circuit, 3_000_000, 3);

        assert!(roi.is_profitable);
        assert_eq!(roi.complexity_tier, 1);
        assert!(roi.profit > 0);
    }

    #[test]
    fn test_unprofitable_tier5_job() {
        let calculator = ROICalculator::default();
        let circuit = CircuitType::FheComputation(FheOperation::Histogram {
            bins: vec![
                HistogramBin::new(0, 10, "bin1"),
                HistogramBin::new(11, 20, "bin2"),
                HistogramBin::new(21, 30, "bin3"),
                HistogramBin::new(31, 40, "bin4"),
                HistogramBin::new(41, 50, "bin5"),
            ],
        });

        // Histogram 5 bins should cost ~0.324 SOL per prover
        // But we're only offering 0.1 SOL per prover = not profitable
        let roi = calculator.evaluate_job(&circuit, 300_000_000, 3);

        assert!(!roi.is_profitable);
        assert_eq!(roi.complexity_tier, 5);
        assert!(roi.profit < 0);
    }

    #[test]
    fn test_minimum_price_calculation() {
        let calculator = ROICalculator::default();
        let circuit = CircuitType::FheComputation(FheOperation::Add(5));

        let min_price = calculator.get_minimum_price(&circuit, 3);

        // Should be cost × overhead × (1 + ROI) × provers
        // 0.001 SOL × 1.5 × 1.2 × 3 = 0.0054 SOL = 5_400_000 lamports
        assert!(min_price >= 5_000_000); // At least 0.005 SOL
    }

    #[test]
    fn test_roi_percentage_calculation() {
        let calculator = ROICalculator::new(50.0, 1.0); // 50% min ROI, no overhead
        let circuit = CircuitType::FheComputation(FheOperation::Multiply(10));

        // 0.003 SOL for 3 provers, cost is 0.001 SOL
        // Profit = 0.001 - 0.001 = 0, ROI = 0%
        let roi = calculator.evaluate_job(&circuit, 3_000_000, 3);

        assert!(!roi.is_profitable); // Need 50% ROI
        assert!((roi.roi_percentage - 0.0).abs() < 0.1);
    }
}
