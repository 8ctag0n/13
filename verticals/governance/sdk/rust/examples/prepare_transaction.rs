use futarchy_sdk::*;
use solana_sdk::pubkey::Pubkey;

fn main() -> anyhow::Result<()> {
    let program_id = Pubkey::new_unique();
    let zk_generator_program = Pubkey::new_unique();

    let bettor = Pubkey::new_unique();
    let payer = Pubkey::new_unique();
    let market_id = 1;

    let bet_commitment = [1u8; 32];
    let proof = vec![0u8; 256];
    let public_inputs = vec![1u8; 80];
    let amount = 500_000_000;
    let circuit_type = 30;
    let ciphertext_hash = Some([5u8; 32]);

    let fhe_accounts = Some(FheAccounts {
        fhe_job: Pubkey::new_unique(),
        fhe_consensus: Pubkey::new_unique(),
        fhe_escrow: Pubkey::new_unique(),
        fhe_generator_program: Pubkey::new_unique(),
    });

    let ix = build_place_bet_ix(
        &program_id,
        &bettor,
        market_id,
        bet_commitment,
        proof,
        public_inputs,
        amount,
        circuit_type,
        ciphertext_hash,
        None,
        None,
        &zk_generator_program,
        fhe_accounts,
    )?;

    let unsigned_tx = prepare_unsigned_transaction(&[ix], &payer)?;

    println!("Unsigned transaction prepared successfully!");
    println!("Base64 encoded transaction length: {} bytes", unsigned_tx.len());
    println!("\nTransaction (base64):\n{}", unsigned_tx);

    Ok(())
}
