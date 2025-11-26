use prover_node::halo2_prover::OrchardWitness;

/// Create a valid test witness for testing
pub fn create_test_witness() -> OrchardWitness {
    OrchardWitness {
        spend_auth_sig: [42u8; 64],
        note_value: 1_000_000,
        note_rho: [1u8; 32],
        note_rseed: [2u8; 32],
        merkle_path: vec![[0u8; 32]; 32],
        merkle_position: 12345,
        recipient_address: [3u8; 43],
        output_value: 900_000,
        rcv: [4u8; 32],
    }
}

/// Create an invalid witness (zero value) for negative testing
#[allow(dead_code)]
pub fn create_invalid_witness() -> OrchardWitness {
    let mut witness = create_test_witness();
    witness.note_value = 0; // Invalid: note value cannot be zero
    witness
}
