//! Prover Authentication via Solana Signature
//!
//! Provers authenticate using ed25519 signatures from their Solana keypair.
//! This provides cryptographic proof of identity without requiring JWT tokens.

use actix_web::{dev::Payload, error::ErrorUnauthorized, Error, FromRequest, HttpRequest};
use blake2::{Blake2s256, Digest};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use std::future::{ready, Ready};
use std::str::FromStr;
use zyberlink_chain_client::SignatureVerifier;

/// Maximum allowed timestamp drift (5 minutes)
const MAX_TIMESTAMP_DRIFT_SECS: i64 = 300;

/// Prover authentication extractor
///
/// Validates ed25519 signature from Solana prover keypair.
/// Required headers:
///   - X-Prover-Pubkey: base58 encoded pubkey
///   - X-Prover-Signature: base58 encoded signature
///   - X-Timestamp: unix timestamp of request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProverAuth {
    pub pubkey: Pubkey,
}

impl ProverAuth {
    /// Verify a prover's signature
    ///
    /// The signed message format is: "{method}:{path}:{timestamp}:{body_hash}"
    /// Example: "POST:/gateway/prover/zk/123/submit:1702500000:abc123..."
    pub fn verify_signature(
        pubkey_b58: &str,
        signature_b58: &str,
        message: &[u8],
    ) -> Result<bool, Error> {
        // Parse pubkey
        let pubkey = Pubkey::from_str(pubkey_b58)
            .map_err(|e| ErrorUnauthorized(format!("Invalid pubkey: {}", e)))?;

        // Parse signature
        let signature = Signature::from_str(signature_b58)
            .map_err(|e| ErrorUnauthorized(format!("Invalid signature: {}", e)))?;

        // Verify signature
        let valid = signature.verify(pubkey.as_ref(), message);

        Ok(valid)
    }

    /// Verify signature using chain-agnostic SignatureVerifier trait
    ///
    /// New recommended method supporting multiple chains.
    /// This method provides a generic interface for signature verification
    /// that works across different blockchain implementations.
    pub fn verify_signature_generic<V: SignatureVerifier>(
        verifier: &V,
        pubkey: &str,
        signature: &str,
        message: &[u8],
    ) -> Result<bool, Error> {
        verifier
            .verify_signature(pubkey, signature, message)
            .map_err(|e| ErrorUnauthorized(format!("Signature verification failed: {}", e)))
    }

    /// Build the message to sign for a request
    pub fn build_message(method: &str, path: &str, timestamp: i64, body: &[u8]) -> Vec<u8> {
        // Hash the body
        let mut hasher = Blake2s256::new();
        hasher.update(body);
        let body_hash = hex::encode(hasher.finalize());

        // Build message: "METHOD:PATH:TIMESTAMP:BODY_HASH"
        format!("{}:{}:{}:{}", method, path, timestamp, body_hash).into_bytes()
    }

    /// Validate timestamp is within acceptable range
    fn validate_timestamp(timestamp: i64) -> Result<(), Error> {
        let now = chrono::Utc::now().timestamp();
        let diff = (now - timestamp).abs();

        if diff > MAX_TIMESTAMP_DRIFT_SECS {
            return Err(ErrorUnauthorized(format!(
                "Timestamp drift too large: {} seconds",
                diff
            )));
        }

        Ok(())
    }
}

/// Actix-web FromRequest implementation
///
/// Extracts and validates prover authentication from request headers.
impl FromRequest for ProverAuth {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        // Extract headers
        let pubkey_header = match req.headers().get("X-Prover-Pubkey") {
            Some(h) => match h.to_str() {
                Ok(s) => s.to_string(),
                Err(_) => {
                    return ready(Err(ErrorUnauthorized("Invalid X-Prover-Pubkey header")))
                }
            },
            None => return ready(Err(ErrorUnauthorized("Missing X-Prover-Pubkey header"))),
        };

        let signature_header = match req.headers().get("X-Prover-Signature") {
            Some(h) => match h.to_str() {
                Ok(s) => s.to_string(),
                Err(_) => {
                    return ready(Err(ErrorUnauthorized("Invalid X-Prover-Signature header")))
                }
            },
            None => return ready(Err(ErrorUnauthorized("Missing X-Prover-Signature header"))),
        };

        let timestamp_str = match req.headers().get("X-Timestamp") {
            Some(h) => match h.to_str() {
                Ok(s) => s.to_string(),
                Err(_) => return ready(Err(ErrorUnauthorized("Invalid X-Timestamp header"))),
            },
            None => return ready(Err(ErrorUnauthorized("Missing X-Timestamp header"))),
        };

        // Parse timestamp
        let timestamp: i64 = match timestamp_str.parse() {
            Ok(t) => t,
            Err(_) => return ready(Err(ErrorUnauthorized("Invalid timestamp format"))),
        };

        // Validate timestamp
        if let Err(e) = Self::validate_timestamp(timestamp) {
            return ready(Err(e));
        }

        // Build message to verify
        // Note: For GET requests, body is empty
        let method = req.method().as_str();
        let path = req.path();
        let body = vec![]; // We'll extract body in the handler if needed

        let message = Self::build_message(method, path, timestamp, &body);

        // Verify signature
        match Self::verify_signature(&pubkey_header, &signature_header, &message) {
            Ok(true) => {
                // Parse pubkey for return value
                match Pubkey::from_str(&pubkey_header) {
                    Ok(pubkey) => ready(Ok(ProverAuth { pubkey })),
                    Err(e) => ready(Err(ErrorUnauthorized(format!("Invalid pubkey: {}", e)))),
                }
            }
            Ok(false) => ready(Err(ErrorUnauthorized("Signature verification failed"))),
            Err(e) => ready(Err(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_message() {
        let method = "POST";
        let path = "/gateway/prover/zk/123/submit";
        let timestamp = 1702500000;
        let body = b"test payload";

        let message = ProverAuth::build_message(method, path, timestamp, body);
        let message_str = String::from_utf8(message).unwrap();

        assert!(message_str.starts_with("POST:/gateway/prover/zk/123/submit:1702500000:"));
        assert_eq!(message_str.matches(':').count(), 3);
    }

    #[test]
    fn test_validate_timestamp() {
        // Current timestamp should be valid
        let now = chrono::Utc::now().timestamp();
        assert!(ProverAuth::validate_timestamp(now).is_ok());

        // 4 minutes ago should be valid
        assert!(ProverAuth::validate_timestamp(now - 240).is_ok());

        // 10 minutes ago should be invalid
        assert!(ProverAuth::validate_timestamp(now - 600).is_err());

        // 10 minutes in the future should be invalid
        assert!(ProverAuth::validate_timestamp(now + 600).is_err());
    }
}
