use anyhow::{anyhow, Result};
use ark_bn254::Fr;
use ark_ff::{BigInteger, Field, PrimeField};
use blake2::{Blake2s256, Digest};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct MerkleProof {
    pub path: Vec<Fr>,
    pub indices: Vec<u8>,
    pub root: Fr,
    pub leaf: Fr,
}

pub struct MerkleTree {
    leaves: Vec<Fr>,
    layers: Vec<Vec<Fr>>,
}

impl MerkleTree {
    pub fn new(leaves: Vec<Fr>) -> Self {
        let mut padded = leaves;
        // pad to power of two
        let size = padded.len().next_power_of_two();
        while padded.len() < size {
            padded.push(Fr::ZERO);
        }
        let mut layers = Vec::new();
        layers.push(padded.clone());
        let mut current = padded;
        while current.len() > 1 {
            let mut next = Vec::with_capacity(current.len() / 2);
            for i in (0..current.len()).step_by(2) {
                let left = current[i];
                let right = current.get(i + 1).copied().unwrap_or(Fr::ZERO);
                next.push(poseidon_hash(&[left, right]));
            }
            layers.push(next.clone());
            current = next;
        }
        Self { leaves: layers[0].clone(), layers }
    }

    pub fn root(&self) -> Fr {
        *self.layers.last().unwrap().first().unwrap()
    }

    pub fn proof(&self, index: usize) -> Result<MerkleProof> {
        if index >= self.leaves.len() {
            return Err(anyhow!("index out of range"));
        }
        let mut path = Vec::new();
        let mut indices = Vec::new();
        let mut idx = index;
        for level in 0..self.layers.len() - 1 {
            let is_right = (idx % 2) == 1;
            let sibling_idx = if is_right { idx - 1 } else { idx + 1 };
            let sibling = self.layers[level].get(sibling_idx).copied().unwrap_or(Fr::ZERO);
            path.push(sibling);
            indices.push(if is_right { 1 } else { 0 });
            idx /= 2;
        }
        Ok(MerkleProof {
            path,
            indices,
            root: self.root(),
            leaf: self.leaves[index],
        })
    }

    pub fn verify(leaf: Fr, proof: &MerkleProof) -> bool {
        let mut acc = leaf;
        for (i, sibling) in proof.path.iter().enumerate() {
            let is_right = proof.indices.get(i).copied().unwrap_or(0) == 1;
            acc = if is_right { hash_pair(*sibling, acc) } else { hash_pair(acc, *sibling) };
        }
        acc == proof.root
    }
}

/// Convert a big-endian hex string into Fr
pub fn fr_from_hex(hex_str: &str) -> Result<Fr> {
    let cleaned = hex_str.trim_start_matches("0x");
    let bytes = hex::decode(cleaned)?;
    Ok(Fr::from_be_bytes_mod_order(&bytes))
}

/// Convert decimal string to Fr
pub fn fr_from_dec(dec_str: &str) -> Result<Fr> {
    let f = Fr::from_str(dec_str).map_err(|e| anyhow!("failed to parse dec to Fr: {e:?}"))?;
    Ok(f)
}

fn hash_pair(left: Fr, right: Fr) -> Fr {
    poseidon_hash(&[left, right])
}

/// Placeholder Poseidon: use Blake2s to derive a field element. Not circuit-compatible.
/// TODO: replace with real Poseidon parameters once available.
pub fn poseidon_hash(inputs: &[Fr]) -> Fr {
    let mut hasher = Blake2s256::new();
    for f in inputs {
        hasher.update(f.into_bigint().to_bytes_be());
    }
    let digest = hasher.finalize();
    Fr::from_be_bytes_mod_order(&digest)
}

pub fn generate_commitment(value: Fr, blinding: Fr) -> Fr {
    poseidon_hash(&[value, blinding])
}

pub fn generate_nullifier(secret: Fr, context: Fr) -> Fr {
    poseidon_hash(&[secret, context])
}
