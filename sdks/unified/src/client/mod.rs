use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::Keypair;
use std::sync::Arc;
use crate::core::{Result, NetworkConfig};
use crate::query::{JobQuery, UnifiedJob};

pub struct ZyberUnified {
    #[allow(dead_code)]
    client: Arc<RpcClient>,
    #[allow(dead_code)]
    payer: Keypair,
    #[allow(dead_code)]
    config: NetworkConfig,
    pub jobs: JobQuery,
}

impl ZyberUnified {
    pub async fn connect(_network: &str) -> Result<Self> {
        todo!("Connect to network and initialize all sub-clients")
    }

    pub async fn from_config(_config: NetworkConfig) -> Result<Self> {
        todo!("Initialize from NetworkConfig")
    }

    pub fn builder() -> ZyberUnifiedBuilder {
        ZyberUnifiedBuilder::default()
    }

    pub async fn get_pending_jobs(&self) -> Result<Vec<UnifiedJob>> {
        self.jobs.get_pending_jobs().await
    }

    pub async fn get_job(&self, address: &solana_sdk::pubkey::Pubkey) -> Result<UnifiedJob> {
        self.jobs.get_job(address).await
    }

    pub async fn claim_job(&self, _job: &UnifiedJob) -> Result<solana_sdk::signature::Signature> {
        todo!("Auto-route claim_job based on job type")
    }
}

#[derive(Default)]
pub struct ZyberUnifiedBuilder {}

impl ZyberUnifiedBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn rpc_url(self, _url: String) -> Self {
        todo!("Set RPC URL")
    }

    pub fn payer(self, _keypair: Keypair) -> Self {
        todo!("Set payer keypair")
    }

    pub async fn build(self) -> Result<ZyberUnified> {
        todo!("Build ZyberUnified client")
    }
}
