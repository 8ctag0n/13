//! Signature verification abstraction for multi-chain support

use crate::error::Result;

/// Trait for verifying cryptographic signatures across different chains
///
/// Each chain implementation handles its own encoding and signature algorithm:
/// - Solana: ed25519, base58 encoding
/// - Aptos: ed25519, hex encoding
/// - Starknet: ECDSA over STARK curve, hex encoding
pub trait SignatureVerifier: Send + Sync {
    /// Verify a signature against a public key and message
    ///
    /// # Arguments
    /// * `public_key` - Public key as string (base58 for Solana, hex for others)
    /// * `signature` - Signature as string (base58 for Solana, hex for others)
    /// * `message` - Raw message bytes that were signed
    ///
    /// # Returns
    /// * `Ok(true)` - Signature is valid
    /// * `Ok(false)` - Signature is invalid
    /// * `Err(_)` - Parse error (malformed pubkey/signature)
    fn verify_signature(
        &self,
        public_key: &str,
        signature: &str,
        message: &[u8],
    ) -> Result<bool>;

    /// Get the signature encoding format used by this chain
    fn signature_encoding(&self) -> &str;

    /// Get the signature algorithm used by this chain
    fn signature_algorithm(&self) -> &str;
}
