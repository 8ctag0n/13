use anyhow::{Context, Result};
use borsh::{BorshDeserialize, BorshSerialize};
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng as AeadRng},
    ChaCha20Poly1305,
};
use rand::rngs::OsRng;
use reqwest::Client;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{read_keypair_file, Signer},
    transaction::Transaction,
};
use std::env;
use x25519_dalek::{EphemeralSecret, PublicKey};
use zyberlink_sdk::MarketplaceClient;
use zyberlink_types::{CircuitType, FheOperation};

/// Encrypted witness envelope (must match prover format)
#[derive(BorshSerialize, BorshDeserialize)]
struct EncryptedWitness {
    ephemeral_public_key: [u8; 32],
    nonce: [u8; 12],
    ciphertext: Vec<u8>,
}

/// Dummy Orchard witness for testing (matches prover structure)
#[derive(BorshSerialize, BorshDeserialize)]
struct OrchardWitness {
    spend_auth_sig: [u8; 64],
    note_value: u64,
    note_rho: [u8; 32],
    note_rseed: [u8; 32],
    merkle_path: Vec<[u8; 32]>,
    merkle_position: u32,
    recipient_address: [u8; 43],
    output_value: u64,
    rcv: [u8; 32],
}

/// Encrypt witness data for a recipient (same logic as prover)
fn encrypt_witness(witness: &OrchardWitness, recipient_pubkey: &[u8; 32]) -> Result<Vec<u8>> {
    // Parse recipient public key
    let recipient_pubkey = PublicKey::from(*recipient_pubkey);

    // Generate ephemeral keypair for this encryption
    let ephemeral_private = EphemeralSecret::random_from_rng(OsRng);
    let ephemeral_public = PublicKey::from(&ephemeral_private);

    // Perform X25519 key exchange to derive shared secret
    let shared_secret = ephemeral_private.diffie_hellman(&recipient_pubkey);

    // Derive encryption key from shared secret using Solana's hash function
    // MUST match prover's derive_encryption_key() function
    use solana_sdk::hash::hash;
    let hash_result = hash(shared_secret.as_bytes());
    let encryption_key: [u8; 32] = hash_result.to_bytes();

    // Serialize witness using borsh
    let witness_bytes = borsh::to_vec(&witness).context("Failed to serialize witness")?;

    // Generate random nonce (12 bytes for ChaCha20-Poly1305)
    let nonce = ChaCha20Poly1305::generate_nonce(&mut AeadRng);

    // Encrypt witness with ChaCha20-Poly1305 AEAD
    let cipher = ChaCha20Poly1305::new(&encryption_key.into());
    let ciphertext = cipher
        .encrypt(&nonce, witness_bytes.as_ref())
        .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

    // Package everything into encrypted envelope
    let encrypted = EncryptedWitness {
        ephemeral_public_key: *ephemeral_public.as_bytes(),
        nonce: nonce.into(),
        ciphertext,
    };

    // Serialize encrypted envelope
    let encrypted_bytes =
        borsh::to_vec(&encrypted).context("Failed to serialize encrypted witness")?;

    Ok(encrypted_bytes)
}

