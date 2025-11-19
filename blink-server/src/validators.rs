use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::Utc;
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use sqlx::PgPool;
use std::str::FromStr;
use tfhe::ServerKey;

use crate::db::NonceQueries;

/// Request structure for job validation
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ValidateJobRequest {
    pub creator_pubkey: String,
    pub encrypted_data: String,  // base64
    pub server_key: String,      // base64
    pub message: String,         // "create_job:{job_id}:{timestamp}:{nonce}"
    pub signature: String,       // base64
    pub nonce: String,
    pub operation: String,       // "add" | "multiply"
    pub operation_value: u8,
    pub price_lamports: u64,
    pub required_provers: u8,
    pub consensus_threshold: u8,
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
}

/// Job validator with size limits and security checks
pub struct JobValidator;

impl JobValidator {
    // Size limits for encrypted data and server key
    const MAX_ENCRYPTED_DATA_SIZE: usize = 10 * 1024; // 10 KB
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

        // 4. Decode base64
        let encrypted_data = STANDARD.decode(&req.encrypted_data)
            .map_err(|e| anyhow!("Invalid base64 in encrypted_data: {}", e))?;
        let server_key = STANDARD.decode(&req.server_key)
            .map_err(|e| anyhow!("Invalid base64 in server_key: {}", e))?;

        // 5. Validate sizes
        Self::validate_sizes(&encrypted_data, &server_key)?;

        // 6. Deserialize ServerKey (format validation)
        let _server_key_obj: ServerKey = bincode::deserialize(&server_key)
            .map_err(|e| anyhow!("Invalid server_key format: {}", e))?;

        // 7. Validate operation
        Self::validate_operation(&req.operation)?;

        // 8. Validate consensus config
        Self::validate_consensus_config(req.required_provers, req.consensus_threshold)?;

        // 9. Parse creator pubkey
        let creator = Pubkey::from_str(&req.creator_pubkey)
            .map_err(|e| anyhow!("Invalid creator pubkey: {}", e))?;

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
        })
    }

    /// Verify signature and extract job_id from message
    /// Message format: "create_job:{job_id}:{timestamp}:{nonce}"
    fn verify_signature(req: &ValidateJobRequest) -> Result<i64> {
        // Parse message to extract job_id
        let parts: Vec<&str> = req.message.split(':').collect();
        if parts.len() != 4 || parts[0] != "create_job" {
            return Err(anyhow!("Invalid message format. Expected: create_job:{{job_id}}:{{timestamp}}:{{nonce}}"));
        }

        let job_id = parts[1].parse::<i64>()
            .map_err(|e| anyhow!("Invalid job_id in message: {}", e))?;

        // Parse creator pubkey
        let creator_pubkey = Pubkey::from_str(&req.creator_pubkey)
            .map_err(|e| anyhow!("Invalid creator pubkey: {}", e))?;

        // Decode signature
        let signature_bytes = STANDARD.decode(&req.signature)
            .map_err(|e| anyhow!("Invalid base64 signature: {}", e))?;

        if signature_bytes.len() != 64 {
            return Err(anyhow!("Invalid signature length: expected 64 bytes, got {}", signature_bytes.len()));
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

        let timestamp = parts[2].parse::<i64>()
            .map_err(|e| anyhow!("Invalid timestamp: {}", e))?;

        let now = Utc::now().timestamp();
        let age = now - timestamp;

        if age < 0 {
            return Err(anyhow!("Timestamp is in the future"));
        }

        if age > Self::MESSAGE_EXPIRY_SECS {
            return Err(anyhow!("Message expired (age: {}s, max: {}s)", age, Self::MESSAGE_EXPIRY_SECS));
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
            "add" | "multiply" => Ok(()),
            _ => Err(anyhow!("Invalid operation: {}. Must be 'add' or 'multiply'", operation)),
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_operation() {
        assert!(JobValidator::validate_operation("add").is_ok());
        assert!(JobValidator::validate_operation("multiply").is_ok());
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
}
