use anyhow::Result;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
    commitment_config::CommitmentConfig,
};
use solana_client::rpc_client::RpcClient;
use std::str::FromStr;

#[tokio::test(flavor = "multi_thread")]
async fn test_initialize_marketplace() -> Result<()> {
    println!("\n=== Testing Initialize Marketplace ===\n");

    let rpc_url = "http://127.0.0.1:8899";
    let program_id = solana_sdk::pubkey::Pubkey::from_str("bn2XNLkXi23NPMjH1qNdGWg1tuUFtpVkQvqxTD9v3Ys")?;

    // Create RPC client
    let rpc_client = RpcClient::new_with_commitment(
        rpc_url.to_string(),
        CommitmentConfig::confirmed(),
    );

    // Generate authority keypair
    let authority = Keypair::new();
    println!("Authority: {}", authority.pubkey());

    // Airdrop SOL to authority
    println!("Requesting airdrop...");
    let airdrop_sig = rpc_client.request_airdrop(&authority.pubkey(), 10_000_000_000)?;

    // Wait for airdrop to be finalized
    println!("Waiting for airdrop confirmation...");
    for _ in 0..30 {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let balance = rpc_client.get_balance(&authority.pubkey())?;
        if balance >= 10_000_000_000 {
            println!("Airdrop confirmed. Balance: {} SOL", balance as f64 / 1_000_000_000.0);
            break;
        }
    }

    // Double check balance
    let balance = rpc_client.get_balance(&authority.pubkey())?;
    if balance == 0 {
        anyhow::bail!("Airdrop failed - balance is still 0");
    }

    // Create SDK client
    let sdk_client = zyberlink_sdk::MarketplaceClient::new(
        rpc_url.to_string(),
        program_id,
    );

    // Build Initialize instruction
    println!("Creating Initialize instruction...");
    let initialize_ix = sdk_client.initialize_instruction(
        &authority.pubkey(),
        1000,      // 10% platform fee
        1_000_000_000, // 1 SOL minimum stake
        500,       // 500/1000 minimum reputation
        600,       // 10 minutes default timeout
    )?;

    // Get recent blockhash
    let recent_blockhash = rpc_client.get_latest_blockhash()?;

    // Create and sign transaction
    let mut transaction = Transaction::new_with_payer(
        &[initialize_ix],
        Some(&authority.pubkey()),
    );
    transaction.sign(&[&authority], recent_blockhash);

    // Send transaction
    println!("Sending Initialize transaction...");
    let signature = rpc_client.send_and_confirm_transaction(&transaction)?;

    println!("\n✅ Initialize succeeded!");
    println!("   Transaction: {}", signature);

    // Verify config was created
    let (config_pda, _) = sdk_client.get_config_pda();
    let config_account = rpc_client.get_account(&config_pda)?;
    println!("   Config PDA: {}", config_pda);
    println!("   Config data length: {} bytes", config_account.data.len());

    assert!(config_account.data.len() > 0, "Config account should have data");

    Ok(())
}
