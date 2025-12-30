#!/bin/bash
# =============================================================================
# ZyberLink Demo Script - Interactive CLI for Grant Video
# =============================================================================
set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m' # No Color
BOLD='\033[1m'

# =============================================================================
# PROFILES CONFIGURATION
# =============================================================================

declare -A PROFILES

# Local profile (localnet)
PROFILES[local_rpc]="http://localhost:8899"
PROFILES[local_backend]="http://localhost:9000"
PROFILES[local_keypair]="/tmp/demo-keypair.json"
PROFILES[local_futarchy_program]="2V8E8DJ3M3J2Aombv8hxRq5t2RecjbdW3GpUmSVruuhX"
PROFILES[local_fhe_program]="GkYw9vNT6EZFZTwoEM2UBuwsAD6nwzArLGghBZqpsF5L"
PROFILES[local_zk_program]="3MBUZ4Gy5g8FJSKjBJVKixr5vFZZXgFqd9tqhwmMWNPv"
PROFILES[local_bedrock_program]="8oSWVqrU78YW3FH6BqrqJqRyqK4p6k5ikzbT7UyekJrH"

# Devnet profile (TODO: update with real devnet program IDs)
PROFILES[devnet_rpc]="https://api.devnet.solana.com"
PROFILES[devnet_backend]="https://api.zyberlink.dev"
PROFILES[devnet_keypair]="$HOME/.config/solana/id.json"
PROFILES[devnet_futarchy_program]="2V8E8DJ3M3J2Aombv8hxRq5t2RecjbdW3GpUmSVruuhX"
PROFILES[devnet_fhe_program]="GkYw9vNT6EZFZTwoEM2UBuwsAD6nwzArLGghBZqpsF5L"
PROFILES[devnet_zk_program]="3MBUZ4Gy5g8FJSKjBJVKixr5vFZZXgFqd9tqhwmMWNPv"
PROFILES[devnet_bedrock_program]="8oSWVqrU78YW3FH6BqrqJqRyqK4p6k5ikzbT7UyekJrH"

# Current profile (default: local)
CURRENT_PROFILE="local"

# Load profile into env vars
load_profile() {
    local profile=$1
    # Core URLs
    export RPC_URL="${PROFILES[${profile}_rpc]}"
    export SOLANA_RPC_URL="${PROFILES[${profile}_rpc]}"
    export BACKEND_URL="${PROFILES[${profile}_backend]}"
    export KEYPAIR_PATH="${PROFILES[${profile}_keypair]}"
    export USER_KEYPAIR="${PROFILES[${profile}_keypair]}"

    # Program IDs (matching CLI expected env vars)
    export FUTARCHY_PROGRAM_ID="${PROFILES[${profile}_futarchy_program]}"
    export FHE_GENERATOR_PROGRAM_ID="${PROFILES[${profile}_fhe_program]}"
    export ZK_GENERATOR_PROGRAM_ID="${PROFILES[${profile}_zk_program]}"
    export BEDROCK_PROGRAM_ID="${PROFILES[${profile}_bedrock_program]}"
    export PROGRAM_ID="${PROFILES[${profile}_fhe_program]}"  # Legacy compat

    CURRENT_PROFILE=$profile
}

# =============================================================================
# UTILITY FUNCTIONS
# =============================================================================

print_header() {
    clear
    echo -e "${CYAN}"
    echo "  ╔═══════════════════════════════════════════════════════════════╗"
    echo "  ║                                                               ║"
    echo "  ║   ███████╗██╗   ██╗██████╗ ███████╗██████╗ ██╗     ██╗███╗   ██╗██╗  ██╗   ║"
    echo "  ║   ╚══███╔╝╚██╗ ██╔╝██╔══██╗██╔════╝██╔══██╗██║     ██║████╗  ██║██║ ██╔╝   ║"
    echo "  ║     ███╔╝  ╚████╔╝ ██████╔╝█████╗  ██████╔╝██║     ██║██╔██╗ ██║█████╔╝    ║"
    echo "  ║    ███╔╝    ╚██╔╝  ██╔══██╗██╔══╝  ██╔══██╗██║     ██║██║╚██╗██║██╔═██╗    ║"
    echo "  ║   ███████╗   ██║   ██████╔╝███████╗██║  ██║███████╗██║██║ ╚████║██║  ██╗   ║"
    echo "  ║   ╚══════╝   ╚═╝   ╚═════╝ ╚══════╝╚═╝  ╚═╝╚══════╝╚═╝╚═╝  ╚═══╝╚═╝  ╚═╝   ║"
    echo "  ║                                                               ║"
    echo "  ║            Private Computation Infrastructure                 ║"
    echo "  ╚═══════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
    echo -e "  ${YELLOW}Profile: ${GREEN}${CURRENT_PROFILE}${NC}  |  ${YELLOW}RPC: ${NC}${RPC_URL}"
    echo ""
}

