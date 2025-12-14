use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::bs58;
use blake2::{Blake2s256, Digest};

/// Genera headers de autenticación para requests al gateway
///
/// El mensaje firmado es: "{METHOD}:{PATH}:{TIMESTAMP}:{BODY_HASH}"
pub fn sign_gateway_request(
    keypair: &Keypair,
    method: &str,
    path: &str,
    body: &[u8],
) -> GatewayAuthHeaders {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let body_hash = if body.is_empty() {
        String::new()
    } else {
        let mut hasher = Blake2s256::new();
        hasher.update(body);
        hex::encode(hasher.finalize())
    };

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
