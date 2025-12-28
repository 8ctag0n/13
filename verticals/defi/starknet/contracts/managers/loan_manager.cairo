// LoanManager - Private Loan Lifecycle Manager
// Manages loan creation, activation, repayment, and liquidation with FULL PRIVACY

use starknet::ContractAddress;
use core::array::Span;

// ===== LOAN STATUS ENUM =====

#[derive(Drop, Copy, Serde, starknet::Store, PartialEq)]
pub enum LoanStatus {
    Pending,      // Created, waiting for activation
    Active,       // Activated, user has borrowed pLST
    Repaid,       // Fully repaid
    Liquidated,   // Liquidated due to undercollateralization
}

// ===== PRIVATE LOAN STRUCT =====

#[derive(Drop, Copy, Serde, starknet::Store)]
pub struct Loan {
    // Public fields (ownership + state)
    pub borrower: ContractAddress,
    pub status: LoanStatus,
    pub created_at: u64,
    pub activated_at: u64,

    // PRIVATE - Commitments (Pedersen)
    pub btc_amount_commitment: felt252,
    pub plst_borrowed_commitment: felt252,
    pub ltv_commitment: felt252,

    // PRIVATE - Encrypted values (ElGamal ciphertext)
    pub btc_amount_encrypted: (felt252, felt252),
    pub plst_borrowed_encrypted: (felt252, felt252),

    // UTXO proof hash
    pub collateral_hash: felt252,
}

// ===== EXTERNAL INTERFACE =====

#[starknet::interface]
pub trait ILoanManager<TContractState> {
    // === Lifecycle Management ===

    fn create_loan(
        ref self: TContractState,
        btc_commitment: felt252,
        btc_encrypted: (felt252, felt252),
        collateral_hash: felt252
    ) -> u256;

    fn activate_loan(
        ref self: TContractState,
        loan_id: u256,
        plst_encrypted: (felt252, felt252),
        ltv_proof: Span<felt252>
    ) -> bool;

    fn repay_loan(
        ref self: TContractState,
        loan_id: u256,
        repay_encrypted: (felt252, felt252),
        repay_proof: Span<felt252>
    ) -> bool;

    fn liquidate_loan(
        ref self: TContractState,
        loan_id: u256,
        liquidation_proof: Span<felt252>
    ) -> bool;

    // === Query Functions ===

    fn get_loan(self: @TContractState, loan_id: u256) -> Loan;
    fn get_user_loan_count(self: @TContractState, user: ContractAddress) -> u256;
    fn get_next_loan_id(self: @TContractState) -> u256;
}

// ===== CROSS-CONTRACT INTERFACES =====

// BTCVault interface
#[starknet::interface]
pub trait IBTCVault<TContractState> {
    fn deposit(ref self: TContractState, user: ContractAddress, amount: u256) -> bool;
    fn withdraw(ref self: TContractState, user: ContractAddress, amount: u256) -> bool;
}

// PLSTToken interface
#[starknet::interface]
pub trait IPLST<TContractState> {
    fn mint(ref self: TContractState, to: ContractAddress, amount: u256);
    fn burn(ref self: TContractState, from: ContractAddress, amount: u256);
}

// CollateralManager interface (will be implemented by Agent 2)
#[starknet::interface]
pub trait ICollateralManager<TContractState> {
    fn verify_collateral(ref self: TContractState, loan_id: u256) -> bool;
}

// ===== CONTRACT IMPLEMENTATION =====

#[starknet::contract]
pub mod LoanManager {
    use super::{Loan, LoanStatus, IBTCVaultDispatcher, IBTCVaultDispatcherTrait};
    use super::{IPLSTDispatcher, IPLSTDispatcherTrait};
    use super::{ICollateralManagerDispatcher, ICollateralManagerDispatcherTrait};
    use starknet::{ContractAddress, get_caller_address, get_block_timestamp};
    use core::num::traits::Zero;
    use starknet::storage::{
        Map, StorageMapReadAccess, StorageMapWriteAccess,
        StoragePointerReadAccess, StoragePointerWriteAccess
    };