print_section() {
    echo -e "\n${MAGENTA}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BOLD}  $1${NC}"
    echo -e "${MAGENTA}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
}

prompt() {
    local var_name=$1
    local prompt_text=$2
    local default=$3

    if [ -n "$default" ]; then
        echo -ne "  ${CYAN}$prompt_text${NC} [${GREEN}$default${NC}]: "
        read input
        eval "$var_name=\"${input:-$default}\""
    else
        echo -ne "  ${CYAN}$prompt_text${NC}: "
        read input
        eval "$var_name=\"$input\""
    fi
}

confirm() {
    echo -ne "\n  ${YELLOW}$1${NC} [Y/n]: "
    read -r response
    [[ -z "$response" || "$response" =~ ^[Yy]$ ]]
}

success() {
    echo -e "\n  ${GREEN}✓ $1${NC}"
}

error() {
    echo -e "\n  ${RED}✗ $1${NC}"
}

info() {
    echo -e "  ${BLUE}ℹ $1${NC}"
}

run_cmd() {
    echo -e "\n  ${YELLOW}Running:${NC}"
    echo -e "  ${CYAN}$1${NC}\n"
    eval "$1"
}

pause() {
    echo -e "\n  ${YELLOW}Press Enter to continue...${NC}"
    read
}

# =============================================================================
# PROFILE MANAGEMENT
# =============================================================================

select_profile() {
    print_header
    print_section "Select Profile"

    echo -e "  ${BOLD}1)${NC} local   - Local validator (localhost:8899)"
    echo -e "  ${BOLD}2)${NC} devnet  - Solana Devnet"
    echo ""

    echo -ne "  ${CYAN}Select profile [1-2]:${NC} "
    read choice

    case $choice in
        1) load_profile "local" ;;
        2) load_profile "devnet" ;;
        *) load_profile "local" ;;
    esac

    success "Profile loaded: $CURRENT_PROFILE"
}

show_current_config() {
    print_header
    print_section "Current Configuration"

    echo -e "  ${BOLD}Profile:${NC}          $CURRENT_PROFILE"
    echo -e "  ${BOLD}RPC URL:${NC}          $RPC_URL"
    echo -e "  ${BOLD}Backend URL:${NC}      $BACKEND_URL"
    echo -e "  ${BOLD}Keypair:${NC}          $KEYPAIR_PATH"
    echo -e "  ${BOLD}Futarchy Program:${NC} $FUTARCHY_PROGRAM"
    echo -e "  ${BOLD}FHE Program:${NC}      $FHE_PROGRAM"
    echo -e "  ${BOLD}ZK Program:${NC}       $ZK_PROGRAM"

    pause
}

# =============================================================================
# FUTARCHY V3 (FULLY BLIND MARKETS)
# =============================================================================

