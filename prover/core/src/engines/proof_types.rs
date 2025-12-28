/// Types of proofs the prover can generate
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofType {
    /// Halo2 Orchard proofs (circuits 0-3)
    Halo2Orchard,
    /// FHE computation proofs (circuits 4-11)
    FheComputation,
    /// Snarkjs Groth16 proofs (circuits 10-49)
    SnarkjsGroth16,
}

impl ProofType {
    /// Get proof type from circuit ID
    pub fn from_circuit_id(circuit_id: u8) -> Self {
        match circuit_id {
            0..=3 => ProofType::Halo2Orchard,
            4..=9 => ProofType::FheComputation,
            10..=49 => ProofType::SnarkjsGroth16,
            _ => ProofType::SnarkjsGroth16, // Default for unknown circuits
        }
    }
}
