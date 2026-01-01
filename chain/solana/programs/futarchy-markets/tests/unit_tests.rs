//! Unit tests using Mollusk for ultra-fast testing
//!
//! These tests run in native mode without BPF, making them extremely fast.
//! Perfect for TDD and CI/CD pipelines.
//!
//! Note: Processor tests are in integration_tests.rs using solana-program-test
//! because they require CPIs which Mollusk doesn't support well yet.

mod unit {
    mod state {
        mod executable_action_test;
        mod market_test;
        mod position_test;
        mod user_eligibility_test;
    }
}
