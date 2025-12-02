.PHONY: help demo-up demo-down demo-restart start stop status logs clean build build-all deploy init-marketplace check-provers start-provers stop-provers tunnel-help dev-up dev-down dev-logs dev-rebuild dev-status dev-shell-backend dev-shell-db serve c0 c1 c2 c3 c4 c-status c-scale c-restart

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
	@if [ ! -f "src/blink-server/.env" ]; then \
		echo "$(RED)ERROR: src/blink-server/.env not found. Run 'make start' first$(NC)"; \
		exit 1; \
	fi
	@export $$(grep -v '^#' src/blink-server/.env | xargs) && \
	cargo run --manifest-path src/sdk/Cargo.toml --example initialize_program

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

start-backend: ## Start backend server only
	@echo "$(BLUE) Starting backend server...$(NC)"
	@if [ -f "src/blink-server/.env" ]; then \
		export $$(grep -v '^#' src/blink-server/.env | xargs); \
	fi; \
	RUST_LOG=info ./target/release/blink-server > /tmp/blink-server.log 2>&1 &
	@sleep 2
	@echo "$(GREEN) Backend started$(NC)"

stop-backend: ## Stop backend server
	@pkill -f blink-server || true
	@echo "$(GREEN) Backend stopped$(NC)"

start-frontend: ## Start frontend dev server
	@echo "$(BLUE) Starting frontend...$(NC)"
	@mkdir -p ~/zyberlink-logs
	@cd src/webapp && VITE_API_URL=http://localhost:8080 npm run dev > ~/zyberlink-logs/frontend.log 2>&1 &
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
	@cd src/programs && cargo build-sbf
	@echo "$(GREEN) Program built$(NC)"

build-backend: ## Build backend server
	@echo "$(BLUE) Building backend server...$(NC)"
	@cargo build --release --bin blink-server
	@echo "$(GREEN) Backend built$(NC)"

build-prover: ## Build prover node
	@echo "$(BLUE) Building prover node...$(NC)"
	@cargo build --release --bin zyberlink-prover
	@echo "$(GREEN) Prover built$(NC)"

build-frontend: ## Build frontend
	@echo "$(BLUE) Building frontend...$(NC)"
	@cd src/webapp && npm install && npm run build
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
	@cd src/programs && cargo test-sbf

test-e2e: ## Run end-to-end tests
	@echo "$(BLUE) Running E2E tests...$(NC)"
	@cargo test --test e2e_workflow_test

# ============================================================================
# Job Creation Commands
# ============================================================================

create-job: ## Create a single test job
	@echo "$(BLUE) Creating test job...$(NC)"
	@if [ ! -f "src/blink-server/.env" ]; then \
		echo "$(RED)ERROR: src/blink-server/.env not found. Run 'make start' first$(NC)"; \
		exit 1; \
	fi
	@export $$(grep -v '^#' src/blink-server/.env | xargs) && \
	cargo run --manifest-path src/sdk/Cargo.toml --example create_test_job

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
	@if [ ! -f "src/blink-server/.env" ]; then \
		echo "$(RED)ERROR: src/blink-server/.env not found. Run 'make start' first$(NC)"; \
		exit 1; \
	fi
	@export $$(grep -v '^#' src/blink-server/.env | xargs) && \
	cargo run --manifest-path src/sdk/Cargo.toml --example inspect_accounts

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
	@cd src/programs && cargo clean
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
# Docker Full Stack Commands
# ============================================================================

docker-up: ## Start full stack with Docker Compose (postgres + backend + frontend)
	@echo "$(BLUE)Starting full Docker stack...$(NC)"
	@docker-compose -f infra/docker/docker-compose.full.yml up -d
	@echo "$(GREEN)Stack started!$(NC)"
	@echo "  - Frontend: http://localhost:5173"
	@echo "  - Backend:  http://localhost:8080"
	@echo "  - Postgres: localhost:5432"

docker-down: ## Stop Docker stack
	@echo "$(BLUE)Stopping Docker stack...$(NC)"
	@docker-compose -f infra/docker/docker-compose.full.yml down

docker-logs: ## Show Docker stack logs
	@docker-compose -f infra/docker/docker-compose.full.yml logs -f

docker-build: ## Rebuild Docker images
	@echo "$(BLUE)Building Docker images...$(NC)"
	@docker-compose -f infra/docker/docker-compose.full.yml build

docker-restart: docker-down docker-up ## Restart Docker stack

docker-status: ## Show Docker container status
	@docker-compose -f infra/docker/docker-compose.full.yml ps

# ============================================================================
# Production Docker Commands
# ============================================================================

prod-up: ## Start production stack (nginx + backend + postgres)
	@echo "$(BLUE)Starting production stack...$(NC)"
	@docker-compose -f infra/docker/docker-compose.prod.yml up -d --build
	@echo "$(GREEN)Production stack running!$(NC)"
	@echo "  - Web: http://localhost (or port in .env)"
	@echo "  - API: proxied through nginx at /api/"

