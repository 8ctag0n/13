use crate::serialization::{deserialize_ciphertext, serialize_ciphertext};
use crate::FheError;
use bincode;
use sha2::{Digest, Sha256};
use tfhe::prelude::*;
use tfhe::{generate_keys, set_server_key, ClientKey, ConfigBuilder, FheUint64, ServerKey};

/// Cliente FHE para encriptar bet amounts en Futarchy Markets
pub struct FutarchyFheClient {
    client_key: ClientKey,
    server_key: ServerKey,
}

impl FutarchyFheClient {
    /// Crear nuevo client con keys generadas
    ///
    /// NOTA: La generación de keys puede tardar varios segundos
    pub fn new() -> Result<Self, FheError> {
        // Configurar parámetros FHE para u64
        let config = ConfigBuilder::default().build();

        // Generar keys (esto tarda un tiempo)
        let (client_key, server_key) = generate_keys(config);

        Ok(Self {
            client_key,
            server_key,
        })
    }

    /// Cargar client desde keys guardadas
    ///
    /// # Arguments
    /// * `client_key_bytes` - Client key serializada (PRIVADA)
    /// * `server_key_bytes` - Server key serializada (PÚBLICA)
    pub fn from_keys(client_key_bytes: &[u8], server_key_bytes: &[u8]) -> Result<Self, FheError> {
        let client_key: ClientKey = bincode::deserialize(client_key_bytes)
            .map_err(|e| FheError::DeserializationError(format!("Client key: {}", e)))?;

        let server_key: ServerKey = bincode::deserialize(server_key_bytes)
            .map_err(|e| FheError::DeserializationError(format!("Server key: {}", e)))?;

        Ok(Self {
            client_key,
            server_key,
        })
    }

    /// Encriptar bet amount para enviar on-chain
    ///
    /// # Arguments
    /// * `amount` - Monto de la apuesta en tokens base
    ///
    /// # Returns
    /// Ciphertext serializado listo para transmitir (500-2000 bytes típicamente)
    pub fn encrypt_bet_amount(&self, amount: u64) -> Result<Vec<u8>, FheError> {
        // Encriptar el monto
        let encrypted: FheUint64 = FheUint64::encrypt(amount, &self.client_key);

        // Serializar para transmisión
        serialize_ciphertext(&encrypted)
            .map_err(|e| FheError::EncryptionError(e.to_string()))
    }

    /// Desencriptar pool (solo para testing/debugging)
    ///
    /// NOTA: En producción, los pools encriptados permanecen encriptados.
    /// Esta función es útil para testing y validación.
    ///
    /// # Arguments
    /// * `encrypted` - Ciphertext serializado
    pub fn decrypt_pool(&self, encrypted: &[u8]) -> Result<u64, FheError> {
        // Deserializar el ciphertext
        let ct = deserialize_ciphertext(encrypted)
            .map_err(|e| FheError::DecryptionError(e.to_string()))?;

        // Desencriptar
        let decrypted: u64 = ct.decrypt(&self.client_key);

        Ok(decrypted)
    }

    /// Obtener server key serializada
    ///
    /// Esta key es PÚBLICA y debe compartirse con los provers
    /// para que puedan realizar computación homomorfica.
    pub fn get_server_key_bytes(&self) -> Vec<u8> {
        bincode::serialize(&self.server_key).expect("Failed to serialize server key")
    }

    /// Guardar client key (PRIVADA - mantener segura)
    ///
    /// Esta key es PRIVADA y permite desencriptar.
    /// El usuario debe guardarla de forma segura.
    pub fn get_client_key_bytes(&self) -> Vec<u8> {
        bincode::serialize(&self.client_key).expect("Failed to serialize client key")
    }

    /// Verificar que las keys funcionan correctamente
    pub fn verify_keys(&self) -> Result<(), FheError> {
        // Test rápido de encriptación/desencriptación
        let test_value: u64 = 42;
        let encrypted = self.encrypt_bet_amount(test_value)?;
        let decrypted = self.decrypt_pool(&encrypted)?;

        if decrypted == test_value {
            Ok(())
        } else {
            Err(FheError::KeyGenerationError(
                "Key verification failed".to_string(),
            ))
        }
    }

    /// Establecer la server key en el contexto global para operaciones FHE
    ///
    /// Esto es necesario antes de realizar operaciones homorfomicas
    pub fn set_server_key(&self) {
        set_server_key(self.server_key.clone());
    }

    /// Encriptar múltiples bet amounts de una vez
    ///
    /// Útil para batch processing en el cliente
    pub fn encrypt_batch(&self, amounts: &[u64]) -> Result<Vec<Vec<u8>>, FheError> {
        amounts
            .iter()
            .map(|&amount| self.encrypt_bet_amount(amount))
            .collect()
    }

