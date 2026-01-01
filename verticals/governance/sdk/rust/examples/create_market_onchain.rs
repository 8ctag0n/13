use futarchy_sdk::*;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signer},
    transaction::Transaction,
};
use std::str::FromStr;

fn main() -> anyhow::Result<()> {
    let program_id = Pubkey::from_str("5B8x1aJEHsMLqqKDYVQe2dTA38hbie1QXX2JSmPAxWJT")?;
    let rpc = RpcClient::new("http://localhost:8899");

    // Load keypair
    let keypair = read_keypair_file("/home/deploy/.config/solana/id.json")
        .map_err(|e| anyhow::anyhow!("Failed to read keypair: {}", e))?;

    println!("Creator/Oracle: {}", keypair.pubkey());

    let client = FutarchyClient::new("http://localhost:8899", program_id);

    let market_id: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(2); // Default to 2 if not provided
    let question_hash = {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(b"Test FHE E2E Market");
        let result = hasher.finalize();
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&result);
        arr
    };
    // End time: 1 hour from now
    let end_time: i64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64 + 3600;
    let max_bet: u64 = 1_000_000_000; // 1 SOL

    println!("Creating market...");
    println!("  Market ID: {}", market_id);
    println!("  End time: {}", end_time);
    println!("  Max bet: {} lamports", max_bet);

    let create_market_ix = client.create_market(
        &keypair.pubkey(),
        market_id,
        question_hash,
        &keypair.pubkey(), // oracle = creator for testing
        end_time,
        max_bet,
    )?;

    let recent_blockhash = rpc.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&[create_market_ix], Some(&keypair.pubkey()));
    tx.sign(&[&keypair], recent_blockhash);

    println!("Sending transaction...");
    let signature = rpc.send_and_confirm_transaction(&tx)?;
    println!("Market created! TX: {}", signature);

    let market_pda = client.find_market_pda(market_id);
    println!("Market PDA: {}", market_pda.address);

    Ok(())
}
