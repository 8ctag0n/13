mod jobs; // Job marketplace for prover-node integration
mod collateral;
mod zk_verifier;
mod fhe_verifier; // Sprint 2: ECDSA verification for FHE results
mod liquidation;
mod tokens;
mod vault;
mod crypto_lib;
mod managers;
mod core;

#[cfg(test)]
mod tests;
