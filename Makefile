.PHONY: help demo-up demo-down demo-restart start stop status logs clean build build-all deploy init-marketplace check-provers start-provers stop-provers tunnel-help

# Colors
GREEN  := \033[0;32m
BLUE   := \033[0;34m
YELLOW := \033[1;33m
RED    := \033[0;31m
NC     := \033[0m # No Color

help: ## Show this help message
	@echo ""
	@echo "$(BLUE)ZyberLink Demo - Makefile Commands$(NC)"
	@echo "======================================"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(GREEN)%-20s$(NC) %s\n", $$1, $$2}'
	@echo ""

# ============================================================================
# Demo Commands (Main)
# ============================================================================

demo-up: ## Start complete demo stack (validator + backend + provers + frontend)
	@echo "$(BLUE)Starting ZyberLink demo stack...$(NC)"
	@./start-demo.sh

demo-down: stop ## Stop all demo services

demo-restart: demo-down demo-up ## Restart complete demo stack

# ============================================================================
# Development Commands (Full Setup)
# ============================================================================

start: ## Start complete localnet (setup + deploy + services)
	@echo "$(BLUE)Starting ZyberLink localnet (full setup)...$(NC)"
	@scripts/start-localnet.sh

stop: ## Stop all services
	@echo "$(BLUE)Stopping all services...$(NC)"
	@scripts/stop-localnet.sh

restart: stop start ## Restart all services (full setup)

status: ## Check status of all services
	@scripts/localnet-status.sh

# ============================================================================
# Marketplace Commands
# ============================================================================

init-marketplace: ## Initialize marketplace on-chain
	@echo "$(BLUE) Initializing marketplace...$(NC)"
	@if [ ! -f "blink-server/.env" ]; then \
		echo "$(RED)ERROR: blink-server/.env not found. Run 'make start' first$(NC)"; \
		exit 1; \
	fi
	@export $$(grep -v '^#' blink-server/.env | xargs) && \
	cargo run --manifest-path sdk/Cargo.toml --example initialize_program

check-provers: ## Check if provers are registered in marketplace
	@echo "$(BLUE) Checking prover registration...$(NC)"
	@for i in 1 2 3; do \
		if [ -f "/tmp/prover-$$i-keypair.json" ]; then \
			ADDR=$$(solana address --keypair /tmp/prover-$$i-keypair.json); \
			echo "  Prover $$i: $$ADDR"; \
		fi; \
	done
	@echo "$(YELLOW)Note: Provers auto-register when they start$(NC)"

# ============================================================================
# Prover Commands
# ============================================================================

start-provers: ## Start 3 prover nodes
	@echo "$(BLUE) Starting prover nodes...$(NC)"
	@scripts/localnet-start-provers.sh

stop-provers: ## Stop all prover nodes
	@echo "$(BLUE)⏹️  Stopping provers...$(NC)"
	@pkill -f cypherlink-prover || true
	@echo "$(GREEN) Provers stopped$(NC)"

restart-provers: stop-provers start-provers ## Restart prover nodes

# ============================================================================
# Individual Service Commands
# ============================================================================

start-validator: ## Start Solana validator only
	@echo "$(BLUE)  Starting Solana validator...$(NC)"
	@solana-test-validator --reset --rpc-port 8899 --ledger $$HOME/.zyberlink-localnet-ledger --limit-ledger-size 10000000 > /tmp/solana-validator.log 2>&1 &
	@sleep 5
	@echo "$(GREEN) Validator started$(NC)"

stop-validator: ## Stop Solana validator
	@pkill -f solana-test-validator || true
	@echo "$(GREEN) Validator stopped$(NC)"

start-backend: ## Start backend server only
	@echo "$(BLUE) Starting backend server...$(NC)"
	@if [ -f "blink-server/.env" ]; then \
		export $$(grep -v '^#' blink-server/.env | xargs); \
	fi; \
	RUST_LOG=info ./target/release/blink-server > /tmp/blink-server.log 2>&1 &
	@sleep 2
	@echo "$(GREEN) Backend started$(NC)"

stop-backend: ## Stop backend server
	@pkill -f blink-server || true
	@echo "$(GREEN) Backend stopped$(NC)"

start-frontend: ## Start frontend dev server
	@echo "$(BLUE) Starting frontend...$(NC)"
	@cd webapp && npm run dev > ~/zyberlink-logs/frontend.log 2>&1 &
	@echo "$(GREEN) Frontend started at http://localhost:5173$(NC)"

stop-frontend: ## Stop frontend server
	@pkill -f vite || true
	@echo "$(GREEN) Frontend stopped$(NC)"

# ============================================================================
# Log Commands
# ============================================================================

logs-validator: ## Tail validator logs
	@tail -f /tmp/solana-validator.log

logs-backend: ## Tail backend logs
	@tail -f /tmp/blink-server.log

logs-provers: ## Tail all prover logs
	@tail -f /tmp/prover-*.log

logs-frontend: ## Tail frontend logs
	@tail -f ~/zyberlink-logs/frontend.log

logs: ## Tail all logs
	@echo "$(BLUE) Tailing all logs (Ctrl+C to stop)...$(NC)"
	@tail -f /tmp/solana-validator.log /tmp/blink-server.log /tmp/prover-*.log ~/zyberlink-logs/frontend.log 2>/dev/null

# ============================================================================
# Build Commands
# ============================================================================

build-program: ## Build Solana program
	@echo "$(BLUE) Building Solana program...$(NC)"
	@cd programs && cargo build-sbf
	@echo "$(GREEN) Program built$(NC)"

build-backend: ## Build backend server
	@echo "$(BLUE) Building backend server...$(NC)"
	@cargo build --release --bin blink-server
	@echo "$(GREEN) Backend built$(NC)"

build-prover: ## Build prover node
	@echo "$(BLUE) Building prover node...$(NC)"
	@cargo build --release --bin cypherlink-prover
	@echo "$(GREEN) Prover built$(NC)"

build-frontend: ## Build frontend
	@echo "$(BLUE) Building frontend...$(NC)"
	@cd webapp && npm install && npm run build
	@echo "$(GREEN) Frontend built$(NC)"

build-all: build-program build-backend build-prover ## Build all components
	@echo "$(GREEN) All components built successfully!$(NC)"

# ============================================================================
# Setup Commands
# ============================================================================

setup: ## Setup infrastructure (validator, database, deploy program)
	@echo "$(BLUE)  Setting up infrastructure...$(NC)"
	@scripts/setup-localnet.sh

deploy: build-program ## Build and deploy Solana program
	@echo "$(BLUE) Deploying program to localnet...$(NC)"
	@scripts/setup-localnet.sh

# ============================================================================
# Database Commands
# ============================================================================

db-start: ## Start PostgreSQL database
	@echo "$(BLUE) Starting PostgreSQL...$(NC)"
	@podman-compose up -d
	@echo "$(GREEN) Database started$(NC)"

db-stop: ## Stop PostgreSQL database
	@echo "$(BLUE) Stopping PostgreSQL...$(NC)"
	@podman-compose down

db-logs: ## Show PostgreSQL logs
	@podman logs -f zyberlink-postgres

db-shell: ## Open PostgreSQL shell
	@podman exec -it zyberlink-postgres psql -U zyberlink -d zyberlink

db-reset: ## Reset database (drop and recreate)
	@echo "$(YELLOW)  Resetting database...$(NC)"
	@podman exec zyberlink-postgres psql -U zyberlink -d postgres -c "DROP DATABASE IF EXISTS zyberlink;"
	@podman exec zyberlink-postgres psql -U zyberlink -d postgres -c "CREATE DATABASE zyberlink;"
	@echo "$(GREEN) Database reset$(NC)"

# ============================================================================
# Testing Commands
# ============================================================================

test: ## Run tests
	@echo "$(BLUE) Running tests...$(NC)"
	@cargo test

test-program: ## Run Solana program tests
	@echo "$(BLUE) Running program tests...$(NC)"
	@cd programs && cargo test-sbf

test-e2e: ## Run end-to-end tests
	@echo "$(BLUE) Running E2E tests...$(NC)"
	@cargo test --test e2e_workflow_test

# ============================================================================
# Job Creation Commands
# ============================================================================

create-job: ## Create a single test job
	@echo "$(BLUE) Creating test job...$(NC)"
	@if [ ! -f "blink-server/.env" ]; then \
		echo "$(RED)ERROR: blink-server/.env not found. Run 'make start' first$(NC)"; \
		exit 1; \
	fi
	@export $$(grep -v '^#' blink-server/.env | xargs) && \
	cargo run --manifest-path sdk/Cargo.toml --example create_test_job

start-job-creator: ## Start job creator (creates jobs every 10 seconds)
	@echo "$(BLUE) Starting job creator...$(NC)"
	@scripts/start-job-creator.sh

list-jobs: ## List all jobs from backend API
	@echo "$(BLUE) Fetching jobs from backend...$(NC)"
	@curl -s http://localhost:8080/api/jobs | jq '.jobs[] | {job_id, status, operation, price: (.price_lamports / 1000000000), created_at}'

list-jobs-active: ## List only active jobs
	@echo "$(BLUE) Fetching active jobs...$(NC)"
	@curl -s "http://localhost:8080/api/jobs?status=active" | jq '.jobs[] | {job_id, status, operation, price: (.price_lamports / 1000000000)}'

inspect-accounts: ## Inspect all on-chain accounts (jobs, provers)
	@echo "$(BLUE) Inspecting on-chain accounts...$(NC)"
	@if [ ! -f "blink-server/.env" ]; then \
		echo "$(RED)ERROR: blink-server/.env not found. Run 'make start' first$(NC)"; \
		exit 1; \
	fi
	@export $$(grep -v '^#' blink-server/.env | xargs) && \
	cargo run --manifest-path sdk/Cargo.toml --example inspect_accounts

watch-jobs: ## Watch jobs in real-time (refresh every 5s)
	@echo "$(BLUE) Watching jobs (Ctrl+C to stop)...$(NC)"
	@while true; do \
		clear; \
		echo "=== ZyberLink Jobs (refreshing every 5s) ==="; \
		echo ""; \
		curl -s http://localhost:8080/api/jobs 2>/dev/null | jq -r '.jobs[] | "\(.job_id) | \(.status) | \(.operation) | \(.price_lamports / 1000000000) SOL"' | column -t -s'|' || echo "Backend not responding"; \
		sleep 5; \
	done

# ============================================================================
# Cleanup Commands
# ============================================================================

clean: ## Clean build artifacts
	@echo "$(BLUE) Cleaning build artifacts...$(NC)"
	@cargo clean
	@cd programs && cargo clean
	@echo "$(GREEN) Build artifacts cleaned$(NC)"

clean-logs: ## Remove all log files
	@echo "$(BLUE) Cleaning logs...$(NC)"
	@rm -f /tmp/solana-validator.log /tmp/blink-server.log /tmp/prover-*.log
	@rm -f ~/zyberlink-logs/*.log
	@echo "$(GREEN) Logs cleaned$(NC)"

clean-ledger: ## Remove validator ledger data
	@echo "$(BLUE) Cleaning validator ledger...$(NC)"
	@rm -rf $$HOME/.zyberlink-localnet-ledger
	@echo "$(GREEN) Ledger cleaned$(NC)"

clean-all: clean clean-logs clean-ledger ## Clean everything (builds + logs + ledger)
	@echo "$(GREEN) Everything cleaned!$(NC)"

# ============================================================================
# Development Helpers
# ============================================================================

tunnel-help: ## Show SSH tunnel setup instructions
	@echo ""
	@echo "$(BLUE)======================================"
	@echo "  ZyberLink SSH Tunnels Setup"
	@echo "======================================$(NC)"
	@echo ""
	@echo "Puertos necesarios:"
	@echo "  - 5173: Frontend (Vite dev server)"
	@echo "  - 8080: Backend API"
	@echo "  - 8899: Solana RPC HTTP"
	@echo "  - 8900: Solana RPC WebSocket"
	@echo "  - 9900: Solana Faucet"
	@echo "  - 5432: PostgreSQL"
	@echo ""
	@echo "$(YELLOW)Ejecutá esto en tu máquina LOCAL:$(NC)"
	@echo ""
	@echo "ssh -L 5173:localhost:5173 \\"
	@echo "    -L 8080:localhost:8080 \\"
	@echo "    -L 8899:localhost:8899 \\"
	@echo "    -L 8900:localhost:8900 \\"
	@echo "    -L 9900:localhost:9900 \\"
	@echo "    -L 5432:localhost:5432 \\"
	@echo "    deploy@YOUR_SERVER_IP"
	@echo ""
	@echo "$(BLUE)Luego accedé a:$(NC)"
	@echo "  - Frontend:  http://localhost:5173"
	@echo "  - Backend:   http://localhost:8080/health"
	@echo "  - Validator: http://localhost:8899 (RPC)"
	@echo ""

health: ## Check health of all services
	@echo "$(BLUE) Checking service health...$(NC)"
	@echo ""
	@echo -n "Validator: "
	@curl -s http://localhost:8899 -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' 2>/dev/null | grep -q "ok" && echo "$(GREEN) OK$(NC)" || echo "$(RED) Down$(NC)"
	@echo -n "Backend:   "
	@curl -s http://localhost:8080/health > /dev/null 2>&1 && echo "$(GREEN) OK$(NC)" || echo "$(RED) Down$(NC)"
	@echo -n "Database:  "
	@podman exec zyberlink-postgres psql -U zyberlink -d zyberlink -c "SELECT 1" > /dev/null 2>&1 && echo "$(GREEN) OK$(NC)" || echo "$(RED) Down$(NC)"
	@echo -n "Frontend:  "
	@curl -s http://localhost:5173 > /dev/null 2>&1 && echo "$(GREEN) OK$(NC)" || echo "$(RED) Down$(NC)"
	@echo ""

airdrop: ## Airdrop SOL to prover wallets
	@echo "$(BLUE) Airdropping SOL to provers...$(NC)"
	@for i in 1 2 3; do \
		if [ -f "/tmp/prover-$$i-keypair.json" ]; then \
			ADDR=$$(solana address --keypair /tmp/prover-$$i-keypair.json); \
			echo "  Prover $$i: $$ADDR"; \
			solana airdrop 5 $$ADDR --url http://localhost:8899 2>/dev/null || true; \
		fi; \
	done
	@echo "$(GREEN) Airdrops complete!$(NC)"

airdrop-job-creator: ## Airdrop 100 SOL to job creator
	@echo "$(BLUE) Airdropping to job creator...$(NC)"
	@if [ ! -f "/tmp/job-creator-keypair.json" ]; then \
		echo "Creating job creator keypair..."; \
		solana-keygen new --no-bip39-passphrase --force --outfile /tmp/job-creator-keypair.json >/dev/null 2>&1; \
	fi
	@ADDR=$$(solana address --keypair /tmp/job-creator-keypair.json); \
	echo "  Job creator: $$ADDR"; \
	solana airdrop 100 $$ADDR --url http://localhost:8899
	@echo "$(GREEN) Airdrop complete!$(NC)"

check-balances: ## Check balances of all wallets
	@echo "$(BLUE) Wallet Balances:$(NC)"
	@echo ""
	@echo "Provers:"
	@for i in 1 2 3; do \
		if [ -f "/tmp/prover-$$i-keypair.json" ]; then \
			ADDR=$$(solana address --keypair /tmp/prover-$$i-keypair.json); \
			BALANCE=$$(solana balance --keypair /tmp/prover-$$i-keypair.json --url http://localhost:8899 2>/dev/null || echo "0"); \
			echo "  Prover $$i: $$BALANCE ($$ADDR)"; \
		fi; \
	done
	@echo ""
	@if [ -f "/tmp/job-creator-keypair.json" ]; then \
		ADDR=$$(solana address --keypair /tmp/job-creator-keypair.json); \
		BALANCE=$$(solana balance --keypair /tmp/job-creator-keypair.json --url http://localhost:8899 2>/dev/null || echo "0"); \
		echo "Job Creator: $$BALANCE ($$ADDR)"; \
	fi
	@echo ""

# ============================================================================
# Quick Start Workflows
# ============================================================================

quickstart: build-all start init-marketplace ## Full setup from scratch (recommended for first time)
	@echo ""
	@echo "$(GREEN)======================================"
	@echo "  ZyberLink is ready!"
	@echo "======================================$(NC)"
	@echo ""
	@echo "Next steps:"
	@echo "  1. Setup SSH tunnels:  make tunnel-help"
	@echo "  2. Check status:       make status"
	@echo "  3. View logs:          make logs"
	@echo "  4. Open frontend:      http://localhost:5173"
	@echo ""

dev: demo-up ## Quick start for development (use after first setup)
	@echo "$(GREEN) Demo stack is running!$(NC)"
	@make health
