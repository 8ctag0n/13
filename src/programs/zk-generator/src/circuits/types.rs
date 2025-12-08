//! Circuit type definitions for ZK proof generation
//!
//! This module defines all supported circuit types for the ZyberLink protocol.
//! Circuit types are used to route proof verification to the appropriate verifier.

use borsh::{BorshDeserialize, BorshSerialize};

/// Supported circuit types for ZK proof generation
///
/// Circuit IDs are grouped by category:
/// - 0-9: Legacy circuits (v1.x compatibility)
/// - 10-19: Core privacy primitives
/// - 20-29: Voting circuits
/// - 30-39: Market circuits
/// - 40-49: Portfolio circuits
/// - 50+: Future extensions
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[borsh(use_discriminant = true)]
#[repr(u8)]
pub enum CircuitType {
    // Legacy circuits (v1.x) - maintain backwards compatibility
    ZcashOrchard = 0,
    ZcashSapling = 1,
    AnonymousVote = 2,
    Credential = 3,

    // Core privacy primitives (v2.0)
    ProofOfInnocence = 10,

    // Voting circuits (v2.0)
    PrivateVote = 20,
    PrivateVoteWithPoI = 21,

    // Market circuits (v2.0)
    MarketBet = 30,
    MarketBetWithPoI = 31,
    MarketClaim = 32,

    // Portfolio circuits (v2.0)
    PortfolioCompliance = 40,
    PortfolioNetWorth = 41,

    // Future extensions
    FutarchyConditional = 50,
}

impl Default for CircuitType {
    fn default() -> Self {
        CircuitType::ProofOfInnocence
    }
}

impl CircuitType {
    /// Convert from u8 to CircuitType
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(CircuitType::ZcashOrchard),
            1 => Some(CircuitType::ZcashSapling),
            2 => Some(CircuitType::AnonymousVote),
            3 => Some(CircuitType::Credential),
            10 => Some(CircuitType::ProofOfInnocence),
            20 => Some(CircuitType::PrivateVote),
            21 => Some(CircuitType::PrivateVoteWithPoI),
            30 => Some(CircuitType::MarketBet),
            31 => Some(CircuitType::MarketBetWithPoI),
            32 => Some(CircuitType::MarketClaim),
            40 => Some(CircuitType::PortfolioCompliance),
            41 => Some(CircuitType::PortfolioNetWorth),
            50 => Some(CircuitType::FutarchyConditional),
            _ => None,
        }
    }

    /// Convert to u8
    pub fn to_u8(self) -> u8 {
        self as u8
    }

    /// Returns the expected public inputs count for this circuit type
    pub fn expected_public_inputs(&self) -> usize {
        match self {
            // Legacy circuits
            CircuitType::ZcashOrchard => 4,
            CircuitType::ZcashSapling => 4,
            CircuitType::AnonymousVote => 4,
            CircuitType::Credential => 3,

            // Core primitives
            CircuitType::ProofOfInnocence => 3, // blacklist_root, threshold, timestamp

            // Voting
            CircuitType::PrivateVote => 4,      // poll_id, vote_commitment, eligibility_root, nullifier
            CircuitType::PrivateVoteWithPoI => 5, // + blacklist_root

            // Markets
            CircuitType::MarketBet => 4,        // market_id, bet_commitment, balance_proof, position
            CircuitType::MarketBetWithPoI => 5, // + blacklist_root
            CircuitType::MarketClaim => 3,      // market_id, outcome, claim_commitment

            // Portfolio
            CircuitType::PortfolioCompliance => 3, // compliance_root, threshold, timestamp
            CircuitType::PortfolioNetWorth => 3,   // min_threshold, timestamp, commitment

            // Future
            CircuitType::FutarchyConditional => 5,
        }
    }

    /// Returns the expected proof size in bytes for this circuit type
    pub fn expected_proof_size(&self) -> usize {
        match self {
            // Groth16 proofs are 256 bytes (2 G1 points + 1 G2 point)
            CircuitType::ZcashOrchard
            | CircuitType::ZcashSapling
            | CircuitType::ProofOfInnocence
            | CircuitType::PrivateVote
            | CircuitType::PrivateVoteWithPoI
            | CircuitType::MarketBet
            | CircuitType::MarketBetWithPoI
            | CircuitType::MarketClaim
            | CircuitType::PortfolioCompliance
            | CircuitType::PortfolioNetWorth => 256,

            // Legacy circuits may have different sizes
            CircuitType::AnonymousVote | CircuitType::Credential => 256,

            // PLONK proofs (future) would be larger
            CircuitType::FutarchyConditional => 512,
        }
    }

    /// Returns the circuit name for logging/display
    pub fn name(&self) -> &'static str {
        match self {
            CircuitType::ZcashOrchard => "zcash_orchard",
            CircuitType::ZcashSapling => "zcash_sapling",
            CircuitType::AnonymousVote => "anonymous_vote",
            CircuitType::Credential => "credential",
            CircuitType::ProofOfInnocence => "proof_of_innocence",
            CircuitType::PrivateVote => "private_vote",
            CircuitType::PrivateVoteWithPoI => "private_vote_poi",
            CircuitType::MarketBet => "market_bet",
            CircuitType::MarketBetWithPoI => "market_bet_poi",
            CircuitType::MarketClaim => "market_claim",
            CircuitType::PortfolioCompliance => "portfolio_compliance",
            CircuitType::PortfolioNetWorth => "portfolio_net_worth",
            CircuitType::FutarchyConditional => "futarchy_conditional",
        }
    }

    /// Check if this is a v2.0 circuit type
    pub fn is_v2(&self) -> bool {
        (*self as u8) >= 10
    }

    /// Check if this circuit requires PoI integration
    pub fn requires_poi(&self) -> bool {
        matches!(
            self,
            CircuitType::PrivateVoteWithPoI
                | CircuitType::MarketBetWithPoI
                | CircuitType::PortfolioCompliance
        )
    }
}

/// Check if a u8 value represents a valid circuit type
pub fn is_valid_circuit_type(value: u8) -> bool {
    CircuitType::from_u8(value).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_type_roundtrip() {
        let circuits = [
            CircuitType::ZcashOrchard,
            CircuitType::ProofOfInnocence,
            CircuitType::PrivateVote,
            CircuitType::MarketBet,
        ];

        for circuit in circuits {
            let value = circuit.to_u8();
            let recovered = CircuitType::from_u8(value).unwrap();
            assert_eq!(circuit, recovered);
        }
    }

    #[test]
    fn test_invalid_circuit_type() {
        assert!(CircuitType::from_u8(255).is_none());
        assert!(CircuitType::from_u8(100).is_none());
    }
}
