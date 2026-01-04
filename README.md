# zyb-platform

**Infrastructure orchestration and deployment configuration for the ZyberLink ecosystem**

This repository is part of the ZyberLink ecosystem (Fibonacci ID: 13) - the central platform repository containing deployment configurations, Docker compositions, infrastructure scripts, and integration testing tools for running ZyberLink in local, devnet, and production environments.

## Overview

zyb-platform provides the orchestration layer that ties together all ZyberLink components into a cohesive system. It includes:

- Multi-chain deployment configurations (Solana, Aptos, Starknet)
- Docker Compose setups for local development and production
- Automated deployment scripts and infrastructure management
- Integration testing framework
- Environment configuration templates
- Nginx reverse proxy configurations
- Makefile-driven development workflows

This repository serves as the entry point for developers setting up complete ZyberLink environments.

## Repository Structure

```
zyb-platform/
├── deployment/
│   ├── docker/              # Dockerfile definitions
│   │   ├── aptos/              # Aptos node setup
│   │   └── starknet/           # Starknet devnet setup
│   ├── infra/               # Infrastructure configurations
│   │   ├── docker/             # Production docker-compose files
│   │   │   ├── docker-compose.prod.yml
│   │   │   ├── docker-compose.full.yml
│   │   │   ├── docker-compose.localnet.yml
│   │   │   └── docker-compose.aptos.yml
│   │   └── nginx/              # Nginx reverse proxy configs
│   │       └── Dockerfile
│   └── scripts/             # Deployment automation scripts
│       ├── start-localnet.sh      # Start local development environment
│       ├── stop-localnet.sh       # Stop all services
│       ├── localnet-status.sh     # Check service health
│       └── start-demo.sh          # Start complete demo stack
├── docs/                    # Platform documentation
│   └── .gitbook.yaml           # GitBook integration
├── tests/                   # Integration tests
│   └── e2e/                    # End-to-end test suites
├── docker-compose.yml       # Default local development stack
├── docker-compose.dev.yml   # Development with hot-reload
├── docker-compose.devnet.yml # Solana devnet integration
├── docker-compose.aptos-mvp.yml # Aptos MVP deployment
├── docker-compose.pbtcfi.yml    # pBTC.fi vertical
├── docker-compose.starknet.yml  # Starknet integration
├── Makefile                 # Development workflow commands
├── zyb.example.toml         # CLI configuration template
├── nginx.conf               # Nginx routing configuration
├── .env.example             # Environment variables template
├── .env.devnet.template     # Devnet-specific env vars
├── demo.sh                  # Interactive demo script
├── Cargo.toml               # Rust workspace configuration
└── package.json             # Node.js tooling
```

## Requirements

- Docker 24+ and Docker Compose 2.20+
- Rust 1.75+ (for building workspace)
- Node.js 18+ and pnpm (for frontend and SDK development)
- Solana CLI 1.18+ (for Solana deployments)
- Make (for using Makefile commands)

Optional (for multi-chain):
- Aptos CLI (for Aptos deployments)
- Starknet Devnet (for Starknet testing)

## Quick Start

### 1. Clone and Setup

```bash
# Navigate to platform directory
cd zyb-platform

# Copy configuration template
cp zyb.example.toml zyb.toml
cp .env.example .env

# Edit configuration as needed
vim zyb.toml
```

### 2. Start Local Development Environment

Using Docker Compose:

```bash
# Start full local stack (validator + services + frontend)
docker-compose up -d

# Check service status
docker-compose ps

# View logs
docker-compose logs -f
```

Using Makefile (recommended):

```bash
# Start complete localnet with auto-deployment
make start

# Check status of all services
make status

# View logs for specific service
make logs

# Stop all services
make stop
```

### 3. Initialize Marketplace

After services are running:

```bash
# Initialize on-chain marketplace
make init-marketplace

# Start prover nodes
make start-provers

# Check prover registration
make check-provers
```

### 4. Access Services

- Solana Validator: http://localhost:8899
- Blink Server (Internal API): http://localhost:8080
- Public API: http://localhost:9000
- X402 Server: http://localhost:8402
- Frontend Webapp: http://localhost:3000

## Usage

### Development Workflows

#### Full Stack Development

```bash
# Start with hot-reload for backend and frontend
docker-compose -f docker-compose.dev.yml up

# Rebuild after dependency changes
make dev-rebuild

# Shell into backend container
make dev-shell-backend

# Shell into database
make dev-shell-db
```

#### Service-Specific Development

