use futarchy_sdk::*;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use std::str::FromStr;

fn main() -> anyhow::Result<()> {
    let program_id = Pubkey::from_str("AQUUuRSwDhB1eeC2Caa8GPVGV4YzZkJ1YiSvZd3BBPij")?;
    let zk_generator_program = Pubkey::from_str("ZKGENxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx")?;

    let client = FutarchyClient::new("https://api.devnet.solana.com", program_id);

    let bettor = Keypair::new();
    let market_id = 1;

    let bet_commitment = [1u8; 32];
    let proof = vec![0u8; 256];
    let public_inputs = vec![1u8; 80];
    let amount = 500_000_000;
    let circuit_type = 30;

    let place_bet_ix = client.place_bet(
        &bettor.pubkey(),
        market_id,
        bet_commitment,
        proof,
        public_inputs,
        amount,
        circuit_type,
        None,
        None,
        None,
        &zk_generator_program,
        None,
    )?;

    println!("PlaceBet instruction built successfully!");
    println!("Program ID: {}", place_bet_ix.program_id);
    println!("Accounts: {}", place_bet_ix.accounts.len());
    println!("Data size: {} bytes", place_bet_ix.data.len());

    let market_pda = client.find_market_pda(market_id);
    println!("\nMarket PDA: {}", market_pda.address);

    let position_pda = client.find_position_pda(market_id, &bettor.pubkey());
    println!("Position PDA: {}", position_pda.address);
    println!("Position PDA bump: {}", position_pda.bump);

    println!("\n--- With FHE encrypted bet ---");

    let encrypted_bet_amount = vec![3u8; 128];
    let ciphertext_hash = Some([5u8; 32]);
    let side = Some(true);

    let fhe_accounts = FheAccounts {
        fhe_job: Pubkey::new_unique(),
        fhe_consensus: Pubkey::new_unique(),
        fhe_escrow: Pubkey::new_unique(),
        fhe_generator_program: Pubkey::new_unique(),
    };

    let place_bet_fhe_ix = client.place_bet(
        &bettor.pubkey(),
        market_id,
        bet_commitment,
        vec![0u8; 256],
        vec![1u8; 80],
        amount,
        31,
        ciphertext_hash,
        Some(encrypted_bet_amount),
        side,
        &zk_generator_program,
        Some(fhe_accounts),
    )?;

    println!("PlaceBet (with FHE) instruction built successfully!");
    println!("Accounts: {}", place_bet_fhe_ix.accounts.len());

    Ok(())
}