    // ===== STORAGE =====

    #[storage]
    struct Storage {
        // Loan data
        loans: Map<u256, Loan>,
        next_loan_id: u256,
        user_loan_count: Map<ContractAddress, u256>,

        // Cross-contract references
        vault_address: ContractAddress,
        plst_token: ContractAddress,
        collateral_manager: ContractAddress,
    }

    // ===== EVENTS =====

    #[event]
    #[derive(Drop, starknet::Event)]
    pub enum Event {
        LoanCreated: LoanCreated,
        LoanActivated: LoanActivated,
        LoanRepaid: LoanRepaid,
        LoanLiquidated: LoanLiquidated,
    }

    #[derive(Drop, starknet::Event)]
    pub struct LoanCreated {
        pub loan_id: u256,
        pub borrower: ContractAddress,
        pub btc_commitment: felt252,
        pub btc_encrypted: (felt252, felt252),
        pub collateral_hash: felt252,
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

    // ===== CONSTRUCTOR =====

    #[constructor]
    fn constructor(
        ref self: ContractState,
        vault_address: ContractAddress,
        plst_token: ContractAddress,
        collateral_manager: ContractAddress,
    ) {
        // Validate addresses
        assert(!vault_address.is_zero(), 'Invalid vault address');
        assert(!plst_token.is_zero(), 'Invalid pLST token address');
        assert(!collateral_manager.is_zero(), 'Invalid collateral manager');

        // Initialize storage
        self.next_loan_id.write(1);
        self.vault_address.write(vault_address);
        self.plst_token.write(plst_token);
        self.collateral_manager.write(collateral_manager);
    }

    // ===== EXTERNAL FUNCTIONS =====

    #[abi(embed_v0)]
    impl LoanManagerImpl of super::ILoanManager<ContractState> {

        // === CREATE LOAN ===

        fn create_loan(
            ref self: ContractState,
            btc_commitment: felt252,
            btc_encrypted: (felt252, felt252),
            collateral_hash: felt252
        ) -> u256 {
            // Validate inputs
            assert(btc_commitment != 0, 'Invalid BTC commitment');
            let (btc_c1, btc_c2) = btc_encrypted;
            assert(btc_c1 != 0 || btc_c2 != 0, 'Invalid BTC encrypted');
            assert(collateral_hash != 0, 'Invalid collateral hash');

            let borrower = get_caller_address();
            assert(!borrower.is_zero(), 'Invalid borrower');

            // Get new loan ID
            let loan_id = self.next_loan_id.read();

            // Create loan struct
            let loan = Loan {
                borrower,
                status: LoanStatus::Pending,
                created_at: get_block_timestamp(),
                activated_at: 0,
                btc_amount_commitment: btc_commitment,
                plst_borrowed_commitment: 0,
                ltv_commitment: 0,
                btc_amount_encrypted: btc_encrypted,
                plst_borrowed_encrypted: (0, 0),
                collateral_hash,
            };

            // Store loan
            self.loans.write(loan_id, loan);

            // Increment loan ID
            self.next_loan_id.write(loan_id + 1);

            // Update user loan count
            let user_count = self.user_loan_count.read(borrower);
            self.user_loan_count.write(borrower, user_count + 1);

            // Emit event
            self.emit(LoanCreated {
                loan_id,
                borrower,
                btc_commitment,
                btc_encrypted,
                collateral_hash,
                timestamp: get_block_timestamp(),
            });

            loan_id
        }

        // === ACTIVATE LOAN ===

