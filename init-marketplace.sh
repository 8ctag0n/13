#!/bin/bash
#
# Initialize the marketplace on localnet (ADMIN ONLY)
#

set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

log_step() { echo -e "\n${BLUE}[STEP]${NC} $1"; }
log_ok() { echo -e "${GREEN}[OK]${NC} $1"; }

# Load config
export $(grep -v '^#' blink-server/.env | xargs)

echo ""
echo "==========================================="
echo "  Initialize Marketplace (Admin)"
echo "==========================================="
echo ""

# Check localnet
if ! curl -s http://127.0.0.1:8080/health >/dev/null 2>&1; then
    echo "ERROR: Localnet not running. Start with: ./start-localnet.sh"
    exit 1
fi

# Admin keypair
ADMIN_KEYPAIR="${ADMIN_KEYPAIR:-$HOME/.config/solana/id.json}"

log_step "Admin keypair: $ADMIN_KEYPAIR"
solana address -k "$ADMIN_KEYPAIR"

log_step "Checking balance..."
solana balance -k "$ADMIN_KEYPAIR"

log_step "Building init tool..."
mkdir -p /tmp/marketplace-init/src
cat > /tmp/marketplace-init/Cargo.toml <<EOF
[package]
name = "marketplace-init"
version = "0.1.0"
edition = "2021"

[dependencies]
cypherlink-sdk = { path = "$(pwd)/sdk" }
solana-sdk = "2.3"
solana-client = "2.3"
anyhow = "1.0"
serde_json = "1.0"
EOF

cat > /tmp/marketplace-init/src/main.rs <<'EOFRUST'
use anyhow::{Context, Result};
use cypherlink_sdk::MarketplaceSDK;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

fn main() -> Result<()> {
    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "http://localhost:8899".to_string());
    let program_id: solana_sdk::pubkey::Pubkey = std::env::var("PROGRAM_ID")
        .context("PROGRAM_ID required")?
        .parse()?;
    let keypair_path = std::env::var("ADMIN_KEYPAIR")
        .unwrap_or_else(|_| format!("{}/.config/solana/id.json", std::env::var("HOME").unwrap()));

    let keypair_json: Vec<u8> = serde_json::from_str(&std::fs::read_to_string(&keypair_path)?)?;
    let admin = Keypair::try_from(keypair_json.as_slice())?;

    println!("Admin: {}", admin.pubkey());
    println!("Program: {}", program_id);
    println!();

    let rpc = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());
    let sdk = MarketplaceSDK::new(program_id);

    let init_ix = sdk.initialize(
        admin.pubkey(),
        1000,           // 10% fee
        5_000_000_000,  // 5 SOL min stake
        500,            // min reputation
        600,            // 10 min timeout
    )?;

    println!("Config: fee=10%, stake=5 SOL, reputation=500, timeout=600s");

    let blockhash = rpc.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&[init_ix], Some(&admin.pubkey()));
    tx.sign(&[&admin], blockhash);

    match rpc.send_and_confirm_transaction(&tx) {
        Ok(sig) => {
            println!("\nSuccess! Signature: {}", sig);
            Ok(())
        }
        Err(e) if e.to_string().contains("already in use") => {
            println!("\nMarketplace already initialized");
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}
EOFRUST

cd /tmp/marketplace-init
cargo build --release 2>&1 | tail -5
log_ok "Built"

log_step "Initializing marketplace..."
ADMIN_KEYPAIR="$ADMIN_KEYPAIR" \
SOLANA_RPC_URL="$SOLANA_RPC_URL" \
PROGRAM_ID="$PROGRAM_ID" \
./target/release/marketplace-init

log_ok "Done!"
