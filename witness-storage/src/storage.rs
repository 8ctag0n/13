use blake2::{Blake2b512, Digest};
use std::collections::HashMap;

/// In-memory witness storage
/// Production would use Redis/PostgreSQL/S3
pub struct WitnessStorage {
    data: HashMap<[u8; 32], Vec<u8>>,
}

impl WitnessStorage {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Store witness and return commitment (blake2b hash)
    pub fn store(&mut self, witness: Vec<u8>) -> [u8; 32] {
        let mut hasher = Blake2b512::new();
        hasher.update(&witness);
        let hash = hasher.finalize();

        let mut commitment = [0u8; 32];
        commitment.copy_from_slice(&hash[..32]);

        self.data.insert(commitment, witness);
        commitment
    }

    /// Retrieve witness by commitment
    pub fn retrieve(&self, commitment: &[u8; 32]) -> Option<Vec<u8>> {
        self.data.get(commitment).cloned()
    }

    /// Get storage stats
    pub fn stats(&self) -> (usize, usize) {
        let count = self.data.len();
        let total_bytes: usize = self.data.values().map(|v| v.len()).sum();
        (count, total_bytes)
    }
}

impl Default for WitnessStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_retrieve() {
        let mut storage = WitnessStorage::new();

        let data = b"test witness data";
        let commitment = storage.store(data.to_vec());

        let retrieved = storage.retrieve(&commitment).unwrap();
        assert_eq!(retrieved, data);
    }

    #[test]
    fn test_commitment_is_deterministic() {
        let mut storage1 = WitnessStorage::new();
        let mut storage2 = WitnessStorage::new();

        let data = b"same data";
        let c1 = storage1.store(data.to_vec());
        let c2 = storage2.store(data.to_vec());

        assert_eq!(c1, c2);
    }

    #[test]
    fn test_stats() {
        let mut storage = WitnessStorage::new();

        storage.store(vec![1, 2, 3]);
        storage.store(vec![4, 5, 6, 7]);

        let (count, bytes) = storage.stats();
        assert_eq!(count, 2);
        assert_eq!(bytes, 7);
    }
}
