# Repository Map

ZyberLink is split across multiple repositories. This GitBook is built from `zyb-platform/docs`, so some guides point to other repos depending on the component you need.

Use this page to pick the correct repo when a guide says "clone the repository".

## Core Repositories

| Repo | Purpose | Assets | URL |
| --- | --- | --- | --- |
| **zyb-platform** | Orchestration, deployment, docker-compose | Makefile, scripts | https://github.com/8ctag0n/13 |
| **zyb-chain** | On-chain programs (Solana, Starknet, Aptos) + SDKs | 269 | https://github.com/8ctag0n/21 |
| **zyb-kernel** | Core libs (crypto, fhe, types, jobs) | 19 | https://github.com/8ctag0n/5 |
| **zyb-circuits** | ZK circuits (Halo2, Circom) | 30 | https://github.com/8ctag0n/8 |
| **zyb-cli** | Main CLI (`zyb`) | 24 | https://github.com/8ctag0n/144 |
| **zyb-compute** | Prover nodes and compute runtime | 59 | https://github.com/8ctag0n/34 |
| **zyb-sdks** | Multi-language SDKs (Rust, TS, Go) | 52 | https://github.com/8ctag0n/89 |
| **zyb-services** | Backend services (API, sync, gateway) | 61 | https://github.com/8ctag0n/55 |
| **zyb-apps** | Frontend apps (wallet, webapp, design-system) | 40 | https://github.com/8ctag0n/233 |

Other workspace repositories in this checkout:
- `verticals`: Vertical-specific examples and integrations.

## Clone Examples

Organization: `8ctagon`

```bash
git clone https://github.com/8ctagon/zyb-platform.git
git clone https://github.com/8ctagon/zyb-chain.git
git clone https://github.com/8ctagon/zyb-compute.git
git clone https://github.com/8ctagon/zyb-cli.git
git clone https://github.com/8ctagon/zyb-apps.git
git clone https://github.com/8ctagon/zyb-services.git
git clone https://github.com/8ctagon/zyb-sdks.git
git clone https://github.com/8ctagon/zyb-kernel.git
git clone https://github.com/8ctagon/zyb-circuits.git
```

## Issue Reporting

Open issues in the repo that owns the component you are working on. If you are unsure, start with `zyb-platform` and link to the relevant guide.

## Changelogs

Changelogs live in each repo. Start here:

- https://github.com/8ctag0n/13/blob/main/CHANGELOG.md
- https://github.com/8ctag0n/21/blob/main/CHANGELOG.md
- https://github.com/8ctag0n/5/blob/main/CHANGELOG.md
- https://github.com/8ctag0n/8/blob/main/CHANGELOG.md
- https://github.com/8ctag0n/144/blob/main/CHANGELOG.md
- https://github.com/8ctag0n/34/blob/main/CHANGELOG.md
- https://github.com/8ctag0n/89/blob/main/CHANGELOG.md
- https://github.com/8ctag0n/55/blob/main/CHANGELOG.md
- https://github.com/8ctag0n/233/blob/main/CHANGELOG.md
