use serde::{Deserialize, Serialize};

/// Datos serializados de la client key
/// PRIVADA - El usuario debe mantenerla segura
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientKeyData {
    pub bytes: Vec<u8>,
}

impl ClientKeyData {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Guardar en formato hexadecimal para facilitar almacenamiento
    pub fn to_hex(&self) -> String {
        hex::encode(&self.bytes)
    }

    /// Cargar desde formato hexadecimal
    pub fn from_hex(hex_str: &str) -> Result<Self, crate::FheError> {
        let bytes = hex::decode(hex_str)
            .map_err(|e| crate::FheError::DeserializationError(e.to_string()))?;
        Ok(Self { bytes })
    }
}

/// Datos serializados de la server key
/// PÚBLICA - Los provers necesitan esto para computación homomorfica
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerKeyData {
    pub bytes: Vec<u8>,
}

impl ServerKeyData {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Guardar en formato hexadecimal
    pub fn to_hex(&self) -> String {
        hex::encode(&self.bytes)
    }

    /// Cargar desde formato hexadecimal
    pub fn from_hex(hex_str: &str) -> Result<Self, crate::FheError> {
        let bytes = hex::decode(hex_str)
            .map_err(|e| crate::FheError::DeserializationError(e.to_string()))?;
        Ok(Self { bytes })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_key_data_serialization() {
        let original_bytes = vec![1, 2, 3, 4, 5];
        let key_data = ClientKeyData::new(original_bytes.clone());

        assert_eq!(key_data.as_bytes(), &original_bytes);
    }

    #[test]
    fn test_hex_roundtrip() {
        let original_bytes = vec![0xde, 0xad, 0xbe, 0xef];
        let key_data = ClientKeyData::new(original_bytes.clone());

        let hex = key_data.to_hex();
        let restored = ClientKeyData::from_hex(&hex).expect("Failed to restore from hex");

        assert_eq!(restored.as_bytes(), &original_bytes);
    }
}
