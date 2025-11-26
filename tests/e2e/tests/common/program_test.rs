/// Self-Executing Test Setup using solana-program-test
///
/// This module provides helpers for running tests without external validator.
/// Uses in-memory program test environment for fast, deterministic testing.

use anyhow::Result;
use zyberlink_sdk::MarketplaceClient;
use solana_program_test::{ProgramTest, BanksClient, processor};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
    instruction::Instruction,
};
use std::str::FromStr;

/// Program ID for zyberlink marketplace
pub const PROGRAM_ID: &str = "bn2XNLkXi23NPMjH1qNdGWg1tuUFtpVkQvqxTD9v3Ys";

/// Test context containing all initialized accounts and clients
pub struct TestContext {
    pub banks_client: BanksClient,
    pub payer: Keypair,
    pub program_id: Pubkey,
    pub authority: Keypair,
    pub config_pda: Pubkey,
}

impl TestContext {
    /// Get SDK client configured for this test environment
    pub fn sdk_client(&self) -> MarketplaceClient {
        // Note: MarketplaceClient expects RPC URL, but in program_test we use BanksClient
        // For now, we'll use instruction builders directly
        MarketplaceClient::new(
            "http://localhost:8899".to_string(), // Dummy URL
            self.program_id,
        )
    }

    /// Execute transaction using banks_client
    pub async fn execute_transaction(
        &mut self,
        instructions: &[Instruction],
        signers: &[&Keypair],
    ) -> Result<()> {
        let recent_blockhash = self.banks_client.get_latest_blockhash().await?;

        let mut transaction = Transaction::new_with_payer(
            instructions,
            Some(&self.payer.pubkey()),
        );

        let mut all_signers = vec![&self.payer];
        all_signers.extend(signers);

        transaction.sign(&all_signers, recent_blockhash);

        self.banks_client.process_transaction(transaction).await?;
        Ok(())
    }

    /// Fund an account with SOL
    pub async fn fund_account(&mut self, pubkey: &Pubkey, lamports: u64) -> Result<()> {
        use solana_sdk::system_instruction;

        let ix = system_instruction::transfer(
            &self.payer.pubkey(),
            pubkey,
            lamports,
        );

        self.execute_transaction(&[ix], &[]).await
    }
}

/// Setup test environment with zyberlink program loaded
///
/// Returns initialized TestContext with:
/// - Banks client (in-memory validator)
/// - Funded payer account
/// - Initialized marketplace config
/// - Authority keypair
///
/// # Example
/// ```no_run
/// let mut ctx = setup_test_environment().await.unwrap();
/// // Run test logic...
/// ```
pub async fn setup_test_environment() -> Result<TestContext> {
    let program_id = Pubkey::from_str(PROGRAM_ID)?;

    // Create program test with our program
    let mut program_test = ProgramTest::new(
        "zyberlink",
        program_id,
        processor!(zyberlink::process_instruction),
    );

    // Start test environment
    let (banks_client, payer, recent_blockhash) = program_test.start().await;

    // Create authority keypair
    let authority = Keypair::new();

    // Derive config PDA
    let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &program_id);

    let ctx = TestContext {
        banks_client,
        payer,
        program_id,
        authority,
        config_pda,
    };

    Ok(ctx)
}

/// Setup and initialize marketplace in one call
///
/// Convenience function that:
/// 1. Sets up test environment
/// 2. Initializes marketplace config
/// 3. Returns ready-to-use TestContext
pub async fn setup_initialized_marketplace() -> Result<TestContext> {
    let mut ctx = setup_test_environment().await?;

    // Fund authority
    ctx.fund_account(&ctx.authority.pubkey(), 10_000_000_000).await?;

    // Initialize marketplace
    let sdk = ctx.sdk_client();
    let authority_pubkey = ctx.authority.pubkey();
    let initialize_ix = sdk.initialize_instruction(
        &authority_pubkey,
        100,  // 1% fee
        5_000_000_000,  // 5 SOL min stake
        100,  // min reputation
        600,  // 10min timeout
    )?;

    // Clone authority to avoid borrow issues
    let authority_clone = Keypair::from_bytes(&ctx.authority.to_bytes())?;
    ctx.execute_transaction(&[initialize_ix], &[&authority_clone]).await?;

    Ok(ctx)
}

