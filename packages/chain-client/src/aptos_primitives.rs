//! Minimal Aptos transaction primitives
//!
//! This module provides the minimum necessary structures and functions to build,
//! sign, and submit transactions to Aptos blockchain without the full SDK.

use serde::{Deserialize, Serialize};

#[cfg(feature = "aptos")]
use {
    ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey},
    sha3::{Digest, Sha3_256},
};

/// Aptos account address (32 bytes)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountAddress([u8; 32]);

impl AccountAddress {
    /// Parse address from hex string (with or without 0x prefix)
    pub fn from_hex(s: &str) -> Result<Self, String> {
        let s = s.strip_prefix("0x").unwrap_or(s);

        // Pad short addresses with leading zeros
        let padded = if s.len() < 64 {
            format!("{:0>64}", s)
        } else if s.len() == 64 {
            s.to_string()
        } else {
            return Err(format!("Address too long: {} chars (max 64)", s.len()));
        };

        let mut bytes = [0u8; 32];
        hex::decode_to_slice(&padded, &mut bytes)
            .map_err(|e| format!("Invalid hex: {}", e))?;

        Ok(Self(bytes))
    }

    /// Get address as hex string with 0x prefix
    pub fn to_hex(&self) -> String {
        format!("0x{}", hex::encode(&self.0))
    }

    /// Get raw bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Standard addresses
    pub fn zero() -> Self {
        Self([0u8; 32])
    }

    pub fn one() -> Self {
        let mut bytes = [0u8; 32];
        bytes[31] = 1;
        Self(bytes)
    }
}

/// Entry function payload for smart contract execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryFunction {
    pub module: ModuleId,
    pub function: String,
    pub ty_args: Vec<TypeTag>,
    pub args: Vec<Vec<u8>>,
}

/// Module identifier (address + name)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleId {
    pub address: AccountAddress,
    pub name: String,
}

/// Type tag for generic types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeTag {
    Bool,
    U8,
    U64,
    U128,
    Address,
    Signer,
    Vector(Box<TypeTag>),
    Struct(StructTag),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructTag {
    pub address: AccountAddress,
    pub module: String,
    pub name: String,
    pub type_args: Vec<TypeTag>,
}

/// Transaction payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionPayload {
    EntryFunction(EntryFunction),
}

/// Raw transaction (before signing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawTransaction {
    pub sender: AccountAddress,
    pub sequence_number: u64,
    pub payload: TransactionPayload,
    pub max_gas_amount: u64,
    pub gas_unit_price: u64,
    pub expiration_timestamp_secs: u64,
    pub chain_id: u8,
}

impl RawTransaction {
    /// Create a simple entry function transaction
    pub fn new_entry_function(
        sender: AccountAddress,
        sequence_number: u64,
        module_address: &str,
        module_name: &str,
        function_name: &str,
        ty_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
        chain_id: u8,
    ) -> Result<Self, String> {
        let module_addr = AccountAddress::from_hex(module_address)?;

        let payload = TransactionPayload::EntryFunction(EntryFunction {
            module: ModuleId {
                address: module_addr,
                name: module_name.to_string(),
            },
            function: function_name.to_string(),
            ty_args,
            args,
        });

        Ok(Self {
            sender,
            sequence_number,
            payload,
            max_gas_amount: 200_000,        // Conservative default
            gas_unit_price: 100,            // Standard price
            expiration_timestamp_secs: current_timestamp() + 600, // 10 min
            chain_id,
        })
    }

    #[cfg(feature = "aptos")]
    /// Sign this transaction with a private key
    pub fn sign(&self, private_key: &SigningKey) -> Result<SignedTransaction, String> {
        // Serialize transaction for signing (prefix + BCS encoding)
        let mut signing_message = Vec::new();
        signing_message.extend_from_slice(b"APTOS::RawTransaction");

        let tx_bytes = bcs::to_bytes(self)
            .map_err(|e| format!("BCS serialization failed: {}", e))?;
        signing_message.extend_from_slice(&tx_bytes);

        // Hash the signing message with SHA3-256
        let mut hasher = Sha3_256::new();
        hasher.update(&signing_message);
        let hash = hasher.finalize();

        // Sign the hash
        let signature = private_key.sign(&hash);

        // Get public key
        let public_key = private_key.verifying_key();

        Ok(SignedTransaction {
            raw_txn: self.clone(),
            authenticator: TransactionAuthenticator::Ed25519(AccountAuthenticatorEd25519 {
                public_key: Ed25519PublicKey(public_key.to_bytes()),
                signature: Ed25519Signature(signature.to_bytes()),
            }),
        })
    }

