use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

/// Type of ZK circuit/proof being requested
#[derive(
    Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize,
)]
pub enum CircuitType {
    /// Zcash Orchard shielded transaction proof
    ZcashOrchard,

    /// Anonymous voting proof
    AnonymousVote,

    /// Private credential proof
    Credential,

    /// Custom circuit type (for future extensibility)
    Custom(String),
}

impl CircuitType {
    /// Get a human-readable name for this circuit type
    pub fn name(&self) -> &str {
        match self {
            CircuitType::ZcashOrchard => "Zcash Orchard",
            CircuitType::AnonymousVote => "Anonymous Vote",
            CircuitType::Credential => "Credential",
            CircuitType::Custom(name) => name,
        }
    }

    /// Estimate typical proving time in seconds for this circuit
    /// These are rough estimates for a mid-range desktop CPU
    pub fn estimated_proving_time_secs(&self) -> u32 {
        match self {
            CircuitType::ZcashOrchard => 15,  // 10-15 seconds typical
            CircuitType::AnonymousVote => 5,  // Simpler circuit
            CircuitType::Credential => 8,     // Medium complexity
            CircuitType::Custom(_) => 20,     // Conservative estimate
        }
    }

    /// Estimate typical witness size in bytes
    pub fn estimated_witness_size_bytes(&self) -> usize {
        match self {
            CircuitType::ZcashOrchard => 2048,   // ~2KB
            CircuitType::AnonymousVote => 512,   // ~512B
            CircuitType::Credential => 1024,     // ~1KB
            CircuitType::Custom(_) => 4096,      // ~4KB conservative
        }
    }
}

impl Default for CircuitType {
    fn default() -> Self {
        Self::ZcashOrchard
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_type_names() {
        assert_eq!(CircuitType::ZcashOrchard.name(), "Zcash Orchard");
        assert_eq!(CircuitType::AnonymousVote.name(), "Anonymous Vote");
        assert_eq!(CircuitType::Credential.name(), "Credential");
        assert_eq!(CircuitType::Custom("MyCircuit".to_string()).name(), "MyCircuit");
    }

    #[test]
    fn test_circuit_serialization() {
        let circuit = CircuitType::ZcashOrchard;
        let serialized = borsh::to_vec(&circuit).unwrap();
        let deserialized: CircuitType = borsh::from_slice(&serialized).unwrap();
        assert_eq!(circuit, deserialized);
    }

    #[test]
    fn test_circuit_custom_serialization() {
        let circuit = CircuitType::Custom("TestCircuit".to_string());
        let serialized = borsh::to_vec(&circuit).unwrap();
        let deserialized: CircuitType = borsh::from_slice(&serialized).unwrap();
        assert_eq!(circuit, deserialized);
    }

    #[test]
    fn test_circuit_estimates() {
        assert!(CircuitType::ZcashOrchard.estimated_proving_time_secs() > 0);
        assert!(CircuitType::ZcashOrchard.estimated_witness_size_bytes() > 0);
    }
}
