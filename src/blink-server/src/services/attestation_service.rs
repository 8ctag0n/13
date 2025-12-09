//! Attestation Service for ZK Proof Verification
//!
//! Verifies Groth16 proofs using arkworks and generates attestation witnesses
//! for potential on-chain dispute resolution.

use anyhow::{anyhow, Result};
use ark_bn254::{Bn254, Fq, Fq2, Fr, G1Affine, G1Projective, G2Affine, G2Projective};
use ark_ec::AffineRepr;
use ark_ff::BigInteger256;
use ark_groth16::{Groth16, PreparedVerifyingKey, Proof, VerifyingKey};
use ark_snark::SNARK;
use num_bigint::BigUint;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;
use std::time::Instant;

// =============================================================================
// Data Structures
// =============================================================================

/// Attestation service for verifying Groth16 proofs
pub struct AttestationService {
    /// Cache of prepared verification keys by circuit_type
    prepared_vks: HashMap<u8, PreparedVerifyingKey<Bn254>>,
    /// VK hashes for attestation
    vk_hashes: HashMap<u8, String>,
}

/// Result of proof verification and attestation
#[derive(Debug, Serialize, Deserialize)]
pub struct AttestationResult {
    /// Whether the proof is valid
    pub valid: bool,
    /// Verification time in milliseconds
    pub verification_time_ms: u64,
    /// Attestation witness (only if valid)
    pub witness: Option<AttestationWitness>,
}

/// Attestation witness that can be used to regenerate verification on-chain
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AttestationWitness {
    /// Proof point A (G1)
    pub proof_a: [String; 2],
    /// Proof point B (G2)
    pub proof_b: [[String; 2]; 2],
    /// Proof point C (G1)
    pub proof_c: [String; 2],
    /// Verification key hash (for identifying VK)
    pub vk_hash: String,
    /// Public inputs as hex strings
    pub public_inputs: Vec<String>,
    /// Hash of public inputs
    pub public_inputs_hash: String,
    /// Verification result (0 = invalid, 1 = valid)
    pub verification_result: u8,
}

/// Proof in snarkjs JSON format
#[derive(Debug, Deserialize)]
pub struct SnarkjsProof {
    pub pi_a: [String; 3],
    pub pi_b: [[String; 2]; 3],
    pub pi_c: [String; 3],
    pub protocol: String,
    pub curve: String,
}

/// Verification key in snarkjs JSON format
#[derive(Debug, Deserialize)]
pub struct SnarkjsVKey {
    pub protocol: String,
    pub curve: String,
    #[serde(rename = "nPublic")]
    pub n_public: usize,
    pub vk_alpha_1: [String; 3],
    pub vk_beta_2: [[String; 2]; 3],
    pub vk_gamma_2: [[String; 2]; 3],
    pub vk_delta_2: [[String; 2]; 3],
    #[serde(rename = "vk_alphabeta_12")]
    pub vk_alphabeta_12: Vec<Vec<Vec<String>>>,
    #[serde(rename = "IC")]
    pub ic: Vec<[String; 3]>,
}

// =============================================================================
// Implementation
// =============================================================================

impl AttestationService {
    /// Create new attestation service by loading verification keys from directory
    pub fn new(vk_directory: &Path) -> Result<Self> {
        log::info!("Initializing AttestationService from: {:?}", vk_directory);

        let mut prepared_vks = HashMap::new();
        let mut vk_hashes = HashMap::new();

        // Load verification keys for each circuit type
        // Circuit types: 10-19 (Core), 20-29 (Voting), 30-39 (Market), 40-49 (Portfolio)
        for circuit_type in 10..=49 {
            let vk_path = vk_directory.join(format!("circuit_{}_vkey.json", circuit_type));

            if vk_path.exists() {
                match Self::load_verification_key(&vk_path) {
                    Ok((vk, vk_hash)) => {
                        // Prepare VK for faster verification
                        let prepared_vk = Groth16::<Bn254>::process_vk(&vk)
                            .map_err(|e| anyhow!("Failed to prepare VK for circuit {}: {:?}", circuit_type, e))?;

                        prepared_vks.insert(circuit_type as u8, prepared_vk);
                        vk_hashes.insert(circuit_type as u8, vk_hash);

                        log::info!("Loaded VK for circuit_type {}", circuit_type);
                    }
                    Err(e) => {
                        log::warn!("Failed to load VK for circuit_type {}: {}", circuit_type, e);
                    }
                }
            }
        }

        if prepared_vks.is_empty() {
            log::warn!("No verification keys loaded! Service will reject all proofs.");
        }

        Ok(Self {
            prepared_vks,
            vk_hashes,
        })
    }