create_market_v3() {
    print_header
    print_section "Create Fully Blind Market (V3)"

    info "This creates a market where bet amounts are encrypted with FHE"
    info "and bet sides are hidden using ZK proofs.\n"

    # Generate market ID based on timestamp
    local default_market_id=$(($(date +%s) % 100000))
    local default_end_time=$(($(date +%s) + 3600))  # 1 hour from now
    local default_threshold_pubkey="0000000000000000000000000000000000000000000000000000000000000000"

    prompt market_id "Market ID" "$default_market_id"
    prompt question "Market Question" "Will BTC reach 100k by end of Q1 2025?"
    prompt end_time "End Time (unix timestamp)" "$default_end_time"
    prompt max_bet "Max Bet (lamports)" "10000000000"
    prompt threshold_pubkey "Threshold Pubkey (hex)" "$default_threshold_pubkey"

    echo ""
    info "Market Configuration:"
    echo -e "    ID: $market_id"
    echo -e "    Question: $question"
    echo -e "    End Time: $end_time ($(date -d @$end_time 2>/dev/null || date -r $end_time))"
    echo -e "    Max Bet: $max_bet lamports"

    if confirm "Create this market?"; then
        run_cmd "./target/release/zyb market create-v3 \
            --market-id $market_id \
            --question \"$question\" \
            --end-time $end_time \
            --max-bet $max_bet \
            --threshold-pubkey $threshold_pubkey \
            --keypair $KEYPAIR_PATH \
            --rpc-url $RPC_URL"

        success "Market V3 created!"
        echo -e "\n  ${GREEN}Market ID: $market_id${NC}"
    fi

    pause
}

bet_blind_v3() {
    print_header
    print_section "Place Blind Bet (V3)"

    info "Place a fully private bet - amount encrypted, side hidden.\n"

    prompt market_id "Market ID" ""
    prompt amount "Bet Amount (lamports)" "1000000000"

    echo -e "\n  ${BOLD}Select Side:${NC}"
    echo -e "  ${BOLD}1)${NC} YES"
    echo -e "  ${BOLD}2)${NC} NO"
    echo -ne "  ${CYAN}Choice [1-2]:${NC} "
    read side_choice

    case $side_choice in
        1) side="yes" ;;
        2) side="no" ;;
        *) side="yes" ;;
    esac

    echo ""
    info "Bet Configuration:"
    echo -e "    Market: $market_id"
    echo -e "    Amount: $amount lamports"
    echo -e "    Side: $side (will be hidden in ZK proof)"

    if confirm "Place this blind bet?"; then
        run_cmd "./target/release/zyb market bet-blind \
            --market-id $market_id \
            --amount $amount \
            --side $side \
            --keypair $KEYPAIR_PATH \
            --rpc-url $RPC_URL"

        success "Blind bet placed!"
    fi

    pause
}

settle_market_v3() {
    print_header
    print_section "Settle Market V3"

    info "Settle a V3 market with threshold decryption of pool totals.\n"

    prompt market_id "Market ID" ""

    echo -e "\n  ${BOLD}Select Outcome:${NC}"
    echo -e "  ${BOLD}1)${NC} YES wins"
    echo -e "  ${BOLD}2)${NC} NO wins"
    echo -ne "  ${CYAN}Choice [1-2]:${NC} "
    read outcome_choice

    case $outcome_choice in
        1) outcome="true" ;;
        2) outcome="false" ;;
        *) outcome="true" ;;
    esac

    prompt pool_yes "Decrypted YES Pool (lamports)" "5000000000"
    prompt pool_no "Decrypted NO Pool (lamports)" "3000000000"

    if confirm "Settle market $market_id with outcome $outcome?"; then
        run_cmd "./target/release/zyb market settle-v3 \
            --market-id $market_id \
            --outcome $outcome \
            --decrypted-pool-yes $pool_yes \
            --decrypted-pool-no $pool_no \
            --keypair $KEYPAIR_PATH \
            --rpc-url $RPC_URL"

        success "Market settled!"
    fi

    pause
}

claim_v3() {
    print_header
    print_section "Claim Winnings (V3)"

    info "Claim your winnings from a settled V3 market.\n"

    prompt market_id "Market ID" ""
    prompt witness_file "Witness File" "bet_witness_v3.json"

    if confirm "Claim winnings from market $market_id?"; then
        run_cmd "./target/release/zyb market claim-v3 \
            --market-id $market_id \
            --witness-file $witness_file \
            --keypair $KEYPAIR_PATH \
            --rpc-url $RPC_URL"

        success "Winnings claimed!"
    fi

    pause
}

# =============================================================================
# FHE OPERATIONS
# =============================================================================

