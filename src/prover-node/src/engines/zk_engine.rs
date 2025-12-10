use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// Result of ZK proof generation
#[derive(Debug, Clone)]
pub enum ZkProofResult {
    /// Halo2 proof bytes
    Halo2(Vec<u8>),
    /// Groth16 proof (for future arkworks integration)
    Groth16 {
        proof: Vec<u8>,
        public_inputs: Vec<Vec<u8>>,
    },
}

/// Trait for ZK proof engines
#[async_trait]
pub trait ZkProver: Send + Sync {
    /// Generate a ZK proof for the given circuit and witness
    async fn prove(&self, circuit_id: u8, witness: &[u8]) -> Result<ZkProofResult>;

    /// Verify a proof (optional, for testing)
    async fn verify(
        &self,
        circuit_id: u8,
        proof: &ZkProofResult,
        public_inputs: &[Vec<u8>],
    ) -> Result<bool>;

    /// Check if this prover supports the given circuit
    fn supports_circuit(&self, circuit_id: u8) -> bool;
}

/// ZK Engine that routes to appropriate prover based on circuit type
pub struct ZkEngine {
    halo2_prover: Option<Arc<crate::halo2_prover::Halo2Prover>>,
    // Future: arkworks_prover: Option<Arc<ArkworksProver>>,
}

impl ZkEngine {
    /// Create new ZK engine with optional Halo2 prover
    pub fn new(halo2_prover: Option<Arc<crate::halo2_prover::Halo2Prover>>) -> Self {
        Self { halo2_prover }
    }

    /// Generate proof routing to appropriate engine
    pub async fn generate_proof(&self, circuit_id: u8, witness: &[u8]) -> Result<ZkProofResult> {
        use crate::core::CircuitRegistry;

        match CircuitRegistry::circuit_category(circuit_id) {
            crate::core::CircuitCategory::Halo2Orchard => {
                // Use existing Halo2Prover
                self.generate_halo2_proof(witness).await
            }
            crate::core::CircuitCategory::SnarkjsGroth16 => {
                // Placeholder for future arkworks integration
                Err(anyhow::anyhow!(
                    "Groth16 proofs not yet implemented - awaiting arkworks integration"
                ))
            }
            crate::core::CircuitCategory::FheComputation => Err(anyhow::anyhow!(
                "FHE computations should use FheEngine, not ZkEngine"
            )),
        }
    }

    /// Generate Halo2 proof using existing Halo2Prover
    async fn generate_halo2_proof(&self, witness: &[u8]) -> Result<ZkProofResult> {
        let prover = self
            .halo2_prover
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Halo2Prover not initialized"))?;

        // Deserialize witness and generate proof
        let witness_data: crate::halo2_prover::OrchardWitness =
            bincode::deserialize(witness)
                .map_err(|e| anyhow::anyhow!("Failed to deserialize witness: {}", e))?;

        let proof = prover.generate_orchard_proof(witness_data)?;
        Ok(ZkProofResult::Halo2(proof))
    }

    /// Verify proof (optional, for testing)
    pub async fn verify_proof(
        &self,
        circuit_id: u8,
        proof: &ZkProofResult,
        _public_inputs: &[Vec<u8>],
    ) -> Result<bool> {
        use crate::core::CircuitRegistry;

        match CircuitRegistry::circuit_category(circuit_id) {
            crate::core::CircuitCategory::Halo2Orchard => {
                // Halo2 verification not implemented yet
                // Would require storing verification keys
                Err(anyhow::anyhow!(
                    "Halo2 verification not yet implemented"
                ))
            }
            crate::core::CircuitCategory::SnarkjsGroth16 => Err(anyhow::anyhow!(
                "Groth16 verification not yet implemented - awaiting arkworks integration"
            )),
            crate::core::CircuitCategory::FheComputation => Err(anyhow::anyhow!(
                "FHE computations should use FheEngine, not ZkEngine"
            )),
        }
    }

    /// Check if this engine supports the given circuit
    pub fn supports_circuit(&self, circuit_id: u8) -> bool {
        use crate::core::CircuitRegistry;

        match CircuitRegistry::circuit_category(circuit_id) {
            crate::core::CircuitCategory::Halo2Orchard => self.halo2_prover.is_some(),
            crate::core::CircuitCategory::SnarkjsGroth16 => false, // Not yet implemented
            crate::core::CircuitCategory::FheComputation => false, // Use FheEngine instead
        }
    }
}