prod-down: ## Stop production stack
	@docker-compose -f infra/docker/docker-compose.prod.yml down

prod-logs: ## Show production logs
	@docker-compose -f infra/docker/docker-compose.prod.yml logs -f

prod-restart: prod-down prod-up ## Restart production stack

prod-status: ## Show production container status
	@docker-compose -f infra/docker/docker-compose.prod.yml ps

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
	@scripts/start-localnet.sh

localnet-init: ## [STEP 2] Initialize marketplace on-chain (run once after setup)
	@echo "$(BLUE)Initializing marketplace...$(NC)"
	@scripts/init-marketplace.sh

localnet-jobs: ## [STEP 3] Start job creator (auto-creates jobs every 10s)
	@echo "$(BLUE)Starting job creator (auto-creating jobs)...$(NC)"
	@# Kill any existing job-creator
	@-pkill -f "job-creator" 2>/dev/null || true
	@# Create keypair if needed
	@if [ ! -f "/tmp/job-creator-keypair.json" ]; then \
		echo "Creating job creator keypair..."; \
		solana-keygen new --no-bip39-passphrase --force --outfile /tmp/job-creator-keypair.json >/dev/null 2>&1; \
	fi
	@export $$(grep -v '^#' src/blink-server/.env | xargs) && \
	ADDR=$$(solana address --keypair /tmp/job-creator-keypair.json); \
	echo "  Job creator: $$ADDR"; \
	solana airdrop 100 $$ADDR --url $$SOLANA_RPC_URL 2>/dev/null || echo "  (airdrop may have failed)"; \
	BALANCE=$$(solana balance --keypair /tmp/job-creator-keypair.json --url $$SOLANA_RPC_URL 2>/dev/null); \
	echo "  Balance: $$BALANCE"
	@# Build job-creator
	@echo "  Building job-creator..."
	@cargo build --release --manifest-path src/job-creator/Cargo.toml 2>&1 | tail -3
	@# Start job-creator in background (pointing to local backend on 8080)
	@export $$(grep -v '^#' src/blink-server/.env | xargs) && \
	RUST_LOG=info \
	BACKEND_URL=http://localhost:8080 \
	SOLANA_RPC_URL=$$SOLANA_RPC_URL \
	PROGRAM_ID=$$PROGRAM_ID \
	USER_KEYPAIR=/tmp/job-creator-keypair.json \
	./target/release/job-creator > /tmp/job-creator.log 2>&1 &
	@sleep 2
	@if pgrep -f "job-creator" > /dev/null; then \
		echo "$(GREEN)Job creator running! (creating jobs every 10s)$(NC)"; \
		echo "  Logs: tail -f /tmp/job-creator.log"; \
	else \
		echo "$(RED)Job creator failed to start. Check /tmp/job-creator.log$(NC)"; \
	fi

# ============================================================================
# Helpful Aliases
# ============================================================================

localnet-stop: stop ## Stop all localnet services

l1: localnet-start ## Alias: make l1 = start localnet (validator + backend + provers)
l2: localnet-init  ## Alias: make l2 = init marketplace
l3: localnet-jobs  ## Alias: make l3 = start job creator
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
	@-pkill -f "job-creator" 2>/dev/null || true
	@-pkill -f "zyberlink-prover" 2>/dev/null || true
	@podman-compose down -v 2>/dev/null || true
	@podman system prune -f 2>/dev/null || true
	@rm -f .env.containers 2>/dev/null || true
	@echo "$(GREEN)Container environment reset. Ready for 'make c1'$(NC)"

c1: ## [CONTAINER] Start: validator (host) + containers (postgres, backend, webapp, nginx)
	@scripts/start-containers.sh

c2: ## [CONTAINER] Initialize marketplace + register provers
	@echo "$(BLUE)Initializing marketplace and registering provers...$(NC)"
	@if [ ! -f ".env.containers" ]; then \
		echo "$(RED)ERROR: .env.containers not found. Run 'make c1' first$(NC)"; \
		exit 1; \
	fi
	@echo ""
	@echo "Step 1/3: Initializing marketplace..."
	@export $$(grep -v '^#' .env.containers | xargs) && \
	SOLANA_RPC_URL=http://localhost:8899 cargo run --manifest-path src/sdk/Cargo.toml --example initialize_program
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
	@echo "$(YELLOW)Next: make c3 (start provers + job creator)$(NC)"

