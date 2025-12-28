use futarchy_sdk::*;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use std::str::FromStr;

fn main() -> anyhow::Result<()> {
    let program_id = Pubkey::from_str("AQUUuRSwDhB1eeC2Caa8GPVGV4YzZkJ1YiSvZd3BBPij")?;

    let client = FutarchyClient::new("https://api.devnet.solana.com", program_id);

    let authority = Keypair::new();
    let oracle = Keypair::new().pubkey();

    let market_id = 1;
    let question_hash = [0u8; 32];
    let end_time = 1735689600;
    let max_bet = 1_000_000_000;

    let create_market_ix = client.create_market(
        &authority.pubkey(),
        market_id,
        question_hash,
        &oracle,
        end_time,
        max_bet,
    )?;

    println!("CreateMarket instruction built successfully!");
    println!("Program ID: {}", create_market_ix.program_id);
    println!("Accounts: {}", create_market_ix.accounts.len());
    println!("Data size: {} bytes", create_market_ix.data.len());

    let market_pda = client.find_market_pda(market_id);
    println!("\nMarket PDA: {}", market_pda.address);
    println!("Market PDA bump: {}", market_pda.bump);

    let escrow_pda = client.find_escrow_pda(market_id);
    println!("Escrow PDA: {}", escrow_pda.address);
    println!("Escrow PDA bump: {}", escrow_pda.bump);

    Ok(())
}
