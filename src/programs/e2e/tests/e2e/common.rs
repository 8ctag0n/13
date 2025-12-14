use solana_program::{pubkey::Pubkey, system_program};
use solana_program_test::{processor, ProgramTest, ProgramTestContext};
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};

// Program IDs para tests
// Usando arrays de bytes para evitar problemas con base58 encoding
pub const BEDROCK_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
    0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
    0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
    0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20,
]);

pub const ZK_GENERATOR_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28,
    0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f, 0x30,
    0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38,
    0x39, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x3f, 0x40,
]);

pub const FHE_GENERATOR_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48,
    0x49, 0x4a, 0x4b, 0x4c, 0x4d, 0x4e, 0x4f, 0x50,
    0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58,
    0x59, 0x5a, 0x5b, 0x5c, 0x5d, 0x5e, 0x5f, 0x60,
]);

/// Setup del environment de testing con todos los programas
pub fn setup_test_environment() -> ProgramTest {
    let mut program_test = ProgramTest::default();

    // Cargar bedrock program
    program_test.add_program(
        "bedrock",
        BEDROCK_PROGRAM_ID,
        processor!(bedrock::process_instruction),
    );

    // Cargar zk-generator program
    program_test.add_program(
        "zk_generator",
        ZK_GENERATOR_PROGRAM_ID,
        processor!(zk_generator::process_instruction),
    );

    // Cargar fhe-generator program
    program_test.add_program(
        "fhe_generator",
        FHE_GENERATOR_PROGRAM_ID,
        processor!(fhe_generator::process_instruction),
    );

    program_test
}

/// Helper para crear y fondear un keypair
pub async fn create_and_fund_keypair(
    context: &mut solana_program_test::ProgramTestContext,
    lamports: u64,
) -> Keypair {
    let keypair = Keypair::new();

    let tx = Transaction::new_signed_with_payer(
        &[solana_sdk::system_instruction::transfer(
            &context.payer.pubkey(),
            &keypair.pubkey(),
            lamports,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(tx)
        .await
        .expect("Failed to fund keypair");

    keypair
}

/// Helper para derivar PDA de config de bedrock
pub fn get_bedrock_config_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"config"], &BEDROCK_PROGRAM_ID)
}

/// Helper para derivar PDA de prover registry
pub fn get_prover_registry_pda(prover: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"prover", prover.as_ref()],
        &BEDROCK_PROGRAM_ID,
    )
}

/// Helper para derivar PDA de job ZK
pub fn get_zk_job_pda(creator: &Pubkey, job_id: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"zk_job", creator.as_ref(), job_id],
        &ZK_GENERATOR_PROGRAM_ID,
    )
}

/// Helper para derivar PDA de escrow ZK
pub fn get_zk_escrow_pda(creator: &Pubkey, job_id: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"zk_escrow", creator.as_ref(), job_id],
        &ZK_GENERATOR_PROGRAM_ID,
    )
}

/// Helper para derivar PDA de job FHE
pub fn get_fhe_job_pda(job_id: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"fhe_job", job_id],
        &FHE_GENERATOR_PROGRAM_ID,
    )
}

/// Helper para derivar PDA de consensus FHE
pub fn get_fhe_consensus_pda(job_id: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"fhe_consensus", job_id],
        &FHE_GENERATOR_PROGRAM_ID,
    )
}

/// Helper para derivar PDA de escrow FHE
pub fn get_fhe_escrow_pda(job_id: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"fhe_escrow", job_id],
        &FHE_GENERATOR_PROGRAM_ID,
    )
}

/// Helper para inicializar bedrock con config básica
pub async fn initialize_bedrock(
    context: &mut ProgramTestContext,
    admin: &Keypair,
) -> Result<(), Box<dyn std::error::Error>> {
    let (config_pda, _) = get_bedrock_config_pda();

    // Crear instrucción de inicialización
    let ix = bedrock_sdk::instructions::initialize(
        &BEDROCK_PROGRAM_ID,
        &admin.pubkey(),
        &config_pda,
        &ZK_GENERATOR_PROGRAM_ID,
        &FHE_GENERATOR_PROGRAM_ID,
    );

    // Crear y enviar transacción
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&admin.pubkey()),
        &[admin],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(tx)
        .await
        .map_err(|e| format!("Failed to initialize bedrock: {}", e))?;

    Ok(())
}