/// Register a prover in the test environment
///
/// Automatically funds prover and executes registration
pub async fn register_test_prover(
    ctx: &mut TestContext,
    prover: &Keypair,
    stake_amount: u64,
) -> Result<Pubkey> {
    // Fund prover
    ctx.fund_account(&prover.pubkey(), 20_000_000_000).await?;

    // Register
    let sdk = ctx.sdk_client();
    let encryption_key = [99u8; 32];  // Test key

    let register_ix = sdk.register_prover_instruction(
        &prover.pubkey(),
        stake_amount,
        encryption_key,
    )?;

    ctx.execute_transaction(&[register_ix], &[prover]).await?;

    // Return prover PDA
    let (prover_pda, _) = Pubkey::find_program_address(
        &[b"prover", prover.pubkey().as_ref()],
        &ctx.program_id,
    );

    Ok(prover_pda)
}

/// Create FHE job in test environment
///
/// Automatically funds creator and creates job
pub async fn create_test_fhe_job(
    ctx: &mut TestContext,
    creator: &Keypair,
    required_provers: u8,
    consensus_threshold: u8,
) -> Result<Pubkey> {
    use blake2::{Blake2s256, Digest as Blake2Digest};
    use zyberlink_types::FheOperation;

    // Fund creator
    ctx.fund_account(&creator.pubkey(), 10_000_000_000).await?;

    // Create dummy encrypted input commitment
    let mut hasher = Blake2s256::new();
    hasher.update(b"test encrypted input");
    let witness_commitment: [u8; 32] = hasher.finalize().into();

    // Create job using SDK
    let sdk = ctx.sdk_client();

    // Get next job ID from config
    // Note: In program_test, we need to read account data directly
    let config_account = ctx.banks_client
        .get_account(ctx.config_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Config account not found"))?;

    use borsh::BorshDeserialize;
    let config: zyberlink_sdk::MarketplaceConfig =
        borsh::from_slice(&config_account.data)?;

    let job_id = config.next_job_id;

    // Create FHE job instruction
    let create_job_ix = sdk.create_job_instruction(
        &creator.pubkey(),
        job_id,
        zyberlink_types::CircuitType::FheComputation(FheOperation::Add(10)),
        witness_commitment,
        65856,  // TFHE-rs FheUint8 ciphertext size
        (required_provers as u64) * 3_000_000,  // 0.003 SOL per prover
        300,  // 5min timeout
        Some(zyberlink_types::FheConsensusConfig {
            required_provers,
            consensus_threshold,
            submission_timeout_secs: 300,
            operation: FheOperation::Add(10),
        }),
    )?;

    ctx.execute_transaction(&[create_job_ix], &[creator]).await?;

    // Return job PDA
    let job_id_bytes = job_id.to_le_bytes();
    let (job_pda, _) = Pubkey::find_program_address(
        &[b"job", creator.pubkey().as_ref(), &job_id_bytes],
        &ctx.program_id,
    );

    Ok(job_pda)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_setup_environment() {
        let ctx = setup_test_environment().await.unwrap();
        assert_ne!(ctx.payer.pubkey(), Pubkey::default());
    }

    #[tokio::test]
    async fn test_initialize_marketplace() {
        let ctx = setup_initialized_marketplace().await.unwrap();

        // Verify config account exists
        let config = ctx.banks_client.get_account(ctx.config_pda).await.unwrap();
        assert!(config.is_some());
    }

    #[tokio::test]
    async fn test_register_prover() {
        let mut ctx = setup_initialized_marketplace().await.unwrap();
        let prover = Keypair::new();

        let prover_pda = register_test_prover(
            &mut ctx,
            &prover,
            5_000_000_000,
        ).await.unwrap();

        // Verify prover account exists
        let prover_account = ctx.banks_client.get_account(prover_pda).await.unwrap();
        assert!(prover_account.is_some());
    }

    #[tokio::test]
    async fn test_create_fhe_job() {
        let mut ctx = setup_initialized_marketplace().await.unwrap();
        let creator = Keypair::new();

        let job_pda = create_test_fhe_job(
            &mut ctx,
            &creator,
            3,  // required_provers
            2,  // consensus_threshold
        ).await.unwrap();

        // Verify job account exists
        let job_account = ctx.banks_client.get_account(job_pda).await.unwrap();
        assert!(job_account.is_some());
    }
}