// ============================================================================
// Placeholder for future Arkworks Groth16 integration
// ============================================================================

/// Placeholder for future Arkworks Groth16 integration
/// Will be implemented when circuits are ready
#[allow(dead_code)]
pub struct ArkworksProver {
    circuits_path: std::path::PathBuf,
    // Future fields:
    // verification_keys: HashMap<u8, ark_groth16::VerifyingKey<Bn254>>,
    // proving_keys: HashMap<u8, ark_groth16::ProvingKey<Bn254>>,
}

#[allow(dead_code)]
impl ArkworksProver {
    /// Create new Arkworks prover with circuits directory
    pub fn new(circuits_path: std::path::PathBuf) -> Self {
        Self { circuits_path }
    }

    /// Placeholder - will use arkworks-groth16 when ready
    pub async fn prove(&self, _circuit_id: u8, _witness: &[u8]) -> Result<ZkProofResult> {
        Err(anyhow::anyhow!(
            "ArkworksProver not yet implemented - waiting for arkworks integration"
        ))
    }

    /// Placeholder - will verify Groth16 proofs when ready
    pub async fn verify(
        &self,
        _circuit_id: u8,
        _proof: &ZkProofResult,
        _public_inputs: &[Vec<u8>],
    ) -> Result<bool> {
        Err(anyhow::anyhow!(
            "ArkworksProver verification not yet implemented"
        ))
    }

    /// Check if this prover supports the given circuit
    pub fn supports_circuit(&self, circuit_id: u8) -> bool {
        // Will support circuits 10-49 when implemented
        circuit_id >= 10 && circuit_id <= 49
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::halo2_prover::{Halo2Prover, OrchardWitness};

    #[tokio::test]
    async fn test_zk_engine_creation() {
        // Test creating engine without Halo2 prover
        let engine = ZkEngine::new(None);
        assert!(!engine.supports_circuit(0));

        // Test creating engine with Halo2 prover
        let halo2_prover = Halo2Prover::new().expect("Failed to create Halo2Prover");
        let engine = ZkEngine::new(Some(Arc::new(halo2_prover)));
        assert!(engine.supports_circuit(0));
        assert!(engine.supports_circuit(3));
        assert!(!engine.supports_circuit(10));
    }

    #[tokio::test]
    async fn test_zk_engine_routing() {
        // Create engine without provers
        let engine = ZkEngine::new(None);

        // Test that Halo2 circuits fail without prover
        let witness = bincode::serialize(&OrchardWitness::dummy()).unwrap();
        let result = engine.generate_proof(0, &witness).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Halo2Prover not initialized"));

        // Test that Groth16 circuits return proper error
        let result = engine.generate_proof(10, &witness).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not yet implemented"));

        // Test that FHE circuits return proper error
        let result = engine.generate_proof(4, &witness).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("FheEngine"));
    }

    #[tokio::test]
    async fn test_halo2_proof_generation() {
        // Create and setup Halo2 prover
        let mut halo2_prover = Halo2Prover::new().expect("Failed to create Halo2Prover");
        halo2_prover.setup().expect("Failed to setup Halo2Prover");

        // Create engine with Halo2 prover
        let engine = ZkEngine::new(Some(Arc::new(halo2_prover)));

        // Generate proof
        let witness = bincode::serialize(&OrchardWitness::dummy()).unwrap();
        let result = engine.generate_proof(0, &witness).await;

        assert!(result.is_ok(), "Proof generation should succeed");

        let proof_result = result.unwrap();
        match proof_result {
            ZkProofResult::Halo2(proof) => {
                assert!(!proof.is_empty(), "Proof should not be empty");
                assert!(proof.len() >= 500, "Proof should be at least 500 bytes");
            }
            _ => panic!("Expected Halo2 proof result"),
        }
    }

    #[test]
    fn test_arkworks_prover_placeholder() {
        let prover = ArkworksProver::new(std::path::PathBuf::from("/tmp/circuits"));

        // Test that it reports support for Groth16 circuits
        assert!(!prover.supports_circuit(0));
        assert!(!prover.supports_circuit(9));
        assert!(prover.supports_circuit(10));
        assert!(prover.supports_circuit(49));
        assert!(!prover.supports_circuit(50));
    }
}
