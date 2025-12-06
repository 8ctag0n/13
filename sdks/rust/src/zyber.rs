//! # Zyber - Simple FHE Client
//!
//! High-level client for ZyberLink that makes privacy-preserving computation trivial.
//!
//! ## Example
//!
//! ```ignore
//! use zyberlink_sdk::Zyber;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Connect to network
//!     let zyber = Zyber::connect("devnet").await?;
//!
//!     // Compute sum of private values
//!     let sum = zyber.sum(&[100, 200, 300]).await?;
//!     println!("Sum: {}", sum);
//!
//!     Ok(())
//! }
//! ```

use anyhow::{anyhow, Context, Result};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{read_keypair_file, Keypair, Signer},
    transaction::Transaction,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::client::MarketplaceClient;
use crate::config::{default_keypair_path, keys_dir, NetworkConfig};
use zyberlink_types::{FheConsensusConfig, FheOperation, FhePredicate};

/// High-level ZyberLink client
///
/// Provides a simple interface for privacy-preserving computation.
/// All complexity (keys, encryption, transactions, polling) is handled internally.
pub struct Zyber {
    /// Network configuration
    config: NetworkConfig,
    /// Low-level marketplace client
    client: MarketplaceClient,
    /// User keypair for signing transactions
    keypair: Arc<Keypair>,
    /// HTTP client for backend
    http: reqwest::Client,
    /// FHE client key (cached)
    fhe_client_key: Option<tfhe::ClientKey>,
    /// FHE server key (cached)
    fhe_server_key: Option<tfhe::ServerKey>,
    /// Job timeout
    timeout: Duration,
    /// Number of provers for consensus
    required_provers: u8,
    /// Price per job in lamports
    price_lamports: u64,
}

/// Builder for Zyber client
pub struct ZyberBuilder {
    network: Option<String>,
    rpc_url: Option<String>,
    program_id: Option<solana_sdk::pubkey::Pubkey>,
    backend_url: Option<String>,
    keypair_path: Option<PathBuf>,
    timeout: Duration,
    required_provers: u8,
    price_lamports: u64,
}

impl Default for ZyberBuilder {
    fn default() -> Self {
        Self {
            network: None,
            rpc_url: None,
            program_id: None,
            backend_url: None,
            keypair_path: None,
            timeout: Duration::from_secs(120),
            required_provers: 2,
            price_lamports: 10_000_000, // 0.01 SOL default
        }
    }
}

impl ZyberBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set network by name (devnet, mainnet, localnet)
    pub fn network(mut self, name: &str) -> Self {
        self.network = Some(name.to_string());
        self
    }

    /// Set custom RPC URL
    pub fn rpc_url(mut self, url: &str) -> Self {
        self.rpc_url = Some(url.to_string());
        self
    }

    /// Set custom program ID
    pub fn program_id(mut self, id: solana_sdk::pubkey::Pubkey) -> Self {
        self.program_id = Some(id);
        self
    }

    /// Set custom backend URL
    pub fn backend_url(mut self, url: &str) -> Self {
        self.backend_url = Some(url.to_string());
        self
    }

    /// Set keypair path
    pub fn keypair(mut self, path: &str) -> Self {
        self.keypair_path = Some(PathBuf::from(path));
        self
    }

    /// Set job timeout
    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = duration;
        self
    }

    /// Set number of required provers for consensus
    pub fn provers(mut self, count: u8) -> Self {
        self.required_provers = count;
        self
    }

    /// Set price per job in lamports
    pub fn price(mut self, lamports: u64) -> Self {
        self.price_lamports = lamports;
        self
    }

    /// Set price per job in SOL
    pub fn price_sol(mut self, sol: f64) -> Self {
        self.price_lamports = (sol * 1_000_000_000.0) as u64;
        self
    }

    /// Build the Zyber client
    pub async fn build(self) -> Result<Zyber> {
        // Determine network config
        let mut config = if let Some(network) = &self.network {
            NetworkConfig::from_name(network)
                .ok_or_else(|| anyhow!("Unknown network: {}", network))?
        } else {
            NetworkConfig::localnet()
        };

        // Override with custom values if provided
        if let Some(rpc_url) = self.rpc_url {
            config.rpc_url = rpc_url;
        }
        if let Some(program_id) = self.program_id {
            config.program_id = program_id;
        }
        if let Some(backend_url) = self.backend_url {
            config.backend_url = backend_url;
        }

        // Load keypair
        let keypair_path = self.keypair_path.unwrap_or_else(default_keypair_path);
        let keypair = read_keypair_file(&keypair_path)
            .map_err(|e| anyhow!("Failed to load keypair from {:?}: {}", keypair_path, e))?;

        // Create low-level client
        let client = MarketplaceClient::new_with_commitment(
            config.rpc_url.clone(),
            config.program_id,
            CommitmentConfig::confirmed(),
        );

        // Create HTTP client
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Zyber {
            config,
            client,
            keypair: Arc::new(keypair),
            http,
            fhe_client_key: None,
            fhe_server_key: None,
            timeout: self.timeout,
            required_provers: self.required_provers,
            price_lamports: self.price_lamports,
        })
    }
}