    /// Sumar dos ciphertexts (operación homomórfica)
    ///
    /// Esta operación es usada por los FHE provers para actualizar pools:
    /// new_pool = current_pool + bet_amount
    ///
    /// NOTA: Requiere que set_server_key() haya sido llamado
    pub fn homomorphic_add(
        &self,
        ct_a: &[u8],
        ct_b: &[u8],
    ) -> Result<Vec<u8>, FheError> {
        use crate::serialization::{deserialize_ciphertext, serialize_ciphertext};

        // Deserializar ambos ciphertexts
        let a: FheUint64 = deserialize_ciphertext(ct_a)
            .map_err(|e| FheError::DeserializationError(format!("ct_a: {}", e)))?;
        let b: FheUint64 = deserialize_ciphertext(ct_b)
            .map_err(|e| FheError::DeserializationError(format!("ct_b: {}", e)))?;

        // Establecer server key para operaciones
        set_server_key(self.server_key.clone());

        // Realizar suma homomórfica
        let result = &a + &b;

        // Serializar resultado
        serialize_ciphertext(&result)
            .map_err(|e| FheError::SerializationError(e.to_string()))
    }

    /// Encriptar cero - útil para inicializar pools
    pub fn encrypt_zero(&self) -> Result<Vec<u8>, FheError> {
        self.encrypt_bet_amount(0)
    }

    /// Comparar si el ciphertext es mayor que un valor cleartext
    ///
    /// Retorna un ciphertext de boolean encriptado
    /// NOTA: Comparaciones FHE son costosas en tiempo
    pub fn homomorphic_gt(
        &self,
        ct: &[u8],
        cleartext: u64,
    ) -> Result<Vec<u8>, FheError> {
        use crate::serialization::deserialize_ciphertext;

        let encrypted: FheUint64 = deserialize_ciphertext(ct)
            .map_err(|e| FheError::DeserializationError(e.to_string()))?;

        // Establecer server key
        set_server_key(self.server_key.clone());

        // Comparación homomórfica
        let result = encrypted.gt(cleartext);

        // Serializar el resultado booleano
        bincode::serialize(&result)
            .map_err(|e| FheError::SerializationError(e.to_string()))
    }

    /// Generar hash SHA256 de un ciphertext
    ///
    /// Este hash se usa como referencia on-chain mientras el ciphertext
    /// real se almacena off-chain (servidor/IPFS).
    ///
    /// # Returns
    /// Hash de 32 bytes del ciphertext
    pub fn hash_ciphertext(ciphertext: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(ciphertext);
        hasher.finalize().into()
    }

    /// Encriptar bet amount y obtener tanto el ciphertext como su hash
    ///
    /// Flujo típico:
    /// 1. Usuario encripta con esta función
    /// 2. Envía `ciphertext` al servidor de la app
    /// 3. Envía `hash` on-chain como referencia
    /// 4. Provers buscan ciphertext del servidor usando el hash
    ///
    /// # Returns
    /// `EncryptedBet` con ciphertext (off-chain) y hash (on-chain)
    pub fn encrypt_bet_with_hash(&self, amount: u64) -> Result<EncryptedBet, FheError> {
        let ciphertext = self.encrypt_bet_amount(amount)?;
        let hash = Self::hash_ciphertext(&ciphertext);

        Ok(EncryptedBet { ciphertext, hash })
    }

    /// Verificar que un ciphertext corresponde a un hash dado
    pub fn verify_ciphertext_hash(ciphertext: &[u8], expected_hash: &[u8; 32]) -> bool {
        let actual_hash = Self::hash_ciphertext(ciphertext);
        actual_hash == *expected_hash
    }
}

/// Resultado de encriptar un bet amount
///
/// Contiene tanto el ciphertext (para storage off-chain) como
/// su hash (para referencia on-chain).
#[derive(Debug, Clone)]
pub struct EncryptedBet {
    /// Ciphertext completo - guardar off-chain (servidor/IPFS)
    /// Tamaño: ~500KB para FheUint64
    pub ciphertext: Vec<u8>,

    /// Hash SHA256 del ciphertext - guardar on-chain
    /// Tamaño: 32 bytes
    pub hash: [u8; 32],
}

impl EncryptedBet {
    /// Hash como hex string (útil para APIs)
    pub fn hash_hex(&self) -> String {
        hex::encode(self.hash)
    }

    /// Tamaño del ciphertext en bytes
    pub fn ciphertext_size(&self) -> usize {
        self.ciphertext.len()
    }
}