    #[cfg(not(feature = "aptos"))]
    pub fn sign(&self, _private_key: &()) -> Result<SignedTransaction, String> {
        Err("Signing requires 'aptos' feature".to_string())
    }
}

/// Signed transaction (ready for submission)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTransaction {
    pub raw_txn: RawTransaction,
    pub authenticator: TransactionAuthenticator,
}

impl SignedTransaction {
    /// Serialize to BCS for submission
    pub fn to_bcs(&self) -> Result<Vec<u8>, String> {
        bcs::to_bytes(self).map_err(|e| format!("BCS serialization failed: {}", e))
    }
}

/// Ed25519 public key wrapper for BCS serialization
#[derive(Debug, Clone)]
pub struct Ed25519PublicKey(pub [u8; 32]);

impl Serialize for Ed25519PublicKey {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // BCS expects the raw bytes with a length prefix for vectors
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for Ed25519PublicKey {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bytes: Vec<u8> = Vec::deserialize(deserializer)?;
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

/// Ed25519 signature wrapper for BCS serialization
#[derive(Debug, Clone)]
pub struct Ed25519Signature(pub [u8; 64]);

impl Serialize for Ed25519Signature {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for Ed25519Signature {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bytes: Vec<u8> = Vec::deserialize(deserializer)?;
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

/// Account authenticator for Ed25519
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountAuthenticatorEd25519 {
    pub public_key: Ed25519PublicKey,
    pub signature: Ed25519Signature,
}

/// Transaction authenticator (signature scheme)
/// BCS format: variant_index (ULEB128) + variant_data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionAuthenticator {
    Ed25519(AccountAuthenticatorEd25519),
}

/// Aptos account with keypair
#[cfg(feature = "aptos")]
pub struct AptosAccount {
    pub private_key: SigningKey,
    pub address: AccountAddress,
}

#[cfg(feature = "aptos")]
impl AptosAccount {
    /// Create a new random account
    pub fn generate() -> Self {
        use rand::RngCore;

        let mut rng = rand::rngs::OsRng;
        let mut key_bytes = [0u8; 32];
        rng.fill_bytes(&mut key_bytes);

        let private_key = SigningKey::from_bytes(&key_bytes);

        // Derive address from public key
        let public_key = private_key.verifying_key();
        let address = derive_address_from_public_key(&public_key);

        Self {
            private_key,
            address,
        }
    }

    /// Create account from hex private key
    pub fn from_private_key_hex(hex_key: &str) -> Result<Self, String> {
        let key_bytes = hex::decode(hex_key.strip_prefix("0x").unwrap_or(hex_key))
            .map_err(|e| format!("Invalid hex key: {}", e))?;

        if key_bytes.len() != 32 {
            return Err(format!("Private key must be 32 bytes, got {}", key_bytes.len()));
        }

        let mut key_array = [0u8; 32];
        key_array.copy_from_slice(&key_bytes);

        let private_key = SigningKey::from_bytes(&key_array);
        let public_key = private_key.verifying_key();
        let address = derive_address_from_public_key(&public_key);

        Ok(Self {
            private_key,
            address,
        })
    }

    /// Get private key as hex
    pub fn private_key_hex(&self) -> String {
        hex::encode(self.private_key.to_bytes())
    }

    /// Get address as hex
    pub fn address_hex(&self) -> String {
        self.address.to_hex()
    }
}

/// Derive Aptos address from Ed25519 public key
#[cfg(feature = "aptos")]
fn derive_address_from_public_key(public_key: &VerifyingKey) -> AccountAddress {
    let mut hasher = Sha3_256::new();

    // Hash: public_key || 0x00 (single signature scheme)
    hasher.update(public_key.as_bytes());
    hasher.update(&[0x00]); // Single signature scheme

    let hash = hasher.finalize();
    let mut address = [0u8; 32];
    address.copy_from_slice(&hash[..]);

    AccountAddress(address)
}

/// Get current Unix timestamp
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_parsing() {
        let addr1 = AccountAddress::from_hex(
            "0x0000000000000000000000000000000000000000000000000000000000000001"
        ).unwrap();
        assert_eq!(addr1, AccountAddress::one());

        let addr2 = AccountAddress::from_hex(
            "0000000000000000000000000000000000000000000000000000000000000001"
        ).unwrap();
        assert_eq!(addr2, AccountAddress::one());
    }

    #[test]
    #[cfg(feature = "aptos")]
    fn test_account_generation() {
        let account = AptosAccount::generate();
        assert_eq!(account.private_key_hex().len(), 64); // 32 bytes = 64 hex chars
        assert!(account.address_hex().starts_with("0x"));
    }
}
