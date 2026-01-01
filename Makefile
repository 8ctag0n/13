.PHONY: help demo-up demo-down demo-restart start stop status logs clean build build-all deploy init-marketplace check-provers start-provers stop-provers tunnel-help dev-up dev-down dev-logs dev-rebuild dev-status dev-shell-backend dev-shell-db serve c0 c1 c2 c3 c4 c-status c-scale c-restart build-public-api build-x402 start-x402 start-public-api start-api-stack stop-x402 stop-public-api stop-api-stack logs-x402 logs-public-api logs-api-stack

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

demo-up: build-all ## Start complete demo stack (validator + backend + provers + frontend)
	@echo "$(BLUE)Starting ZyberLink demo stack...$(NC)"
	@./deployment/scripts/start-demo.sh

demo-down: stop ## Stop all demo services

demo-restart: demo-down demo-up ## Restart complete demo stack

# ============================================================================
# Development Commands (Full Setup)
# ============================================================================

start: ## Start complete localnet (setup + deploy + services)
	@echo "$(BLUE)Starting ZyberLink localnet (full setup)...$(NC)"
	@deployment/scripts/start-localnet.sh

stop: ## Stop all services
	@echo "$(BLUE)Stopping all services...$(NC)"
	@deployment/scripts/stop-localnet.sh

restart: stop start ## Restart all services (full setup)

status: ## Check status of all services
	@deployment/scripts/localnet-status.sh

# ============================================================================
# Marketplace Commands
# ============================================================================

init-marketplace: ## Initialize marketplace on-chain
	@echo "$(BLUE) Initializing marketplace...$(NC)"
	@if [ ! -f "services/blink-server/.env" ]; then \
		echo "$(RED)ERROR: services/blink-server/.env not found. Run 'make start' first$(NC)"; \
		exit 1; \
	fi
	@export $$(grep -v '^#' services/blink-server/.env | xargs) && \
	cargo run --manifest-path chain/solana/programs/sdks/bedrock-sdk/Cargo.toml --example initialize_bedrock

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
	@deployment/scripts/localnet-start-provers.sh

stop-provers: ## Stop all prover nodes
	@echo "$(BLUE)⏹️  Stopping provers...$(NC)"
	@pkill -f zyberlink-prover || true
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

start-backend: ## Start blink-server (internal backend on :8080)
	@echo "$(BLUE) Starting blink-server (internal backend)...$(NC)"
	@if [ -f "services/blink-server/.env" ]; then \
		export $$(grep -v '^#' services/blink-server/.env | xargs); \
	fi; \
	RUST_LOG=info HOST=127.0.0.1 PORT=8080 ./target/release/blink-server > /tmp/blink-server.log 2>&1 &
	@sleep 2
	@echo "$(GREEN) blink-server started on :8080$(NC)"

start-x402: ## Start x402-server (anti-spam gateway on :8081)
	@echo "$(BLUE) Starting x402-server (anti-spam gateway)...$(NC)"
	@RUST_LOG=info HOST=127.0.0.1 PORT=8081 BLINK_URL=http://localhost:8080 \
		./target/release/x402-server > /tmp/x402-server.log 2>&1 &
	@sleep 1
	@echo "$(GREEN) x402-server started on :8081$(NC)"

start-public-api: ## Start public-api (public gateway on :3000)
	@echo "$(BLUE) Starting public-api (public gateway)...$(NC)"
	@RUST_LOG=info HOST=0.0.0.0 PORT=3000 \
		X402_URL=http://localhost:8081 BLINK_URL=http://localhost:8080 \
		./target/release/public-api > /tmp/public-api.log 2>&1 &
	@sleep 1
	@echo "$(GREEN) public-api started on :3000$(NC)"

start-api-stack: start-backend start-x402 start-public-api ## Start full 3-layer API stack
	@echo "$(GREEN) Full API stack started!$(NC)"
	@echo "  blink:8080 (internal) <- x402:8081 (anti-spam) <- public-api:3000 (public)"

stop-backend: ## Stop blink-server
	@pkill -f blink-server || true
	@echo "$(GREEN) blink-server stopped$(NC)"

stop-x402: ## Stop x402-server
	@pkill -f x402-server || true
	@echo "$(GREEN) x402-server stopped$(NC)"

stop-public-api: ## Stop public-api
	@pkill -f public-api || true
	@echo "$(GREEN) public-api stopped$(NC)"

stop-api-stack: stop-public-api stop-x402 stop-backend ## Stop full API stack
	@echo "$(GREEN) API stack stopped$(NC)"

start-frontend: ## Start frontend dev server
	@echo "$(BLUE) Starting frontend...$(NC)"
	@mkdir -p ~/zyberlink-logs
	@cd src/webapp && VITE_API_URL=http://localhost:3000 npm run dev > ~/zyberlink-logs/frontend.log 2>&1 &
	@echo "$(GREEN) Frontend started at http://localhost:5173$(NC)"

stop-frontend: ## Stop frontend server
	@pkill -f vite || true
	@echo "$(GREEN) Frontend stopped$(NC)"

# ============================================================================
# Log Commands
# ============================================================================

logs-validator: ## Tail validator logs
	@tail -f /tmp/solana-validator.log

logs-backend: ## Tail blink-server logs
	@tail -f /tmp/blink-server.log

logs-x402: ## Tail x402-server logs
	@tail -f /tmp/x402-server.log

logs-public-api: ## Tail public-api logs
	@tail -f /tmp/public-api.log

logs-api-stack: ## Tail all API stack logs (blink + x402 + public-api)
	@tail -f /tmp/blink-server.log /tmp/x402-server.log /tmp/public-api.log

logs-provers: ## Tail all prover logs
	@tail -f /tmp/prover-*.log

logs-frontend: ## Tail frontend logs
	@tail -f ~/zyberlink-logs/frontend.log

logs: ## Tail all logs
	@echo "$(BLUE) Tailing all logs (Ctrl+C to stop)...$(NC)"
	@tail -f /tmp/solana-validator.log /tmp/blink-server.log /tmp/x402-server.log /tmp/public-api.log /tmp/prover-*.log ~/zyberlink-logs/frontend.log 2>/dev/null

# ============================================================================
# Build Commands
# ============================================================================

build-program: ## Build Solana program
	@echo "$(BLUE) Building Solana program...$(NC)"
	@cd programs && cargo build-sbf
	@echo "$(GREEN) Program built$(NC)"

build-backend: ## Build backend server (blink-server)
	@echo "$(BLUE) Building blink-server...$(NC)"
	@cargo build --release --bin blink-server
	@echo "$(GREEN) blink-server built$(NC)"

build-public-api: ## Build public-api gateway
	@echo "$(BLUE) Building public-api...$(NC)"
	@cargo build --release --bin public-api
	@echo "$(GREEN) public-api built$(NC)"

build-x402: ## Build x402 anti-spam server
	@echo "$(BLUE) Building x402-server...$(NC)"
	@cargo build --release --bin x402-server
	@echo "$(GREEN) x402-server built$(NC)"

build-prover: ## Build prover node
	@echo "$(BLUE) Building prover node...$(NC)"
	@cargo build --release --bin zyberlink-prover
	@echo "$(GREEN) Prover built$(NC)"

build-frontend: ## Build frontend
	@echo "$(BLUE) Building frontend...$(NC)"
	@cd src/webapp && npm install && npm run build
	@echo "$(GREEN) Frontend built$(NC)"

