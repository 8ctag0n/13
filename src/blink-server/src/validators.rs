use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::Utc;
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use sqlx::PgPool;
use std::str::FromStr;
use tfhe::ServerKey;
use zyberlink_types::fhe::FhePredicate;

use crate::db::{NonceQueries, ServerKeyQueries};

/// Request structure for job validation
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ValidateJobRequest {
    pub creator_pubkey: String,
    pub encrypted_data: String, // base64
    /// Server key as base64 (legacy, large ~155MB base64)
    /// Use server_key_hash instead for pre-uploaded keys
    #[serde(default)]
    pub server_key: Option<String>,
    /// Hash of pre-uploaded server key (hex-encoded Blake2s256)
    /// Use this instead of server_key to avoid large payload uploads
    pub server_key_hash: Option<String>,
    pub message: String,        // "create_job:{job_id}:{timestamp}:{nonce}"
    pub signature: String,      // base64
    pub nonce: String,
    pub operation: String, // "add" | "multiply"
    pub operation_value: u8,
    pub price_lamports: u64,
    pub required_provers: u8,
    pub consensus_threshold: u8,
    #[serde(default = "default_payment_method")]
    pub payment_method: String, // "SOL" | "wZEC" (defaults to "SOL")
    pub expected_count: Option<u16>,
    pub predicate: Option<FhePredicate>,
}

/// Default payment method if not specified
fn default_payment_method() -> String {
    "SOL".to_string()
}

/// Validated job data ready for database insertion
#[derive(Debug, Clone)]
pub struct ValidatedJob {
    pub job_id: i64,
    pub encrypted_data: Vec<u8>,
    pub server_key: Vec<u8>,
    pub creator: Pubkey,
    pub operation: String,
    pub operation_value: u8,
    pub price_lamports: u64,
    pub required_provers: u8,
    pub consensus_threshold: u8,
    pub payment_method: String,
    pub payment_token_mint: Option<String>,
    pub expected_count: u16,
    pub predicate: Option<FhePredicate>,
}

/// Job validator with size limits and security checks
pub struct JobValidator;

impl JobValidator {
    // Size limits for encrypted data and server key
    const MAX_ENCRYPTED_DATA_SIZE: usize = 1024 * 1024; // 1 MB (TFHE encrypted data can be large)
    const MAX_SERVER_KEY_SIZE: usize = 120 * 1024 * 1024; // 120 MB
    const MIN_SERVER_KEY_SIZE: usize = 40 * 1024 * 1024; // 40 MB
    const MESSAGE_EXPIRY_SECS: i64 = 300; // 5 minutes

    /// Validate job request (all checks)
    pub async fn validate(req: &ValidateJobRequest, pool: &PgPool) -> Result<ValidatedJob> {
        // 1. Verify signature
        let job_id = Self::verify_signature(req)?;

        // 2. Verify timestamp (not expired)
        Self::verify_timestamp(&req.message)?;

        // 3. Verify nonce (anti-replay)
        Self::verify_nonce(&req.nonce, pool).await?;

        // 4. Decode encrypted_data (always required as base64)
        let encrypted_data = STANDARD
            .decode(&req.encrypted_data)
            .map_err(|e| anyhow!("Invalid base64 in encrypted_data: {}", e))?;

        // 5. Get server_key: either from hash (pre-uploaded) or from base64 (legacy)
        let server_key = Self::resolve_server_key(req, pool).await?;

        // 6. Validate sizes
        Self::validate_sizes(&encrypted_data, &server_key)?;

        // 7. Deserialize ServerKey (format validation) - skip if from hash (already validated)
        if req.server_key_hash.is_none() {
            let _server_key_obj: ServerKey = bincode::deserialize(&server_key)
                .map_err(|e| anyhow!("Invalid server_key format: {}", e))?;
        }

        // 7. Validate operation and related params
        Self::validate_operation(&req.operation)?;

        // 7b. Extract FHE operation parameters (defaults)
        let expected_count = req.expected_count.unwrap_or(1);
        if req.operation == "count_if" && req.predicate.is_none() {
            return Err(anyhow!("predicate is required for count_if operation"));
        }

        // 8. Validate consensus config
        Self::validate_consensus_config(req.required_provers, req.consensus_threshold)?;

        // 9. Parse creator pubkey
        let creator = Pubkey::from_str(&req.creator_pubkey)
            .map_err(|e| anyhow!("Invalid creator pubkey: {}", e))?;

        // 10. Validate payment method and determine token mint
        let (payment_method, payment_token_mint) =
            Self::validate_payment_method(&req.payment_method)?;

        Ok(ValidatedJob {
            job_id,
            encrypted_data,
            server_key,
            creator,
            operation: req.operation.clone(),
            operation_value: req.operation_value,
            price_lamports: req.price_lamports,
            required_provers: req.required_provers,
            consensus_threshold: req.consensus_threshold,
            payment_method,
            payment_token_mint,
            expected_count,
            predicate: req.predicate.clone(),
        })
    }

