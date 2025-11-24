use borsh::BorshDeserialize;
use cypherlink_sdk::{JobAccount, MarketplaceClient, ProverAccount};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

fn main() -> anyhow::Result<()> {
    let program_id = std::env::var("PROGRAM_ID").expect("PROGRAM_ID env var required");
    let program_id = Pubkey::from_str(&program_id)?;

    let rpc_url =
        std::env::var("SOLANA_RPC_URL").unwrap_or_else(|_| "http://localhost:8899".to_string());

    let client = MarketplaceClient::new(rpc_url, program_id);

    println!("Program ID: {}", program_id);
    println!("\n=== Getting all program accounts ===");

    let accounts = client.rpc_client.get_program_accounts(&program_id)?;
    println!("Total accounts: {}", accounts.len());

    for (i, (pubkey, account)) in accounts.iter().enumerate() {
        println!("\nAccount #{}: {}", i, pubkey);
        println!("  Data length: {} bytes", account.data.len());
        println!("  Owner: {}", account.owner);
        println!("  Lamports: {}", account.lamports);

        // Try to deserialize as different types
        if let Ok(job) = JobAccount::deserialize(&mut &account.data[..]) {
            println!("  ✓ Deserialized as JobAccount!");
            println!("    ID: {}", job.id);
            println!("    Status: {:?}", job.status);
            println!("    Circuit: {:?}", job.circuit_type);
            println!("    Price: {} lamports", job.price_lamports);
            println!("    Creator: {}", job.creator);
            if let Some(prover) = job.prover {
                println!("    Prover: {}", prover);
            }
        } else if let Ok(prover) = ProverAccount::deserialize(&mut &account.data[..]) {
            println!("  ✓ Deserialized as ProverAccount!");
            println!("    Authority: {}", prover.authority);
            println!("    Total jobs: {}", prover.total_jobs_completed);
            // Note: active_jobs field removed from ProverAccount
        } else {
            println!("  ✗ Could not deserialize as known type");
            println!(
                "    First 50 bytes: {:?}",
                &account.data[..account.data.len().min(50)]
            );
        }
    }

    Ok(())
}
