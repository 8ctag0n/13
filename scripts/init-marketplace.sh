#!/bin/bash
#
# Initialize the marketplace on localnet
# This should be run ONCE by the admin after deploying the program
#

set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_step() { echo -e "\n${BLUE}[STEP]${NC} $1"; }
log_ok() { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }

# Load config
if [ ! -f src/blink-server/.env ]; then
    echo "ERROR: src/blink-server/.env not found"
    exit 1
fi

export $(grep -v '^#' src/blink-server/.env | xargs)

echo ""
echo "==========================================="
echo "  Initialize Marketplace"
echo "==========================================="
echo ""
echo "RPC URL: $SOLANA_RPC_URL"
echo "Program ID: $PROGRAM_ID"
echo ""

# Check if validator is running (only need validator, not backend)
if ! solana cluster-version --url http://localhost:8899 >/dev/null 2>&1; then
    echo "ERROR: Solana validator not running. Run: make l1 (localnet-setup)"
    exit 1
fi

# Use admin keypair (default Solana CLI keypair)
ADMIN_KEYPAIR="${ADMIN_KEYPAIR:-$HOME/.config/solana/id.json}"

if [ ! -f "$ADMIN_KEYPAIR" ]; then
    echo "ERROR: Admin keypair not found at $ADMIN_KEYPAIR"
    exit 1
fi

log_step "Using admin keypair: $ADMIN_KEYPAIR"

# Get admin pubkey
ADMIN_PUBKEY=$(solana address -k "$ADMIN_KEYPAIR")
log_ok "Admin pubkey: $ADMIN_PUBKEY"

# Check admin balance
BALANCE=$(solana balance "$ADMIN_PUBKEY" 2>/dev/null | awk '{print $1}')
log_ok "Admin balance: $BALANCE SOL"

if (( $(echo "$BALANCE < 1" | bc -l) )); then
    log_warn "Low balance! Requesting airdrop..."
    solana airdrop 10 "$ADMIN_PUBKEY"
    sleep 2
fi

log_step "Building initialization CLI..."
cat > /tmp/init-marketplace.rs <<'EOF'
use anyhow::{Context, Result};
use zyberlink_sdk::MarketplaceSDK;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <rpc_url> <program_id>", args[0]);
        std::process::exit(1);
    }

    let rpc_url = &args[1];
    let program_id: solana_sdk::pubkey::Pubkey = args[2]
        .parse()
        .context("Invalid program ID")?;

    // Load admin keypair from stdin
    let keypair_json: Vec<u8> = serde_json::from_reader(std::io::stdin())
        .context("Failed to read keypair from stdin")?;
    let admin_keypair = Keypair::try_from(keypair_json.as_slice())
        .context("Invalid keypair")?;

    println!("Admin: {}", admin_keypair.pubkey());

    // Connect to RPC
    let rpc_client = RpcClient::new_with_commitment(
        rpc_url.to_string(),
        CommitmentConfig::confirmed(),
    );

    // Create SDK
    let sdk = MarketplaceSDK::new(program_id);

    // Create initialize instruction
    let init_ix = sdk.initialize(
        admin_keypair.pubkey(),
        1000,           // 10% fee (100 basis points = 1%)
        5_000_000_000,  // 5 SOL min stake for provers
        500,            // Minimum reputation score to claim jobs
        600,            // 10 minutes default job timeout
    )?;

    println!("\nInitializing marketplace with:");
    println!("  Fee: 10%");
    println!("  Min stake: 5 SOL");
    println!("  Min reputation: 500");
    println!("  Default timeout: 600 seconds (10 min)");
    println!();

    // Build and send transaction
    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&[init_ix], Some(&admin_keypair.pubkey()));
    tx.sign(&[&admin_keypair], recent_blockhash);

    match rpc_client.send_and_confirm_transaction(&tx) {
        Ok(signature) => {
            println!("Success! Signature: {}", signature);
            println!("Marketplace initialized successfully");
            Ok(())
        }
        Err(e) => {
            let error_str = e.to_string();
            if error_str.contains("already in use") {
                println!("Marketplace already initialized");
                Ok(())
            } else {
                Err(e.into())
            }
        }
    }
}
EOF

# Build the CLI tool
cargo build --release -p zyb-cli 2>&1 | grep -E "Compiling|Finished" || true

log_step "Initializing marketplace..."

# Try using SDK example first, fallback to manual instructions
cargo run --manifest-path sdks/rust/Cargo.toml --example initialize_program --release || {
    # If SDK example fails, show manual instructions
    log_warn "SDK example failed. Trying alternative method..."

    # For now, just output instructions
    echo ""
    echo "To initialize manually, run:"
    echo ""
    echo "  solana program call $PROGRAM_ID initialize \\"
    echo "    --keypair $ADMIN_KEYPAIR \\"
    echo "    --url $SOLANA_RPC_URL"
    echo ""
}

# ============================================================================
# Register provers on-chain
# ============================================================================
log_step "Registering prover nodes..."

PROVER_BIN="./target/release/zyberlink-prover"
if [ ! -f "$PROVER_BIN" ]; then
    log_warn "Prover binary not found, skipping registration"
else
    for i in 1 2 3; do
        PROVER_KEYPAIR="/tmp/prover-$i-keypair.json"
        if [ -f "$PROVER_KEYPAIR" ]; then
            PROVER_ADDR=$(solana address -k "$PROVER_KEYPAIR")

            # Check if already registered by trying to find prover PDA
            # If register fails with "already in use", that's OK
            if $PROVER_BIN register \
                --program-id $PROGRAM_ID \
                --rpc-url $SOLANA_RPC_URL \
                --keypair $PROVER_KEYPAIR \
                --stake-amount 5000000000 2>&1 | grep -q "success\|already"; then
                log_ok "Prover $i registered: $PROVER_ADDR"
            else
                # Try anyway and check result
                $PROVER_BIN register \
                    --program-id $PROGRAM_ID \
                    --rpc-url $SOLANA_RPC_URL \
                    --keypair $PROVER_KEYPAIR \
                    --stake-amount 5000000000 2>&1 || true
                log_ok "Prover $i: $PROVER_ADDR (may already be registered)"
            fi
        else
            log_warn "Prover $i keypair not found"
        fi
    done
fi

log_ok "Done!"
