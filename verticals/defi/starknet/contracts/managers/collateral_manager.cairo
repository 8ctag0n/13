//! collateral_manager.cairo - Private Collateral Management
//!
//! Manages BTC collateral with FULL PRIVACY using encrypted values
//! and zero-knowledge proofs.
//!
//! Key features:
//! - Encrypted collateral amounts
//! - Committed price values
//! - ZK proof verification
//! - Private LTV calculations
//! - Private health factor monitoring

use starknet::{ContractAddress, get_caller_address, get_block_timestamp};
use core::num::traits::Zero;
use starknet::storage::{
    Map, StorageMapReadAccess, StorageMapWriteAccess, StoragePointerReadAccess,
    StoragePointerWriteAccess
};
use super::super::crypto_lib;

/// CollateralInfo - Private collateral data structure
/// All sensitive values are encrypted or committed
#[derive(Drop, Serde, starknet::Store)]
pub struct CollateralInfo {
    loan_id: u256,
    // PRIVATE - Encrypted BTC amount
    btc_amount_encrypted: (felt252, felt252),
    // PRIVATE - Committed BTC price
    btc_price_commitment: felt252,
    // PRIVATE - Encrypted collateral value in USD
    collateral_value_encrypted: (felt252, felt252),
    // PUBLIC - Proof metadata
    proof_hash: felt252,
    verified: bool,
    registered_at: u64,
}

#[starknet::interface]
pub trait ICollateralManager<TContractState> {
    /// Register collateral for a loan
    /// All sensitive data is encrypted
    fn register_collateral(
        ref self: TContractState,
        loan_id: u256,
        btc_encrypted: (felt252, felt252),
        proof_hash: felt252
    ) -> bool;

    /// Verify collateral using ZK proof
    fn verify_collateral(ref self: TContractState, loan_id: u256) -> bool;

    /// Calculate Loan-to-Value ratio (returns encrypted)
    fn calculate_ltv(self: @TContractState, loan_id: u256) -> (felt252, felt252);

    /// Check health factor (returns commitment)
    fn check_health_factor(self: @TContractState, loan_id: u256) -> felt252;

    /// Check if position is liquidatable
    fn is_liquidatable(self: @TContractState, loan_id: u256) -> bool;

    /// Get collateral info (encrypted values preserved)
    fn get_collateral_info(self: @TContractState, loan_id: u256) -> CollateralInfo;

    /// Update BTC price (admin only)
    fn update_btc_price(ref self: TContractState, loan_id: u256, new_price_commitment: felt252);
}

#[starknet::contract]
mod CollateralManager {
    use super::{CollateralInfo, ICollateralManager};
    use starknet::{ContractAddress, get_caller_address, get_block_timestamp};
    use core::num::traits::Zero;
    use starknet::storage::{
        Map, StorageMapReadAccess, StorageMapWriteAccess, StoragePointerReadAccess,
        StoragePointerWriteAccess
    };
    use super::crypto_lib;