    /// Load verification key from snarkjs JSON file
    fn load_verification_key(vk_path: &Path) -> Result<(VerifyingKey<Bn254>, String)> {
        let vk_json = std::fs::read_to_string(vk_path)
            .map_err(|e| anyhow!("Failed to read VK file: {}", e))?;

        let vk: SnarkjsVKey = serde_json::from_str(&vk_json)
            .map_err(|e| anyhow!("Failed to parse VK JSON: {}", e))?;

        // Validate protocol
        if vk.protocol != "groth16" {
            return Err(anyhow!("Unsupported protocol: {}", vk.protocol));
        }

        if vk.curve != "bn128" {
            return Err(anyhow!("Unsupported curve: {}", vk.curve));
        }

        // Calculate VK hash (Keccak256 of serialized VK)
        let vk_hash = Self::hash_vk(&vk_json);

        // Parse VK manually from snarkjs format
        let ark_vk = Self::parse_verification_key(&vk)?;

        Ok((ark_vk, vk_hash))
    }

    /// Parse verification key from snarkjs format to arkworks format
    fn parse_verification_key(vk: &SnarkjsVKey) -> Result<VerifyingKey<Bn254>> {
        // Parse alpha_g1
        let alpha_g1 = Self::parse_g1_point(&vk.vk_alpha_1)?;

        // Parse beta_g2
        let beta_g2 = Self::parse_g2_point(&vk.vk_beta_2)?;

        // Parse gamma_g2
        let gamma_g2 = Self::parse_g2_point(&vk.vk_gamma_2)?;

        // Parse delta_g2
        let delta_g2 = Self::parse_g2_point(&vk.vk_delta_2)?;

        // Parse IC (gamma_abc_g1)
        let gamma_abc_g1: Result<Vec<_>> = vk.ic.iter()
            .map(|point| Self::parse_g1_point(point))
            .collect();
        let gamma_abc_g1 = gamma_abc_g1?;

        Ok(VerifyingKey {
            alpha_g1,
            beta_g2,
            gamma_g2,
            delta_g2,
            gamma_abc_g1,
        })
    }

    /// Parse G1 point from snarkjs format [x, y, z] using projective coordinates
    fn parse_g1_point(coords: &[String; 3]) -> Result<G1Affine> {
        let x = Self::fq_from_str(&coords[0])?;
        let y = Self::fq_from_str(&coords[1])?;
        let z = Self::fq_from_str(&coords[2])?;
        Ok(G1Affine::from(G1Projective::new(x, y, z)))
    }

    /// Parse G2 point from snarkjs format [[x0, x1], [y0, y1], [z0, z1]] using projective coordinates
    fn parse_g2_point(coords: &[[String; 2]; 3]) -> Result<G2Affine> {
        let x = Fq2::new(
            Self::fq_from_str(&coords[0][0])?,
            Self::fq_from_str(&coords[0][1])?,
        );
        let y = Fq2::new(
            Self::fq_from_str(&coords[1][0])?,
            Self::fq_from_str(&coords[1][1])?,
        );
        let z = Fq2::new(
            Self::fq_from_str(&coords[2][0])?,
            Self::fq_from_str(&coords[2][1])?,
        );
        Ok(G2Affine::from(G2Projective::new(x, y, z)))
    }

    /// Convert decimal string to Fq field element (matching snarkjs format)
    fn fq_from_str(s: &str) -> Result<Fq> {
        let big = BigUint::from_str(s)
            .map_err(|e| anyhow!("Failed to parse BigUint: {}", e))?;
        let big_int: BigInteger256 = big.try_into()
            .map_err(|_| anyhow!("BigUint too large for BigInteger256"))?;
        Ok(big_int.into())
    }

