//! pbtcfi_core.cairo - Orchestrator Contract
//!
//! Main entry point for PBTCFi protocol. Coordinates loan_manager,
//! collateral_manager, vault, and token contracts to provide a unified
//! user-facing API with privacy guarantees.
//!
//! Key responsibilities:
//! - Orchestrate all protocol contracts
//! - Provide simplified user API
//! - Manage permissions and pausing
//! - Emit aggregated events for gateway monitoring

use starknet::{ContractAddress, get_caller_address, get_block_timestamp};
use core::num::traits::Zero;
use starknet::storage::{StoragePointerReadAccess, StoragePointerWriteAccess};
use core::array::Span;

// Import manager interfaces and types
use crate::managers::loan_manager::{ILoanManagerDispatcher, ILoanManagerDispatcherTrait, Loan};
use crate::managers::collateral_manager::{
    ICollateralManagerDispatcher, ICollateralManagerDispatcherTrait, CollateralInfo
};

// ===== EXTERNAL INTERFACE =====

#[starknet::interface]
pub trait IPBTCFiCore<TContractState> {
    // === User-facing Loan Operations ===

    /// Step 1: Create loan with encrypted BTC amount
    /// Returns loan_id for tracking
    fn create_loan(
        ref self: TContractState,
        btc_commitment: felt252,
        btc_encrypted: (felt252, felt252),
        collateral_hash: felt252
    ) -> u256;

    /// Step 2: Register collateral (calls collateral_manager)
    fn register_collateral(
        ref self: TContractState,
        loan_id: u256,
        btc_encrypted: (felt252, felt252),
        proof_hash: felt252
    ) -> bool;

    /// Step 3: Activate loan (deposits BTC, mints pLST)
    fn activate_loan(
        ref self: TContractState,
        loan_id: u256,
        plst_encrypted: (felt252, felt252),
        ltv_proof: Span<felt252>
    ) -> bool;

    /// Step 4: Repay loan
    fn repay_loan(
        ref self: TContractState,
        loan_id: u256,
        repay_encrypted: (felt252, felt252),
        repay_proof: Span<felt252>
    ) -> bool;

    /// Liquidation
    fn liquidate_loan(
        ref self: TContractState,
        loan_id: u256,
        liquidation_proof: Span<felt252>
    ) -> bool;

    // === Query Functions ===

    fn get_loan(self: @TContractState, loan_id: u256) -> Loan;
    fn get_user_loan_count(self: @TContractState, user: ContractAddress) -> u256;
    fn get_collateral_info(self: @TContractState, loan_id: u256) -> CollateralInfo;

    // === Admin Functions ===

    fn pause(ref self: TContractState);
    fn unpause(ref self: TContractState);
    fn is_paused(self: @TContractState) -> bool;
    fn get_owner(self: @TContractState) -> ContractAddress;
}

// ===== CONTRACT IMPLEMENTATION =====

#[starknet::contract]
pub mod PBTCFiCore {
    use super::{
        ILoanManagerDispatcher, ILoanManagerDispatcherTrait, Loan, ICollateralManagerDispatcher,
        ICollateralManagerDispatcherTrait, CollateralInfo
    };
    use starknet::{ContractAddress, get_caller_address, get_block_timestamp};
    use core::num::traits::Zero;
    use starknet::storage::{StoragePointerReadAccess, StoragePointerWriteAccess};
    use core::array::Span;

    // ===== STORAGE =====

    #[storage]
    struct Storage {
        // Contract references
        loan_manager: ContractAddress,
        collateral_manager: ContractAddress,
        btc_vault: ContractAddress,
        plst_token: ContractAddress,
        // Admin
        owner: ContractAddress,
        paused: bool,
    }

    // ===== EVENTS =====

    #[event]
    #[derive(Drop, starknet::Event)]
    pub enum Event {
        LoanCreated: LoanCreated,
        CollateralRegistered: CollateralRegistered,
        LoanActivated: LoanActivated,
        LoanRepaid: LoanRepaid,
        LoanLiquidated: LoanLiquidated,
        Paused: Paused,
        Unpaused: Unpaused,
    }

