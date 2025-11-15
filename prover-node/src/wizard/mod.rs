pub mod steps;
pub mod terms;
pub mod ui;
pub mod validation;

use anyhow::{Context, Result};
use solana_sdk::signature::{Keypair, Signer};

use crate::config::ProverConfiguration;
use crate::witness_encryption::WitnessEncryption;

/// Main wizard orchestrator
pub struct SetupWizard {
    stake_amount: u64,
}

impl SetupWizard {
    /// Create new setup wizard
    pub fn new(stake_amount: u64) -> Self {
        Self { stake_amount }
    }

    /// Run the complete setup wizard
    pub async fn run(&mut self) -> Result<ProverConfiguration> {
        // Show welcome screen
        ui::print_banner();
        ui::print_welcome_message();
        ui::wait_for_enter()?;

        // Step 1: System validation
        steps::step_system_validation().await?;

        // Step 2: Keypair setup
        let (keypair, keypair_path) = steps::step_keypair_setup().await?;

        // Step 3: Network configuration
        let (network, rpc_url, client) = steps::step_network_setup().await?;

        // Ask for program ID
        let program_id = self.ask_program_id().await?;

        // Initialize witness encryption
        let encryption_seed = derive_encryption_seed(&keypair);
        let witness_encryption = WitnessEncryption::from_seed(encryption_seed)?;
        let encryption_pubkey = hex::encode(witness_encryption.public_key());

        // Step 4: Balance check and funding
        steps::step_balance_check(&client, &keypair, &network, self.stake_amount).await?;

        // Step 5: Terms & Conditions acceptance
        const TERMS_BACKEND_URL: &str = "https://api.zyberlink.io";
        steps::step_terms_acceptance(&keypair, TERMS_BACKEND_URL).await?;

        // Step 6: Register prover
        let prover_pda = steps::step_register_prover(
            &client,
            &keypair,
            &program_id,
            self.stake_amount,
            &witness_encryption,
        )
        .await?;

        // Create configuration
        let config = ProverConfiguration::new(
            network.name().to_string(),
            rpc_url,
            program_id,
            keypair_path.to_string_lossy().to_string(),
            keypair.pubkey(),
            prover_pda,
            encryption_pubkey,
            self.stake_amount,
            "http://localhost:3030".to_string(), // Default witness backend
        );

        // Save configuration
        let config_path = config.save().await?;

        // Show completion screen
        ui::print_completion_screen(
            &config_path.to_string_lossy(),
            &config.prover_authority,
        );

        Ok(config)
    }

    async fn ask_program_id(&self) -> Result<solana_sdk::pubkey::Pubkey> {
        ui::print_step_header(0, 0, "Program Configuration");

        let program_id_str = ui::input(
            "Enter CypherLink Program ID",
            None,
        )?;

        let program_id = program_id_str.parse()
            .context("Invalid program ID format")?;

        println!();
        ui::print_info(&format!("Program ID: {}", program_id));

        ui::wait_for_enter()?;

        Ok(program_id)
    }
}

/// Derive encryption seed from Solana keypair (same as in main.rs)
fn derive_encryption_seed(keypair: &Keypair) -> [u8; 32] {
    use solana_sdk::hash::hash;

    let mut seed_material = b"CYPHERLINK_WITNESS_ENCRYPTION_V1:".to_vec();
    seed_material.extend_from_slice(&keypair.to_bytes());

    let hash_result = hash(&seed_material);
    hash_result.to_bytes()
}
