use anyhow::{Context, Result};
use base64::Engine;
use clap::Parser;
use tfhe::prelude::*;
use tfhe::{generate_keys, ConfigBuilder, FheUint8};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

/// Generate TFHE test keys for E2E testing and CI/CD
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Output directory for generated keys
    #[arg(short, long, default_value = "test-keys")]
    output_dir: PathBuf,

    /// Test value to encrypt (default: 42)
    #[arg(short, long, default_value_t = 42)]
    test_value: u8,

    /// Generate compact keys (faster, for CI/CD)
    #[arg(short, long, default_value_t = false)]
    compact: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("=========================================");
    println!("TFHE Test Key Generator");
    println!("=========================================");
    println!();
    println!("Configuration:");
    println!("  Output directory: {:?}", args.output_dir);
    println!("  Test value: {}", args.test_value);
    println!("  Compact mode: {}", args.compact);
    println!();

    // Create output directory
    fs::create_dir_all(&args.output_dir)
        .context("Failed to create output directory")?;

    println!("Generating TFHE keys...");
    println!("⚠️  This may take 3-7 minutes depending on your CPU");
    println!();

    // Generate keys with smallest parameters for faster testing
    let config = ConfigBuilder::default().build();
    let (client_key, server_key) = generate_keys(config);

    println!("✅ Keys generated successfully!");
    println!();

    // Serialize server key
    let server_key_bytes = bincode::serialize(&server_key)
        .context("Failed to serialize server key")?;

    println!("Server key size: {} bytes ({:.2} MB)",
             server_key_bytes.len(),
             server_key_bytes.len() as f64 / 1024.0 / 1024.0);

    // Encode to base64
    let server_key_b64 = base64::engine::general_purpose::STANDARD.encode(&server_key_bytes);

    // Save server key
    let server_key_path = args.output_dir.join("server_key.b64");
    let mut server_key_file = File::create(&server_key_path)
        .context("Failed to create server_key.b64")?;
    server_key_file.write_all(server_key_b64.as_bytes())
        .context("Failed to write server key")?;

    println!("✅ Server key saved: {:?}", server_key_path);

    // Also save binary version for faster loading in tests
    let server_key_bin_path = args.output_dir.join("server_key.bin");
    fs::write(&server_key_bin_path, &server_key_bytes)
        .context("Failed to write server_key.bin")?;
    println!("✅ Server key (binary) saved: {:?}", server_key_bin_path);

    // Generate encrypted test data
    let encrypted_value = FheUint8::encrypt(args.test_value, &client_key);
    let encrypted_bytes = bincode::serialize(&encrypted_value)
        .context("Failed to serialize encrypted data")?;
    let encrypted_b64 = base64::engine::general_purpose::STANDARD.encode(&encrypted_bytes);

    println!();
    println!("Encrypted test data size: {} bytes", encrypted_bytes.len());

    // Save encrypted data
    let encrypted_path = args.output_dir.join("encrypted_data.b64");
    let mut encrypted_file = File::create(&encrypted_path)
        .context("Failed to create encrypted_data.b64")?;
    encrypted_file.write_all(encrypted_b64.as_bytes())
        .context("Failed to write encrypted data")?;

    println!("✅ Encrypted data saved: {:?}", encrypted_path);

    // Save binary version
    let encrypted_bin_path = args.output_dir.join("encrypted_data.bin");
    fs::write(&encrypted_bin_path, &encrypted_bytes)
        .context("Failed to write encrypted_data.bin")?;
    println!("✅ Encrypted data (binary) saved: {:?}", encrypted_bin_path);

    // Save metadata
    let metadata = format!(
        "# TFHE Test Keys\n\
         Generated: {}\n\
         Test value: {}\n\
         Server key size: {} bytes\n\
         Encrypted data size: {} bytes\n\
         \n\
         ## Usage in E2E tests:\n\
         \n\
         ```bash\n\
         SERVER_KEY=$(cat {}/server_key.b64)\n\
         ENCRYPTED_DATA=$(cat {}/encrypted_data.b64)\n\
         ```\n",
        chrono::Utc::now().to_rfc3339(),
        args.test_value,
        server_key_bytes.len(),
        encrypted_bytes.len(),
        args.output_dir.display(),
        args.output_dir.display()
    );

    let readme_path = args.output_dir.join("README.md");
    fs::write(&readme_path, metadata)
        .context("Failed to write README.md")?;
    println!("✅ Metadata saved: {:?}", readme_path);

    println!();
    println!("=========================================");
    println!("✅ ALL FILES GENERATED SUCCESSFULLY");
    println!("=========================================");
    println!();
    println!("Output directory: {:?}", args.output_dir.canonicalize()?);
    println!();
    println!("Files created:");
    println!("  - server_key.b64      (base64, for API requests)");
    println!("  - server_key.bin      (binary, for direct loading)");
    println!("  - encrypted_data.b64  (base64, for API requests)");
    println!("  - encrypted_data.bin  (binary, for direct loading)");
    println!("  - README.md           (metadata and usage)");
    println!();
    println!("Next steps:");
    println!("  1. Use these keys in E2E tests");
    println!("  2. For CI/CD: commit to test-keys/ directory");
    println!("  3. For local dev: regenerate as needed");
    println!();

    Ok(())
}
