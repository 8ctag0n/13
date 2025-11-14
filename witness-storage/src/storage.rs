use blake2::{Blake2b512, Digest};
use std::collections::HashMap;

#[cfg(feature = "persistence")]
use crate::persistence::DiskPersistence;

/// In-memory witness storage with support for both witness data and FHE results
/// Production would use Redis/PostgreSQL/S3
pub struct WitnessStorage {
    witnesses: HashMap<[u8; 32], Vec<u8>>,
    fhe_results: HashMap<[u8; 32], Vec<u8>>,
    #[cfg(feature = "persistence")]
    persistence: Option<DiskPersistence>,
}

impl WitnessStorage {
    pub fn new() -> Self {
        Self {
            witnesses: HashMap::new(),
            fhe_results: HashMap::new(),
            #[cfg(feature = "persistence")]
            persistence: None,
        }
    }

    #[cfg(feature = "persistence")]
    pub fn with_persistence(persistence: DiskPersistence) -> Self {
        Self {
            witnesses: HashMap::new(),
            fhe_results: HashMap::new(),
            persistence: Some(persistence),
        }
    }

    /// Store witness and return commitment (blake2b hash)
    pub fn store(&mut self, witness: Vec<u8>) -> [u8; 32] {
        let mut hasher = Blake2b512::new();
        hasher.update(&witness);
        let hash = hasher.finalize();

        let mut commitment = [0u8; 32];
        commitment.copy_from_slice(&hash[..32]);

        self.witnesses.insert(commitment, witness.clone());

        // Persist to disk if persistence is enabled
        #[cfg(feature = "persistence")]
        if let Some(ref persistence) = self.persistence {
            let _ = persistence.save_witness(&commitment, &witness);
        }

        commitment
    }

    /// Retrieve witness by commitment
    pub fn retrieve(&self, commitment: &[u8; 32]) -> Option<Vec<u8>> {
        self.witnesses.get(commitment).cloned()
    }

    /// Store FHE result and return commitment (blake2b hash)
    pub fn store_fhe_result(&mut self, result: Vec<u8>) -> [u8; 32] {
        let mut hasher = Blake2b512::new();
        hasher.update(&result);
        let hash = hasher.finalize();

        let mut commitment = [0u8; 32];
        commitment.copy_from_slice(&hash[..32]);

        self.fhe_results.insert(commitment, result.clone());

        // Persist to disk if persistence is enabled
        #[cfg(feature = "persistence")]
        if let Some(ref persistence) = self.persistence {
            let _ = persistence.save_fhe_result(&commitment, &result);
        }

        commitment
    }

    /// Retrieve FHE result by commitment
    pub fn retrieve_fhe_result(&self, commitment: &[u8; 32]) -> Option<Vec<u8>> {
        self.fhe_results.get(commitment).cloned()
    }

    /// Get storage stats (witness_count, witness_bytes, fhe_count, fhe_bytes)
    pub fn stats(&self) -> (usize, usize, usize, usize) {
        let witness_count = self.witnesses.len();
        let witness_bytes: usize = self.witnesses.values().map(|v| v.len()).sum();
        let fhe_count = self.fhe_results.len();
        let fhe_bytes: usize = self.fhe_results.values().map(|v| v.len()).sum();
        (witness_count, witness_bytes, fhe_count, fhe_bytes)
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

        let (witness_count, witness_bytes, fhe_count, fhe_bytes) = storage.stats();
        assert_eq!(witness_count, 2);
        assert_eq!(witness_bytes, 7);
        assert_eq!(fhe_count, 0);
        assert_eq!(fhe_bytes, 0);
    }

    #[test]
    fn test_store_retrieve_fhe_result() {
        let mut storage = WitnessStorage::new();

        let fhe_data = b"encrypted FHE result";
        let commitment = storage.store_fhe_result(fhe_data.to_vec());

        let retrieved = storage.retrieve_fhe_result(&commitment).unwrap();
        assert_eq!(retrieved, fhe_data);
    }

    #[test]
    fn test_fhe_commitment_is_deterministic() {
        let mut storage1 = WitnessStorage::new();
        let mut storage2 = WitnessStorage::new();

        let data = b"same FHE result";
        let c1 = storage1.store_fhe_result(data.to_vec());
        let c2 = storage2.store_fhe_result(data.to_vec());

        assert_eq!(c1, c2);
    }

    #[test]
    fn test_stats_with_both_types() {
        let mut storage = WitnessStorage::new();

        storage.store(vec![1, 2, 3]);
        storage.store_fhe_result(vec![4, 5, 6, 7, 8]);

        let (witness_count, witness_bytes, fhe_count, fhe_bytes) = storage.stats();
        assert_eq!(witness_count, 1);
        assert_eq!(witness_bytes, 3);
        assert_eq!(fhe_count, 1);
        assert_eq!(fhe_bytes, 5);
    }

    #[test]
    #[cfg(feature = "persistence")]
    fn test_witness_persistence() {
        use crate::persistence::DiskPersistence;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let persistence = DiskPersistence::new(temp_dir.path()).unwrap();
        let mut storage = WitnessStorage::with_persistence(persistence);

        let data = b"test witness with persistence";
        let commitment = storage.store(data.to_vec());

        // Verify it's in memory
        let retrieved = storage.retrieve(&commitment).unwrap();
        assert_eq!(retrieved, data);

        // Verify it was persisted to disk
        let persistence = DiskPersistence::new(temp_dir.path()).unwrap();
        let from_disk = persistence.load_witness(&commitment).unwrap().unwrap();
        assert_eq!(from_disk, data);
    }

    #[test]
    #[cfg(feature = "persistence")]
    fn test_fhe_result_persistence() {
        use crate::persistence::DiskPersistence;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let persistence = DiskPersistence::new(temp_dir.path()).unwrap();
        let mut storage = WitnessStorage::with_persistence(persistence);

        let data = b"encrypted FHE result with persistence";
        let commitment = storage.store_fhe_result(data.to_vec());

        // Verify it's in memory
        let retrieved = storage.retrieve_fhe_result(&commitment).unwrap();
        assert_eq!(retrieved, data);

        // Verify it was persisted to disk
        let persistence = DiskPersistence::new(temp_dir.path()).unwrap();
        let from_disk = persistence.load_fhe_result(&commitment).unwrap().unwrap();
        assert_eq!(from_disk, data);
    }
}