```bash
# Start only backend services
make start-backend

# Start only frontend
cd ../zyb-apps/webapp && npm run dev

# Start individual prover node
cargo run --package zyberlink-prover -- \
  --rpc-url http://localhost:8899 \
  --keypair /tmp/prover-1-keypair.json
```

### Multi-Chain Deployments

#### Solana Devnet

```bash
# Use devnet composition
docker-compose -f docker-compose.devnet.yml up -d

# Or with profile
make start --profile devnet
```

#### Aptos MVP

```bash
# Start Aptos local node + ZyberLink services
docker-compose -f docker-compose.aptos-mvp.yml up -d

# Deploy Aptos contracts
cd ../zyb-chain/aptos
aptos move publish --profile local
```

#### Starknet Devnet

```bash
# Start Starknet devnet
docker-compose -f docker-compose.starknet.yml up -d

# Deploy Starknet contracts
cd ../zyb-chain/starknet
scarb build && starkli deploy
```

### Configuration Profiles

Edit `zyb.toml` to configure different deployment profiles:

```toml
[common]
profile = "local"
chain = "solana"

[profiles.local]
rpc_url = "http://localhost:8899"
backend_url = "http://localhost:9000"

[profiles.devnet]
rpc_url = "https://api.devnet.solana.com"
backend_url = "https://api.zyberlink.dev"

[profiles.mainnet]
rpc_url = "https://api.mainnet-beta.solana.com"
backend_url = "https://api.zyberlink.io"
```

Use profiles with CLI:

```bash
zyb dev-job run --profile devnet
zyb market create --profile local
```

## Architecture

### Service Topology

```
┌─────────────────────────────────────────────────────┐
│              zyb-platform (This Repo)               │
├─────────────────────────────────────────────────────┤
│                                                     │
│  ┌──────────────┐    ┌──────────────┐              │
│  │   Frontend   │◄───┤  Nginx       │              │
│  │   (Next.js)  │    │  (Reverse    │              │
│  └──────────────┘    │   Proxy)     │              │
│         │            └──────┬───────┘              │
│         │                   │                       │
│         ▼                   ▼                       │
│  ┌─────────────────────────────────┐               │
│  │      Backend Services           │               │
│  ├─────────────────────────────────┤               │
│  │  - Public API      (:9000)      │               │
│  │  - Blink Server    (:8080)      │               │
│  │  - X402 Server     (:8402)      │               │
│  │  - Witness Storage (:8081)      │               │
│  └────────────┬────────────────────┘               │
│               │                                     │
│               ▼                                     │
│  ┌─────────────────────────────────┐               │
│  │   Blockchain Layer              │               │
│  ├─────────────────────────────────┤               │
│  │  - Solana Validator (:8899)     │               │
│  │  - Aptos Node (optional)        │               │
│  │  - Starknet Devnet (optional)   │               │
│  └────────────┬────────────────────┘               │
│               │                                     │
│               ▼                                     │
│  ┌─────────────────────────────────┐               │
│  │   Prover Nodes (3 instances)    │               │
│  │   - FHE Computation             │               │
│  │   - ZK Proof Generation         │               │
│  └─────────────────────────────────┘               │
│                                                     │
└─────────────────────────────────────────────────────┘
```

### Data Flow

1. **User Request** → Frontend (Next.js)
2. **API Call** → Nginx → Public API / Blink Server
3. **Job Submission** → Blockchain (Solana/Aptos/Starknet)
4. **Job Polling** → Prover Nodes monitor blockchain
5. **Computation** → Prover executes FHE/ZK computation
6. **Result Submission** → Prover submits proof to blockchain
7. **Result Retrieval** → Frontend polls backend for result

## Makefile Commands Reference

```bash
# Main Commands
make help              # Show all available commands
make start             # Start complete localnet (setup + deploy + services)
make stop              # Stop all services
make restart           # Restart all services
make status            # Check status of all services

# Demo Commands
make demo-up           # Start complete demo stack
make demo-down         # Stop demo
make demo-restart      # Restart demo

# Marketplace Commands
make init-marketplace  # Initialize on-chain marketplace
make check-provers     # Check prover registration status

# Prover Commands
make start-provers     # Start 3 prover nodes
make stop-provers      # Stop all prover nodes
make restart-provers   # Restart provers

# Individual Services
make start-validator   # Start Solana validator only
make stop-validator    # Stop validator
make start-backend     # Start blink-server
make start-public-api  # Start public API server
make start-x402        # Start X402 server

# Development
make dev-up            # Start with hot-reload
make dev-down          # Stop dev environment
make dev-rebuild       # Rebuild dev containers
make dev-shell-backend # Shell into backend container
make dev-logs          # View dev logs

# Container Management
make c1                # Start containers (alias for docker-compose up -d)
make c0                # Stop containers (alias for docker-compose down)
make c-status          # Container status
make c-restart         # Restart containers
make logs              # View container logs
make clean             # Clean build artifacts

# Build Commands
make build             # Build workspace
make build-all         # Build all components
make build-public-api  # Build public-api service
make build-x402        # Build x402-server
```

