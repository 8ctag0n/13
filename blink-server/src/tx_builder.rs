use anyhow::{Context, Result};
use solana_sdk::{
    message::Message,
    pubkey::Pubkey,
    system_instruction,
    transaction::Transaction,
};

/// Build a SOL transfer transaction
pub fn build_transfer_transaction(
    from: &Pubkey,
    to: &Pubkey,
    lamports: u64,
) -> Result<Transaction> {
    // Create transfer instruction
    let instruction = system_instruction::transfer(from, to, lamports);

    // Create message with instruction
    let message = Message::new(&[instruction], Some(from));

    // Create unsigned transaction
    // Note: The client (wallet) will add recent blockhash and sign
    let transaction = Transaction::new_unsigned(message);

    Ok(transaction)
}

/// Serialize transaction to base64 for Actions API
pub fn serialize_transaction(tx: &Transaction) -> Result<String> {
    use base64::Engine;

    let serialized = bincode::serialize(tx)
        .context("Failed to serialize transaction")?;

    Ok(base64::engine::general_purpose::STANDARD.encode(&serialized))
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::signature::{Keypair, Signer};

    #[test]
    fn test_build_transfer_transaction() {
        let from = Keypair::new();
        let to = Keypair::new();
        let lamports = 1_000_000_000; // 1 SOL

        let tx = build_transfer_transaction(
            &from.pubkey(),
            &to.pubkey(),
            lamports,
        ).unwrap();

        assert_eq!(tx.message.instructions.len(), 1);
    }

    #[test]
    fn test_serialize_transaction() {
        let from = Keypair::new();
        let to = Keypair::new();

        let tx = build_transfer_transaction(
            &from.pubkey(),
            &to.pubkey(),
            1_000_000_000,
        ).unwrap();

        let encoded = serialize_transaction(&tx).unwrap();
        assert!(!encoded.is_empty());
    }
}
