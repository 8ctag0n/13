use futarchy_sdk::*;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{read_keypair_file, Signer},
    transaction::Transaction,
};
use std::str::FromStr;

fn main() -> anyhow::Result<()> {
    let program_id = Pubkey::from_str("5B8x1aJEHsMLqqKDYVQe2dTA38hbie1QXX2JSmPAxWJT")?;
    let rpc = RpcClient::new("http://localhost:8899");

    let keypair = read_keypair_file("/home/deploy/.config/solana/id.json")
        .map_err(|e| anyhow::anyhow!("Failed to read keypair: {}", e))?;

    println!("User: {}", keypair.pubkey());

    let client = FutarchyClient::new("http://localhost:8899", program_id);

    let market_id: u64 = 1;
    let amount: u64 = 100_000; // 0.0001 SOL - enough for testing

    println!("Depositing {} lamports to market {}", amount, market_id);

    let deposit_ix = client.deposit_to_market(
        &keypair.pubkey(),
        market_id,
        amount,
    )?;

    let recent_blockhash = rpc.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&[deposit_ix], Some(&keypair.pubkey()));
    tx.sign(&[&keypair], recent_blockhash);

    println!("Sending transaction...");
    let signature = rpc.send_and_confirm_transaction(&tx)?;
    println!("Deposit successful! TX: {}", signature);

    let escrow_pda = client.find_user_escrow_pda(&keypair.pubkey(), market_id);
    println!("User Escrow PDA: {}", escrow_pda.address);

    Ok(())
}
