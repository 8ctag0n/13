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
