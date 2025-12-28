//! Test minimal Aptos transaction primitives
//!
//! Run with: cargo run -p zyberlink-chain-client --example test_aptos_primitives --features aptos

use zyberlink_chain_client::aptos_primitives::*;

fn main() -> Result<(), String> {
    println!("Testing Aptos Minimal Primitives\n");
    println!("=================================\n");

    // Test 1: Create random account
    println!("1. Creating random account...");
    let account = AptosAccount::generate();
    println!("   ✓ Address: {}", account.address_hex());
    println!("   ✓ Private Key: {}...", &account.private_key_hex()[..16]);
    println!();

    // Test 2: Parse addresses
    println!("2. Parsing addresses...");
    let addr1 = AccountAddress::from_hex("0x1")?;
    println!("   ✓ Parsed 0x1");

    let addr2 = AccountAddress::from_hex(
        "0x0000000000000000000000000000000000000000000000000000000000000001"
    )?;
    println!("   ✓ Parsed full address");
    println!();

    // Test 3: Create raw transaction
    println!("3. Creating raw transaction...");
    let raw_tx = RawTransaction::new_entry_function(
        account.address.clone(),
        0, // sequence number
        "0x1",
        "aptos_account",
        "transfer",
        vec![], // no type args
        vec![
            bcs::to_bytes(&AccountAddress::one()).unwrap(), // recipient
            bcs::to_bytes(&1000u64).unwrap(),               // amount
        ],
        4, // chain_id (testnet)
    )?;
    println!("   ✓ Created transaction");
    println!("   - Sender: {}", raw_tx.sender.to_hex());
    println!("   - Max gas: {}", raw_tx.max_gas_amount);
    println!();

    // Test 4: Sign transaction
    println!("4. Signing transaction...");
    let signed_tx = raw_tx.sign(&account.private_key)?;
    println!("   ✓ Transaction signed");

    match &signed_tx.authenticator {
        TransactionAuthenticator::Ed25519 { public_key, signature } => {
            println!("   - Public key: {}...", hex::encode(&public_key[..8]));
            println!("   - Signature: {}...", hex::encode(&signature[..8]));
        }
    }
    println!();

    // Test 5: Serialize to BCS
    println!("5. Serializing to BCS...");
    let bcs_bytes = signed_tx.to_bcs()?;
    println!("   ✓ Serialized: {} bytes", bcs_bytes.len());
    println!("   - First 32 bytes: {}", hex::encode(&bcs_bytes[..32.min(bcs_bytes.len())]));
    println!();

    println!("✅ All primitives working correctly!");
    println!("\nSummary:");
    println!("  - Account generation: ✓");
    println!("  - Address parsing: ✓");
    println!("  - Transaction building: ✓");
    println!("  - Transaction signing: ✓");
    println!("  - BCS serialization: ✓");
    println!("\nTotal dependencies added:");
    println!("  • bcs: ~50 KB");
    println!("  • ed25519-dalek: ~100 KB");
    println!("  • sha3: ~30 KB");
    println!("  • hex: ~10 KB");
    println!("  • rand: ~50 KB");
    println!("  Total: ~240 KB (vs 500+ MB del SDK completo)");

    Ok(())
}