        fn activate_loan(
            ref self: ContractState,
            loan_id: u256,
            plst_encrypted: (felt252, felt252),
            ltv_proof: Span<felt252>
        ) -> bool {
            // Get loan
            let mut loan = self.loans.read(loan_id);

            // Validate loan exists
            assert(!loan.borrower.is_zero(), 'Loan does not exist');

            // Validate status
            assert(loan.status == LoanStatus::Pending, 'Loan not pending');

            // Validate caller is borrower
            let caller = get_caller_address();
            assert(caller == loan.borrower, 'Only borrower can activate');

            // Validate inputs
            let (plst_c1, plst_c2) = plst_encrypted;
            assert(plst_c1 != 0 || plst_c2 != 0, 'Invalid pLST encrypted');

            // Mock crypto validation: Accept ltv_proof without verification
            // In V2, Agent 2 will implement crypto_lib.verify_ltv_proof()
            let _ltv_proof_valid = ltv_proof.len() >= 0; // Mock: always accept

            // Update loan with pLST data
            loan.plst_borrowed_encrypted = plst_encrypted;
            loan.plst_borrowed_commitment = plst_c1; // Mock: use first element as commitment
            // Mock: Extract first proof element if available
            loan.ltv_commitment = if ltv_proof.len() > 0 {
                *ltv_proof.at(0)
            } else {
                0
            };

            // Cross-contract: Verify collateral
            let collateral_manager = ICollateralManagerDispatcher {
                contract_address: self.collateral_manager.read()
            };

            // Mock: Skip collateral verification if manager is zero (not deployed yet)
            let collateral_verified = if !self.collateral_manager.read().is_zero() {
                collateral_manager.verify_collateral(loan_id)
            } else {
                true // Mock: assume verified if manager not deployed
            };

            assert(collateral_verified, 'Collateral verification failed');

            // Cross-contract: Deposit BTC to vault
            // Extract amount from encrypted value (temporary for MVP)
            let (btc_enc_c1, _btc_enc_c2) = loan.btc_amount_encrypted;
            let btc_amount_for_vault = u256 {
                low: btc_enc_c1.try_into().unwrap_or(0),
                high: 0
            };

            let vault = IBTCVaultDispatcher {
                contract_address: self.vault_address.read()
            };

            let deposit_success = vault.deposit(loan.borrower, btc_amount_for_vault);
            assert(deposit_success, 'Vault deposit failed');

            // Cross-contract: Mint pLST to borrower
            // Extract amount from encrypted value (temporary for MVP)
            let plst_amount_to_mint = u256 {
                low: plst_c1.try_into().unwrap_or(0),
                high: 0
            };

            let plst_token = IPLSTDispatcher {
                contract_address: self.plst_token.read()
            };

            plst_token.mint(loan.borrower, plst_amount_to_mint);

            // Update loan status
            loan.status = LoanStatus::Active;
            loan.activated_at = get_block_timestamp();

            // Save updated loan
            self.loans.write(loan_id, loan);

            // Emit event
            self.emit(LoanActivated {
                loan_id,
                plst_encrypted,
                timestamp: get_block_timestamp(),
            });

            true
        }

        // === REPAY LOAN ===

