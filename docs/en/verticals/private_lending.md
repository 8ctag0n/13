# Private Lending

FHE-based lending primitives focusing on encrypted risk assessment.

## Overview

This vertical focuses on calculating critical lending metrics, specifically Loan-to-Value (LTV) ratios, using Fully Homomorphic Encryption (FHE). This allows protocols to assess risk without seeing the plaintext value of the user's collateral or debt.

## Components

### FHE Logic (`verticals/private_lending/`)

- `fhe_ltv.rs`: Rust implementation for calculating LTV on encrypted data.
- `types.rs`: Data structures for encrypted loan states.

## How it Works

1. **Encrypted Inputs**: User debt and collateral values are encrypted client-side.
2. **Homomorphic Computation**: The LTV ratio is calculated directly on the ciphertexts.
3. **Result**: The result is a boolean (Safe/Unsafe) or an encrypted ratio, depending on the implementation configuration, ensuring the protocol only learns what is necessary for liquidation or loan approval.

## Status
- **Core Logic**: Rust implementations for FHE LTV calculations are available.
