use anyhow::Result;
use borsh::BorshDeserialize;
use cypherlink_sdk::JobAccount;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_client::rpc_filter::RpcFilterType;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use std::env;

/// Test script para validar que podemos leer y deserializar jobs desde la blockchain
fn main() -> Result<()> {
    println!("=== Blockchain Sync Test ===\n");

    // Load config from env
    let rpc_url = env::var("SOLANA_RPC_URL").unwrap_or_else(|_| "http://localhost:8899".to_string());
    let program_id_str = env::var("PROGRAM_ID").expect("PROGRAM_ID env var required");
    let program_id = program_id_str.parse::<Pubkey>()?;

    println!("RPC URL: {}", rpc_url);
    println!("Program ID: {}\n", program_id);

    // Create RPC client
    let rpc_client = RpcClient::new_with_commitment(
        rpc_url.clone(),
        CommitmentConfig::confirmed(),
    );

    println!("Fetching all job accounts from blockchain...");

    // Configure to fetch only JobAccount (879 bytes)
    let config = RpcProgramAccountsConfig {
        filters: Some(vec![RpcFilterType::DataSize(879)]),
        account_config: RpcAccountInfoConfig {
            encoding: Some(solana_account_decoder::UiAccountEncoding::Base64),
            commitment: Some(CommitmentConfig::confirmed()),
            ..Default::default()
        },
        ..Default::default()
    };

    // Fetch accounts
    let accounts = rpc_client.get_program_accounts_with_config(&program_id, config)?;

    println!("Found {} job accounts\n", accounts.len());

    if accounts.is_empty() {
        println!("No jobs found on-chain. Make sure job-creator is running.");
        return Ok(());
    }

    // Try to deserialize first 5 jobs
    let sample_size = accounts.len().min(5);
    println!("Deserializing first {} jobs...\n", sample_size);

    for (i, (pubkey, account)) in accounts.iter().take(sample_size).enumerate() {
        println!("--- Job #{} ---", i);
        println!("Pubkey: {}", pubkey);
        println!("Data size: {} bytes", account.data.len());
        println!("Lamports: {}", account.lamports);

        match JobAccount::deserialize(&mut &account.data[..]) {
            Ok(job) => {
                println!("✓ Successfully deserialized!");
                println!("  Job ID: {}", job.id);
                println!("  Creator: {}", job.creator);
                println!("  Status: {:?}", job.status);
                println!("  Circuit: {:?}", job.circuit_type);
                println!("  Price: {} lamports ({} SOL)", job.price_lamports, job.price_lamports as f64 / 1e9);
                println!("  Created at: {}", job.created_at);

                if let Some(prover) = job.prover {
                    println!("  Prover: {}", prover);
                }

                if let Some(ref fhe_config) = job.fhe_config {
                    println!("  FHE Config:");
                    println!("    Required provers: {}", fhe_config.required_provers);
                    println!("    Consensus threshold: {}", fhe_config.consensus_threshold);
                    println!("    Operation: {:?}", fhe_config.operation);
                }

                // Test conversion to format for database
                let status_str = format!("{:?}", job.status).to_lowercase();
                let circuit_str = format!("{:?}", job.circuit_type);

                println!("  DB format:");
                println!("    status: '{}'", status_str);
                println!("    circuit_type: '{}'", circuit_str);

                // Test timestamp conversion
                if let Some(dt) = chrono::DateTime::from_timestamp(job.created_at, 0) {
                    println!("    created_at timestamp: {}", dt.to_rfc3339());
                } else {
                    println!("    created_at timestamp: INVALID");
                }
            }
            Err(e) => {
                println!("✗ Failed to deserialize: {}", e);
                println!("  First 50 bytes: {:?}", &account.data[..account.data.len().min(50)]);
            }
        }
        println!();
    }

    // Summary
    println!("=== Summary ===");
    println!("Total jobs on-chain: {}", accounts.len());

    let mut success_count = 0;
    let mut fail_count = 0;

    for (_pubkey, account) in &accounts {
        match JobAccount::deserialize(&mut &account.data[..]) {
            Ok(_) => success_count += 1,
            Err(_) => fail_count += 1,
        }
    }

    println!("Successfully deserialized: {}", success_count);
    println!("Failed to deserialize: {}", fail_count);

    if fail_count > 0 {
        println!("\n⚠ Warning: {} jobs failed to deserialize. Check program compatibility.", fail_count);
    } else {
        println!("\n✓ All jobs deserialized successfully!");
        println!("✓ Ready to implement full sync!");
    }

    Ok(())
}
