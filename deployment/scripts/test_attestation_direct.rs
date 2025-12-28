// Direct test of AttestationService with real proof
// Run with: cargo run --example test_attestation_direct

use std::path::PathBuf;

// Include the attestation service code
mod attestation_test {
    use std::fs;
    use std::path::Path;

    pub fn test_real_proof() {
        let project_root = "/home/deploy/2025q4/13-area2-provers";
        let vk_path = format!("{}/circuits/verification_keys/circuit_10_vkey.json", project_root);
        let proof_path = format!("{}/circuits/poi/proof_e2e.json", project_root);
        let public_path = format!("{}/circuits/poi/public_e2e.json", project_root);

        println!("Loading VK from: {}", vk_path);
        println!("Loading proof from: {}", proof_path);
        println!("Loading public inputs from: {}", public_path);

        // Read files
        let vk_json = fs::read_to_string(&vk_path).expect("Failed to read VK");
        let proof_json = fs::read_to_string(&proof_path).expect("Failed to read proof");
        let public_json = fs::read_to_string(&public_path).expect("Failed to read public inputs");

        println!("\nVK loaded: {} bytes", vk_json.len());
        println!("Proof loaded: {} bytes", proof_json.len());
        println!("Public inputs loaded: {} bytes", public_json.len());

        println!("\n✓ All files loaded successfully");
        println!("\nTo complete E2E verification, run the blink-server tests:");
        println!("  cargo test test_verify_real --release -- --nocapture");
    }
}

fn main() {
    attestation_test::test_real_proof();
}
