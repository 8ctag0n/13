# zyb-cli - ZyberLink Unified CLI

Unified command-line interface for ZyberLink FHE and ZK operations.

## Overview

The `zyb` CLI replaces the previous `fhe-cli` and provides a single entry point for:
- FHE encryption/decryption operations
- ZK proof creation and verification (Fase 2)
- Interactive wizards for job creation

## Installation

```bash
cargo build --release -p zyb-cli
```

The binary will be at `target/release/zyb`

## Usage

### FHE Commands

#### Encrypt data
```bash
# Encrypt a single value
zyb fhe encrypt -v 42

# Encrypt with custom output path
zyb fhe encrypt -v 42 -p ./my-fhe-job

# Encrypt multiple values for aggregation
zyb fhe encrypt --values 10,20,30,40
```

#### Decrypt results
```bash
# Decrypt interactively (will prompt for encrypted result)
zyb fhe decrypt -p ./my-fhe-job

# Decrypt with result provided directly
zyb fhe decrypt -p ./my-fhe-job -r "base64-encoded-result"
```

### ZK Commands (Coming in Fase 2)

```bash
# List available circuits
zyb zk circuits

# Create a ZK proof job
zyb zk create

# Check job status
zyb zk status <job-id>

# Verify a proof
zyb zk verify <proof-file>
```

### Interactive Wizard (Coming Soon)

```bash
zyb wizard
```

### Dev Job Runner

`dev-job` is a testing/demo runner that creates FHE jobs end-to-end.

```bash
# Run default mix (poi + analytics)
zyb dev-job run

# Plan without submitting
zyb dev-job plan --cases mix

# Run specific types
zyb dev-job run --types add,sum,average

# Verify end-to-end
zyb dev-job verify --types count-if

# Webapp flow simulation
zyb dev-job webapp-flow --verify
```

#### Config file (TOML)

By default, `dev-job` reads `dev-job.toml` from the current directory. CLI flags override the file. Example: `src/zyb-cli/dev-job.toml.example`.

```toml
[common]
profile = "local"
rpc_url = "http://localhost:8899"
program_id = "HnRTpCx7Xs3f1BKkVhkeZwcqRfPmSDpVQ6QxgN7Vm8Rt"
backend_url = "http://localhost:3000"
keypair = "/tmp/job-creator-keypair.json"
json = true

[run]
cases = ["mix"]
interval_secs = 10
shuffle = false

[profiles.local]
rpc_url = "http://localhost:8899"
backend_url = "http://localhost:3000"

[profiles.devnet]
rpc_url = "https://api.devnet.solana.com"
backend_url = "https://demo.zyberlink.fun"
```

#### JSON events

Use `--json` to emit structured JSON events in addition to human logs.

```bash
zyb dev-job run --json
```

#### Profiles

Use profiles to switch environments quickly (stored in `dev-job.toml`).

```bash
zyb profile list
zyb profile use devnet
zyb profile show
```

## Project Structure

```
src/zyb-cli/
├── Cargo.toml          # Package configuration
├── src/
│   ├── main.rs         # Entry point with clap subcommands
│   ├── lib.rs          # Shared utilities (FHE operations)
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── fhe/
│   │   │   ├── mod.rs
│   │   │   ├── encrypt.rs
│   │   │   └── decrypt.rs
│   │   └── zk/
│   │       └── mod.rs  # Placeholder for Fase 2
│   ├── client/         # HTTP client (future)
│   └── ui/             # Interactive UI (future)
```

## Migration from fhe-cli

The new CLI maintains compatibility with the old FHE commands:

| Old Command | New Command |
|-------------|-------------|
| `fhe-cli encrypt` | `zyb fhe encrypt` |
| `fhe-cli decrypt` | `zyb fhe decrypt` |

The file format and output structure remain the same.

## Development

### Build for development
```bash
cargo build -p zyb-cli
```

### Run tests
```bash
cargo test -p zyb-cli
```

### Check formatting
```bash
cargo fmt --check -p zyb-cli
```

## Roadmap

- [x] Fase 1: Refactor FHE CLI to modular structure
- [ ] Fase 2: Integrate 8 ZK circuits
- [ ] Fase 3: Add interactive wizard
- [ ] Fase 4: HTTP client for job management
