/// FHE Key Generation Utility
///
/// Generates FHE keypairs (client key + server key) for use with the prover node.
///
/// Usage:
///   cargo run --bin generate-fhe-keys -- --output-dir ./keys
///
/// Output:
///   - fhe_client_key.bin (keep private - for clients only)
///   - fhe_server_key.bin (public - for provers)
use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::path::PathBuf;

// Import from prover_node library and zyberlink-fhe
use prover_node::generate_fhe_keys;
use zyberlink_fhe::{serialize_server_key, serialize_client_key};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Output directory for keys
    #[arg(short, long, default_value = ".")]
    output_dir: PathBuf,

    /// Also generate test encrypted value
    #[arg(short, long)]
    test_value: Option<u8>,
}

fn main() -> Result<()> {
    println!("Generating FHE keypair...");
    println!("This may take 10-30 seconds...\n");

    let args = Args::parse();

    // Create output directory if it doesn't exist
    fs::create_dir_all(&args.output_dir).context("Failed to create output directory")?;

    // Generate keys
    let start = std::time::Instant::now();
    let (client_key, server_key) = generate_fhe_keys().context("Failed to generate FHE keys")?;
    let gen_time = start.elapsed();

    println!("Keys generated in {:?}", gen_time);

    // Serialize and save keys
    let client_key_path = args.output_dir.join("fhe_client_key.bin");
    let server_key_path = args.output_dir.join("fhe_server_key.bin");

    println!("\nSerializing keys...");

    let client_bytes =
        serialize_client_key(&client_key).context("Failed to serialize client key")?;
    let server_bytes =
        serialize_server_key(&server_key).context("Failed to serialize server key")?;

    fs::write(&client_key_path, &client_bytes).context("Failed to write client key file")?;
    fs::write(&server_key_path, &server_bytes).context("Failed to write server key file")?;

    println!("\nKeys saved:");
    println!(
        "  Client key: {} ({:.2} MB)",
        client_key_path.display(),
        client_bytes.len() as f64 / 1_000_000.0
    );
    println!(
        "  Server key: {} ({:.2} MB)",
        server_key_path.display(),
        server_bytes.len() as f64 / 1_000_000.0
    );

    // Generate test encrypted value if requested
    if let Some(test_value) = args.test_value {
        println!("\nGenerating test encrypted value...");

        use zyberlink_fhe::prelude::*;
        use zyberlink_fhe::FheUint8;

        let encrypted = FheUint8::try_encrypt(test_value, &client_key)
            .context("Failed to encrypt test value")?;

        let encrypted_bytes =
            bincode::serialize(&encrypted).context("Failed to serialize encrypted test value")?;

        let test_file = args.output_dir.join("test_encrypted_value.bin");
        fs::write(&test_file, &encrypted_bytes).context("Failed to write test encrypted value")?;

        println!(
            "  Test encrypted value: {} ({} bytes)",
            test_file.display(),
            encrypted_bytes.len()
        );
        println!("  Plaintext value: {}", test_value);

        // Also save metadata
        let metadata = format!(
            r#"{{
  "plaintext_value": {},
  "encrypted_file": "test_encrypted_value.bin",
  "encrypted_size_bytes": {}
}}
"#,
            test_value,
            encrypted_bytes.len()
        );

        let metadata_file = args.output_dir.join("test_metadata.json");
        fs::write(&metadata_file, metadata).context("Failed to write metadata")?;
        println!("  Metadata: {}", metadata_file.display());
    }

    println!("\n{}", "=".repeat(70));
    println!("SUCCESS!");
    println!("{}", "=".repeat(70));
    println!("\nNext steps:");
    println!("1. Keep fhe_client_key.bin private (clients use this to encrypt/decrypt)");
    println!("2. Distribute fhe_server_key.bin to prover nodes");
    println!(
        "3. Configure prover with: --fhe-server-key-path {}",
        server_key_path.display()
    );
    println!("\nExample prover command:");
    println!("  zyberlink-prover \\");
    println!("    --program-id <PROGRAM_ID> \\");
    println!("    --fhe-server-key-path {} \\", server_key_path.display());
    println!("    --rpc-url <RPC_URL>");

    Ok(())
}
