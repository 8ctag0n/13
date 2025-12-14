//! Transaction builder for Solana payments

#![allow(deprecated)] // system_transaction is deprecated but still works

use anyhow::{Context, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    transaction::Transaction,
    signature::Keypair,
    system_transaction,
};
use std::str::FromStr;

/// Builder for payment transactions
pub struct PaymentTxBuilder {
    rpc_client: RpcClient,
}

impl PaymentTxBuilder {
    /// Create a new transaction builder
    pub fn new(rpc_url: &str) -> Result<Self> {
        let rpc_client = RpcClient::new_with_commitment(
            rpc_url.to_string(),
            CommitmentConfig::confirmed(),
        );

        Ok(Self { rpc_client })
    }

    /// Build a payment transaction
    pub async fn build_payment_transaction(
        &self,
        payer: &Keypair,
        recipient: &str,
        amount_lamports: u64,
    ) -> Result<Transaction> {
        // Parse recipient pubkey
        let recipient_pubkey = Pubkey::from_str(recipient)
            .with_context(|| format!("Invalid recipient pubkey: {}", recipient))?;

        // Get recent blockhash
        let recent_blockhash = self.rpc_client
            .get_latest_blockhash()
            .context("Failed to get recent blockhash")?;

        // Create transfer transaction (system_transaction handles instruction creation and signing)
        let transaction = system_transaction::transfer(
            payer,
            &recipient_pubkey,
            amount_lamports,
            recent_blockhash,
        );

        Ok(transaction)
    }

    /// Get account balance
    pub fn get_balance(&self, pubkey: &Pubkey) -> Result<u64> {
        self.rpc_client
            .get_balance(pubkey)
            .context("Failed to get balance")
    }

    /// Estimate transaction fee
    pub async fn estimate_fee(&self, transaction: &Transaction) -> Result<u64> {
        // Get fee for message
        let fee = self.rpc_client
            .get_fee_for_message(transaction.message())
            .context("Failed to estimate fee")?;

        Ok(fee)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tx_builder_creation() {
        let builder = PaymentTxBuilder::new("https://api.devnet.solana.com");
        assert!(builder.is_ok());
    }
}