impl Zyber {
    /// Connect to a network with default settings
    ///
    /// # Example
    /// ```ignore
    /// let zyber = Zyber::connect("devnet").await?;
    /// ```
    pub async fn connect(network: &str) -> Result<Self> {
        ZyberBuilder::new().network(network).build().await
    }

    /// Create a builder for custom configuration
    pub fn builder() -> ZyberBuilder {
        ZyberBuilder::new()
    }

    /// Get the user's public key
    pub fn pubkey(&self) -> solana_sdk::pubkey::Pubkey {
        self.keypair.pubkey()
    }

    /// Get the network name
    pub fn network(&self) -> &str {
        &self.config.name
    }

    // =========================================================================
    // FHE Key Management (internal)
    // =========================================================================

    /// Ensure FHE keys are loaded (generate if needed)
    async fn ensure_keys(&mut self) -> Result<()> {
        if self.fhe_client_key.is_some() && self.fhe_server_key.is_some() {
            return Ok(());
        }

        // Try to load from cache
        let keys_path = keys_dir();
        let client_key_path = keys_path.join("client_key.bin");
        let server_key_path = keys_path.join("server_key.bin");

        if client_key_path.exists() && server_key_path.exists() {
            // Load cached keys
            let client_bytes = std::fs::read(&client_key_path)
                .context("Failed to read cached client key")?;
            let server_bytes = std::fs::read(&server_key_path)
                .context("Failed to read cached server key")?;

            self.fhe_client_key = Some(
                zyberlink_fhe::deserialize_client_key(&client_bytes)
                    .context("Failed to deserialize client key")?,
            );
            self.fhe_server_key = Some(
                zyberlink_fhe::deserialize_server_key(&server_bytes)
                    .context("Failed to deserialize server key")?,
            );
        } else {
            // Generate new keys
            let (client_key, server_key) =
                zyberlink_fhe::generate_keys().context("Failed to generate FHE keys")?;

            // Cache them
            std::fs::create_dir_all(&keys_path)?;
            std::fs::write(
                &client_key_path,
                zyberlink_fhe::serialize_client_key(&client_key)?,
            )?;
            std::fs::write(
                &server_key_path,
                zyberlink_fhe::serialize_server_key(&server_key)?,
            )?;

            self.fhe_client_key = Some(client_key);
            self.fhe_server_key = Some(server_key);
        }

        Ok(())
    }

    /// Get client key (ensures keys exist)
    fn client_key(&self) -> Result<&tfhe::ClientKey> {
        self.fhe_client_key
            .as_ref()
            .ok_or_else(|| anyhow!("FHE keys not initialized. Call ensure_keys() first."))
    }

    /// Get server key (ensures keys exist)
    fn server_key(&self) -> Result<&tfhe::ServerKey> {
        self.fhe_server_key
            .as_ref()
            .ok_or_else(|| anyhow!("FHE keys not initialized. Call ensure_keys() first."))
    }

    // =========================================================================
    // High-Level Operations
    // =========================================================================

    /// Compute sum of values
    ///
    /// # Example
    /// ```ignore
    /// let sum = zyber.sum(&[100, 200, 300]).await?;
    /// assert_eq!(sum, 600);
    /// ```
    pub async fn sum(&mut self, values: &[u8]) -> Result<u64> {
        let op = FheOperation::Sum {
            expected_count: values.len() as u16,
        };
        self.compute(op, values).await
    }

    /// Compute average of values
    ///
    /// # Example
    /// ```ignore
    /// let avg = zyber.average(&[10, 20, 30]).await?;
    /// assert_eq!(avg, 20);
    /// ```
    pub async fn average(&mut self, values: &[u8]) -> Result<u64> {
        let op = FheOperation::Sum {
            expected_count: values.len() as u16,
        };
        let sum = self.compute(op, values).await?;
        Ok(sum / values.len() as u64)
    }