    /// Hash verification key using Keccak256
    fn hash_vk(vk_json: &str) -> String {
        let mut hasher = Keccak256::new();
        hasher.update(vk_json.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Verify a proof and generate attestation witness
    pub async fn verify_and_attest(
        &self,
        circuit_type: u8,
        proof_json: &str,
        public_inputs: &[String],
    ) -> Result<AttestationResult> {
        let start = Instant::now();

        // Get prepared VK for circuit type
        let prepared_vk = self
            .prepared_vks
            .get(&circuit_type)
            .ok_or_else(|| anyhow!("No verification key for circuit_type {}", circuit_type))?;

        let vk_hash = self
            .vk_hashes
            .get(&circuit_type)
            .ok_or_else(|| anyhow!("No VK hash for circuit_type {}", circuit_type))?
            .clone();

        // Parse proof from JSON
        let proof = Self::parse_proof(proof_json)?;

        // Parse public inputs
        let public_inputs_fr = Self::parse_public_inputs(public_inputs)?;

        // Verify the proof (CPU-intensive, run in blocking task)
        let proof_clone = proof.clone();
        let prepared_vk_clone = prepared_vk.clone();
        let public_inputs_clone = public_inputs_fr.clone();

        let valid = tokio::task::spawn_blocking(move || {
            let result = Groth16::<Bn254>::verify_with_processed_vk(
                &prepared_vk_clone,
                &public_inputs_clone,
                &proof_clone,
            );
            log::info!("Groth16 verify result: {:?}", result);
            result
        })
        .await
        .map_err(|e| anyhow!("Verification task failed: {}", e))?
        .unwrap_or(false);

        let verification_time_ms = start.elapsed().as_millis() as u64;

        // Generate attestation witness
        let witness = if valid {
            Some(Self::generate_witness(
                &proof,
                public_inputs,
                &vk_hash,
                true,
            )?)
        } else {
            None
        };

        Ok(AttestationResult {
            valid,
            verification_time_ms,
            witness,
        })
    }

    /// Parse proof from snarkjs JSON format
    fn parse_proof(proof_json: &str) -> Result<Proof<Bn254>> {
        let snarkjs_proof: SnarkjsProof = serde_json::from_str(proof_json)
            .map_err(|e| anyhow!("Failed to parse proof JSON: {}", e))?;

        // Validate protocol
        if snarkjs_proof.protocol != "groth16" {
            return Err(anyhow!("Unsupported protocol: {}", snarkjs_proof.protocol));
        }

        // Convert snarkjs format to ark format
        // Note: snarkjs uses 3-element arrays [x, y, z] where z should be "1"

        // Parse A point (G1)
        let a = Self::parse_g1_point(&snarkjs_proof.pi_a)?;

        // Parse B point (G2)
        let b = Self::parse_g2_point(&snarkjs_proof.pi_b)?;

        // Parse C point (G1)
        let c = Self::parse_g1_point(&snarkjs_proof.pi_c)?;

        Ok(Proof { a, b, c })
    }

    /// Parse public inputs from decimal strings to Fr field elements
    /// Uses Fr::from_str which handles decimal strings correctly
    fn parse_public_inputs(inputs: &[String]) -> Result<Vec<Fr>> {
        inputs
            .iter()
            .map(|s| {
                Fr::from_str(s)
                    .map_err(|_| anyhow!("Failed to parse public input: {}", s))
            })
            .collect()
    }

    /// Generate attestation witness
    fn generate_witness(
        proof: &Proof<Bn254>,
        public_inputs: &[String],
        vk_hash: &str,
        verification_result: bool,
    ) -> Result<AttestationWitness> {
        // Convert proof points to strings for serialization
        // For now, store as hex-encoded serialized bytes
        use ark_serialize::CanonicalSerialize;

        let mut a_bytes = Vec::new();
        proof.a.serialize_compressed(&mut a_bytes)
            .map_err(|e| anyhow!("Failed to serialize proof.a: {:?}", e))?;

        let mut b_bytes = Vec::new();
        proof.b.serialize_compressed(&mut b_bytes)
            .map_err(|e| anyhow!("Failed to serialize proof.b: {:?}", e))?;

        let mut c_bytes = Vec::new();
        proof.c.serialize_compressed(&mut c_bytes)
            .map_err(|e| anyhow!("Failed to serialize proof.c: {:?}", e))?;

        // For compatibility, store as string arrays (simplified representation)
        let proof_a = [hex::encode(&a_bytes), "".to_string()];
        let proof_b = [
            [hex::encode(&b_bytes), "".to_string()],
            ["".to_string(), "".to_string()],
        ];
        let proof_c = [hex::encode(&c_bytes), "".to_string()];

        // Hash public inputs
        let public_inputs_hash = Self::hash_public_inputs(public_inputs);

        Ok(AttestationWitness {
            proof_a,
            proof_b,
            proof_c,
            vk_hash: vk_hash.to_string(),
            public_inputs: public_inputs.to_vec(),
            public_inputs_hash,
            verification_result: if verification_result { 1 } else { 0 },
        })
    }

    /// Hash public inputs using Keccak256
    fn hash_public_inputs(inputs: &[String]) -> String {
        let mut hasher = Keccak256::new();
        for input in inputs {
            hasher.update(input.as_bytes());
        }
        hex::encode(hasher.finalize())
    }

    /// Check if service has VK for circuit type
    pub fn has_vk_for_circuit(&self, circuit_type: u8) -> bool {
        self.prepared_vks.contains_key(&circuit_type)
    }

    /// Get available circuit types
    pub fn available_circuits(&self) -> Vec<u8> {
        let mut types: Vec<u8> = self.prepared_vks.keys().copied().collect();
        types.sort();
        types
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_public_inputs() {
        let inputs = vec!["123".to_string(), "456".to_string()];
        let hash = AttestationService::hash_public_inputs(&inputs);
        assert_eq!(hash.len(), 64); // 32 bytes = 64 hex chars
    }

    #[test]
    fn test_hash_vk() {
        let vk_json = r#"{"protocol":"groth16","curve":"bn128"}"#;
        let hash = AttestationService::hash_vk(vk_json);
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_parse_snarkjs_proof() {
        // Real snarkjs proof from circuits/simple/proof.json (3 * 5 = 15)
        let proof_json = r#"{
            "pi_a": [
                "839153626367611722924325225999268687205447144846832230726526244338479388680",
                "6935984314348026995993445832244635777919189137443618008975337069306276681884",
                "1"
            ],
            "pi_b": [
                [
                    "17080271217825836373836933058073402793972331398764677119195078332666864183997",
                    "4038097863506519386695539303031378347006006149438370892228121020335815250170"
                ],
                [
                    "5368084851408696299370686280525438612242401182840666668219413888207819660672",
                    "11925848919046149722475149418223418946339625623326454748781597957177476421987"
                ],
                [
                    "1",
                    "0"
                ]
            ],
            "pi_c": [
                "3610379328435270247291686493527679213314551180676233187965023369795082172088",
                "15286044869394133871575822828283985739784276545020597004550872036973716363398",
                "1"
            ],
            "protocol": "groth16",
            "curve": "bn128"
        }"#;

        let proof = AttestationService::parse_proof(proof_json);
        assert!(proof.is_ok(), "Failed to parse proof: {:?}", proof.err());

        let proof = proof.unwrap();
        // Verify proof points are valid (not infinity)
        assert!(!proof.a.is_zero(), "pi_a should not be zero");
        assert!(!proof.b.is_zero(), "pi_b should not be zero");
        assert!(!proof.c.is_zero(), "pi_c should not be zero");
    }

    #[test]
    fn test_parse_snarkjs_vk() {
        // Real snarkjs VK from circuits/simple/simple_vkey.json
        let vk_json = r#"{
            "protocol": "groth16",
            "curve": "bn128",
            "nPublic": 1,
            "vk_alpha_1": [
                "5062140312298946326989225741608631954036097066135361622511326148733196590258",
                "18861539519298821229267791556945919616987253692058240011388509984416450130088",
                "1"
            ],
            "vk_beta_2": [
                [
                    "8546682713136717308725271097450987963593683721176776845149488938656579650534",
                    "10090674548676489838428092890651470209580335891040048133242247703378919989771"
                ],
                [
                    "8581281403899649312572122569028900279444703920876841004012706253990877711052",
                    "9603125569850642217288889916137000402178365161378423972075230871840529097921"
                ],
                [
                    "1",
                    "0"
                ]
            ],
            "vk_gamma_2": [
                [
                    "10857046999023057135944570762232829481370756359578518086990519993285655852781",
                    "11559732032986387107991004021392285783925812861821192530917403151452391805634"
                ],
                [
                    "8495653923123431417604973247489272438418190587263600148770280649306958101930",
                    "4082367875863433681332203403145435568316851327593401208105741076214120093531"
                ],
                [
                    "1",
                    "0"
                ]
            ],
            "vk_delta_2": [
                [
                    "4528209701086706766588351157027334389611707905798024333613160690483282218742",
                    "2022274774929070064893207004118662205960998326509197609475724954469764637894"
                ],
                [
                    "14098005932401341394882476945504788403852029994222669127843538049843754367717",
                    "9495743241988306549495366957438683954476405337523009919411798317885901468614"
                ],
                [
                    "1",
                    "0"
                ]
            ],
            "vk_alphabeta_12": [],
            "IC": [
                [
                    "16950575122667325885582900171072018174312660183058128144591475945005187634279",
                    "20025615881405649801158581094734760353153906057834573830893764125047414720580",
                    "1"
                ],
                [
                    "12371418154280325202061225662674180571079205726981953184778406847552770523269",
                    "5726313876778852231154356875022624067887799733711552554192664065368311799739",
                    "1"
                ]
            ]
        }"#;

        let vk: SnarkjsVKey = serde_json::from_str(vk_json).expect("Failed to parse VK JSON");
        let ark_vk = AttestationService::parse_verification_key(&vk);
        assert!(ark_vk.is_ok(), "Failed to parse VK: {:?}", ark_vk.err());
    }

    #[test]
    fn test_verify_snarkjs_proof() {
        // This test verifies that a snarkjs-generated proof passes verification
        // Proof: 3 * 5 = 15 (Multiplier circuit)

        let proof_json = r#"{
            "pi_a": [
                "839153626367611722924325225999268687205447144846832230726526244338479388680",
                "6935984314348026995993445832244635777919189137443618008975337069306276681884",
                "1"
            ],
            "pi_b": [
                [
                    "17080271217825836373836933058073402793972331398764677119195078332666864183997",
                    "4038097863506519386695539303031378347006006149438370892228121020335815250170"
                ],
                [
                    "5368084851408696299370686280525438612242401182840666668219413888207819660672",
                    "11925848919046149722475149418223418946339625623326454748781597957177476421987"
                ],
                [
                    "1",
                    "0"
                ]
            ],
            "pi_c": [
                "3610379328435270247291686493527679213314551180676233187965023369795082172088",
                "15286044869394133871575822828283985739784276545020597004550872036973716363398",
                "1"
            ],
            "protocol": "groth16",
            "curve": "bn128"
        }"#;

        let vk_json = r#"{
            "protocol": "groth16",
            "curve": "bn128",
            "nPublic": 1,
            "vk_alpha_1": [
                "5062140312298946326989225741608631954036097066135361622511326148733196590258",
                "18861539519298821229267791556945919616987253692058240011388509984416450130088",
                "1"
            ],
            "vk_beta_2": [
                [
                    "8546682713136717308725271097450987963593683721176776845149488938656579650534",
                    "10090674548676489838428092890651470209580335891040048133242247703378919989771"
                ],
                [
                    "8581281403899649312572122569028900279444703920876841004012706253990877711052",
                    "9603125569850642217288889916137000402178365161378423972075230871840529097921"
                ],
                [
                    "1",
                    "0"
                ]
            ],
            "vk_gamma_2": [
                [
                    "10857046999023057135944570762232829481370756359578518086990519993285655852781",
                    "11559732032986387107991004021392285783925812861821192530917403151452391805634"
                ],
                [
                    "8495653923123431417604973247489272438418190587263600148770280649306958101930",
                    "4082367875863433681332203403145435568316851327593401208105741076214120093531"
                ],
                [
                    "1",
                    "0"
                ]
            ],
            "vk_delta_2": [
                [
                    "4528209701086706766588351157027334389611707905798024333613160690483282218742",
                    "2022274774929070064893207004118662205960998326509197609475724954469764637894"
                ],
                [
                    "14098005932401341394882476945504788403852029994222669127843538049843754367717",
                    "9495743241988306549495366957438683954476405337523009919411798317885901468614"
                ],
                [
                    "1",
                    "0"
                ]
            ],
            "vk_alphabeta_12": [],
            "IC": [
                [
                    "16950575122667325885582900171072018174312660183058128144591475945005187634279",
                    "20025615881405649801158581094734760353153906057834573830893764125047414720580",
                    "1"
                ],
                [
                    "12371418154280325202061225662674180571079205726981953184778406847552770523269",
                    "5726313876778852231154356875022624067887799733711552554192664065368311799739",
                    "1"
                ]
            ]
        }"#;

        // Parse proof
        let proof = AttestationService::parse_proof(proof_json).expect("Failed to parse proof");

        // Parse VK
        let vk: SnarkjsVKey = serde_json::from_str(vk_json).expect("Failed to parse VK JSON");
        let ark_vk = AttestationService::parse_verification_key(&vk).expect("Failed to parse VK");
        let prepared_vk = Groth16::<Bn254>::process_vk(&ark_vk).expect("Failed to prepare VK");

        // Parse public inputs (output: 15)
        let public_inputs = AttestationService::parse_public_inputs(&["15".to_string()])
            .expect("Failed to parse public inputs");

        // Verify
        let result = Groth16::<Bn254>::verify_with_processed_vk(&prepared_vk, &public_inputs, &proof);

        assert!(result.is_ok(), "Verification returned error: {:?}", result.err());
        assert!(result.unwrap(), "Proof verification should return true (Ok(true))");
    }
}
