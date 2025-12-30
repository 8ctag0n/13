use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::bs58;
use blake2::{Blake2s256, Digest};

/// Genera headers de autenticación para requests al gateway
///
/// El mensaje firmado es: "{METHOD}:{PATH}:{TIMESTAMP}:{BODY_HASH}"
/// Note: x402's FromRequest can't access body, so we use empty body hash for consistency
pub fn sign_gateway_request(
    keypair: &Keypair,
    method: &str,
    path: &str,
    _body: &[u8], // Note: body not included in signature due to x402 FromRequest limitation
) -> GatewayAuthHeaders {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Always use empty body hash - x402's FromRequest can't access the body stream
    // so both sides must use empty body to match
    let mut hasher = Blake2s256::new();
    hasher.update(b""); // empty body
    let body_hash = hex::encode(hasher.finalize());

    let message = format!("{}:{}:{}:{}", method, path, timestamp, body_hash);
    let signature = keypair.sign_message(message.as_bytes());

    GatewayAuthHeaders {
        pubkey: keypair.pubkey().to_string(),
        signature: bs58::encode(signature.as_ref()).into_string(),
        timestamp: timestamp.to_string(),
    }
}

pub struct GatewayAuthHeaders {
    pub pubkey: String,
    pub signature: String,
    pub timestamp: String,
}

impl GatewayAuthHeaders {
    pub fn apply_to_request(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        builder
            .header("X-Prover-Pubkey", &self.pubkey)
            .header("X-Prover-Signature", &self.signature)
            .header("X-Timestamp", &self.timestamp)
    }
}