impl Default for FutarchyFheClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default FHE client")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = FutarchyFheClient::new();
        assert!(client.is_ok());
    }

    #[test]
    fn test_key_verification() {
        let client = FutarchyFheClient::new().expect("Failed to create client");
        assert!(client.verify_keys().is_ok());
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let client = FutarchyFheClient::new().expect("Failed to create client");

        let test_values = vec![0, 1, 100, 1000, 999999, u64::MAX >> 16];

        for value in test_values {
            let encrypted = client
                .encrypt_bet_amount(value)
                .expect(&format!("Failed to encrypt {}", value));
            let decrypted = client
                .decrypt_pool(&encrypted)
                .expect(&format!("Failed to decrypt {}", value));

            assert_eq!(
                value, decrypted,
                "Mismatch for value {}. Got {}",
                value, decrypted
            );
        }
    }

    #[test]
    fn test_keys_persistence() {
        let client = FutarchyFheClient::new().expect("Failed to create client");

        let client_key_bytes = client.get_client_key_bytes();
        let server_key_bytes = client.get_server_key_bytes();

        // Crear nuevo cliente con las mismas keys
        let restored_client =
            FutarchyFheClient::from_keys(&client_key_bytes, &server_key_bytes)
                .expect("Failed to restore client");

        // Encriptar con el cliente original
        let encrypted = client
            .encrypt_bet_amount(12345)
            .expect("Failed to encrypt");

        // Desencriptar con el cliente restaurado
        let decrypted = restored_client
            .decrypt_pool(&encrypted)
            .expect("Failed to decrypt");

        assert_eq!(12345, decrypted);
    }

    #[test]
    fn test_batch_encryption() {
        let client = FutarchyFheClient::new().expect("Failed to create client");

        let amounts = vec![100, 200, 300, 400, 500];
        let encrypted_batch = client
            .encrypt_batch(&amounts)
            .expect("Failed to encrypt batch");

        assert_eq!(amounts.len(), encrypted_batch.len());

        // Verificar cada uno
        for (i, encrypted) in encrypted_batch.iter().enumerate() {
            let decrypted = client
                .decrypt_pool(encrypted)
                .expect("Failed to decrypt batch item");
            assert_eq!(amounts[i], decrypted);
        }
    }

    #[test]
    fn test_randomness_in_encryption() {
        let client = FutarchyFheClient::new().expect("Failed to create client");

        let amount = 777;
        let mut ciphertexts = Vec::new();

        // Encriptar el mismo valor 5 veces
        for _ in 0..5 {
            let encrypted = client
                .encrypt_bet_amount(amount)
                .expect("Failed to encrypt");
            ciphertexts.push(encrypted);
        }

        // Todos los ciphertexts deben ser diferentes
        for i in 0..ciphertexts.len() {
            for j in i + 1..ciphertexts.len() {
                assert_ne!(
                    ciphertexts[i], ciphertexts[j],
                    "Ciphertexts {} and {} should be different",
                    i, j
                );
            }
        }

        // Pero todos deben desencriptar al mismo valor
        for ct in ciphertexts {
            let decrypted = client.decrypt_pool(&ct).expect("Failed to decrypt");
            assert_eq!(amount, decrypted);
        }
    }

    #[test]
    fn test_homomorphic_add() {
        let client = FutarchyFheClient::new().expect("Failed to create client");

        let a: u64 = 1000;
        let b: u64 = 500;

        // Encriptar ambos valores
        let ct_a = client.encrypt_bet_amount(a).expect("Failed to encrypt a");
        let ct_b = client.encrypt_bet_amount(b).expect("Failed to encrypt b");

        // Sumar homomórficamente
        let ct_sum = client.homomorphic_add(&ct_a, &ct_b)
            .expect("Failed to add homomorphically");

        // Desencriptar y verificar
        let decrypted_sum = client.decrypt_pool(&ct_sum)
            .expect("Failed to decrypt sum");

        assert_eq!(a + b, decrypted_sum, "Homomorphic addition failed");
    }

    #[test]
    fn test_homomorphic_add_multiple() {
        let client = FutarchyFheClient::new().expect("Failed to create client");

        // Simular múltiples bets agregados a un pool
        let initial_pool: u64 = 0;
        let bets = vec![100, 250, 75, 300];

        // Inicializar pool encriptado
        let mut encrypted_pool = client.encrypt_bet_amount(initial_pool)
            .expect("Failed to encrypt initial pool");

        // Agregar cada bet al pool
        for bet in &bets {
            let encrypted_bet = client.encrypt_bet_amount(*bet)
                .expect("Failed to encrypt bet");
            encrypted_pool = client.homomorphic_add(&encrypted_pool, &encrypted_bet)
                .expect("Failed to add bet to pool");
        }

        // Desencriptar y verificar
        let final_pool = client.decrypt_pool(&encrypted_pool)
            .expect("Failed to decrypt final pool");

        let expected: u64 = initial_pool + bets.iter().sum::<u64>();
        assert_eq!(expected, final_pool, "Multiple homomorphic additions failed");
    }

    #[test]
    fn test_encrypt_zero() {
        let client = FutarchyFheClient::new().expect("Failed to create client");

        let encrypted_zero = client.encrypt_zero().expect("Failed to encrypt zero");
        let decrypted = client.decrypt_pool(&encrypted_zero)
            .expect("Failed to decrypt zero");

        assert_eq!(0, decrypted);
    }
}
