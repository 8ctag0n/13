//! Starknet event listener for pBTCFi consensus events.
//!
//! Polls ConsensusReached and ResultSubmitted events and logs them for observability.

use std::time::Duration;

use anyhow::{Context, Result};
use tokio::time::sleep;
use tiny_keccak::{Hasher, Keccak};
use zyberlink_chain_client::{ChainClient, StarknetClient};
use zyberlink_chain_client::starknet_extensions::{StarknetEvent, StarknetSpecificOps};

#[derive(Debug, Clone)]
pub struct StarknetEventListenerConfig {
    pub rpc_url: String,
    pub contract_address: String,
    pub poll_interval: Duration,
}

pub struct StarknetEventListener {
    client: StarknetClient,
    config: StarknetEventListenerConfig,
    last_block: u64,
}

impl StarknetEventListener {
    pub async fn new(config: StarknetEventListenerConfig) -> Result<Self> {
        let client = StarknetClient::new(&config.rpc_url)
            .context("Failed to create Starknet client")?;
        let last_block = client
            .get_block_height()
            .await
            .context("Failed to fetch Starknet block height")?;

        Ok(Self {
            client,
            config,
            last_block,
        })
    }

    pub async fn run(mut self) -> Result<()> {
        let consensus_key = selector_from_name("ConsensusReached");
        let submitted_key = selector_from_name("ResultSubmitted");

        loop {
            let latest_block = match self.client.get_block_height().await {
                Ok(height) => height,
                Err(e) => {
                    log::warn!("Starknet event poll failed to get block height: {}", e);
                    sleep(self.config.poll_interval).await;
                    continue;
                }
            };

            if latest_block >= self.last_block {
                let from_block = self.last_block;
                let to_block = latest_block;

                if let Err(e) = self
                    .poll_events(from_block, to_block, &consensus_key, &submitted_key)
                    .await
                {
                    log::warn!("Starknet event poll error: {}", e);
                }

                self.last_block = latest_block.saturating_add(1);
            }

            sleep(self.config.poll_interval).await;
        }
    }

    async fn poll_events(
        &self,
        from_block: u64,
        to_block: u64,
        consensus_key: &str,
        submitted_key: &str,
    ) -> Result<()> {
        let consensus_events = self
            .client
            .get_events(
                from_block,
                to_block,
                Some(&self.config.contract_address),
                Some(vec![vec![consensus_key.to_string()]]),
            )
            .await
            .context("Failed to fetch ConsensusReached events")?;

        for event in consensus_events {
            if let Some(parsed) = parse_consensus_reached(&event) {
                log::info!(
                    "[Starknet] ConsensusReached job_id={} agreeing_provers={} hash={} tx={}",
                    parsed.job_id,
                    parsed.agreeing_provers,
                    parsed.consensus_hash,
                    parsed.transaction_hash
                );
            }
        }

        let submitted_events = self
            .client
            .get_events(
                from_block,
                to_block,
                Some(&self.config.contract_address),
                Some(vec![vec![submitted_key.to_string()]]),
            )
            .await
            .context("Failed to fetch ResultSubmitted events")?;

        for event in submitted_events {
            if let Some(parsed) = parse_result_submitted(&event) {
                log::info!(
                    "[Starknet] ResultSubmitted job_id={} prover={} count={} hash={} tx={}",
                    parsed.job_id,
                    parsed.prover,
                    parsed.submission_count,
                    parsed.result_hash,
                    parsed.transaction_hash
                );
            }
        }

        Ok(())
    }
}

struct ConsensusReachedEvent {
    job_id: u64,
    agreeing_provers: u8,
    consensus_hash: String,
    transaction_hash: String,
}

struct ResultSubmittedEvent {
    job_id: u64,
    prover: String,
    result_hash: String,
    submission_count: u8,
    transaction_hash: String,
}

fn parse_consensus_reached(event: &StarknetEvent) -> Option<ConsensusReachedEvent> {
    let job_id = parse_job_id_from_keys(&event.keys)?;
    let consensus_hash = event.data.get(0).cloned().unwrap_or_default();
    let agreeing_provers = event
        .data
        .get(1)
        .and_then(|v| parse_felt_to_u64(v).ok())
        .and_then(|v| u8::try_from(v).ok())
        .unwrap_or(0);

    Some(ConsensusReachedEvent {
        job_id,
        agreeing_provers,
        consensus_hash,
        transaction_hash: event.transaction_hash.clone(),
    })
}

fn parse_result_submitted(event: &StarknetEvent) -> Option<ResultSubmittedEvent> {
    let job_id = parse_job_id_from_keys(&event.keys)?;
    let prover = event.data.get(0).cloned().unwrap_or_default();
    let result_hash = event.data.get(1).cloned().unwrap_or_default();
    let submission_count = event
        .data
        .get(2)
        .and_then(|v| parse_felt_to_u64(v).ok())
        .and_then(|v| u8::try_from(v).ok())
        .unwrap_or(0);

    Some(ResultSubmittedEvent {
        job_id,
        prover,
        result_hash,
        submission_count,
        transaction_hash: event.transaction_hash.clone(),
    })
}

fn parse_job_id_from_keys(keys: &[String]) -> Option<u64> {
    if keys.len() < 3 {
        return None;
    }

    let low = keys.get(1)?;
    let high = keys.get(2)?;
    parse_u256(low, high).ok()
}

fn parse_u256(low: &str, _high: &str) -> Result<u64> {
    parse_felt_to_u64(low).context("Failed to parse u256 low limb")
}

fn parse_felt_to_u64(felt: &str) -> Result<u64> {
    let s = felt.trim_start_matches("0x");
    u64::from_str_radix(s, 16)
        .with_context(|| format!("Failed to parse felt {}", felt))
}

fn selector_from_name(name: &str) -> String {
    let mut hasher = Keccak::v256();
    let mut output = [0u8; 32];
    hasher.update(name.as_bytes());
    hasher.finalize(&mut output);
    output[0] &= 0x03;
    format!("0x{}", hex::encode(output))
}