/// Helper para registrar un prover en bedrock
pub async fn register_prover(
    context: &mut ProgramTestContext,
    prover: &Keypair,
    stake_amount: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let (prover_pda, _) = get_prover_registry_pda(&prover.pubkey());

    // Crear instrucción de registro
    let ix = bedrock_sdk::instructions::register_prover(
        &BEDROCK_PROGRAM_ID,
        &prover.pubkey(),
        &prover_pda,
        stake_amount,
    );

    // Crear y enviar transacción
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&prover.pubkey()),
        &[prover],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(tx)
        .await
        .map_err(|e| format!("Failed to register prover: {}", e))?;

    Ok(())
}

/// Setup completo: inicializa bedrock y registra un prover
pub async fn setup_with_prover(
    context: &mut ProgramTestContext,
    stake_amount: u64,
) -> Result<(Keypair, Keypair), Box<dyn std::error::Error>> {
    // Crear admin y fondearlo
    let admin = create_and_fund_keypair(context, 10_000_000_000).await;

    // Inicializar bedrock
    initialize_bedrock(context, &admin).await?;

    // Crear prover y fondearlo (más stake_amount para el registro)
    let prover = create_and_fund_keypair(context, 10_000_000_000 + stake_amount).await;

    // Registrar prover
    register_prover(context, &prover, stake_amount).await?;

    Ok((admin, prover))
}

/// Generate job ID the same way as the program does
/// This is needed because create_job generates the job_id internally
pub fn generate_zk_job_id(timestamp: i64, creator: &Pubkey) -> u64 {
    (timestamp as u64) ^ (creator.to_bytes()[0] as u64)
}

/// Generate FHE job ID the same way as the FHE program does
/// FHE uses the same algorithm as ZK
pub fn generate_fhe_job_id(timestamp: i64, creator: &Pubkey) -> u64 {
    (timestamp as u64) ^ (creator.to_bytes()[0] as u64)
}

/// Helper to get current blockchain timestamp
pub async fn get_current_timestamp(
    context: &mut ProgramTestContext,
) -> i64 {
    let clock = context.banks_client.get_sysvar::<solana_program::clock::Clock>().await.unwrap();
    clock.unix_timestamp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_setup_works() {
        let program_test = setup_test_environment();
        let mut context = program_test.start_with_context().await;

        // Verificar que podemos crear y fondear keypairs
        let keypair = create_and_fund_keypair(&mut context, 1_000_000_000).await;

        let account = context
            .banks_client
            .get_account(keypair.pubkey())
            .await
            .expect("Failed to get account")
            .expect("Account not found");

        assert_eq!(account.lamports, 1_000_000_000);
    }

    #[tokio::test]
    async fn test_initialize_bedrock() {
        let program_test = setup_test_environment();
        let mut context = program_test.start_with_context().await;

        // Crear admin
        let admin = create_and_fund_keypair(&mut context, 10_000_000_000).await;

        // Inicializar bedrock
        initialize_bedrock(&mut context, &admin)
            .await
            .expect("Failed to initialize bedrock");

        // Verificar que la cuenta de config fue creada
        let (config_pda, _) = get_bedrock_config_pda();
        let config_account = context
            .banks_client
            .get_account(config_pda)
            .await
            .expect("Failed to get config account")
            .expect("Config account not found");

        assert!(config_account.lamports > 0);
    }

    #[tokio::test]
    async fn test_setup_with_prover() {
        let program_test = setup_test_environment();
        let mut context = program_test.start_with_context().await;

        let stake_amount = 100_000_000; // 0.1 SOL

        // Setup completo
        let (admin, prover) = setup_with_prover(&mut context, stake_amount)
            .await
            .expect("Failed to setup with prover");

        // Verificar que bedrock fue inicializado
        let (config_pda, _) = get_bedrock_config_pda();
        let config_account = context
            .banks_client
            .get_account(config_pda)
            .await
            .expect("Failed to get config account")
            .expect("Config account not found");

        assert!(config_account.lamports > 0);

        // Verificar que el prover fue registrado
        let (prover_pda, _) = get_prover_registry_pda(&prover.pubkey());
        let prover_account = context
            .banks_client
            .get_account(prover_pda)
            .await
            .expect("Failed to get prover account")
            .expect("Prover account not found");

        assert!(prover_account.lamports > 0);

        // Verificar que admin y prover son diferentes
        assert_ne!(admin.pubkey(), prover.pubkey());
    }
}