    #[derive(Drop, starknet::Event)]
    pub struct LoanCreated {
        pub loan_id: u256,
        pub borrower: ContractAddress,
        pub btc_commitment: felt252,
        pub btc_encrypted: (felt252, felt252),
        pub timestamp: u64,
    }

    #[derive(Drop, starknet::Event)]
    pub struct CollateralRegistered {
        pub loan_id: u256,
        pub btc_encrypted: (felt252, felt252),
        pub proof_hash: felt252,
        pub timestamp: u64,
    }

    #[derive(Drop, starknet::Event)]
    pub struct LoanActivated {
        pub loan_id: u256,
        pub plst_encrypted: (felt252, felt252),
        pub timestamp: u64,
    }

    #[derive(Drop, starknet::Event)]
    pub struct LoanRepaid {
        pub loan_id: u256,
        pub timestamp: u64,
    }

    #[derive(Drop, starknet::Event)]
    pub struct LoanLiquidated {
        pub loan_id: u256,
        pub liquidator: ContractAddress,
        pub timestamp: u64,
    }

    #[derive(Drop, starknet::Event)]
    pub struct Paused {
        pub timestamp: u64,
    }

    #[derive(Drop, starknet::Event)]
    pub struct Unpaused {
        pub timestamp: u64,
    }

    // ===== CONSTRUCTOR =====

    #[constructor]
    fn constructor(
        ref self: ContractState,
        loan_manager: ContractAddress,
        collateral_manager: ContractAddress,
        btc_vault: ContractAddress,
        plst_token: ContractAddress,
        owner: ContractAddress,
    ) {
        // Validate all addresses
        assert(!loan_manager.is_zero(), 'Invalid loan_manager');
        assert(!collateral_manager.is_zero(), 'Invalid collateral_manager');
        assert(!btc_vault.is_zero(), 'Invalid btc_vault');
        assert(!plst_token.is_zero(), 'Invalid plst_token');
        assert(!owner.is_zero(), 'Invalid owner');

        // Initialize storage
        self.loan_manager.write(loan_manager);
        self.collateral_manager.write(collateral_manager);
        self.btc_vault.write(btc_vault);
        self.plst_token.write(plst_token);
        self.owner.write(owner);
        self.paused.write(false);
    }

    // ===== INTERNAL HELPERS =====

    #[generate_trait]
    impl InternalImpl of InternalTrait {
        /// Check if contract is paused
        fn assert_not_paused(self: @ContractState) {
            assert(!self.paused.read(), 'Contract is paused');
        }

        /// Check if caller is owner
        fn assert_owner(self: @ContractState) {
            let caller = get_caller_address();
            let owner = self.owner.read();
            assert(caller == owner, 'Only owner can call');
        }
    }

    // ===== EXTERNAL IMPLEMENTATION =====

    #[abi(embed_v0)]
    impl PBTCFiCoreImpl of super::IPBTCFiCore<ContractState> {
        // === USER-FACING OPERATIONS ===

        fn create_loan(
            ref self: ContractState,
            btc_commitment: felt252,
            btc_encrypted: (felt252, felt252),
            collateral_hash: felt252
        ) -> u256 {
            // 1. Check not paused
            self.assert_not_paused();

            // 2. Delegate to loan_manager
            let loan_manager = ILoanManagerDispatcher {
                contract_address: self.loan_manager.read()
            };

            let loan_id = loan_manager
                .create_loan(btc_commitment, btc_encrypted, collateral_hash);

            // 3. Emit aggregated event
            self
                .emit(
                    LoanCreated {
                        loan_id,
                        borrower: get_caller_address(),
                        btc_commitment,
                        btc_encrypted,
                        timestamp: get_block_timestamp(),
                    }
                );

            loan_id
        }