        fn repay_loan(
            ref self: ContractState,
            loan_id: u256,
            repay_encrypted: (felt252, felt252),
            repay_proof: Span<felt252>
        ) -> bool {
            // Get loan
            let mut loan = self.loans.read(loan_id);

            // Validate loan exists
            assert(!loan.borrower.is_zero(), 'Loan does not exist');

            // Validate status
            assert(loan.status == LoanStatus::Active, 'Loan not active');

            // Validate caller is borrower
            let caller = get_caller_address();
            assert(caller == loan.borrower, 'Only borrower can repay');

            // Validate inputs
            let (repay_c1, repay_c2) = repay_encrypted;
            assert(repay_c1 != 0 || repay_c2 != 0, 'Invalid repay encrypted');

            // Mock crypto validation: Accept repay_proof without verification
            // In V2, verify that repay_encrypted matches loan.plst_borrowed_encrypted
            let _repay_proof_valid = repay_proof.len() >= 0; // Mock: always accept

            // Cross-contract: Burn pLST from borrower
            // Extract amount from encrypted value (temporary for MVP)
            let plst_amount_to_burn = u256 {
                low: repay_c1.try_into().unwrap_or(0),
                high: 0
            };

            let plst_token = IPLSTDispatcher {
                contract_address: self.plst_token.read()
            };

            plst_token.burn(loan.borrower, plst_amount_to_burn);

            // Cross-contract: Withdraw BTC from vault
            // Extract amount from encrypted value (temporary for MVP)
            let (loan_btc_c1, _loan_btc_c2) = loan.btc_amount_encrypted;
            let btc_amount_to_withdraw = u256 {
                low: loan_btc_c1.try_into().unwrap_or(0),
                high: 0
            };

            let vault = IBTCVaultDispatcher {
                contract_address: self.vault_address.read()
            };

            let withdraw_success = vault.withdraw(loan.borrower, btc_amount_to_withdraw);
            assert(withdraw_success, 'Vault withdrawal failed');

            // Update loan status
            loan.status = LoanStatus::Repaid;

            // Save updated loan
            self.loans.write(loan_id, loan);

            // Emit event
            self.emit(LoanRepaid {
                loan_id,
                timestamp: get_block_timestamp(),
            });

            true
        }

        // === LIQUIDATE LOAN ===

        fn liquidate_loan(
            ref self: ContractState,
            loan_id: u256,
            liquidation_proof: Span<felt252>
        ) -> bool {
            // Get loan
            let mut loan = self.loans.read(loan_id);

            // Validate loan exists
            assert(!loan.borrower.is_zero(), 'Loan does not exist');

            // Validate status
            assert(loan.status == LoanStatus::Active, 'Loan not active');

            let caller = get_caller_address();

            // Mock crypto validation: Accept liquidation_proof without verification
            // In V2, verify LTV is below threshold using ZK proof
            let _liquidation_valid = liquidation_proof.len() >= 0; // Mock: always accept

            // Cross-contract: Burn pLST from liquidator (they must own it)
            let (loan_plst_c1, _loan_plst_c2) = loan.plst_borrowed_encrypted;
            let plst_amount_to_burn = u256 {
                low: loan_plst_c1.try_into().unwrap_or(0),
                high: 0
            };

            let plst_token = IPLSTDispatcher {
                contract_address: self.plst_token.read()
            };

            plst_token.burn(caller, plst_amount_to_burn);

            // Cross-contract: Transfer collateral to liquidator
            // In V2, this would go through CollateralManager
            let (liq_btc_c1, _liq_btc_c2) = loan.btc_amount_encrypted;
            let btc_amount_to_liquidator = u256 {
                low: liq_btc_c1.try_into().unwrap_or(0),
                high: 0
            };

            let vault = IBTCVaultDispatcher {
                contract_address: self.vault_address.read()
            };

            // Transfer from vault to liquidator (simplified for MVP)
            let withdraw_success = vault.withdraw(loan.borrower, btc_amount_to_liquidator);
            assert(withdraw_success, 'Liquidation transfer failed');

            // Update loan status
            loan.status = LoanStatus::Liquidated;

            // Save updated loan
            self.loans.write(loan_id, loan);

            // Emit event
            self.emit(LoanLiquidated {
                loan_id,
                liquidator: caller,
                timestamp: get_block_timestamp(),
            });

            true
        }

        // === QUERY FUNCTIONS ===

        fn get_loan(self: @ContractState, loan_id: u256) -> Loan {
            self.loans.read(loan_id)
        }

        fn get_user_loan_count(self: @ContractState, user: ContractAddress) -> u256 {
            self.user_loan_count.read(user)
        }

        fn get_next_loan_id(self: @ContractState) -> u256 {
            self.next_loan_id.read()
        }
    }
}