    /// Count values matching a condition
    ///
    /// # Example
    /// ```ignore
    /// // Count values greater than 50
    /// let count = zyber.count_if(&values, ">", 50).await?;
    /// ```
    pub async fn count_if(&mut self, values: &[u8], op: &str, threshold: u8) -> Result<u64> {
        let predicate = match op {
            ">" | "gt" => FhePredicate::GreaterThan(threshold),
            "<" | "lt" => FhePredicate::LessThan(threshold),
            "==" | "eq" | "=" => FhePredicate::Equals(threshold),
            "!=" | "ne" => FhePredicate::NotEquals(threshold),
            _ => return Err(anyhow!("Unknown operator: {}. Use >, <, ==, or !=", op)),
        };

        let operation = FheOperation::CountIf {
            predicate,
            expected_count: values.len() as u16,
        };
        self.compute(operation, values).await
    }

    /// Check if a value meets a threshold
    ///
    /// # Example
    /// ```ignore
    /// let is_adult = zyber.threshold(&[age], ">=", 18).await?;
    /// ```
    pub async fn threshold(&mut self, values: &[u8], op: &str, threshold: u8) -> Result<bool> {
        let count = self.count_if(values, op, threshold).await?;
        Ok(count > 0)
    }

    /// Proof of Innocence - check if wallet has no sanctioned transactions
    ///
    /// # Example
    /// ```ignore
    /// let is_clean = zyber.proof_of_innocence(&wallet_txs, &sanctions_list).await?;
    /// ```
    pub async fn proof_of_innocence(
        &mut self,
        wallet_transactions: &[u8],
        sanctions_list: &[u8],
    ) -> Result<bool> {
        // For each wallet transaction, check if it appears in sanctions list
        // CountIf(sanctions, == tx_id) should be 0 for all transactions
        for &tx_id in wallet_transactions {
            let operation = FheOperation::CountIf {
                predicate: FhePredicate::Equals(tx_id),
                expected_count: sanctions_list.len() as u16,
            };
            let matches = self.compute(operation, sanctions_list).await?;
            if matches > 0 {
                return Ok(false); // Found a match - not innocent
            }
        }
        Ok(true) // No matches - innocent
    }

    // =========================================================================
    // Core Computation
    // =========================================================================

    /// Execute an FHE computation
    async fn compute(&mut self, operation: FheOperation, values: &[u8]) -> Result<u64> {
        // Ensure keys are ready
        self.ensure_keys().await?;

        // Encrypt values
        let encrypted_values = zyberlink_fhe::encrypt_values(values, self.client_key()?)
            .context("Failed to encrypt values")?;

        // Serialize for transport
        let witness_data = bincode::serialize(&encrypted_values)
            .context("Failed to serialize encrypted values")?;

        // Upload witness to backend
        let witness_commitment = self.upload_witness(&witness_data).await?;

        // Upload server key if needed
        self.upload_server_key().await?;

        // Create job on-chain
        let job_id = self.create_job(operation.clone(), &witness_commitment, witness_data.len()).await?;

        // Poll for result
        let encrypted_result = self.poll_result(job_id).await?;

        // Decrypt result
        let result: u8 = zyberlink_fhe::decrypt_result(&encrypted_result, self.client_key()?)
            .context("Failed to decrypt result")?;

        Ok(result as u64)
    }