## Environment Variables

Key environment variables (see `.env.example`):

```bash
# Solana Configuration
SOLANA_RPC_URL=http://localhost:8899
SOLANA_NETWORK=localnet

# Program IDs (update after deployment)
BEDROCK_PROGRAM_ID=Di2Tu6aNpJpPyxbMAasoLQU2yLqYMFWvUoV7cq7sXfvx
FHE_GENERATOR_PROGRAM_ID=C8PpHFCKZ4F2Szbir2EMS4S4H3mwQqHNWUXK1N21nfAB
ZK_GENERATOR_PROGRAM_ID=Dzvy1pzCBgtMw5Fte2GybpeN2PsLPW8t7zDvLfKpxnSS
FUTARCHY_PROGRAM_ID=2V8E8DJ3M3J2Aombv8hxRq5t2RecjbdW3GpUmSVruuhX

# Backend Service Endpoints
BACKEND_URL=http://localhost:9000
WITNESS_STORAGE_URL=http://localhost:8081
X402_URL=http://localhost:8402

# Database
DATABASE_URL=postgresql://zyb:zyb@localhost:5432/zyberlink

# Frontend
NEXT_PUBLIC_API_URL=http://localhost:9000
```

## Dependencies Between Repos

This platform orchestrates all ZyberLink repositories:

- **zyb-kernel** (ID: 5) - Core types and crypto (workspace member)
- **zyb-circuits** (ID: 8) - ZK circuits (mounted for prover nodes)
- **zyb-chain** (ID: 21) - Smart contracts (deployed to blockchain)
- **zyb-compute** (ID: 34) - Prover nodes (runs as Docker services)
- **zyb-services** (ID: 55) - Backend services (Docker containers)
- **zyb-sdks** (ID: 89) - Client SDKs (used by frontend)
- **zyb-cli** (ID: 144) - CLI tool (configured via zyb.toml)
- **zyb-apps** (ID: 233) - Frontend applications (Docker/local dev)

External dependencies:

- **Docker** - Container orchestration
- **Nginx** - Reverse proxy and load balancing
- **PostgreSQL** - Backend database (optional)
- **Solana/Aptos/Starknet** - Blockchain infrastructure

## Development

### Adding a New Service

1. Create Dockerfile in `deployment/docker/<service>/`
2. Add service to appropriate `docker-compose.*.yml`
3. Update `Makefile` with service-specific commands
4. Add environment variables to `.env.example`
5. Update `nginx.conf` if service needs reverse proxy
6. Document service endpoints and usage

### Testing Integration

```bash
# Run integration tests
cd tests/e2e
cargo test --test integration_tests

# Run specific test suite
cargo test --test market_flow
```

### Production Deployment

```bash
# Use production docker-compose
docker-compose -f deployment/infra/docker/docker-compose.prod.yml up -d

# Or with custom configuration
export ZYB_CONFIG=/path/to/zyb.toml
make start --profile mainnet
```

## Troubleshooting

### Port Conflicts

```bash
# Check which services are using ports
lsof -i :8899  # Solana RPC
lsof -i :9000  # Public API
lsof -i :3000  # Frontend

# Stop conflicting services or change ports in docker-compose.yml
```

### Service Health Checks

```bash
# Check all service health
make status

# Check individual service logs
docker-compose logs -f <service-name>
make logs-public-api
make logs-x402
```

### Reset Local Environment

```bash
# Stop all services and clean data
make stop
docker-compose down -v  # Remove volumes
rm -rf ~/.zyberlink-localnet-ledger

# Restart from scratch
make start
```

## Security Considerations

- Never commit `.env` files with secrets
- Use `zyb.toml` for configuration, not hardcoded values
- Store keypairs securely (outside repository)
- Use environment variables for sensitive data in production
- Regularly update Docker base images
- Review `nginx.conf` for proper security headers

## License

Apache-2.0

---

Part of the [ZyberLink](https://github.com/8ctag0n) ecosystem - Privacy-preserving computation marketplace.