/// Simple example to create a test job
/// Usage: cargo run --example create_test_job -- [zk|fhe] [price_in_lamports]
#[tokio::main]
async fn main() -> Result<()> {
    // Parse arguments
    let args: Vec<String> = env::args().collect();
    let job_type = args.get(1).map(|s| s.as_str()).unwrap_or("zk");
    let price = args
        .get(2)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(15_000_000); // Default 0.015 SOL

    // Get configuration from environment or defaults
    let rpc_url =
        env::var("SOLANA_RPC_URL").unwrap_or_else(|_| "http://localhost:8899".to_string());
    let program_id_str = env::var("PROGRAM_ID")
        .or_else(|_| std::fs::read_to_string("../logs/zyberlink_program_id.txt"))
        .or_else(|_| std::fs::read_to_string("/tmp/zyberlink_program_id.txt"))
        .expect("Program ID not found. Set PROGRAM_ID env var or run demo first.");
    let program_id = program_id_str
        .trim()
        .parse()
        .expect("Invalid program ID format");

    let keypair_path = env::var("KEYPAIR_PATH")
        .unwrap_or_else(|_| format!("{}/.config/solana/id.json", env::var("HOME").unwrap()));

    println!("🔧 Configuration:");
    println!("  RPC URL: {}", rpc_url);
    println!("  Program ID: {}", program_id);
    println!("  Keypair: {}", keypair_path);
    println!();

    // Load keypair
    let keypair = read_keypair_file(&keypair_path)
        .map_err(|e| anyhow::anyhow!("Failed to read keypair: {}", e))?;

    println!("👤 User: {}", keypair.pubkey());
    println!();

    // Create client
    let client =
        MarketplaceClient::new_with_commitment(rpc_url, program_id, CommitmentConfig::confirmed());

    // Determine circuit type and create appropriate job
    let (circuit_type, job_name) = match job_type {
        "fhe" => {
            println!("📊 Creating FHE Computation job...");
            (
                CircuitType::FheComputation(FheOperation::Add(10)),
                "FHE Add Operation",
            )
        }
        "zk" => {
            println!("🔐 Creating ZK Zcash Orchard job...");
            (CircuitType::ZcashOrchard, "ZK Zcash Orchard Proof")
        }
        _ => {
            println!("🔐 Creating ZK Zcash Orchard job...");
            (CircuitType::ZcashOrchard, "ZK Zcash Orchard Proof")
        }
    };

    println!("  Type: {}", job_name);
    println!(
        "  Price: {} lamports ({:.4} SOL)",
        price,
        price as f64 / 1e9
    );
    println!();

    // Load prover's encryption public key
    println!("🔑 Loading prover encryption public key...");
    let prover_pubkey_hex = std::fs::read_to_string("../logs/prover_encryption_pubkey.txt")
        .context("Failed to read prover public key. Run ./demo/01-setup.sh first.")?;
    let prover_pubkey_bytes =
        hex::decode(prover_pubkey_hex.trim()).context("Invalid prover public key hex")?;

    if prover_pubkey_bytes.len() != 32 {
        anyhow::bail!(
            "Invalid prover public key length: expected 32, got {}",
            prover_pubkey_bytes.len()
        );
    }

    let mut prover_pubkey = [0u8; 32];
    prover_pubkey.copy_from_slice(&prover_pubkey_bytes);
    println!("  Prover pubkey: {}...", &prover_pubkey_hex.trim()[..16]);

    // Create a dummy witness for testing
    let witness = OrchardWitness {
        spend_auth_sig: [0u8; 64],
        note_value: 100_000_000, // 0.1 ZEC
        note_rho: [1u8; 32],
        note_rseed: [2u8; 32],
        merkle_path: vec![[3u8; 32]; 32],
        merkle_position: 0,
        recipient_address: [4u8; 43],
        output_value: 100_000_000,
        rcv: [5u8; 32],
    };

    // Encrypt witness with prover's public key
    println!("🔐 Encrypting witness with prover's public key...");
    let encrypted_witness = encrypt_witness(&witness, &prover_pubkey)?;
    println!("  Encrypted witness: {} bytes", encrypted_witness.len());

    // Upload encrypted witness to storage backend
    println!("📤 Uploading encrypted witness to storage...");
    let http_client = Client::new();
    let witness_url =
        env::var("WITNESS_BACKEND_URL").unwrap_or_else(|_| "http://localhost:3030".to_string());

    let upload_response = http_client
        .post(format!("{}/witness", witness_url))
        .body(encrypted_witness.clone())
        .send()
        .await?;

    if !upload_response.status().is_success() {
        anyhow::bail!("Failed to upload witness: {}", upload_response.status());
    }

    let upload_result: serde_json::Value = upload_response.json().await?;
    let commitment_hex = upload_result["commitment"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No commitment in response"))?;

    // Convert hex commitment to bytes
    let commitment_bytes = hex::decode(commitment_hex)?;
    let mut witness_commitment = [0u8; 32];
    witness_commitment.copy_from_slice(&commitment_bytes[..32]);

    println!("  Witness uploaded: {}", commitment_hex);
    println!();

    println!("📝 Creating job...");

    // Get next job ID
    let job_id = client.fetch_next_job_id()?;
    println!("  Job ID: {}", job_id);

    // Create job instruction
    let ix = client.create_job_instruction(
        &keypair.pubkey(),
        job_id,
        circuit_type,
        witness_commitment,
        encrypted_witness.len() as u32,
        price,
        600,  // 10 minutes timeout
        None, // No FHE consensus config for now
    )?;

    // Get recent blockhash
    let recent_blockhash = client.rpc_client.get_latest_blockhash()?;

    // Create and sign transaction
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&keypair.pubkey()),
        &[&keypair],
        recent_blockhash,
    );

    // Send transaction
    println!("📤 Sending transaction...");
    let signature = client
        .rpc_client
        .send_and_confirm_transaction_with_spinner(&tx)?;

    println!("✅ Transaction confirmed!");
    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Job created successfully!");
    println!("  Signature: {}", signature);
    println!("  Job ID: {}", job_id);
    println!("  Type: {}", job_name);
    println!("  Price: {:.4} SOL", price as f64 / 1e9);
    println!();
    println!("  👀 Watch the Prover logs to see it being processed!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}
