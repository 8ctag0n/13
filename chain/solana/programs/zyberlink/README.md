# Zyberlink Program

> **DEPRECATED**: This program is deprecated and will be removed in a future release.
> Use the modular programs instead: `bedrock`, `zk-generator`, `fhe-generator`, `threshold`.

## Overview

Legacy monolithic Solana program that combined ZK verification, FHE encryption, and job management.

## Migration

The functionality has been split into separate, focused programs:

| Legacy | New Location |
|--------|--------------|
| ZK verification | `bedrock/` |
| ZK proof generation | `zk-generator/` |
| FHE encryption | `fhe-generator/` |
| Threshold crypto | `threshold/` |

## Status

- No new features will be added
- Only critical security fixes will be applied
- Planned removal after full migration to modular programs
