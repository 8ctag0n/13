use crate::FheError;
use bincode;
use tfhe::FheUint64;

/// Serializar un FheUint64 a bytes
pub fn serialize_ciphertext(ct: &FheUint64) -> Result<Vec<u8>, FheError> {
    bincode::serialize(ct)
        .map_err(|e| FheError::SerializationError(e.to_string()))
}

/// Deserializar bytes a FheUint64
pub fn deserialize_ciphertext(bytes: &[u8]) -> Result<FheUint64, FheError> {
    bincode::deserialize(bytes)
        .map_err(|e| FheError::DeserializationError(e.to_string()))
}

/// Convertir ciphertext a formato hexadecimal para transmisión
pub fn ciphertext_to_hex(ct: &FheUint64) -> Result<String, FheError> {
    let bytes = serialize_ciphertext(ct)?;
    Ok(hex::encode(bytes))
}

/// Convertir desde formato hexadecimal a ciphertext
pub fn ciphertext_from_hex(hex_str: &str) -> Result<FheUint64, FheError> {
    let bytes = hex::decode(hex_str)
        .map_err(|e| FheError::DeserializationError(e.to_string()))?;
    deserialize_ciphertext(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FutarchyFheClient;

    #[test]
    fn test_ciphertext_serialization_roundtrip() {
        let client = FutarchyFheClient::new().expect("Failed to create client");
        let amount: u64 = 1234;

        // Encriptar
        let encrypted_bytes = client.encrypt_bet_amount(amount)
            .expect("Failed to encrypt");

        // Deserializar el ciphertext
        let ct = deserialize_ciphertext(&encrypted_bytes)
            .expect("Failed to deserialize");

        // Re-serializar
        let reserialized = serialize_ciphertext(&ct)
            .expect("Failed to serialize");

        // Debe ser idéntico
        assert_eq!(encrypted_bytes, reserialized);
    }

    #[test]
    fn test_hex_format_conversion() {
        let client = FutarchyFheClient::new().expect("Failed to create client");
        let amount: u64 = 5678;

        let encrypted_bytes = client.encrypt_bet_amount(amount)
            .expect("Failed to encrypt");

        let ct = deserialize_ciphertext(&encrypted_bytes)
            .expect("Failed to deserialize");

        // Convertir a hex y de vuelta
        let hex = ciphertext_to_hex(&ct).expect("Failed to convert to hex");
        let restored_ct = ciphertext_from_hex(&hex)
            .expect("Failed to restore from hex");

        // Reserializar y comparar
        let restored_bytes = serialize_ciphertext(&restored_ct)
            .expect("Failed to serialize restored");

        assert_eq!(encrypted_bytes, restored_bytes);
    }
}
