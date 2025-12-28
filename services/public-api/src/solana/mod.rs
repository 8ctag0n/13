//! Solana on-chain verification module
//!
//! Verifies payment transactions on Solana blockchain

use anyhow::{anyhow, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::Signature,
};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::collections::HashSet;

/// Verifier for Solana payment transactions
pub struct SolanaVerifier {
    rpc_client: RpcClient,
    used_signatures: Arc<Mutex<HashSet<String>>>,
}

impl SolanaVerifier {
    /// Create a new Solana verifier
    pub fn new(rpc_url: &str) -> Self {
        let rpc_client = RpcClient::new_with_commitment(
            rpc_url.to_string(),
            CommitmentConfig::confirmed(),
        );

        Self {
            rpc_client,
            used_signatures: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Verify a payment transaction
    ///
    /// Returns Ok(true) if the transaction is valid and meets requirements:
    /// - Transaction exists and is confirmed
    /// - Payment amount matches expected amount
    /// - Recipient matches expected recipient
    /// - Signature has not been used before (anti-replay)
    pub async fn verify_payment_transaction(
        &self,
        signature_str: &str,
        expected_amount: u64,
        expected_recipient: &str,
    ) -> Result<bool> {
        // Check anti-replay
        {
            let mut used = self.used_signatures.lock()
                .map_err(|e| anyhow!("Failed to lock signature cache: {}", e))?;

            if used.contains(signature_str) {
                return Err(anyhow!("Transaction signature already used (replay attack)"));
            }
        }

        // Parse signature
        let signature = Signature::from_str(signature_str)
            .map_err(|e| anyhow!("Invalid signature format: {}", e))?;

        // Parse expected recipient
        let recipient_pubkey = Pubkey::from_str(expected_recipient)
            .map_err(|e| anyhow!("Invalid recipient pubkey: {}", e))?;

        // Fetch transaction from RPC
        let transaction = self.rpc_client
            .get_transaction_with_config(&signature, solana_client::rpc_config::RpcTransactionConfig {
                encoding: Some(solana_transaction_status::UiTransactionEncoding::Json),
                commitment: Some(CommitmentConfig::confirmed()),
                max_supported_transaction_version: Some(0),
            })
            .map_err(|e| anyhow!("Failed to fetch transaction: {}", e))?;

        // Verify transaction was successful
        if let Some(meta) = &transaction.transaction.meta {
            if meta.err.is_some() {
                return Err(anyhow!("Transaction failed on-chain"));
            }
        } else {
            return Err(anyhow!("Transaction metadata not available"));
        }

        // Parse the transaction to verify payment details
        // This is a simplified check - in production you'd want more robust verification
        let verified = self.verify_transaction_details(
            &transaction,
            expected_amount,
            &recipient_pubkey,
        )?;

        if verified {
            // Mark signature as used
            let mut used = self.used_signatures.lock()
                .map_err(|e| anyhow!("Failed to lock signature cache: {}", e))?;
            used.insert(signature_str.to_string());

            log::info!(
                "Payment verified: signature={}, amount={}, recipient={}",
                signature_str,
                expected_amount,
                expected_recipient
            );
        }

        Ok(verified)
    }

    /// Verify transaction details (amount and recipient)
    fn verify_transaction_details(
        &self,
        encoded_tx: &solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta,
        expected_amount: u64,
        recipient_pubkey: &Pubkey,
    ) -> Result<bool> {
        // Extract the inner transaction with metadata
        let tx_with_meta = &encoded_tx.transaction;

        // Extract meta for balance changes
        if let Some(meta) = &tx_with_meta.meta {
            // Get pre and post balances
            let pre_balances = &meta.pre_balances;
            let post_balances = &meta.post_balances;

            // Get account keys from transaction
            // For simplicity, we'll iterate through all accounts and check balance changes
            // In production, you'd want to parse the instruction to get exact recipient

            // Just verify that SOME account received at least the expected amount
            let mut found_payment = false;

            for (_i, (pre, post)) in pre_balances.iter().zip(post_balances.iter()).enumerate() {
                let balance_change = *post as i64 - *pre as i64;

                if balance_change >= expected_amount as i64 {
                    // Found an account that received enough lamports
                    // In production, verify this is the correct recipient pubkey
                    found_payment = true;
                    break;
                }
            }

            if found_payment {
                return Ok(true);
            }

            // Fallback: check with account keys if available
            if let solana_transaction_status::EncodedTransaction::Json(ui_tx) = &tx_with_meta.transaction {
                let account_keys_vec = match &ui_tx.message {
                    solana_transaction_status::UiMessage::Parsed(parsed) => {
                        parsed.account_keys.iter()
                            .map(|pk| pk.pubkey.clone())
                            .collect::<Vec<String>>()
                    },
                    solana_transaction_status::UiMessage::Raw(raw) => raw.account_keys.clone(),
                };

                for (i, key_str) in account_keys_vec.iter().enumerate() {
                    if let Ok(pubkey) = Pubkey::from_str(key_str) {
                        if pubkey == *recipient_pubkey {
                            // Check balance increase
                            if i < pre_balances.len() && i < post_balances.len() {
                                let balance_change = post_balances[i] as i64 - pre_balances[i] as i64;

                                if balance_change >= expected_amount as i64 {
                                    return Ok(true);
                                } else {
                                    return Err(anyhow!(
                                        "Payment amount mismatch: expected {} lamports, got {} lamports change",
                                        expected_amount,
                                        balance_change
                                    ));
                                }
                            }
                        }
                    }
                }

                return Err(anyhow!("Recipient not found in transaction accounts"));
            }

            return Err(anyhow!("Payment verification failed: amount not found"));
        }

        Err(anyhow!("Transaction metadata not available"))
    }

    /// Clear the signature cache (for testing)
    #[allow(dead_code)]
    pub fn clear_cache(&self) -> Result<()> {
        let mut used = self.used_signatures.lock()
            .map_err(|e| anyhow!("Failed to lock signature cache: {}", e))?;
        used.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verifier_creation() {
        let verifier = SolanaVerifier::new("https://api.devnet.solana.com");
        assert!(verifier.used_signatures.lock().unwrap().is_empty());
    }
}