    #[storage]
    struct Storage {
        collateral_info: Map<u256, CollateralInfo>,
        // Contract references
        zk_verifier: ContractAddress,
        price_oracle: ContractAddress,
        loan_manager: ContractAddress,
        // Admin
        owner: ContractAddress,
    }

    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        CollateralRegistered: CollateralRegistered,
        CollateralVerified: CollateralVerified,
        PriceUpdated: PriceUpdated,
    }

    #[derive(Drop, starknet::Event)]
    struct CollateralRegistered {
        loan_id: u256,
        btc_encrypted: (felt252, felt252),
        proof_hash: felt252,
        timestamp: u64,
    }

    #[derive(Drop, starknet::Event)]
    struct CollateralVerified {
        loan_id: u256,
        timestamp: u64,
    }

    #[derive(Drop, starknet::Event)]
    struct PriceUpdated {
        loan_id: u256,
        new_price_commitment: felt252,
        timestamp: u64,
    }

    #[constructor]
    fn constructor(
        ref self: ContractState,
        zk_verifier: ContractAddress,
        price_oracle: ContractAddress,
        loan_manager: ContractAddress,
        owner: ContractAddress,
    ) {
        self.zk_verifier.write(zk_verifier);
        self.price_oracle.write(price_oracle);
        self.loan_manager.write(loan_manager);
        self.owner.write(owner);
    }

    #[abi(embed_v0)]
    impl CollateralManagerImpl of ICollateralManager<ContractState> {
        fn register_collateral(
            ref self: ContractState,
            loan_id: u256,
            btc_encrypted: (felt252, felt252),
            proof_hash: felt252
        ) -> bool {
            // 1. Validate loan_id not already registered
            let existing = self.collateral_info.read(loan_id);
            assert(existing.loan_id == 0, 'Collateral already exists');

            // 2. Mock: Get BTC price (hardcoded 60000 USD for MVP)
            // V2: Query real price oracle
            let btc_price_usd: u256 = u256 { low: 60000, high: 0 };
            let price_commitment = crypto_lib::mock_commit(btc_price_usd, 12345);

            // 3. Calculate collateral value (mock homomorphic multiplication)
            // collateral_value = btc_amount * btc_price
            let collateral_value_encrypted =
                crypto_lib::mock_fhe_multiply(btc_encrypted, btc_price_usd);

            // 4. Store CollateralInfo
            let info = CollateralInfo {
                loan_id,
                btc_amount_encrypted: btc_encrypted,
                btc_price_commitment: price_commitment,
                collateral_value_encrypted,
                proof_hash,
                verified: false,
                registered_at: get_block_timestamp(),
            };

            self.collateral_info.write(loan_id, info);

            // 5. Emit event
            self
                .emit(
                    CollateralRegistered {
                        loan_id, btc_encrypted, proof_hash, timestamp: get_block_timestamp(),
                    }
                );

            true
        }

        fn verify_collateral(ref self: ContractState, loan_id: u256) -> bool {
            // 1. Get collateral info
            let mut info = self.collateral_info.read(loan_id);
            assert(info.loan_id != 0, 'Collateral not found');
            assert(!info.verified, 'Already verified');

            // 2. Mock: Always verify successfully
            // V2: Call zk_verifier contract with real proof verification
            let empty_proof: Array<felt252> = array![];
            let public_inputs: Array<felt252> = array![info.proof_hash];

            let is_valid = crypto_lib::mock_verify_proof(empty_proof.span(), public_inputs.span());

            assert(is_valid, 'Invalid proof');

            // 3. Mark as verified
            info.verified = true;
            self.collateral_info.write(loan_id, info);

            // 4. Emit event
            self.emit(CollateralVerified { loan_id, timestamp: get_block_timestamp(), });

            true
        }

        fn calculate_ltv(self: @ContractState, loan_id: u256) -> (felt252, felt252) {
            // Returns encrypted LTV
            let info = self.collateral_info.read(loan_id);
            assert(info.loan_id != 0, 'Collateral not found');
            assert(info.verified, 'Collateral not verified');

            // Mock: LTV = 75% of collateral value
            // V2: Calculate based on actual debt amount
            let ltv_percentage = u256 { low: 75, high: 0 };
            let ltv_encrypted =
                crypto_lib::mock_fhe_multiply(info.collateral_value_encrypted, ltv_percentage);

            ltv_encrypted
        }

        fn check_health_factor(self: @ContractState, loan_id: u256) -> felt252 {
            // Returns commitment of health factor
            // HF = collateral_value / debt_value
            let info = self.collateral_info.read(loan_id);
            assert(info.loan_id != 0, 'Collateral not found');
            assert(info.verified, 'Collateral not verified');

            // Mock: HF = 1.5 (healthy position)
            // V2: Calculate real HF from collateral and debt
            let mock_hf = u256 { low: 150, high: 0 }; // 1.5 in basis points (150/100)
            crypto_lib::mock_commit(mock_hf, 54321)
        }

        fn is_liquidatable(self: @ContractState, loan_id: u256) -> bool {
            // Check if position can be liquidated
            let info = self.collateral_info.read(loan_id);
            assert(info.loan_id != 0, 'Collateral not found');
            assert(info.verified, 'Collateral not verified');

            // Mock: Never liquidatable for MVP
            // V2: Check if HF < 1.0 (100 basis points)
            // let hf = self.check_health_factor(loan_id);
            // In V2: verify hf_commitment < threshold using ZK proof

            false
        }

        fn get_collateral_info(self: @ContractState, loan_id: u256) -> CollateralInfo {
            let info = self.collateral_info.read(loan_id);
            assert(info.loan_id != 0, 'Collateral not found');
            info
        }

        fn update_btc_price(
            ref self: ContractState, loan_id: u256, new_price_commitment: felt252
        ) {
            // Only owner can update price
            let caller = get_caller_address();
            let owner = self.owner.read();
            assert(caller == owner, 'Only owner can update price');

            let mut info = self.collateral_info.read(loan_id);
            assert(info.loan_id != 0, 'Collateral not found');

            // Update price commitment
            info.btc_price_commitment = new_price_commitment;
            self.collateral_info.write(loan_id, info);

            self.emit(PriceUpdated { loan_id, new_price_commitment, timestamp: get_block_timestamp(), });
        }
    }
}
