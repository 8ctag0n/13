use borsh::BorshDeserialize;
use solana_client::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use solana_sdk::commitment_config::CommitmentConfig;

use crate::{
    accounts::*,
    error::{FutarchyError, Result},
    state::*,
};

pub async fn query_market(
    rpc_client: &RpcClient,
    program_id: &Pubkey,
    market_id: u64,
) -> Result<Market> {
    let market_pda = find_market_pda(program_id, market_id);

    let account = rpc_client
        .get_account_with_commitment(&market_pda.address, CommitmentConfig::confirmed())
        .map_err(|e| FutarchyError::RpcError(Box::new(e)))?
        .value
        .ok_or(FutarchyError::MarketNotFound)?;

    Market::try_from_slice(&account.data).map_err(|e| e.into())
}

pub async fn query_position(
    rpc_client: &RpcClient,
    program_id: &Pubkey,
    market_id: u64,
    user: &Pubkey,
    bet_commitment: &[u8; 32],
) -> Result<Position> {
    let position_pda = find_position_pda(program_id, market_id, user, bet_commitment);

    let account = rpc_client
        .get_account_with_commitment(&position_pda.address, CommitmentConfig::confirmed())
        .map_err(|e| FutarchyError::RpcError(Box::new(e)))?
        .value
        .ok_or(FutarchyError::PositionNotFound)?;

    Position::try_from_slice(&account.data).map_err(|e| e.into())
}

pub async fn query_all_markets(
    rpc_client: &RpcClient,
    program_id: &Pubkey,
) -> Result<Vec<(Pubkey, Market)>> {
    let accounts = rpc_client.get_program_accounts_with_config(
        program_id,
        solana_client::rpc_config::RpcProgramAccountsConfig {
            filters: None,
            account_config: solana_client::rpc_config::RpcAccountInfoConfig {
                encoding: Some(solana_account_decoder::UiAccountEncoding::Base64),
                commitment: Some(CommitmentConfig::confirmed()),
                ..Default::default()
            },
            with_context: None,
            sort_results: None,
        },
    ).map_err(|e| FutarchyError::RpcError(Box::new(e)))?;

    let mut markets = Vec::new();

    for (pubkey, account) in accounts {
        if let Ok(market) = Market::try_from_slice(&account.data) {
            markets.push((pubkey, market));
        }
    }

    Ok(markets)
}

pub async fn find_pending_fhe_jobs(
    rpc_client: &RpcClient,
    program_id: &Pubkey,
) -> Result<Vec<FutarchyFheJob>> {
    let all_markets = query_all_markets(rpc_client, program_id).await?;

    let mut pending_jobs = Vec::new();

    for (market_pubkey, market) in all_markets {
        // Only process markets with a pending FHE job
        if let Some(job_id) = market.pending_pool_update_job {
            // For first bet, pool may be empty - use empty vec as "zero" pool
            // The prover will handle this case by initializing the pool
            pending_jobs.push(FutarchyFheJob {
                market_id: market.market_id,
                market_pubkey,
                job_id: Some(job_id),
                // Determine side based on which pool is non-empty or default to YES for first bet
                // TODO: Store the bet side in the job metadata instead of guessing
                side: !market.encrypted_pool_yes.is_empty() || market.encrypted_pool_no.is_empty(),
                encrypted_pool: if !market.encrypted_pool_yes.is_empty() {
                    market.encrypted_pool_yes.clone()
                } else if !market.encrypted_pool_no.is_empty() {
                    market.encrypted_pool_no.clone()
                } else {
                    // First bet - empty pool (prover should initialize with encrypted zero)
                    Vec::new()
                },
            });
        }
    }

    Ok(pending_jobs)
}

pub async fn get_pool_ciphertext(
    rpc_client: &RpcClient,
    program_id: &Pubkey,
    market_id: u64,
    side: bool,
) -> Result<Vec<u8>> {
    let market = query_market(rpc_client, program_id, market_id).await?;

    let ciphertext = if side {
        market.encrypted_pool_yes
    } else {
        market.encrypted_pool_no
    };

    if ciphertext.is_empty() {
        Err(FutarchyError::Custom("Pool ciphertext is empty".to_string()))
    } else {
        Ok(ciphertext)
    }
}

pub async fn query_user_eligibility(
    rpc_client: &RpcClient,
    program_id: &Pubkey,
    user: &Pubkey,
) -> Result<Option<UserEligibility>> {
    let user_eligibility_pda = find_user_eligibility_pda(program_id, user);

    let account = rpc_client
        .get_account_with_commitment(&user_eligibility_pda.address, CommitmentConfig::confirmed())
        .map_err(|e| FutarchyError::RpcError(Box::new(e)))?
        .value;

    match account {
        Some(acc) => {
            let eligibility = UserEligibility::try_from_slice(&acc.data)?;
            Ok(Some(eligibility))
        }
        None => Ok(None),
    }
}

pub async fn query_user_escrow(
    rpc_client: &RpcClient,
    program_id: &Pubkey,
    user: &Pubkey,
    market_id: u64,
) -> Result<Option<UserEscrow>> {
    let user_escrow_pda = find_user_escrow_pda(program_id, user, market_id);

    let account = rpc_client
        .get_account_with_commitment(&user_escrow_pda.address, CommitmentConfig::confirmed())
        .map_err(|e| FutarchyError::RpcError(Box::new(e)))?
        .value;

    match account {
        Some(acc) => {
            let escrow = UserEscrow::try_from_slice(&acc.data)?;
            Ok(Some(escrow))
        }
        None => Ok(None),
    }
}

pub async fn query_active_markets(
    rpc_client: &RpcClient,
    program_id: &Pubkey,
) -> Result<Vec<(Pubkey, Market)>> {
    let all_markets = query_all_markets(rpc_client, program_id).await?;

    Ok(all_markets
        .into_iter()
        .filter(|(_, market)| market.is_active())
        .collect())
}

pub async fn query_settled_markets(
    rpc_client: &RpcClient,
    program_id: &Pubkey,
) -> Result<Vec<(Pubkey, Market)>> {
    let all_markets = query_all_markets(rpc_client, program_id).await?;

    Ok(all_markets
        .into_iter()
        .filter(|(_, market)| market.is_settled())
        .collect())
}

#[derive(Debug, Clone)]
pub struct FutarchyFheJob {
    pub market_id: u64,
    pub market_pubkey: Pubkey,
    pub job_id: Option<u64>,
    pub side: bool,
    pub encrypted_pool: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_futarchy_fhe_job_creation() {
        let job = FutarchyFheJob {
            market_id: 1,
            market_pubkey: Pubkey::new_unique(),
            job_id: Some(42),
            side: true,
            encrypted_pool: vec![1, 2, 3, 4],
        };

        assert_eq!(job.market_id, 1);
        assert!(job.side);
        assert_eq!(job.encrypted_pool.len(), 4);
    }
}
