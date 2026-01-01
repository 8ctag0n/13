// BTCVault - WBTC Custody Manager
// Manages collateral deposits and withdrawals

use starknet::ContractAddress;

#[starknet::interface]
pub trait IBTCVault<TContractState> {
    // Core functionality
    fn deposit(ref self: TContractState, user: ContractAddress, amount: u256) -> bool;
    fn withdraw(ref self: TContractState, user: ContractAddress, amount: u256) -> bool;

    // View functions
    fn get_user_deposit(self: @TContractState, user: ContractAddress) -> u256;
    fn get_total_deposits(self: @TContractState) -> u256;
    fn get_wbtc_token(self: @TContractState) -> ContractAddress;
}

#[starknet::contract]
pub mod BTCVault {
    use starknet::{ContractAddress, get_caller_address, get_contract_address};
    use core::num::traits::Zero;
    use starknet::storage::{
        Map, StorageMapReadAccess, StorageMapWriteAccess,
        StoragePointerReadAccess, StoragePointerWriteAccess
    };

    // Import ERC-20 interface for WBTC interaction
    #[starknet::interface]
    trait IERC20<TContractState> {
        fn transfer_from(ref self: TContractState, sender: ContractAddress, recipient: ContractAddress, amount: u256) -> bool;
        fn transfer(ref self: TContractState, recipient: ContractAddress, amount: u256) -> bool;
        fn balance_of(self: @TContractState, account: ContractAddress) -> u256;
    }

    #[storage]
    struct Storage {
        // WBTC token contract address
        wbtc_token: ContractAddress,

        // Access control - only core contract can call deposit/withdraw
        core_contract: ContractAddress,

        // User deposits tracking
        user_deposits: Map<ContractAddress, u256>,

        // Total deposits
        total_deposits: u256,
    }

    #[event]
    #[derive(Drop, starknet::Event)]
    pub enum Event {
        Deposited: Deposited,
        Withdrawn: Withdrawn,
    }

    #[derive(Drop, starknet::Event)]
    pub struct Deposited {
        pub user: ContractAddress,
        pub amount: u256,
        pub timestamp: u64,
    }

    #[derive(Drop, starknet::Event)]
    pub struct Withdrawn {
        pub user: ContractAddress,
        pub amount: u256,
        pub timestamp: u64,
    }

    #[constructor]
    fn constructor(
        ref self: ContractState,
        wbtc_token: ContractAddress,
        core_contract: ContractAddress,
    ) {
        self.wbtc_token.write(wbtc_token);
        self.core_contract.write(core_contract);
        self.total_deposits.write(0);
    }

    #[abi(embed_v0)]
    impl BTCVaultImpl of super::IBTCVault<ContractState> {
        fn deposit(ref self: ContractState, user: ContractAddress, amount: u256) -> bool {
            // Only core contract can deposit
            let caller = get_caller_address();
            assert(caller == self.core_contract.read(), 'Only core contract can deposit');

            assert(!user.is_zero(), 'Invalid user address');
            assert(amount > 0, 'Amount must be positive');

            // Transfer WBTC from user to vault
            let wbtc_token = self.wbtc_token.read();
            let vault_address = get_contract_address();

            let wbtc_dispatcher = IERC20Dispatcher { contract_address: wbtc_token };
            let success = wbtc_dispatcher.transfer_from(user, vault_address, amount);
            assert(success, 'WBTC transfer failed');

            // Update user deposit
            let current_deposit = self.user_deposits.read(user);
            self.user_deposits.write(user, current_deposit + amount);

            // Update total deposits
            let current_total = self.total_deposits.read();
            self.total_deposits.write(current_total + amount);

            // Emit event
            self.emit(Deposited {
                user,
                amount,
                timestamp: starknet::get_block_timestamp()
            });

            true
        }

        fn withdraw(ref self: ContractState, user: ContractAddress, amount: u256) -> bool {
            // Only core contract can withdraw
            let caller = get_caller_address();
            assert(caller == self.core_contract.read(), 'Only core contract can withdraw');

            assert(!user.is_zero(), 'Invalid user address');
            assert(amount > 0, 'Amount must be positive');

            // Check user has sufficient deposit
            let current_deposit = self.user_deposits.read(user);
            assert(current_deposit >= amount, 'Insufficient deposit');

            // Transfer WBTC from vault to user
            let wbtc_token = self.wbtc_token.read();
            let wbtc_dispatcher = IERC20Dispatcher { contract_address: wbtc_token };
            let success = wbtc_dispatcher.transfer(user, amount);
            assert(success, 'WBTC transfer failed');

            // Update user deposit
            self.user_deposits.write(user, current_deposit - amount);

            // Update total deposits
            let current_total = self.total_deposits.read();
            self.total_deposits.write(current_total - amount);

            // Emit event
            self.emit(Withdrawn {
                user,
                amount,
                timestamp: starknet::get_block_timestamp()
            });

            true
        }

        fn get_user_deposit(self: @ContractState, user: ContractAddress) -> u256 {
            self.user_deposits.read(user)
        }

        fn get_total_deposits(self: @ContractState) -> u256 {
            self.total_deposits.read()
        }

        fn get_wbtc_token(self: @ContractState) -> ContractAddress {
            self.wbtc_token.read()
        }
    }
}