fhe_encrypt() {
    print_header
    print_section "FHE Encrypt"

    info "Encrypt a value using Fully Homomorphic Encryption.\n"

    prompt value "Value to encrypt (0-255)" "42"
    prompt output_path "Output path" "./fhe-output"

    if confirm "Encrypt value $value?"; then
        run_cmd "./target/release/zyb fhe encrypt --value $value --path $output_path"

        success "Value encrypted!"
        echo -e "\n  Output files in: $output_path"
    fi

    pause
}

fhe_decrypt() {
    print_header
    print_section "FHE Decrypt"

    info "Decrypt an FHE computation result.\n"

    prompt input_path "FHE output path (with client_key.bin)" "./fhe-output"
    prompt result "Encrypted result (base64, or leave empty to prompt)" ""

    if confirm "Decrypt result?"; then
        if [ -n "$result" ]; then
            run_cmd "./target/release/zyb fhe decrypt --path $input_path --result \"$result\""
        else
            run_cmd "./target/release/zyb fhe decrypt --path $input_path"
        fi

        success "Result decrypted!"
    fi

    pause
}

# =============================================================================
# DEV JOB OPERATIONS
# =============================================================================

dev_job_run() {
    print_header
    print_section "Dev Job - Run FHE Computations"

    info "Run FHE computation jobs for testing.\n"

    echo -e "  ${BOLD}Select Job Type:${NC}"
    echo -e "  ${BOLD}1)${NC} add       - Addition (50 + 10)"
    echo -e "  ${BOLD}2)${NC} multiply  - Multiplication (5 * 3)"
    echo -e "  ${BOLD}3)${NC} sum       - Sum [10,20,30]"
    echo -e "  ${BOLD}4)${NC} threshold - Threshold 75 >= 50"
    echo -e "  ${BOLD}5)${NC} count-if  - Count values >= 18 (Proof of Innocence)"
    echo -e "  ${BOLD}6)${NC} mix       - Run all job types"
    echo ""

    echo -ne "  ${CYAN}Select [1-6]:${NC} "
    read job_choice

    case $job_choice in
        1) job_type="add" ;;
        2) job_type="multiply" ;;
        3) job_type="sum" ;;
        4) job_type="threshold" ;;
        5) job_type="count-if" ;;
        6) job_type="add,multiply,sum,threshold,count-if" ;;
        *) job_type="add" ;;
    esac

    if confirm "Run job type(s): $job_type?"; then
        run_cmd "RUST_LOG=info \
            PROGRAM_ID=$FHE_PROGRAM \
            SOLANA_RPC_URL=$RPC_URL \
            BACKEND_URL=$BACKEND_URL \
            USER_KEYPAIR=$KEYPAIR_PATH \
            ./target/release/zyb dev-job run --types $job_type --once"

        success "Job(s) completed!"
    fi

    pause
}

dev_job_verify() {
    print_header
    print_section "Dev Job - Verify End-to-End"

    info "Submit job, wait for completion, decrypt and verify result.\n"

    echo -e "  ${BOLD}Select Job Type:${NC}"
    echo -e "  ${BOLD}1)${NC} add       - Addition"
    echo -e "  ${BOLD}2)${NC} sum       - Sum"
    echo -e "  ${BOLD}3)${NC} threshold - Threshold check"
    echo -e "  ${BOLD}4)${NC} all       - Verify all types"
    echo ""

    echo -ne "  ${CYAN}Select [1-4]:${NC} "
    read job_choice

    case $job_choice in
        1) job_type="add" ; all_flag="" ;;
        2) job_type="sum" ; all_flag="" ;;
        3) job_type="threshold" ; all_flag="" ;;
        4) job_type="" ; all_flag="--all" ;;
        *) job_type="add" ; all_flag="" ;;
    esac

    if confirm "Run verification?"; then
        if [ -n "$all_flag" ]; then
            run_cmd "RUST_LOG=info \
                PROGRAM_ID=$FHE_PROGRAM \
                SOLANA_RPC_URL=$RPC_URL \
                BACKEND_URL=$BACKEND_URL \
                USER_KEYPAIR=$KEYPAIR_PATH \
                ./target/release/zyb dev-job verify $all_flag"
        else
            run_cmd "RUST_LOG=info \
                PROGRAM_ID=$FHE_PROGRAM \
                SOLANA_RPC_URL=$RPC_URL \
                BACKEND_URL=$BACKEND_URL \
                USER_KEYPAIR=$KEYPAIR_PATH \
                ./target/release/zyb dev-job verify --types $job_type"
        fi

        success "Verification complete!"
    fi

    pause
}