c3: ## [CONTAINER] Start provers + job creator (local binaries)
	@echo "$(BLUE)Starting provers and job creator...$(NC)"
	@if [ ! -f ".env.containers" ]; then \
		echo "$(RED)ERROR: .env.containers not found. Run 'make c1' first$(NC)"; \
		exit 1; \
	fi
	@-pkill -f "job-creator" 2>/dev/null || true
	@-pkill -f "zyberlink-prover" 2>/dev/null || true
	@echo ""
	@echo "Step 1/2: Starting 3 prover nodes..."
	@export $$(grep -v '^#' .env.containers | xargs) && \
	for i in 1 2 3; do \
		KEYPAIR="/tmp/prover-$$i-keypair.json"; \
		if [ -f "$$KEYPAIR" ]; then \
			RUST_LOG=info WITNESS_BACKEND_URL=http://localhost:9000 \
			./target/release/zyberlink-prover \
				--program-id $$PROGRAM_ID \
				--rpc-url http://localhost:8899 \
				--witness-backend-url http://localhost:9000 \
				--keypair $$KEYPAIR \
				run > /tmp/prover-$$i.log 2>&1 & \
			echo "  Prover $$i started (PID: $$!)"; \
		fi; \
	done
	@sleep 2
	@RUNNING=$$(pgrep -c -f "zyberlink-prover" || echo 0); \
	echo "$(GREEN)  $$RUNNING provers running$(NC)"
	@echo ""
	@echo "Step 2/2: Starting job creator..."
	@if [ ! -f "/tmp/job-creator-keypair.json" ]; then \
		solana-keygen new --no-bip39-passphrase --force --outfile /tmp/job-creator-keypair.json >/dev/null 2>&1; \
	fi
	@ADDR=$$(solana address --keypair /tmp/job-creator-keypair.json); \
	echo "  Job creator: $$ADDR"; \
	solana airdrop 100 $$ADDR --url http://localhost:8899 2>/dev/null || true; \
	BALANCE=$$(solana balance --keypair /tmp/job-creator-keypair.json --url http://localhost:8899 2>/dev/null); \
	echo "  Balance: $$BALANCE"
	@cargo build --release --manifest-path src/job-creator/Cargo.toml 2>&1 | tail -2
	@export $$(grep -v '^#' .env.containers | xargs) && \
	RUST_LOG=info \
	BACKEND_URL=http://localhost:9000 \
	SOLANA_RPC_URL=http://localhost:8899 \
	USER_KEYPAIR=/tmp/job-creator-keypair.json \
	./target/release/job-creator > /tmp/job-creator.log 2>&1 &
	@sleep 2
	@if pgrep -f "job-creator" > /dev/null; then \
		echo "$(GREEN)  Job creator running$(NC)"; \
	else \
		echo "$(RED)  Job creator failed. Check /tmp/job-creator.log$(NC)"; \
	fi
	@echo ""
	@echo "$(GREEN)=== DEMO RUNNING ===$(NC)"
	@echo "  Frontend:     http://localhost:9000"
	@echo "  API:          http://localhost:9000/api/jobs"
	@echo ""
	@echo "  Logs:"
	@echo "    - Provers:    tail -f /tmp/prover-*.log"
	@echo "    - Job creator: tail -f /tmp/job-creator.log"
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
	elif [ -f "src/blink-server/.env" ]; then \
		ENV_FILE="src/blink-server/.env"; \
		BACKEND="http://localhost:8080"; \
	else \
		echo "$(RED)ERROR: No env file found. Run 'make c1' or 'make l1' first$(NC)"; \
		exit 1; \
	fi; \
	echo "Using env: $$ENV_FILE, backend: $$BACKEND"; \
	echo "Building job-creator..."; \
	cargo build --release --manifest-path src/job-creator/Cargo.toml 2>&1 | tail -3; \
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
	./src/job-creator/target/release/job-creator verify-poi

e2e-sum: ## [E2E] Run Sum verification test: Sum([10,20,30]) -> expect 60
	@echo "$(BLUE)Running Sum E2E Verification Test...$(NC)"
	@# Detect which env file to use (containers vs local)
	@if [ -f ".env.containers" ]; then \
		ENV_FILE=".env.containers"; \
		BACKEND="http://localhost:9000"; \
	elif [ -f "src/blink-server/.env" ]; then \
		ENV_FILE="src/blink-server/.env"; \
		BACKEND="http://localhost:8080"; \
	else \
		echo "$(RED)ERROR: No env file found. Run 'make c1' or 'make l1' first$(NC)"; \
		exit 1; \
	fi; \
	echo "Using env: $$ENV_FILE, backend: $$BACKEND"; \
	echo "Building job-creator..."; \
	cargo build --release --manifest-path src/job-creator/Cargo.toml 2>&1 | tail -3; \
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
	./src/job-creator/target/release/job-creator verify-sum

e2e-all: ## [E2E] Run all verification tests (sequential)
	@echo "$(BLUE)Running ALL E2E Verification Tests (sequential)...$(NC)"
	@echo ""
	@$(MAKE) e2e-sum
	@echo ""
	@$(MAKE) e2e-poi
	@echo ""
	@echo "$(GREEN)All E2E tests completed!$(NC)"
