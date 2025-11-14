use anyhow::{anyhow, Result};
use solana_sdk::{
    hash::Hash,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

/// Fluent transaction builder for composing CypherLink transactions
///
/// This builder provides a wallet-compatible way to construct transactions.
/// It can build both unsigned (for wallet signing) and signed (for CLI) transactions.
///
/// # Example - Wallet Mode
/// ```no_run
/// use cypherlink_sdk::transaction::TransactionBuilder;
/// use solana_sdk::hash::Hash;
///
/// let tx = TransactionBuilder::new()
///     .add_instruction(instruction1)
///     .add_instruction(instruction2)
///     .payer(payer_pubkey)
///     .blockhash(recent_blockhash)
///     .build_unsigned()?;
///
/// // Wallet signs the transaction
/// ```
///
/// # Example - CLI Mode
/// ```no_run
/// let tx = TransactionBuilder::new()
///     .add_instruction(instruction)
///     .payer(payer_pubkey)
///     .blockhash(recent_blockhash)
///     .build_and_sign(&[&keypair])?;
///
/// // Transaction is signed and ready to send
/// ```
pub struct TransactionBuilder {
    instructions: Vec<Instruction>,
    payer: Option<Pubkey>,
    recent_blockhash: Option<Hash>,
}

impl TransactionBuilder {
    /// Create a new transaction builder
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            payer: None,
            recent_blockhash: None,
        }
    }

    /// Add a single instruction to the transaction
    pub fn add_instruction(mut self, instruction: Instruction) -> Self {
        self.instructions.push(instruction);
        self
    }

    /// Add multiple instructions to the transaction
    pub fn add_instructions(mut self, instructions: Vec<Instruction>) -> Self {
        self.instructions.extend(instructions);
        self
    }

    /// Set the fee payer for the transaction
    pub fn payer(mut self, payer: Pubkey) -> Self {
        self.payer = Some(payer);
        self
    }

    /// Set the recent blockhash for the transaction
    pub fn blockhash(mut self, blockhash: Hash) -> Self {
        self.recent_blockhash = Some(blockhash);
        self
    }

    /// Build an unsigned transaction (for wallet signing)
    ///
    /// This creates a transaction that can be signed by a wallet.
    /// The wallet will handle the signing process.
    ///
    /// # Returns
    /// An unsigned `Transaction` ready for wallet signing
    ///
    /// # Errors
    /// Returns error if payer or blockhash is not set
    pub fn build_unsigned(self) -> Result<Transaction> {
        let payer = self
            .payer
            .ok_or_else(|| anyhow!("Transaction payer not set"))?;
        let blockhash = self
            .recent_blockhash
            .ok_or_else(|| anyhow!("Transaction blockhash not set"))?;

        if self.instructions.is_empty() {
            return Err(anyhow!("Transaction has no instructions"));
        }

        let mut tx = Transaction::new_with_payer(&self.instructions, Some(&payer));
        tx.message.recent_blockhash = blockhash;

        Ok(tx)
    }

    /// Build and sign a transaction (for CLI/testing)
    ///
    /// This creates and signs a transaction in one step.
    /// Useful for CLI tools and automated testing.
    ///
    /// # Arguments
    /// * `signers` - Array of keypairs to sign the transaction
    ///
    /// # Returns
    /// A fully signed `Transaction` ready to send
    ///
    /// # Errors
    /// Returns error if payer, blockhash not set or signing fails
    pub fn build_and_sign(self, signers: &[&Keypair]) -> Result<Transaction> {
        if signers.is_empty() {
            return Err(anyhow!("No signers provided"));
        }

        let blockhash = self
            .recent_blockhash
            .ok_or_else(|| anyhow!("Transaction blockhash not set"))?;

        let mut tx = self.build_unsigned()?;
        tx.sign(signers, blockhash);

        Ok(tx)
    }

    /// Get the number of instructions in the transaction
    pub fn instruction_count(&self) -> usize {
        self.instructions.len()
    }

    /// Check if transaction is ready to build
    pub fn is_ready(&self) -> bool {
        self.payer.is_some() && self.recent_blockhash.is_some() && !self.instructions.is_empty()
    }
}

impl Default for TransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::{
        instruction::AccountMeta,
        pubkey::Pubkey,
        signature::Keypair,
        signer::Signer,
    };

    #[test]
    fn test_builder_empty() {
        let builder = TransactionBuilder::new();
        assert_eq!(builder.instruction_count(), 0);
        assert!(!builder.is_ready());
    }

    #[test]
    fn test_builder_add_instruction() {
        let ix = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[],
            vec![AccountMeta::new(Pubkey::new_unique(), true)],
        );

        let builder = TransactionBuilder::new().add_instruction(ix);
        assert_eq!(builder.instruction_count(), 1);
    }

    #[test]
    fn test_builder_is_ready() {
        let ix = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[],
            vec![AccountMeta::new(Pubkey::new_unique(), true)],
        );

        let builder = TransactionBuilder::new()
            .add_instruction(ix)
            .payer(Pubkey::new_unique())
            .blockhash(Hash::default());

        assert!(builder.is_ready());
    }

    #[test]
    fn test_build_unsigned() {
        let ix = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[],
            vec![AccountMeta::new(Pubkey::new_unique(), true)],
        );

        let payer = Pubkey::new_unique();
        let tx = TransactionBuilder::new()
            .add_instruction(ix)
            .payer(payer)
            .blockhash(Hash::default())
            .build_unsigned()
            .unwrap();

        assert_eq!(tx.message.instructions.len(), 1);
        assert_eq!(tx.signatures.len(), 1); // Unsigned has placeholder signature
    }

    #[test]
    fn test_build_and_sign() {
        let program_id = Pubkey::new_unique();
        let payer = Keypair::new();

        let ix = Instruction::new_with_bytes(
            program_id,
            &[],
            vec![AccountMeta::new(payer.pubkey(), true)],
        );

        let tx = TransactionBuilder::new()
            .add_instruction(ix)
            .payer(payer.pubkey())
            .blockhash(Hash::default())
            .build_and_sign(&[&payer])
            .unwrap();

        assert_eq!(tx.message.instructions.len(), 1);
        assert!(!tx.signatures[0].as_ref().iter().all(|&b| b == 0)); // Has real signature
    }

    #[test]
    fn test_build_without_payer_fails() {
        let ix = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[],
            vec![AccountMeta::new(Pubkey::new_unique(), true)],
        );

        let result = TransactionBuilder::new()
            .add_instruction(ix)
            .blockhash(Hash::default())
            .build_unsigned();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("payer"));
    }

    #[test]
    fn test_build_without_blockhash_fails() {
        let ix = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[],
            vec![AccountMeta::new(Pubkey::new_unique(), true)],
        );

        let result = TransactionBuilder::new()
            .add_instruction(ix)
            .payer(Pubkey::new_unique())
            .build_unsigned();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("blockhash"));
    }

    #[test]
    fn test_build_without_instructions_fails() {
        let result = TransactionBuilder::new()
            .payer(Pubkey::new_unique())
            .blockhash(Hash::default())
            .build_unsigned();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no instructions"));
    }
}