    /// Verify signature and extract job_id from message
    /// Message format: "create_job:{job_id}:{timestamp}:{nonce}"
    fn verify_signature(req: &ValidateJobRequest) -> Result<i64> {
        // Parse message to extract job_id
        let parts: Vec<&str> = req.message.split(':').collect();
        if parts.len() != 4 || parts[0] != "create_job" {
            return Err(anyhow!(
                "Invalid message format. Expected: create_job:{{job_id}}:{{timestamp}}:{{nonce}}"
            ));
        }

        let job_id = parts[1]
            .parse::<i64>()
            .map_err(|e| anyhow!("Invalid job_id in message: {}", e))?;

        // Parse creator pubkey
        let creator_pubkey = Pubkey::from_str(&req.creator_pubkey)
            .map_err(|e| anyhow!("Invalid creator pubkey: {}", e))?;

        // Decode signature from base58 (Solana standard format)
        let signature_bytes = bs58::decode(&req.signature)
            .into_vec()
            .map_err(|e| anyhow!("Invalid base58 signature: {}", e))?;

        if signature_bytes.len() != 64 {
            return Err(anyhow!(
                "Invalid signature length: expected 64 bytes, got {}",
                signature_bytes.len()
            ));
        }

        let mut sig_array = [0u8; 64];
        sig_array.copy_from_slice(&signature_bytes);
        let signature = Signature::from(sig_array);

        // Verify signature
        let message_bytes = req.message.as_bytes();
        if !signature.verify(creator_pubkey.as_ref(), message_bytes) {
            return Err(anyhow!("Invalid signature: verification failed"));
        }

        log::info!("Signature verified for job_id: {}", job_id);
        Ok(job_id)
    }

    /// Verify that the message timestamp is not expired
    /// Message format: "create_job:{job_id}:{timestamp}:{nonce}"
    fn verify_timestamp(message: &str) -> Result<()> {
        let parts: Vec<&str> = message.split(':').collect();
        if parts.len() != 4 {
            return Err(anyhow!("Invalid message format"));
        }

        let timestamp = parts[2]
            .parse::<i64>()
            .map_err(|e| anyhow!("Invalid timestamp: {}", e))?;

        let now = Utc::now().timestamp();
        let age = now - timestamp;

        if age < 0 {
            return Err(anyhow!("Timestamp is in the future"));
        }

        if age > Self::MESSAGE_EXPIRY_SECS {
            return Err(anyhow!(
                "Message expired (age: {}s, max: {}s)",
                age,
                Self::MESSAGE_EXPIRY_SECS
            ));
        }

        Ok(())
    }

    /// Verify nonce hasn't been used before (anti-replay)
    async fn verify_nonce(nonce: &str, pool: &PgPool) -> Result<()> {
        if nonce.is_empty() || nonce.len() > 128 {
            return Err(anyhow!("Invalid nonce length"));
        }

        // Check if nonce has been used
        if NonceQueries::is_nonce_used(pool, nonce).await? {
            return Err(anyhow!("Nonce already used (replay attack detected)"));
        }

        // Mark nonce as used
        NonceQueries::mark_nonce_used(pool, nonce).await?;

        Ok(())
    }

    /// Validate sizes of encrypted data and server key
    fn validate_sizes(encrypted_data: &[u8], server_key: &[u8]) -> Result<()> {
        if encrypted_data.is_empty() {
            return Err(anyhow!("Encrypted data is empty"));
        }

        if encrypted_data.len() > Self::MAX_ENCRYPTED_DATA_SIZE {
            return Err(anyhow!(
                "Encrypted data too large: {} bytes (max: {} bytes)",
                encrypted_data.len(),
                Self::MAX_ENCRYPTED_DATA_SIZE
            ));
        }

        if server_key.len() > Self::MAX_SERVER_KEY_SIZE {
            return Err(anyhow!(
                "Server key too large: {} bytes (max: {} bytes)",
                server_key.len(),
                Self::MAX_SERVER_KEY_SIZE
            ));
        }

        if server_key.len() < Self::MIN_SERVER_KEY_SIZE {
            return Err(anyhow!(
                "Server key too small: {} bytes (min: {} bytes)",
                server_key.len(),
                Self::MIN_SERVER_KEY_SIZE
            ));
        }

        Ok(())
    }

    /// Validate operation type
    fn validate_operation(operation: &str) -> Result<()> {
        match operation {
            "add" | "multiply" | "sum" | "count_if" => Ok(()),
            _ => Err(anyhow!(
                "Invalid operation: {}. Must be 'add', 'multiply', 'sum', or 'count_if'",
                operation
            )),
        }
    }

