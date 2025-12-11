//! Solana payment integration for ZyberLink CLI
//!
//! Handles keypair loading, transaction building, signing, and sending

pub mod tx_builder;
pub mod signer;

pub use tx_builder::PaymentTxBuilder;
pub use signer::{load_keypair, sign_and_send_transaction};

use anyhow::Result;
use solana_sdk::signature::Signature;

/// Complete payment flow: build, sign, send, and wait for confirmation
pub async fn execute_payment(
    rpc_url: &str,
    keypair_path: &str,
    recipient: &str,
    amount_lamports: u64,
) -> Result<Signature> {
    // Load keypair
    let keypair = load_keypair(keypair_path)?;

    // Build transaction
    let tx_builder = PaymentTxBuilder::new(rpc_url)?;
    let transaction = tx_builder.build_payment_transaction(
        &keypair,
        recipient,
        amount_lamports,
    ).await?;

    // Sign and send
    let signature = sign_and_send_transaction(
        rpc_url,
        transaction,
        &keypair,
    ).await?;

    Ok(signature)
}