webapp_flow() {
    print_header
    print_section "Webapp Flow Simulation"

    info "Simulate the complete webapp flow:"
    info "  1. Generate FHE keys"
    info "  2. Upload server key"
    info "  3. Create job via validate-and-build"
    info "  4. Sign and submit transaction"
    info "  5. Wait for completion"
    info "  6. Decrypt and verify result\n"

    if confirm "Run webapp flow with verification?"; then
        run_cmd "RUST_LOG=info \
            PROGRAM_ID=$FHE_PROGRAM \
            SOLANA_RPC_URL=$RPC_URL \
            BACKEND_URL=$BACKEND_URL \
            USER_KEYPAIR=$KEYPAIR_PATH \
            ./target/release/zyb dev-job webapp-flow --verify"

        success "Webapp flow completed!"
    fi

    pause
}

proof_of_innocence() {
    print_header
    print_section "Proof of Innocence Demo"

    info "Demonstrate privacy-preserving compliance check:"
    info "  - User's transaction history is encrypted"
    info "  - FHE computation checks against sanctioned addresses"
    info "  - Result: count of matches (0 = innocent)"
    info "  - Server never sees actual history!\n"

    if confirm "Run Proof of Innocence demo?"; then
        run_cmd "RUST_LOG=info \
            PROGRAM_ID=$FHE_PROGRAM \
            SOLANA_RPC_URL=$RPC_URL \
            BACKEND_URL=$BACKEND_URL \
            USER_KEYPAIR=$KEYPAIR_PATH \
            ./target/release/zyb dev-job webapp-flow-poi"

        success "Proof of Innocence verified!"
    fi

    pause
}

# =============================================================================
# QUICK DEMO FLOWS
# =============================================================================

quick_demo_fhe() {
    print_header
    print_section "Quick Demo: FHE Computation"

    info "This will run a complete FHE computation demo:"
    info "  1. Encrypt values locally"
    info "  2. Submit to network for computation"
    info "  3. Receive encrypted result"
    info "  4. Decrypt locally\n"

    echo -e "  ${BOLD}Demo: Sum of encrypted values [10, 20, 30]${NC}"
    echo -e "  Expected result: 60\n"

    if confirm "Start FHE demo?"; then
        echo -e "\n${YELLOW}Step 1: Submitting encrypted computation job...${NC}\n"

        run_cmd "RUST_LOG=info \
            PROGRAM_ID=$FHE_PROGRAM \
            SOLANA_RPC_URL=$RPC_URL \
            BACKEND_URL=$BACKEND_URL \
            USER_KEYPAIR=$KEYPAIR_PATH \
            ./target/release/zyb dev-job verify --types sum"

        success "FHE Demo Complete!"
    fi

    pause
}

quick_demo_futarchy() {
    print_header
    print_section "Quick Demo: Futarchy Prediction Market"

    info "This will demonstrate a fully blind prediction market:"
    info "  1. Create market with encrypted pools"
    info "  2. Place blind bet (side hidden, amount encrypted)"
    info "  3. Settlement with threshold decryption\n"

    local demo_market_id=$(($(date +%s) % 100000))

    echo -e "  ${BOLD}Demo Market ID: $demo_market_id${NC}"
    echo -e "  ${BOLD}Question: Will this demo work?${NC}\n"

    if confirm "Start Futarchy demo?"; then
        echo -e "\n${YELLOW}Creating blind market...${NC}"
        # Add actual commands here when ready
        info "Demo market creation would happen here"

        success "Futarchy Demo Complete!"
    fi

    pause
}

# =============================================================================
# MAIN MENU
# =============================================================================