    /// Validate consensus configuration
    fn validate_consensus_config(required_provers: u8, consensus_threshold: u8) -> Result<()> {
        if required_provers == 0 {
            return Err(anyhow!("required_provers must be > 0"));
        }

        if consensus_threshold == 0 {
            return Err(anyhow!("consensus_threshold must be > 0"));
        }

        if consensus_threshold > required_provers {
            return Err(anyhow!(
                "consensus_threshold ({}) cannot be greater than required_provers ({})",
                consensus_threshold,
                required_provers
            ));
        }

        Ok(())
    }

    /// Validate payment method and return (payment_method, payment_token_mint)
    fn validate_payment_method(payment_method: &str) -> Result<(String, Option<String>)> {
        match payment_method {
            "SOL" => Ok(("SOL".to_string(), None)),
            "wZEC" => {
                // wZEC mint address on Solana mainnet
                let wzec_mint = "sXpG9BWgA6hxz9BTVLNTqWSHpbbQKa2LqKH6qD2fCAZ".to_string();
                Ok(("wZEC".to_string(), Some(wzec_mint)))
            }
            _ => Err(anyhow!(
                "Invalid payment_method: '{}'. Must be 'SOL' or 'wZEC'",
                payment_method
            )),
        }
    }

    /// Resolve server_key from either server_key_hash (pre-uploaded) or server_key (legacy base64)
    ///
    /// Priority:
    /// 1. If server_key_hash is provided, fetch from database
    /// 2. If server_key (base64) is provided, decode it
    /// 3. If neither, return error
    async fn resolve_server_key(req: &ValidateJobRequest, pool: &PgPool) -> Result<Vec<u8>> {
        // Option 1: Use pre-uploaded server key by hash
        if let Some(ref hash) = req.server_key_hash {
            log::info!("Resolving server key from hash: {}", hash);

            // Validate hash format (should be 64 hex chars for Blake2s256)
            if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(anyhow!("Invalid server_key_hash format: expected 64 hex characters"));
            }

            // Fetch from database
            match ServerKeyQueries::get_server_key(pool, hash).await? {
                Some(data) => {
                    log::info!("Server key found in database, size: {} bytes", data.len());
                    return Ok(data);
                }
                None => {
                    return Err(anyhow!(
                        "Server key not found for hash: {}. Please upload via POST /api/server-key/upload first.",
                        hash
                    ));
                }
            }
        }

        // Option 2: Use legacy base64-encoded server_key
        if let Some(ref server_key_b64) = req.server_key {
            if !server_key_b64.is_empty() {
                log::info!("Using legacy base64 server_key ({} chars)", server_key_b64.len());
                let server_key = STANDARD
                    .decode(server_key_b64)
                    .map_err(|e| anyhow!("Invalid base64 in server_key: {}", e))?;
                return Ok(server_key);
            }
        }

        // Neither provided
        Err(anyhow!(
            "Either server_key (base64) or server_key_hash must be provided. \
            For large keys, use POST /api/server-key/upload first to get a hash."
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_operation() {
        assert!(JobValidator::validate_operation("add").is_ok());
        assert!(JobValidator::validate_operation("multiply").is_ok());
        assert!(JobValidator::validate_operation("sum").is_ok());
        assert!(JobValidator::validate_operation("count_if").is_ok());
        assert!(JobValidator::validate_operation("divide").is_err());
    }

    #[test]
    fn test_validate_consensus_config() {
        assert!(JobValidator::validate_consensus_config(3, 2).is_ok());
        assert!(JobValidator::validate_consensus_config(1, 1).is_ok());
        assert!(JobValidator::validate_consensus_config(0, 1).is_err());
        assert!(JobValidator::validate_consensus_config(3, 0).is_err());
        assert!(JobValidator::validate_consensus_config(2, 3).is_err());
    }

    #[test]
    fn test_validate_payment_method() {
        // Test valid SOL
        let result = JobValidator::validate_payment_method("SOL");
        assert!(result.is_ok());
        let (method, mint) = result.unwrap();
        assert_eq!(method, "SOL");
        assert_eq!(mint, None);

        // Test valid wZEC
        let result = JobValidator::validate_payment_method("wZEC");
        assert!(result.is_ok());
        let (method, mint) = result.unwrap();
        assert_eq!(method, "wZEC");
        assert_eq!(
            mint,
            Some("sXpG9BWgA6hxz9BTVLNTqWSHpbbQKa2LqKH6qD2fCAZ".to_string())
        );

        // Test invalid payment method
        assert!(JobValidator::validate_payment_method("INVALID").is_err());
        assert!(JobValidator::validate_payment_method("BTC").is_err());
        assert!(JobValidator::validate_payment_method("").is_err());
        assert!(JobValidator::validate_payment_method("sol").is_err()); // Case sensitive
    }
}