        fn register_collateral(
            ref self: ContractState,
            loan_id: u256,
            btc_encrypted: (felt252, felt252),
            proof_hash: felt252
        ) -> bool {
            // 1. Check not paused
            self.assert_not_paused();

            // 2. Delegate to collateral_manager
            let collateral_mgr = ICollateralManagerDispatcher {
                contract_address: self.collateral_manager.read()
            };

            let success = collateral_mgr
                .register_collateral(loan_id, btc_encrypted, proof_hash);

            // 3. Emit event
            self
                .emit(
                    CollateralRegistered {
                        loan_id, btc_encrypted, proof_hash, timestamp: get_block_timestamp(),
                    }
                );

            success
        }

        fn activate_loan(
            ref self: ContractState,
            loan_id: u256,
            plst_encrypted: (felt252, felt252),
            ltv_proof: Span<felt252>
        ) -> bool {
            // 1. Check not paused
            self.assert_not_paused();

            // 2. Delegate to loan_manager (which handles vault + plst_token internally)
            let loan_manager = ILoanManagerDispatcher {
                contract_address: self.loan_manager.read()
            };

            let success = loan_manager.activate_loan(loan_id, plst_encrypted, ltv_proof);

            // 3. Emit event
            self
                .emit(
                    LoanActivated { loan_id, plst_encrypted, timestamp: get_block_timestamp(), }
                );

            success
        }

        fn repay_loan(
            ref self: ContractState,
            loan_id: u256,
            repay_encrypted: (felt252, felt252),
            repay_proof: Span<felt252>
        ) -> bool {
            // 1. Check not paused
            self.assert_not_paused();

            // 2. Delegate to loan_manager
            let loan_manager = ILoanManagerDispatcher {
                contract_address: self.loan_manager.read()
            };

            let success = loan_manager.repay_loan(loan_id, repay_encrypted, repay_proof);

            // 3. Emit event
            self.emit(LoanRepaid { loan_id, timestamp: get_block_timestamp(), });

            success
        }

        fn liquidate_loan(
            ref self: ContractState,
            loan_id: u256,
            liquidation_proof: Span<felt252>
        ) -> bool {
            // 1. Check not paused
            self.assert_not_paused();

            // 2. Delegate to loan_manager
            let loan_manager = ILoanManagerDispatcher {
                contract_address: self.loan_manager.read()
            };

            let success = loan_manager.liquidate_loan(loan_id, liquidation_proof);

            // 3. Emit event
            self
                .emit(
                    LoanLiquidated {
                        loan_id, liquidator: get_caller_address(), timestamp: get_block_timestamp(),
                    }
                );

            success
        }

        // === QUERY FUNCTIONS ===

        fn get_loan(self: @ContractState, loan_id: u256) -> Loan {
            let loan_manager = ILoanManagerDispatcher {
                contract_address: self.loan_manager.read()
            };
            loan_manager.get_loan(loan_id)
        }

        fn get_user_loan_count(self: @ContractState, user: ContractAddress) -> u256 {
            let loan_manager = ILoanManagerDispatcher {
                contract_address: self.loan_manager.read()
            };
            loan_manager.get_user_loan_count(user)
        }

        fn get_collateral_info(self: @ContractState, loan_id: u256) -> CollateralInfo {
            let collateral_mgr = ICollateralManagerDispatcher {
                contract_address: self.collateral_manager.read()
            };
            collateral_mgr.get_collateral_info(loan_id)
        }

        // === ADMIN FUNCTIONS ===

        fn pause(ref self: ContractState) {
            self.assert_owner();
            self.paused.write(true);
            self.emit(Paused { timestamp: get_block_timestamp() });
        }

        fn unpause(ref self: ContractState) {
            self.assert_owner();
            self.paused.write(false);
            self.emit(Unpaused { timestamp: get_block_timestamp() });
        }

        fn is_paused(self: @ContractState) -> bool {
            self.paused.read()
        }

        fn get_owner(self: @ContractState) -> ContractAddress {
            self.owner.read()
        }
    }
}