main_menu() {
    while true; do
        print_header

        echo -e "  ${BOLD}MAIN MENU${NC}\n"

        echo -e "  ${MAGENTA}── Quick Demos ──${NC}"
        echo -e "  ${BOLD}1)${NC}  FHE Computation Demo"
        echo -e "  ${BOLD}2)${NC}  Proof of Innocence Demo"
        echo -e "  ${BOLD}3)${NC}  Webapp Flow Demo"
        echo ""

        echo -e "  ${MAGENTA}── Futarchy V3 (Blind Markets) ──${NC}"
        echo -e "  ${BOLD}4)${NC}  Create Market V3"
        echo -e "  ${BOLD}5)${NC}  Place Blind Bet"
        echo -e "  ${BOLD}6)${NC}  Settle Market V3"
        echo -e "  ${BOLD}7)${NC}  Claim Winnings V3"
        echo ""

        echo -e "  ${MAGENTA}── FHE Operations ──${NC}"
        echo -e "  ${BOLD}8)${NC}  FHE Encrypt"
        echo -e "  ${BOLD}9)${NC}  FHE Decrypt"
        echo ""

        echo -e "  ${MAGENTA}── Dev Tools ──${NC}"
        echo -e "  ${BOLD}10)${NC} Dev Job - Run"
        echo -e "  ${BOLD}11)${NC} Dev Job - Verify"
        echo ""

        echo -e "  ${MAGENTA}── Settings ──${NC}"
        echo -e "  ${BOLD}p)${NC}  Change Profile"
        echo -e "  ${BOLD}c)${NC}  Show Config"
        echo -e "  ${BOLD}q)${NC}  Quit"
        echo ""

        echo -ne "  ${CYAN}Select option:${NC} "
        read choice

        case $choice in
            1)  quick_demo_fhe ;;
            2)  proof_of_innocence ;;
            3)  webapp_flow ;;
            4)  create_market_v3 ;;
            5)  bet_blind_v3 ;;
            6)  settle_market_v3 ;;
            7)  claim_v3 ;;
            8)  fhe_encrypt ;;
            9)  fhe_decrypt ;;
            10) dev_job_run ;;
            11) dev_job_verify ;;
            p|P) select_profile ;;
            c|C) show_current_config ;;
            q|Q)
                echo -e "\n  ${GREEN}Goodbye!${NC}\n"
                exit 0
                ;;
            *)
                error "Invalid option"
                sleep 1
                ;;
        esac
    done
}

# =============================================================================
# INITIALIZATION
# =============================================================================

init() {
    # Check if zyb binary exists
    if [ ! -f "./target/release/zyb" ]; then
        echo -e "${RED}Error: zyb binary not found at ./target/release/zyb${NC}"
        echo -e "Run: cargo build --release -p zyb-cli"
        exit 1
    fi

    # Create demo keypair if not exists
    if [ ! -f "/tmp/demo-keypair.json" ]; then
        echo -e "${YELLOW}Creating demo keypair...${NC}"
        solana-keygen new --no-bip39-passphrase -o /tmp/demo-keypair.json 2>/dev/null || true
    fi

    # Load default profile
    load_profile "local"
}

# =============================================================================
# ENTRY POINT
# =============================================================================

# Handle command line args for non-interactive use
if [ "$1" == "--help" ] || [ "$1" == "-h" ]; then
    echo "ZyberLink Demo Script"
    echo ""
    echo "Usage:"
    echo "  ./demo.sh              # Start interactive menu"
    echo "  ./demo.sh --profile X  # Start with profile (local/devnet)"
    echo "  source demo.sh         # Load functions into shell"
    echo ""
    echo "After sourcing, use: load_profile local|devnet"
    echo ""
    exit 0
fi

if [ "$1" == "--profile" ]; then
    load_profile "${2:-local}"
    shift 2
fi

# Always load default profile so vars are available
load_profile "${CURRENT_PROFILE:-local}"

# If sourced, just load functions. If executed, run menu.
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    init
    main_menu
else
    # When sourced, show loaded config
    echo -e "${GREEN}Demo environment loaded!${NC}"
    echo -e "  Profile: ${CURRENT_PROFILE}"
    echo -e "  RPC: ${RPC_URL}"
    echo -e "  Backend: ${BACKEND_URL}"
    echo ""
    echo -e "Commands: load_profile local|devnet"
fi
