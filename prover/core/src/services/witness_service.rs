use anyhow::Result;
use std::sync::Arc;
use crate::witness_encryption::WitnessEncryption;
use crate::witness_fetcher::WitnessFetcher;
use crate::halo2_prover::OrchardWitness;

/// Service for witness operations (download, decrypt, encrypt)
pub struct WitnessService {
    fetcher: Arc<WitnessFetcher>,
    encryption: Arc<WitnessEncryption>,
}

impl WitnessService {
    /// Create a new WitnessService
    pub fn new(fetcher: Arc<WitnessFetcher>, encryption: Arc<WitnessEncryption>) -> Self {
        Self {
            fetcher,
            encryption,
        }
    }

    /// Download and decrypt witness from backend
    ///
    /// This is the primary method for retrieving witness data for proving.
    /// It combines download and decryption in a single operation.
    pub async fn get_witness(&self, witness_hash: &[u8; 32]) -> Result<OrchardWitness> {
        let encrypted = self.download_encrypted(witness_hash).await?;
        self.decrypt(&encrypted)
    }

    /// Download raw encrypted witness from backend
    ///
    /// Returns the encrypted witness bytes without decrypting.
    /// Useful for caching or when decryption should happen later.
    pub async fn download_encrypted(&self, witness_hash: &[u8; 32]) -> Result<Vec<u8>> {
        self.fetcher.download_witness(witness_hash).await
    }

    /// Decrypt witness data
    ///
    /// Takes encrypted witness bytes and returns the decrypted OrchardWitness.
    pub fn decrypt(&self, encrypted: &[u8]) -> Result<OrchardWitness> {
        self.encryption.decrypt_witness(encrypted)
    }

    /// Get encryption public key
    ///
    /// This public key should be shared with clients so they can encrypt
    /// witness data that only this prover can decrypt.
    pub fn public_key(&self) -> [u8; 32] {
        self.encryption.public_key()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::witness_encryption::WitnessEncryption;
    use crate::witness_fetcher::WitnessFetcher;

    #[test]
    fn test_witness_service_creation() {
        let fetcher = Arc::new(WitnessFetcher::new("http://localhost:8080".to_string()));
        let encryption = Arc::new(WitnessEncryption::new().unwrap());

        let service = WitnessService::new(fetcher, encryption);

        let pubkey = service.public_key();
        assert_eq!(pubkey.len(), 32);
    }

    #[test]
    fn test_decrypt() {
        let fetcher = Arc::new(WitnessFetcher::new("http://localhost:8080".to_string()));
        let encryption = Arc::new(WitnessEncryption::new().unwrap());
        let service = WitnessService::new(fetcher, encryption.clone());

        // Create test witness
        let witness = OrchardWitness::dummy();

        // Encrypt it
        let encrypted = WitnessEncryption::encrypt_witness(&witness, &service.public_key()).unwrap();

        // Decrypt via service
        let decrypted = service.decrypt(&encrypted).unwrap();

        assert_eq!(witness.note_value, decrypted.note_value);
        assert_eq!(witness.spend_auth_sig, decrypted.spend_auth_sig);
    }
}