build-all: build-program build-backend build-public-api build-x402 build-prover ## Build all components
	@echo "$(GREEN) All components built successfully!$(NC)"

# ============================================================================
# Setup Commands
# ============================================================================

setup: ## Setup infrastructure (validator, database, deploy program)
	@echo "$(BLUE)  Setting up infrastructure...$(NC)"
	@deployment/scripts/setup-localnet.sh

deploy: build-program ## Build and deploy Solana program
	@echo "$(BLUE) Deploying program to localnet...$(NC)"
	@deployment/scripts/setup-localnet.sh

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

db-migrate: ## Run database migrations manually
	@echo "$(BLUE)Running database migrations...$(NC)"
	@for f in services/blink-server/migrations/*.sql; do \
		echo "  Applying: $$(basename $$f)"; \
		podman exec -i zyberlink-postgres psql -U zyberlink -d zyberlink < "$$f" 2>/dev/null || true; \
	done
	@echo "$(GREEN)Migrations complete$(NC)"

db-clean-jobs: ## Clean jobs, provers, witnesses and FHE results (frees disk space)
	@echo "$(YELLOW)  Cleaning all job-related data...$(NC)"
	@podman exec zyberlink-postgres psql -U zyberlink -d zyberlink -c "TRUNCATE blockchain_jobs, temp_job_data, provers, witnesses, fhe_results CASCADE;" 2>/dev/null || true
	@echo "$(GREEN) Cleaned: blockchain_jobs, temp_job_data, provers, witnesses, fhe_results$(NC)"

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
	@if [ ! -f "services/blink-server/.env" ]; then \
		echo "$(RED)ERROR: services/blink-server/.env not found. Run 'make start' first$(NC)"; \
		exit 1; \
	fi
	@export $$(grep -v '^#' services/blink-server/.env | xargs) && \
	cargo run --manifest-path sdk/rust/Cargo.toml --example create_test_job

start-job-creator: ## Start dev-job runner (creates jobs every 10 seconds)
	@echo "$(BLUE) Starting dev-job...$(NC)"
	@deployment/scripts/start-job-creator.sh

list-jobs: ## List all jobs from API
	@echo "$(BLUE) Fetching jobs from public-api...$(NC)"
	@curl -s http://localhost:3000/api/jobs/zk | jq '.jobs[] | {job_id, status, circuit_type, price: (.price_lamports / 1000000000), created_at}'

list-jobs-active: ## List only active jobs
	@echo "$(BLUE) Fetching active jobs...$(NC)"
	@curl -s "http://localhost:3000/api/jobs/zk?status=active" | jq '.jobs[] | {job_id, status, circuit_type, price: (.price_lamports / 1000000000)}'

inspect-accounts: ## Inspect all on-chain accounts (jobs, provers)
	@echo "$(BLUE) Inspecting on-chain accounts...$(NC)"
	@if [ ! -f "services/blink-server/.env" ]; then \
		echo "$(RED)ERROR: services/blink-server/.env not found. Run 'make start' first$(NC)"; \
		exit 1; \
	fi
	@export $$(grep -v '^#' services/blink-server/.env | xargs) && \
	cargo run --manifest-path sdk/rust/Cargo.toml --example inspect_accounts

watch-jobs: ## Watch jobs in real-time (refresh every 5s)
	@echo "$(BLUE) Watching jobs (Ctrl+C to stop)...$(NC)"
	@while true; do \
		clear; \
		echo "=== ZyberLink Jobs (refreshing every 5s) ==="; \
		echo ""; \
		curl -s http://localhost:3000/api/jobs/zk 2>/dev/null | jq -r '.jobs[] | "\(.job_id) | \(.status) | \(.circuit_type) | \(.price_lamports / 1000000000) SOL"' | column -t -s'|' || echo "API not responding"; \
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
	@echo "  - 3000: Public API (gateway)"
	@echo "  - 8899: Solana RPC HTTP"
	@echo "  - 8900: Solana RPC WebSocket"
	@echo "  - 9900: Solana Faucet"
	@echo "  - 5432: PostgreSQL"
	@echo ""
	@echo "$(YELLOW)Ejecutá esto en tu máquina LOCAL:$(NC)"
	@echo ""
	@echo "ssh -L 5173:localhost:5173 \\"
	@echo "    -L 3000:localhost:3000 \\"
	@echo "    -L 8899:localhost:8899 \\"
	@echo "    -L 8900:localhost:8900 \\"
	@echo "    -L 9900:localhost:9900 \\"
	@echo "    -L 5432:localhost:5432 \\"
	@echo "    deploy@YOUR_SERVER_IP"
	@echo ""
	@echo "$(BLUE)Luego accedé a:$(NC)"
	@echo "  - Frontend:  http://localhost:5173"
	@echo "  - API:       http://localhost:3000/health"
	@echo "  - Validator: http://localhost:8899 (RPC)"
	@echo ""

health: ## Check health of all services (3-layer architecture)
	@echo "$(BLUE) Checking service health...$(NC)"
	@echo ""
	@echo "=== Infrastructure ==="
	@echo -n "Validator:   "
	@curl -s http://localhost:8899 -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' 2>/dev/null | grep -q "ok" && echo "$(GREEN)OK$(NC)" || echo "$(RED)Down$(NC)"
	@echo -n "Database:    "
	@podman exec zyberlink-postgres psql -U zyberlink -d zyberlink -c "SELECT 1" > /dev/null 2>&1 && echo "$(GREEN)OK$(NC)" || echo "$(RED)Down$(NC)"
	@echo ""
	@echo "=== API Stack (3 capas) ==="
	@echo -n "blink:8080   "
	@curl -s http://localhost:8080/health > /dev/null 2>&1 && echo "$(GREEN)OK$(NC) (internal backend)" || echo "$(RED)Down$(NC)"
	@echo -n "x402:8081    "
	@curl -s http://localhost:8081/health > /dev/null 2>&1 && echo "$(GREEN)OK$(NC) (anti-spam)" || echo "$(RED)Down$(NC)"
	@echo -n "public:3000  "
	@curl -s http://localhost:3000/health > /dev/null 2>&1 && echo "$(GREEN)OK$(NC) (public gateway)" || echo "$(RED)Down$(NC)"
	@echo ""
	@echo "=== Frontend ==="
	@echo -n "Frontend:    "
	@curl -s http://localhost:5173 > /dev/null 2>&1 && echo "$(GREEN)OK$(NC)" || echo "$(RED)Down$(NC)"
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

airdrop-job-creator: ## Airdrop 100 SOL to dev-job wallet
	@echo "$(BLUE) Airdropping to dev-job...$(NC)"
	@if [ ! -f "/tmp/job-creator-keypair.json" ]; then \
		echo "Creating dev-job keypair..."; \
		solana-keygen new --no-bip39-passphrase --force --outfile /tmp/job-creator-keypair.json >/dev/null 2>&1; \
	fi
	@ADDR=$$(solana address --keypair /tmp/job-creator-keypair.json); \
	echo "  Dev job: $$ADDR"; \
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
		echo "Dev Job: $$BALANCE ($$ADDR)"; \
	fi
	@echo ""

# ============================================================================
# Docker Full Stack Commands
# ============================================================================

docker-up: ## Start full stack with Docker Compose (postgres + backend + frontend)
	@echo "$(BLUE)Starting full Docker stack...$(NC)"
	@docker-compose -f deployment/infra/docker/docker-compose.full.yml up -d
	@echo "$(GREEN)Stack started!$(NC)"
	@echo "  - Frontend: http://localhost:5173"
	@echo "  - Backend:  http://localhost:8080"
	@echo "  - Postgres: localhost:5432"

docker-down: ## Stop Docker stack
	@echo "$(BLUE)Stopping Docker stack...$(NC)"
	@docker-compose -f deployment/infra/docker/docker-compose.full.yml down

docker-logs: ## Show Docker stack logs
	@docker-compose -f deployment/infra/docker/docker-compose.full.yml logs -f

docker-build: ## Rebuild Docker images
	@echo "$(BLUE)Building Docker images...$(NC)"
	@docker-compose -f deployment/infra/docker/docker-compose.full.yml build

docker-restart: docker-down docker-up ## Restart Docker stack

docker-status: ## Show Docker container status
	@docker-compose -f deployment/infra/docker/docker-compose.full.yml ps

# ============================================================================
# Production Docker Commands
# ============================================================================

prod-up: ## Start production stack (nginx + backend + postgres)
	@echo "$(BLUE)Starting production stack...$(NC)"
	@docker-compose -f deployment/infra/docker/docker-compose.prod.yml up -d --build
	@echo "$(GREEN)Production stack running!$(NC)"
	@echo "  - Web: http://localhost (or port in .env)"
	@echo "  - API: proxied through nginx at /api/"

prod-down: ## Stop production stack
	@docker-compose -f deployment/infra/docker/docker-compose.prod.yml down

prod-logs: ## Show production logs
	@docker-compose -f deployment/infra/docker/docker-compose.prod.yml logs -f

prod-restart: prod-down prod-up ## Restart production stack

prod-status: ## Show production container status
	@docker-compose -f deployment/infra/docker/docker-compose.prod.yml ps

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

# ============================================================================
# Localnet Setup (Step by Step) - Working Flow
# ============================================================================

localnet-start: ## [STEP 1] Start localnet: validator + db + deploy + backend + provers
	@echo "$(BLUE)Starting ZyberLink localnet (full setup)...$(NC)"
	@deployment/scripts/start-localnet.sh

localnet-init: ## [STEP 2] Initialize marketplace on-chain (run once after setup)
	@echo "$(BLUE)Initializing marketplace...$(NC)"
	@deployment/scripts/init-marketplace.sh

localnet-jobs: ## [STEP 3] Start dev-job runner (auto-creates jobs every 10s)
	@echo "$(BLUE)Starting dev-job (auto-creating jobs)...$(NC)"
	@# Kill any existing dev-job runner
	@-pkill -f "zyb dev-job" 2>/dev/null || true
	@# Create keypair if needed
	@if [ ! -f "/tmp/job-creator-keypair.json" ]; then \
		echo "Creating dev-job keypair..."; \
		solana-keygen new --no-bip39-passphrase --force --outfile /tmp/job-creator-keypair.json >/dev/null 2>&1; \
	fi
	@export $$(grep -v '^#' services/blink-server/.env | xargs) && \
	ADDR=$$(solana address --keypair /tmp/job-creator-keypair.json); \
	echo "  Dev job: $$ADDR"; \
	solana airdrop 100 $$ADDR --url $$SOLANA_RPC_URL 2>/dev/null || echo "  (airdrop may have failed)"; \
	BALANCE=$$(solana balance --keypair /tmp/job-creator-keypair.json --url $$SOLANA_RPC_URL 2>/dev/null); \
	echo "  Balance: $$BALANCE"
	@# Build zyb-cli
	@echo "  Building zyb-cli..."
	@cargo build --release -p zyb-cli --manifest-path cli/core/Cargo.toml 2>&1 | tail -3
	@# Start dev-job in background (pointing to public-api on 3000)
	@export $$(grep -v '^#' services/blink-server/.env | xargs) && \
	RUST_LOG=info \
	BACKEND_URL=http://localhost:3000 \
	SOLANA_RPC_URL=$$SOLANA_RPC_URL \
	PROGRAM_ID=$$PROGRAM_ID \
	USER_KEYPAIR=/tmp/job-creator-keypair.json \
	./target/release/zyb dev-job run > /tmp/dev-job.log 2>&1 &
	@sleep 2
	@if pgrep -f "zyb dev-job" > /dev/null; then \
		echo "$(GREEN)Dev job running! (creating jobs every 10s)$(NC)"; \
		echo "  Logs: tail -f /tmp/dev-job.log"; \
	else \
		echo "$(RED)Dev job failed to start. Check /tmp/dev-job.log$(NC)"; \
	fi

# ============================================================================
# Helpful Aliases
# ============================================================================

localnet-stop: stop ## Stop all localnet services

l1: localnet-start ## Alias: make l1 = start localnet (validator + backend + provers)
l2: localnet-init  ## Alias: make l2 = init marketplace
l3: localnet-jobs  ## Alias: make l3 = start dev-job
l4: start-frontend ## Alias: make l4 = start frontend (webapp)
l0: localnet-stop db-clean-jobs clean-ledger ## Alias: make l0 = stop all + clean jobs + reset validator ledger
	@echo "$(YELLOW)Resetting Solana validator ledger...$(NC)"
	@rm -rf $$HOME/.zyberlink-localnet-ledger 2>/dev/null || true
	@echo "$(GREEN)Ready for fresh start with 'make l1'$(NC)"

# ============================================================================
# Containerized Development (Single Port)
# ============================================================================

dev-up: ## Start containerized dev stack (postgres + backend + frontend/nginx on port 3000)
	@echo "$(BLUE)Starting containerized dev environment...$(NC)"
	@podman-compose -f docker-compose.dev.yml up -d --build
	@echo ""
	@echo "$(GREEN)Dev stack running!$(NC)"
	@echo "  App:     http://localhost:3000"
	@echo "  API:     http://localhost:3000/api/"
	@echo "  Health:  http://localhost:3000/health"
	@echo ""

dev-down: ## Stop containerized dev stack
	@echo "$(BLUE)Stopping containerized dev environment...$(NC)"
	@podman-compose -f docker-compose.dev.yml down
	@echo "$(GREEN)Dev stack stopped$(NC)"

dev-logs: ## Show containerized dev logs
	@podman-compose -f docker-compose.dev.yml logs -f

dev-rebuild: ## Rebuild and restart dev containers
	@echo "$(BLUE)Rebuilding dev containers...$(NC)"
	@podman-compose -f docker-compose.dev.yml down
	@podman-compose -f docker-compose.dev.yml up -d --build --force-recreate
	@echo "$(GREEN)Dev stack rebuilt!$(NC)"

dev-status: ## Show dev container status
	@podman-compose -f docker-compose.dev.yml ps

dev-shell-backend: ## Open shell in backend container
	@podman exec -it zyberlink-backend /bin/bash

dev-shell-db: ## Open psql shell in postgres container
	@podman exec -it zyberlink-postgres psql -U zyberlink -d zyberlink

# Quick alias
serve: dev-up ## Alias: make serve = start containerized dev

# ============================================================================
# Container Test Commands (Production-like with Local Validator)
# ============================================================================
# Similar to l0-l4 but using docker-compose containers with replicas
# Flow: c0 (reset) -> c1 (infra) -> c2 (init) -> c3 (jobs) -> c4 (logs)

c0: ## [CONTAINER] Reset: stop containers + clean volumes
	@echo "$(BLUE)Resetting container environment...$(NC)"
	@-pkill -f "zyb dev-job" 2>/dev/null || true
	@-pkill -f "zyberlink-prover" 2>/dev/null || true
	@podman-compose down -v 2>/dev/null || true
	@podman system prune -f 2>/dev/null || true
	@rm -f .env.containers 2>/dev/null || true
	@echo "$(GREEN)Container environment reset. Ready for 'make c1'$(NC)"

c1: ## [CONTAINER] Start: validator (host) + containers (postgres, backend, webapp, nginx)
	@deployment/scripts/start-containers.sh

c2: ## [CONTAINER] Initialize marketplace + register provers
	@echo "$(BLUE)Initializing marketplace and registering provers...$(NC)"
	@if [ ! -f ".env.containers" ]; then \
		echo "$(RED)ERROR: .env.containers not found. Run 'make c1' first$(NC)"; \
		exit 1; \
	fi
	@echo ""
	@echo "Step 1/3: Initializing marketplace..."
	@export $$(grep -v '^#' .env.containers | xargs) && \
	SOLANA_RPC_URL=http://localhost:8899 cargo run --manifest-path chain/solana/programs/sdks/bedrock-sdk/Cargo.toml --example initialize_bedrock
	@echo "$(GREEN)  Marketplace initialized$(NC)"
	@echo ""
	@echo "Step 2/3: Building prover binary..."
	@cargo build --release --bin zyberlink-prover 2>&1 | tail -3
	@echo "$(GREEN)  Prover built$(NC)"
	@echo ""
	@echo "Step 3/3: Registering provers on-chain..."
	@export $$(grep -v '^#' .env.containers | xargs) && \
	for i in 1 2 3; do \
		KEYPAIR="/tmp/prover-$$i-keypair.json"; \
		if [ -f "$$KEYPAIR" ]; then \
			ADDR=$$(solana address --keypair $$KEYPAIR); \
			./target/release/zyberlink-prover register \
				--program-id $$PROGRAM_ID \
				--rpc-url http://localhost:8899 \
				--keypair $$KEYPAIR \
				--stake-amount 5000000000 2>&1 | tail -1 || true; \
			echo "  Prover $$i registered: $$ADDR"; \
		fi; \
	done
	@echo ""
	@echo "$(GREEN)Marketplace ready with 3 provers!$(NC)"
	@echo "$(YELLOW)Next: make c3 (start provers + dev-job)$(NC)"

c3: ## [CONTAINER] Start provers + dev-job (local binaries)
	@echo "$(BLUE)Starting provers and dev-job...$(NC)"
	@if [ ! -f ".env.containers" ]; then \
		echo "$(RED)ERROR: .env.containers not found. Run 'make c1' first$(NC)"; \
		exit 1; \
	fi
	@-pkill -f "zyb dev-job" 2>/dev/null || true
	@-pkill -f "zyberlink-prover" 2>/dev/null || true
	@echo ""
	@echo "Step 1/2: Starting 3 prover nodes (with FHE/ZK/Futarchy support)..."
	@export $$(grep -v '^#' .env.containers | xargs) && \
	for i in 1 2 3; do \
		KEYPAIR="/tmp/prover-$$i-keypair.json"; \
		if [ -f "$$KEYPAIR" ]; then \
			RUST_LOG=info \
			ZK_GENERATOR_PROGRAM_ID=$$ZK_GENERATOR_PROGRAM_ID \
			FHE_GENERATOR_PROGRAM_ID=$$FHE_GENERATOR_PROGRAM_ID \
			./target/release/zyberlink-prover \
				--program-id $$PROGRAM_ID \
				--rpc-url http://localhost:8899 \
				--gateway-url http://localhost:9000 \
				--blink-backend-url http://localhost:9000 \
				--keypair $$KEYPAIR \
				--enable-futarchy \
				--futarchy-server-url http://localhost:9000 \
				run > /tmp/prover-$$i.log 2>&1 & \
			echo "  Prover $$i started (PID: $$!)"; \
		fi; \
	done
	@sleep 2
	@RUNNING=$$(pgrep -c -f "zyberlink-prover" || echo 0); \
	echo "$(GREEN)  $$RUNNING provers running$(NC)"
	@echo ""
	@echo "Step 2/2: Starting dev-job..."
	@if [ ! -f "/tmp/job-creator-keypair.json" ]; then \
		solana-keygen new --no-bip39-passphrase --force --outfile /tmp/job-creator-keypair.json >/dev/null 2>&1; \
	fi
	@ADDR=$$(solana address --keypair /tmp/job-creator-keypair.json); \
	echo "  Dev job: $$ADDR"; \
	solana airdrop 100 $$ADDR --url http://localhost:8899 2>/dev/null || true; \
	BALANCE=$$(solana balance --keypair /tmp/job-creator-keypair.json --url http://localhost:8899 2>/dev/null); \
	echo "  Balance: $$BALANCE"
	@cargo build --release -p zyb-cli --manifest-path cli/core/Cargo.toml 2>&1 | tail -2
	@export $$(grep -v '^#' .env.containers | xargs) && \
	RUST_LOG=info \
	BACKEND_URL=http://localhost:9000 \
	SOLANA_RPC_URL=http://localhost:8899 \
	USER_KEYPAIR=/tmp/job-creator-keypair.json \
	./target/release/zyb dev-job run > /tmp/dev-job.log 2>&1 &
	@sleep 2
	@if pgrep -f "zyb dev-job" > /dev/null; then \
		echo "$(GREEN)  Dev job running$(NC)"; \
	else \
		echo "$(RED)  Dev job failed. Check /tmp/dev-job.log$(NC)"; \
	fi
	@echo ""
	@echo "$(GREEN)=== DEMO RUNNING ===$(NC)"
	@echo "  Frontend:     http://localhost:9000"
	@echo "  API:          http://localhost:9000/api/jobs"
	@echo ""
	@echo "  Logs:"
	@echo "    - Provers:    tail -f /tmp/prover-*.log"
	@echo "    - Dev job:   tail -f /tmp/dev-job.log"
	@echo "    - Backend:    podman logs -f zyberlink-demo_backend_1"
	@echo ""
	@echo "  Jobs created every 10s, provers claim and process them!"

c4: ## [CONTAINER] Show container logs (all services)
	@echo "$(BLUE)Container logs (Ctrl+C to stop):$(NC)"
	@podman-compose logs -f

c-status: ## [CONTAINER] Show status of containers + validator
	@echo "$(BLUE)Container Stack Status:$(NC)"
	@echo ""
	@echo "Validator:"
	@curl -s http://localhost:8899 -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' 2>/dev/null | grep -q "ok" && echo "  $(GREEN)Running$(NC)" || echo "  $(RED)Not running$(NC)"
	@echo ""
	@echo "Containers:"
	@podman-compose ps 2>/dev/null || echo "  No containers running"
	@echo ""
	@echo "Nginx Health:"
	@curl -s http://localhost:9000/health 2>/dev/null && echo "" || echo "  $(RED)Not responding$(NC)"
	@echo ""
	@echo "Backend Health (via nginx):"
	@curl -s http://localhost:9000/api/health 2>/dev/null || echo "  $(RED)Not responding$(NC)"
	@echo ""

c-scale: ## [CONTAINER] Scale backend to 4 replicas
	@echo "$(BLUE)Scaling backend to 4 replicas...$(NC)"
	@podman-compose up -d --scale backend=4
	@echo "$(GREEN)Scaled!$(NC)"
	@podman-compose ps

c-restart: ## [CONTAINER] Restart containers (keep validator)
	@echo "$(BLUE)Restarting containers...$(NC)"
	@podman-compose restart
	@echo "$(GREEN)Containers restarted$(NC)"

# ============================================================================
# E2E Verification Tests
# ============================================================================

e2e-poi: ## [E2E] Run PoI verification test: CountIf([15,20,25,17], >= 18) -> expect 2
	@echo "$(BLUE)Running PoI E2E Verification Test...$(NC)"
	@# Detect which env file to use (containers vs local)
	@if [ -f ".env.containers" ]; then \
		ENV_FILE=".env.containers"; \
		BACKEND="http://localhost:9000"; \
	elif [ -f "services/blink-server/.env" ]; then \
		ENV_FILE="services/blink-server/.env"; \
		BACKEND="http://localhost:3000"; \
	else \
		echo "$(RED)ERROR: No env file found. Run 'make c1' or 'make l1' first$(NC)"; \
		exit 1; \
	fi; \
	echo "Using env: $$ENV_FILE, backend: $$BACKEND"; \
	echo "Building zyb-cli..."; \
	cargo build --release -p zyb-cli --manifest-path cli/core/Cargo.toml 2>&1 | tail -3; \
	if [ ! -f "/tmp/job-creator-keypair.json" ]; then \
		solana-keygen new --no-bip39-passphrase --force --outfile /tmp/job-creator-keypair.json >/dev/null 2>&1; \
	fi; \
	ADDR=$$(solana address --keypair /tmp/job-creator-keypair.json); \
	solana airdrop 10 $$ADDR --url http://localhost:8899 2>/dev/null || true; \
	export $$(grep -v '^#' $$ENV_FILE | xargs); \
	RUST_LOG=info \
	BACKEND_URL=$$BACKEND \
	SOLANA_RPC_URL=http://localhost:8899 \
	USER_KEYPAIR=/tmp/job-creator-keypair.json \
	./target/release/zyb dev-job verify --types count-if

e2e-sum: ## [E2E] Run Sum verification test: Sum([10,20,30]) -> expect 60
	@echo "$(BLUE)Running Sum E2E Verification Test...$(NC)"
	@# Detect which env file to use (containers vs local)
	@if [ -f ".env.containers" ]; then \
		ENV_FILE=".env.containers"; \
		BACKEND="http://localhost:9000"; \
	elif [ -f "services/blink-server/.env" ]; then \
		ENV_FILE="services/blink-server/.env"; \
		BACKEND="http://localhost:3000"; \
	else \
		echo "$(RED)ERROR: No env file found. Run 'make c1' or 'make l1' first$(NC)"; \
		exit 1; \
	fi; \
	echo "Using env: $$ENV_FILE, backend: $$BACKEND"; \
	echo "Building zyb-cli..."; \
	cargo build --release -p zyb-cli --manifest-path cli/core/Cargo.toml 2>&1 | tail -3; \
	if [ ! -f "/tmp/job-creator-keypair.json" ]; then \
		solana-keygen new --no-bip39-passphrase --force --outfile /tmp/job-creator-keypair.json >/dev/null 2>&1; \
	fi; \
	ADDR=$$(solana address --keypair /tmp/job-creator-keypair.json); \
	solana airdrop 10 $$ADDR --url http://localhost:8899 2>/dev/null || true; \
	export $$(grep -v '^#' $$ENV_FILE | xargs); \
	RUST_LOG=info \
	BACKEND_URL=$$BACKEND \
	SOLANA_RPC_URL=http://localhost:8899 \
	USER_KEYPAIR=/tmp/job-creator-keypair.json \
	./target/release/zyb dev-job verify --types sum

e2e-all: ## [E2E] Run all verification tests (sequential)
	@echo "$(BLUE)Running ALL E2E Verification Tests (sequential)...$(NC)"
	@echo ""
	@$(MAKE) e2e-sum
	@echo ""
	@$(MAKE) e2e-poi
	@echo ""
	@echo "$(GREEN)All E2E tests completed!$(NC)"

# ============================================================================
# DEVNET E2E Tests
# ============================================================================

e2e-poi-devnet: ## [DEVNET E2E] Run PoI verification test on devnet
	@echo "$(BLUE)Running PoI E2E Verification Test on DEVNET...$(NC)"
	@if [ ! -f ".env.devnet" ]; then \
		echo "$(RED)ERROR: .env.devnet not found. Run './deployment/scripts/setup-devnet.sh' first$(NC)"; \
		exit 1; \
	fi
	@echo "Building zyb-cli..."
	@cargo build --release -p zyb-cli --manifest-path cli/core/Cargo.toml 2>&1 | tail -3
	@if [ ! -f "keypairs/job-creator.json" ]; then \
		echo "Creating dev-job keypair..."; \
		solana-keygen new --no-bip39-passphrase --force --outfile keypairs/job-creator.json >/dev/null 2>&1; \
	fi
	@export $$(grep -v '^#' .env.devnet | xargs) && \
	ADDR=$$(solana address --keypair keypairs/job-creator.json) && \
	echo "Dev job: $$ADDR" && \
	BALANCE=$$(solana balance keypairs/job-creator.json --url $$SOLANA_RPC_URL 2>/dev/null | cut -d' ' -f1) && \
	echo "Balance: $$BALANCE SOL" && \
	if [ "$$(echo "$$BALANCE < 0.1" | bc -l)" = "1" ]; then \
		echo "$(RED)ERROR: Job creator needs SOL. Fund it first.$(NC)"; \
		exit 1; \
	fi && \
	RUST_LOG=info \
	BACKEND_URL=https://demo.zyberlink.fun \
	SOLANA_RPC_URL=$$SOLANA_RPC_URL \
	PROGRAM_ID=$$PROGRAM_ID \
	USER_KEYPAIR=keypairs/job-creator.json \
	./target/release/zyb dev-job verify --types count-if

e2e-sum-devnet: ## [DEVNET E2E] Run Sum verification test on devnet
	@echo "$(BLUE)Running Sum E2E Verification Test on DEVNET...$(NC)"
	@if [ ! -f ".env.devnet" ]; then \
		echo "$(RED)ERROR: .env.devnet not found. Run './deployment/scripts/setup-devnet.sh' first$(NC)"; \
		exit 1; \
	fi
	@echo "Building zyb-cli..."
	@cargo build --release -p zyb-cli --manifest-path cli/core/Cargo.toml 2>&1 | tail -3
	@if [ ! -f "keypairs/job-creator.json" ]; then \
		echo "Creating dev-job keypair..."; \
		solana-keygen new --no-bip39-passphrase --force --outfile keypairs/job-creator.json >/dev/null 2>&1; \
	fi
	@export $$(grep -v '^#' .env.devnet | xargs) && \
	ADDR=$$(solana address --keypair keypairs/job-creator.json) && \
	echo "Dev job: $$ADDR" && \
	BALANCE=$$(solana balance keypairs/job-creator.json --url $$SOLANA_RPC_URL 2>/dev/null | cut -d' ' -f1) && \
	echo "Balance: $$BALANCE SOL" && \
	if [ "$$(echo "$$BALANCE < 0.1" | bc -l)" = "1" ]; then \
		echo "$(RED)ERROR: Job creator needs SOL. Fund it first.$(NC)"; \
		exit 1; \
	fi && \
	RUST_LOG=info \
	BACKEND_URL=https://demo.zyberlink.fun \
	SOLANA_RPC_URL=$$SOLANA_RPC_URL \
	PROGRAM_ID=$$PROGRAM_ID \
	USER_KEYPAIR=keypairs/job-creator.json \
	./target/release/zyb dev-job verify --types sum

e2e-all-devnet: ## [DEVNET E2E] Run all verification tests on devnet
	@echo "$(BLUE)Running ALL E2E Verification Tests on DEVNET...$(NC)"
	@echo ""
	@$(MAKE) e2e-sum-devnet
	@echo ""
	@$(MAKE) e2e-poi-devnet
	@echo ""
	@echo "$(GREEN)All DEVNET E2E tests completed!$(NC)"

# ============================================================================
# DEVNET Deployment (d0-d3)
# ============================================================================

d0: ## [DEVNET] Reset: stop containers + clean volumes
	@echo "$(BLUE)Resetting devnet environment...$(NC)"
	@-pkill -f "zyb dev-job" 2>/dev/null || true
	@-pkill -f "zyberlink-prover" 2>/dev/null || true
	@podman-compose -f docker-compose.devnet.yml down -v 2>/dev/null || true
	@echo "$(GREEN)Devnet environment reset. Ready for 'make d1'$(NC)"

d1: ## [DEVNET] Start containers (postgres, backend, webapp, nginx) - NO validator
	@echo "$(BLUE)Starting devnet containers (no validator)...$(NC)"
	@if [ ! -f ".env.devnet" ]; then \
		echo "$(RED)ERROR: .env.devnet not found. Run './deployment/scripts/setup-devnet.sh' first$(NC)"; \
		exit 1; \
	fi
	@export $$(grep -v '^#' .env.devnet | xargs) && \
	podman-compose -f docker-compose.devnet.yml up -d --build
	@echo ""
	@echo "$(GREEN)Devnet containers started!$(NC)"
	@echo "  Frontend: http://localhost:9001"
	@echo "  API: http://localhost:9001/api"
	@echo ""
	@echo "$(YELLOW)Next: make d2 (init marketplace)$(NC)"

d1-fresh: ## [DEVNET] Rebuild containers from scratch (no cache) and start
	@echo "$(BLUE)Rebuilding devnet containers (no cache)...$(NC)"
	@if [ ! -f ".env.devnet" ]; then \
		echo "$(RED)ERROR: .env.devnet not found. Run './deployment/scripts/setup-devnet.sh' first$(NC)"; \
		exit 1; \
	fi
	@export $$(grep -v '^#' .env.devnet | xargs) && \
	podman-compose -f docker-compose.devnet.yml build --no-cache && \
	podman-compose -f docker-compose.devnet.yml up -d
	@echo ""
	@echo "$(GREEN)Devnet containers rebuilt and started!$(NC)"
	@echo "  Frontend: http://localhost:9001"
	@echo "  API: http://localhost:9001/api"

d2: ## [DEVNET] Initialize marketplace + register provers on devnet
	@echo "$(BLUE)Initializing marketplace on devnet...$(NC)"
	@if [ ! -f ".env.devnet" ]; then \
		echo "$(RED)ERROR: .env.devnet not found. Run './deployment/scripts/setup-devnet.sh' first$(NC)"; \
		exit 1; \
	fi
	@export $$(grep -v '^#' .env.devnet | xargs) && \
	echo "Program ID: $$PROGRAM_ID" && \
	echo "RPC: $$SOLANA_RPC_URL"
	@echo ""
	@echo "Step 1/3: Initializing marketplace..."
	@export $$(grep -v '^#' .env.devnet | xargs) && \
	cargo run --manifest-path chain/solana/programs/sdks/bedrock-sdk/Cargo.toml --example initialize_bedrock 2>&1 | tail -5
	@echo "$(GREEN)  Marketplace initialized$(NC)"
	@echo ""
	@echo "Step 2/3: Building prover binary..."
	@cargo build --release --bin zyberlink-prover 2>&1 | tail -3
	@echo "$(GREEN)  Prover built$(NC)"
	@echo ""
	@echo "Step 3/3: Registering provers on devnet..."
	@export $$(grep -v '^#' .env.devnet | xargs) && \
	for i in 1 2 3; do \
		KEYPAIR="keypairs/prover-$$i.json"; \
		if [ -f "$$KEYPAIR" ]; then \
			ADDR=$$(solana address --keypair $$KEYPAIR); \
			./target/release/zyberlink-prover register \
				--program-id $$PROGRAM_ID \
				--rpc-url $$SOLANA_RPC_URL \
				--keypair $$KEYPAIR \
				--stake-amount 120000000 2>&1 | tail -1 || true; \
			echo "  Prover $$i registered: $$ADDR"; \
		fi; \
	done
	@echo ""
	@echo "$(GREEN)Marketplace ready on devnet with 3 provers!$(NC)"
	@echo "$(YELLOW)Next: make d3 (start provers)$(NC)"

d3: ## [DEVNET] Start provers (connect to devnet)
	@echo "$(BLUE)Starting provers for devnet...$(NC)"
	@if [ ! -f ".env.devnet" ]; then \
		echo "$(RED)ERROR: .env.devnet not found. Run './deployment/scripts/setup-devnet.sh' first$(NC)"; \
		exit 1; \
	fi
	@-pkill -f "zyberlink-prover" 2>/dev/null || true
	@echo ""
	@echo "Starting 3 prover nodes..."
	@export $$(grep -v '^#' .env.devnet | xargs) && \
	for i in 1 2 3; do \
		KEYPAIR="keypairs/prover-$$i.json"; \
		if [ -f "$$KEYPAIR" ]; then \
			RUST_LOG=info \
			WITNESS_BACKEND_URL=http://localhost:9001 \
			./target/release/zyberlink-prover \
				--program-id $$PROGRAM_ID \
				--rpc-url $$SOLANA_RPC_URL \
				--witness-backend-url http://localhost:9001 \
				--keypair $$KEYPAIR \
				run > /tmp/prover-devnet-$$i.log 2>&1 & \
			echo "  Prover $$i started (PID: $$!)"; \
		fi; \
	done
	@sleep 2
	@RUNNING=$$(pgrep -c -f "zyberlink-prover" || echo 0); \
	echo "$(GREEN)  $$RUNNING provers running$(NC)"
	@echo ""
	@echo "$(GREEN)=== DEVNET RUNNING ===$(NC)"
	@echo "  Frontend: http://localhost:9000"
	@echo "  API: http://localhost:9000/api"
	@echo ""
	@echo "  Logs:"
	@echo "    - Provers: tail -f /tmp/prover-devnet-*.log"
	@echo "    - Backend: podman-compose -f docker-compose.devnet.yml logs -f backend"

d-status: ## [DEVNET] Show status
	@echo "$(BLUE)Devnet Status:$(NC)"
	@echo ""
	@if [ -f ".env.devnet" ]; then \
		export $$(grep -v '^#' .env.devnet | xargs) && \
		echo "Program ID: $$PROGRAM_ID" && \
		echo "RPC URL: $$SOLANA_RPC_URL"; \
	else \
		echo "$(RED).env.devnet not found$(NC)"; \
	fi
	@echo ""
	@echo "Containers:"
	@podman-compose -f docker-compose.devnet.yml ps 2>/dev/null || echo "  No containers running"
	@echo ""
	@echo "Provers:"
	@pgrep -a -f "zyberlink-prover" 2>/dev/null || echo "  No provers running"

# ============================================================================
# x402 Anti-Spam Payment Tests
# ============================================================================

x402-quote: ## [x402] Test quote endpoint
	@echo "$(BLUE)Testing x402 quote endpoint...$(NC)"
	@# Detect backend URL
	@if [ -f ".env.containers" ]; then \
		BACKEND="http://localhost:9000"; \
	else \
		BACKEND="http://localhost:3000"; \
	fi; \
	echo "Backend: $$BACKEND"; \
	echo ""; \
	echo "Circuit Type 10 (PoI - 0.05 SOL):"; \
	curl -s "$$BACKEND/api/x402/quote" \
		-H "Content-Type: application/json" \
		-d '{"circuit_type": 10, "payer": "11111111111111111111111111111111"}' | jq .; \
	echo ""; \
	echo "Circuit Type 30 (Market - 0.075 SOL):"; \
	curl -s "$$BACKEND/api/x402/quote" \
		-H "Content-Type: application/json" \
		-d '{"circuit_type": 30, "payer": "11111111111111111111111111111111"}' | jq .

x402-estimate: ## [x402] Test price estimation (no quote created)
	@echo "$(BLUE)Testing x402 price estimation...$(NC)"
	@if [ -f ".env.containers" ]; then \
		BACKEND="http://localhost:9000"; \
	else \
		BACKEND="http://localhost:3000"; \
	fi; \
	echo "Backend: $$BACKEND"; \
	echo ""; \
	for ct in 0 10 20 30 40; do \
		PRICE=$$(curl -s "$$BACKEND/api/x402/estimate" \
			-H "Content-Type: application/json" \
			-d "{\"circuit_type\": $$ct}" | jq -r '.price_sol'); \
		echo "  Circuit $$ct: $$PRICE SOL"; \
	done

x402-flow: ## [x402] Test full payment flow (quote -> pay -> token)
	@echo "$(BLUE)Testing x402 full payment flow...$(NC)"
	@if [ -f ".env.containers" ]; then \
		BACKEND="http://localhost:9000"; \
	elif [ -f "services/blink-server/.env" ]; then \
		BACKEND="http://localhost:3000"; \
	else \
		echo "$(RED)ERROR: No env file found. Run 'make c1' or 'make l1' first$(NC)"; \
		exit 1; \
	fi; \
	echo "Backend: $$BACKEND"; \
	echo ""; \
	echo "Step 1: Get quote for PrivateVote (circuit 20)..."; \
	QUOTE=$$(curl -s "$$BACKEND/api/x402/quote" \
		-H "Content-Type: application/json" \
		-d '{"circuit_type": 20, "payer": "11111111111111111111111111111111"}'); \
	echo "$$QUOTE" | jq .; \
	QUOTE_ID=$$(echo "$$QUOTE" | jq -r '.quote_id'); \
	PRICE=$$(echo "$$QUOTE" | jq -r '.price_lamports'); \
	echo ""; \
	echo "Quote ID: $$QUOTE_ID"; \
	echo "Price: $$PRICE lamports"; \
	echo ""; \
	echo "Step 2: Build payment instruction..."; \
	curl -s "$$BACKEND/api/x402/build-payment" \
		-H "Content-Type: application/json" \
		-d "{\"quote_id\": \"$$QUOTE_ID\", \"payer\": \"11111111111111111111111111111111\", \"amount\": \"$$PRICE\", \"recipient\": \"ZYBRtreasury11111111111111111111111111111111\"}" | jq .; \
	echo ""; \
	echo "Step 3: Confirm payment (mock signature)..."; \
	TOKEN=$$(curl -s "$$BACKEND/api/x402/confirm" \
		-H "Content-Type: application/json" \
		-d "{\"quote_id\": \"$$QUOTE_ID\", \"signed_transaction\": \"bW9ja190cmFuc2FjdGlvbg==\", \"signature\": \"mock_signature_$(date +%s)\"}"); \
	echo "$$TOKEN" | jq .; \
	TOKEN_ID=$$(echo "$$TOKEN" | jq -r '.token_id'); \
	echo ""; \
	echo "Token ID: $$TOKEN_ID"; \
	echo ""; \
	echo "Step 4: Check token status..."; \
	curl -s "$$BACKEND/api/x402/token/$$TOKEN_ID/status" | jq .; \
	echo ""; \
	echo "$(GREEN)x402 flow test complete!$(NC)"

x402-witness: ## [x402] Test witness upload with token
	@echo "$(BLUE)Testing x402 witness upload...$(NC)"
	@if [ -f ".env.containers" ]; then \
		BACKEND="http://localhost:9000"; \
	else \
		BACKEND="http://localhost:3000"; \
	fi; \
	echo "Backend: $$BACKEND"; \
	echo ""; \
	echo "Step 1: Get quote and token..."; \
	QUOTE=$$(curl -s "$$BACKEND/api/x402/quote" \
		-H "Content-Type: application/json" \
		-d '{"circuit_type": 20, "payer": "test_payer"}'); \
	QUOTE_ID=$$(echo "$$QUOTE" | jq -r '.quote_id'); \
	TOKEN=$$(curl -s "$$BACKEND/api/x402/confirm" \
		-H "Content-Type: application/json" \
		-d "{\"quote_id\": \"$$QUOTE_ID\", \"signed_transaction\": \"bW9jaw==\", \"signature\": \"sig_$(date +%s)\"}"); \
	TOKEN_ID=$$(echo "$$TOKEN" | jq -r '.token_id'); \
	echo "Token: $$TOKEN_ID"; \
	echo ""; \
	echo "Step 2: Upload witness with token..."; \
	WITNESS_DATA="test_witness_data_$(date +%s)"; \
	RESULT=$$(curl -s "$$BACKEND/api/x402/witness" \
		-H "Content-Type: application/octet-stream" \
		-H "X-Payment-Token: $$TOKEN_ID" \
		--data-binary "$$WITNESS_DATA"); \
	echo "$$RESULT" | jq .; \
	COMMITMENT=$$(echo "$$RESULT" | jq -r '.commitment'); \
	echo ""; \
	echo "Witness commitment: $$COMMITMENT"; \
	echo ""; \
	echo "Step 3: Try upload without token (should fail 402)..."; \
	HTTP_CODE=$$(curl -s -o /dev/null -w "%{http_code}" "$$BACKEND/api/x402/witness" \
		-H "Content-Type: application/octet-stream" \
		--data-binary "no_token_data"); \
	if [ "$$HTTP_CODE" = "402" ]; then \
		echo "$(GREEN)Correctly returned 402 Payment Required$(NC)"; \
	else \
		echo "$(RED)Expected 402, got $$HTTP_CODE$(NC)"; \
	fi

x402-all: x402-estimate x402-quote x402-flow x402-witness ## [x402] Run all x402 tests
	@echo ""
	@echo "$(GREEN)=== All x402 tests completed! ===$(NC)"

# Aliases for quick testing
t-x402: x402-all ## Alias: run all x402 tests

# ============================================================================
# Prover Refactor Commands
# ============================================================================

.PHONY: prover-check prover-test prover-bench prover-refactor-health prover-setup-circuits prover-verify-deps

prover-check: ## Check prover-node compilation
	@echo "$(BLUE)Checking prover-node compilation...$(NC)"
	@cd src/prover-node && cargo check --all-targets
	@echo "$(GREEN)✓ Prover check passed$(NC)"

prover-test: ## Run prover-node tests (unit + integration)
	@echo "$(BLUE)Running prover-node tests...$(NC)"
	@cd src/prover-node && cargo test --lib
	@echo "$(GREEN)✓ Prover tests passed$(NC)"

prover-test-all: ## Run all prover tests including ignored ones
	@echo "$(BLUE)Running all prover tests (including E2E)...$(NC)"
	@cd src/prover-node && cargo test --lib
	@cd src/prover-node && cargo test --test '*' -- --ignored
	@echo "$(GREEN)✓ All prover tests passed$(NC)"

prover-bench: ## Run prover benchmarks
	@echo "$(BLUE)Running prover benchmarks...$(NC)"
	@cd src/prover-node && cargo bench --no-run
	@echo "$(GREEN)✓ Benchmarks compiled$(NC)"

prover-lint: ## Run clippy on prover-node
	@echo "$(BLUE)Running clippy on prover-node...$(NC)"
	@cd src/prover-node && cargo clippy --all-targets -- -D warnings
	@echo "$(GREEN)✓ No clippy warnings$(NC)"

prover-fmt-check: ## Check prover-node formatting
	@echo "$(BLUE)Checking prover-node formatting...$(NC)"
	@cd src/prover-node && cargo fmt -- --check
	@echo "$(GREEN)✓ Formatting OK$(NC)"

prover-fmt: ## Format prover-node code
	@echo "$(BLUE)Formatting prover-node...$(NC)"
	@cd src/prover-node && cargo fmt
	@echo "$(GREEN)✓ Code formatted$(NC)"

prover-setup-circuits: ## Setup circuits directory for prover
	@echo "$(BLUE)Setting up prover circuits...$(NC)"
	@bash deployment/scripts/setup_prover_circuits.sh
	@echo "$(GREEN)✓ Circuits setup complete$(NC)"

prover-verify-deps: ## Verify prover dependencies (snarkjs, etc.)
	@echo "$(BLUE)Verifying prover dependencies...$(NC)"
	@echo -n "  rustc: "
	@rustc --version || echo "$(RED)NOT FOUND$(NC)"
	@echo -n "  cargo: "
	@cargo --version || echo "$(RED)NOT FOUND$(NC)"
	@echo -n "  snarkjs: "
	@snarkjs --version 2>/dev/null || echo "$(YELLOW)NOT FOUND (required for ZK proofs)$(NC)"
	@echo -n "  bun: "
	@bun --version 2>/dev/null || (echo -n "node: " && node --version 2>/dev/null || echo "$(YELLOW)Neither bun nor node found$(NC)")
	@echo -n "  docker: "
	@docker --version || echo "$(YELLOW)NOT FOUND (required for E2E tests)$(NC)"
	@echo "$(GREEN)✓ Dependency check complete$(NC)"

prover-refactor-health: prover-verify-deps prover-check prover-lint prover-fmt-check prover-test ## Full health check for prover refactor
	@echo ""
	@echo "$(GREEN)====================================$(NC)"
	@echo "$(GREEN)✓ Prover Refactor Health Check PASS$(NC)"
	@echo "$(GREEN)====================================$(NC)"
	@echo ""
	@echo "Metrics:"
	@echo "  main.rs lines: $$(wc -l prover/core/src/main.rs | awk '{print $$1}')"
	@echo "  Total modules: $$(find prover/core/src -name '*.rs' | wc -l)"
	@echo ""

prover-stats: ## Show prover code statistics
	@echo "$(BLUE)Prover Node Statistics:$(NC)"
	@echo ""
	@echo "File sizes:"
	@wc -l prover/core/src/main.rs
	@wc -l prover/core/src/core/*.rs 2>/dev/null || echo "  core/* (not yet created)"
	@wc -l prover/core/src/engines/*.rs 2>/dev/null || echo "  engines/* (not yet created)"
	@wc -l prover/core/src/services/*.rs 2>/dev/null || echo "  services/* (not yet created)"
	@wc -l prover/core/src/cli/*.rs 2>/dev/null || echo "  cli/* (not yet created)"
	@echo ""
	@echo "Module structure:"
	@tree -L 3 prover/core/src/ 2>/dev/null || ls -R prover/core/src/

prover-clean: ## Clean prover build artifacts
	@echo "$(BLUE)Cleaning prover-node build artifacts...$(NC)"
	@cd src/prover-node && cargo clean
	@echo "$(GREEN)✓ Prover cleaned$(NC)"

# Refactor workflow shortcuts
refactor-sprint1: prover-verify-deps prover-setup-circuits ## Setup for Sprint 1 (A1, A2, A4, B4)
	@echo "$(GREEN)✓ Ready to start Sprint 1$(NC)"
	@echo ""
	@echo "Next steps:"
	@echo "  1. Execute TASK A1: Setup directory structure"
	@echo "  2. Execute TASK A2: Circuit Registry"
	@echo "  3. Execute TASK A4: CLI extraction"
	@echo "  4. See EXECUTION_ROADMAP.md for details"

refactor-checkpoint: prover-check prover-test prover-lint ## Quick checkpoint validation
	@echo "$(GREEN)✓ Checkpoint validation passed$(NC)"

# Aliases
p-check: prover-check
p-test: prover-test
p-lint: prover-lint
p-fmt: prover-fmt
p-health: prover-refactor-health
p-stats: prover-stats

# ============================================================================
# E2E Real Stack Tests (programs e2e)
# ============================================================================

.PHONY: e2e-real e2e-real-zk e2e-real-check e2e-real-all

e2e-real-check: ## Check E2E tests compilation
	@echo "$(BLUE)Checking E2E tests compilation...$(NC)"
	@cd programs/e2e && cargo check
	@echo "$(GREEN)✓ E2E tests compile$(NC)"

e2e-real: ## Run E2E tests against localhost stack (requires l1 running)
	@echo "$(BLUE)Running E2E tests against localhost...$(NC)"
	@echo ""
	@echo "Prerequisites:"
	@echo "  1. make l1 (stack running)"
	@echo "  2. Programs deployed"
	@echo ""
	@cd programs/e2e && cargo test real_stack -- --ignored --nocapture

e2e-real-zk: ## Run only ZK flow E2E test
	@echo "$(BLUE)Running ZK full flow E2E test...$(NC)"
	@cd programs/e2e && cargo test test_zk_full_flow_with_cpi -- --ignored --nocapture

e2e-real-prover: ## Run prover verification E2E test
	@echo "$(BLUE)Running prover verification E2E test...$(NC)"
	@cd programs/e2e && cargo test test_prover_verification -- --ignored --nocapture

e2e-real-connection: ## Test connection to localhost stack
	@echo "$(BLUE)Testing localhost stack connection...$(NC)"
	@cd programs/e2e && cargo test test_real_stack_connection -- --ignored --nocapture

e2e-real-all: e2e-real-check e2e-real ## Run all E2E real stack tests

# Aliases for e2e
e2e-r: e2e-real
e2e-rz: e2e-real-zk
