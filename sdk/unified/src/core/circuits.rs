//! Circuit types for ZK and FHE jobs
//!
//! This module provides metadata and utilities for all supported circuit types.

pub use zyberlink_types::CircuitType;

// ============================================================================
// ZK Circuits (0-3 legacy, 10-41 v2.0)
// ============================================================================

// Legacy ZK circuits (0-3)
pub const CIRCUIT_ZCASH_ORCHARD: u8 = 0;
pub const CIRCUIT_ZCASH_SAPLING: u8 = 1;
pub const CIRCUIT_ANONYMOUS_VOTE: u8 = 2;
pub const CIRCUIT_CREDENTIAL: u8 = 3;

// v2.0 Core ZK circuits
// NOTE: Circuit ID 10 is shared with FHE_COUNT_IF, but they're in different programs
pub const CIRCUIT_PROOF_OF_INNOCENCE: u8 = 10;

// v2.0 Voting circuits (20-29)
pub const CIRCUIT_PRIVATE_VOTE: u8 = 20;
pub const CIRCUIT_PRIVATE_VOTE_POI: u8 = 21;

// v2.0 Market circuits (30-39)
pub const CIRCUIT_MARKET_BET: u8 = 30;
pub const CIRCUIT_MARKET_BET_POI: u8 = 31;
pub const CIRCUIT_MARKET_CLAIM: u8 = 32;

// v2.0 Portfolio circuits (40-49)
pub const CIRCUIT_PORTFOLIO_COMPLIANCE: u8 = 40;
pub const CIRCUIT_PORTFOLIO_NET_WORTH: u8 = 41;

// ============================================================================
// FHE Circuits (4-11)
// ============================================================================

pub const CIRCUIT_FHE_ADD: u8 = 4;
pub const CIRCUIT_FHE_MULTIPLY: u8 = 5;
pub const CIRCUIT_FHE_SUM: u8 = 6;
pub const CIRCUIT_FHE_THRESHOLD: u8 = 7;
pub const CIRCUIT_FHE_RANGE_CHECK: u8 = 8;
pub const CIRCUIT_FHE_AVERAGE: u8 = 9;
pub const CIRCUIT_FHE_COUNT_IF: u8 = 10;
pub const CIRCUIT_FHE_HISTOGRAM: u8 = 11;

// Special FHE circuit for Futarchy pool updates
pub const CIRCUIT_FHE_FUTARCHY_POOL_UPDATE: u8 = 12;

// ============================================================================
// Circuit Metadata
// ============================================================================

/// Get human-readable name for a circuit type
pub fn circuit_name(circuit_type: u8) -> &'static str {
    match circuit_type {
        // ZK Legacy
        CIRCUIT_ZCASH_ORCHARD => "Zcash Orchard",
        CIRCUIT_ZCASH_SAPLING => "Zcash Sapling",
        CIRCUIT_ANONYMOUS_VOTE => "Anonymous Vote",
        CIRCUIT_CREDENTIAL => "Credential",

        // ZK v2.0 Core
        CIRCUIT_PROOF_OF_INNOCENCE => "Proof of Innocence",

        // ZK v2.0 Voting
        CIRCUIT_PRIVATE_VOTE => "Private Vote",
        CIRCUIT_PRIVATE_VOTE_POI => "Private Vote with PoI",

        // ZK v2.0 Market
        CIRCUIT_MARKET_BET => "Market Bet",
        CIRCUIT_MARKET_BET_POI => "Market Bet with PoI",
        CIRCUIT_MARKET_CLAIM => "Market Claim",

        // ZK v2.0 Portfolio
        CIRCUIT_PORTFOLIO_COMPLIANCE => "Portfolio Compliance",
        CIRCUIT_PORTFOLIO_NET_WORTH => "Portfolio Net Worth",

        // FHE
        CIRCUIT_FHE_ADD => "FHE Add",
        CIRCUIT_FHE_MULTIPLY => "FHE Multiply",
        CIRCUIT_FHE_SUM => "FHE Sum",
        CIRCUIT_FHE_THRESHOLD => "FHE Threshold",
        CIRCUIT_FHE_RANGE_CHECK => "FHE Range Check",
        CIRCUIT_FHE_AVERAGE => "FHE Average",
        CIRCUIT_FHE_COUNT_IF => "FHE Count If",
        CIRCUIT_FHE_HISTOGRAM => "FHE Histogram",
        CIRCUIT_FHE_FUTARCHY_POOL_UPDATE => "FHE Futarchy Pool Update",

        _ => "Unknown Circuit",
    }
}

/// Check if circuit type is ZK (legacy or v2.0)
pub fn is_zk_circuit(circuit_type: u8) -> bool {
    (circuit_type <= 3) || (circuit_type >= 10 && circuit_type <= 41)
}

/// Check if circuit type is FHE
pub fn is_fhe_circuit(circuit_type: u8) -> bool {
    circuit_type >= 4 && circuit_type <= 12
}

/// Check if circuit type is legacy (0-3)
pub fn is_legacy_circuit(circuit_type: u8) -> bool {
    circuit_type <= 3
}

/// Check if circuit type is v2.0 (10-41)
pub fn is_v2_circuit(circuit_type: u8) -> bool {
    circuit_type >= 10 && circuit_type <= 41
}

/// Check if circuit requires Proof of Innocence verification
pub fn requires_poi(circuit_type: u8) -> bool {
    matches!(
        circuit_type,
        CIRCUIT_PRIVATE_VOTE_POI | CIRCUIT_MARKET_BET_POI
    )
}

/// Get circuit category
pub enum CircuitCategory {
    ZkLegacy,
    ZkV2Core,
    ZkV2Voting,
    ZkV2Market,
    ZkV2Portfolio,
    Fhe,
    Unknown,
}

pub fn circuit_category(circuit_type: u8) -> CircuitCategory {
    match circuit_type {
        0..=3 => CircuitCategory::ZkLegacy,
        10 => CircuitCategory::ZkV2Core,
        20..=29 => CircuitCategory::ZkV2Voting,
        30..=39 => CircuitCategory::ZkV2Market,
        40..=49 => CircuitCategory::ZkV2Portfolio,
        4..=12 => CircuitCategory::Fhe,
        _ => CircuitCategory::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_classification() {
        // ZK legacy
        assert!(is_zk_circuit(0));
        assert!(is_zk_circuit(3));
        assert!(is_legacy_circuit(0));
        assert!(!is_fhe_circuit(0));

        // FHE
        assert!(is_fhe_circuit(4));
        assert!(is_fhe_circuit(11));
        assert!(!is_zk_circuit(4));
        assert!(!is_legacy_circuit(4));

        // ZK v2.0
        assert!(is_zk_circuit(10));
        assert!(is_zk_circuit(30));
        assert!(is_v2_circuit(10));
        assert!(!is_fhe_circuit(10));
        assert!(!is_legacy_circuit(10));
    }

    #[test]
    fn test_poi_requirement() {
        assert!(requires_poi(CIRCUIT_PRIVATE_VOTE_POI));
        assert!(requires_poi(CIRCUIT_MARKET_BET_POI));
        assert!(!requires_poi(CIRCUIT_PRIVATE_VOTE));
        assert!(!requires_poi(CIRCUIT_MARKET_BET));
    }

    #[test]
    fn test_circuit_names() {
        assert_eq!(circuit_name(0), "Zcash Orchard");
        assert_eq!(circuit_name(4), "FHE Add");
        assert_eq!(circuit_name(10), "Proof of Innocence");
        assert_eq!(circuit_name(30), "Market Bet");
        assert_eq!(circuit_name(255), "Unknown Circuit");
    }
}
