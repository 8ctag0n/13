use serde_json::Value;
use std::fs;
use std::path::PathBuf;

use ark_bn254::Fr;
use std::str::FromStr;

use zyberlink_sdk::{load_vkey_json, CircuitId, prepare_vk, proof_from_json, verify, vk_from_json};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from("../../circuits/poi").join(name)
}

#[test]
fn verify_poi_proof_ok() {
    let proof: Value = serde_json::from_str(&fs::read_to_string(fixture("proof.json")).unwrap()).unwrap();
    let public: Vec<String> =
        serde_json::from_str(&fs::read_to_string(fixture("public.json")).unwrap()).unwrap();

    let vk_json = load_vkey_json(CircuitId::ProofOfInnocence).unwrap();
    let vk = vk_from_json(&vk_json).unwrap();
    let pvk = prepare_vk(&vk);
    let proof_parsed = proof_from_json(&proof).unwrap();
    let public_fr: Vec<Fr> = public.iter().map(|s| Fr::from_str(s).unwrap()).collect();

    assert!(verify(&proof_parsed, &public_fr, &pvk));
}

#[test]
fn verify_poi_proof_fails_with_tamper() {
    let proof: Value = serde_json::from_str(&fs::read_to_string(fixture("proof.json")).unwrap()).unwrap();
    let mut public: Vec<String> =
        serde_json::from_str(&fs::read_to_string(fixture("public.json")).unwrap()).unwrap();
    public[0] = "0".to_string(); // tamper

    let vk_json = load_vkey_json(CircuitId::ProofOfInnocence).unwrap();
    let vk = vk_from_json(&vk_json).unwrap();
    let pvk = prepare_vk(&vk);
    let proof_parsed = proof_from_json(&proof).unwrap();
    let public_fr: Vec<Fr> = public.iter().map(|s| Fr::from_str(s).unwrap()).collect();

    assert!(!verify(&proof_parsed, &public_fr, &pvk));
}