    /// Upload witness data to backend
    async fn upload_witness(&self, data: &[u8]) -> Result<[u8; 32]> {
        let url = format!("{}/api/witness", self.config.backend_url);

        let response = self
            .http
            .post(&url)
            .body(data.to_vec())
            .send()
            .await
            .context("Failed to upload witness")?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Witness upload failed: {}",
                response.status()
            ));
        }

        let result: serde_json::Value = response.json().await?;
        let commitment_hex = result["commitment"]
            .as_str()
            .ok_or_else(|| anyhow!("No commitment in response"))?;

        let commitment_bytes = hex::decode(commitment_hex)?;
        let mut commitment = [0u8; 32];
        commitment.copy_from_slice(&commitment_bytes[..32]);

        Ok(commitment)
    }

    /// Upload server key to backend (if not already uploaded)
    async fn upload_server_key(&self) -> Result<()> {
        let server_key_bytes = zyberlink_fhe::serialize_server_key(self.server_key()?)?;

        let url = format!("{}/api/server-key", self.config.backend_url);

        // Check if already uploaded
        let check_response = self.http.get(&url).send().await;
        if let Ok(resp) = check_response {
            if resp.status().is_success() {
                return Ok(()); // Already uploaded
            }
        }

        // Upload
        let response = self
            .http
            .post(&url)
            .body(server_key_bytes)
            .send()
            .await
            .context("Failed to upload server key")?;

        if !response.status().is_success() {
            return Err(anyhow!("Server key upload failed: {}", response.status()));
        }

        Ok(())
    }

    /// Create job on-chain
    async fn create_job(
        &self,
        operation: FheOperation,
        witness_commitment: &[u8; 32],
        witness_size: usize,
    ) -> Result<u64> {
        // Get next job ID
        let job_id = self.fetch_next_job_id()?;

        // Build FHE config
        let fhe_config = FheConsensusConfig {
            operation,
            required_provers: self.required_provers,
            consensus_threshold: self.required_provers, // Same as required for now
            submission_timeout_secs: self.timeout.as_secs() as i64,
        };

        // Create instruction
        let ix = self.client.create_job_instruction(
            &self.keypair.pubkey(),
            job_id,
            zyberlink_types::CircuitType::FheComputation(fhe_config.operation.clone()),
            *witness_commitment,
            witness_size as u32,
            self.price_lamports,
            self.timeout.as_secs() as i64,
            Some(fhe_config),
        )?;

        // Send transaction
        let recent_blockhash = self.client.rpc_client.get_latest_blockhash()?;
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.keypair.pubkey()),
            &[&*self.keypair],
            recent_blockhash,
        );

        self.client
            .rpc_client
            .send_and_confirm_transaction_with_spinner(&tx)?;

        Ok(job_id)
    }

    /// Fetch next job ID from chain
    fn fetch_next_job_id(&self) -> Result<u64> {
        let (config_pda, _) = self.client.get_config_pda();
        let account = self.client.rpc_client.get_account(&config_pda)?;

        // Layout: authority(32) + fee(2) + min_stake(8) + min_rep(4) + timeout(8) + protocol_fee_recipient(32) + next_job_id(8)
        // Offset: 86 bytes
        let next_job_id = u64::from_le_bytes(
            account.data[86..94]
                .try_into()
                .context("Invalid config data")?,
        );

        Ok(next_job_id)
    }

    /// Poll for job result
    async fn poll_result(&self, job_id: u64) -> Result<Vec<u8>> {
        let url = format!("{}/api/jobs/{}/result", self.config.backend_url, job_id);
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > self.timeout {
                return Err(anyhow!("Job {} timed out after {:?}", job_id, self.timeout));
            }

            let response = self.http.get(&url).send().await?;

            if response.status().is_success() {
                let result: serde_json::Value = response.json().await?;

                if let Some(status) = result["status"].as_str() {
                    match status {
                        "completed" => {
                            let encrypted_result = result["encrypted_result"]
                                .as_str()
                                .ok_or_else(|| anyhow!("No encrypted_result in response"))?;
                            use base64::Engine;
                            return Ok(base64::engine::general_purpose::STANDARD.decode(encrypted_result)?);
                        }
                        "failed" => {
                            let error = result["error"].as_str().unwrap_or("Unknown error");
                            return Err(anyhow!("Job failed: {}", error));
                        }
                        _ => {
                            // Still processing, wait and retry
                        }
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_defaults() {
        let builder = ZyberBuilder::default();
        assert_eq!(builder.timeout, Duration::from_secs(120));
        assert_eq!(builder.required_provers, 2);
        assert_eq!(builder.price_lamports, 10_000_000);
    }

    #[test]
    fn test_builder_chain() {
        let builder = ZyberBuilder::new()
            .network("devnet")
            .timeout(Duration::from_secs(60))
            .provers(3)
            .price_sol(0.05);

        assert_eq!(builder.network, Some("devnet".to_string()));
        assert_eq!(builder.timeout, Duration::from_secs(60));
        assert_eq!(builder.required_provers, 3);
        assert_eq!(builder.price_lamports, 50_000_000);
    }

    #[test]
    fn test_network_config() {
        let devnet = NetworkConfig::from_name("devnet").unwrap();
        assert_eq!(devnet.name, "devnet");
        assert!(devnet.rpc_url.contains("devnet"));

        let localnet = NetworkConfig::from_name("localnet").unwrap();
        assert_eq!(localnet.name, "localnet");
        assert!(localnet.rpc_url.contains("localhost"));

        assert!(NetworkConfig::from_name("unknown").is_none());
    }
}
