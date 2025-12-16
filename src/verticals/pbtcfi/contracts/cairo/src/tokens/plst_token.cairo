// PLSTToken - Private Liquid Staking Token
// Standard ERC-20 implementation with privacy extensions ready for V2

use starknet::ContractAddress;

#[starknet::interface]
pub trait IPLST<TContractState> {
    // Standard ERC-20
    fn name(self: @TContractState) -> ByteArray;
    fn symbol(self: @TContractState) -> ByteArray;
    fn decimals(self: @TContractState) -> u8;
    fn total_supply(self: @TContractState) -> u256;
    fn balance_of(self: @TContractState, account: ContractAddress) -> u256;
    fn transfer(ref self: TContractState, recipient: ContractAddress, amount: u256) -> bool;
    fn transfer_from(
        ref self: TContractState, sender: ContractAddress, recipient: ContractAddress, amount: u256
    ) -> bool;
    fn approve(ref self: TContractState, spender: ContractAddress, amount: u256) -> bool;
    fn allowance(self: @TContractState, owner: ContractAddress, spender: ContractAddress) -> u256;

    // Minting (restricted to pBTCFiCore)
    fn mint(ref self: TContractState, to: ContractAddress, amount: u256);
    fn burn(ref self: TContractState, from: ContractAddress, amount: u256);

    // Privacy extensions (V2 - prepared interfaces)
    fn encrypted_balance_of(self: @TContractState, account: ContractAddress) -> (felt252, felt252);
}

#[starknet::contract]
pub mod PLSTToken {
    use starknet::{ContractAddress, get_caller_address};
    use core::num::traits::Zero;
    use starknet::storage::{
        Map, StorageMapReadAccess, StorageMapWriteAccess,
        StoragePointerReadAccess, StoragePointerWriteAccess
    };

    #[storage]
    struct Storage {
        // Standard ERC-20
        total_supply: u256,
        balances: Map<ContractAddress, u256>,
        allowances: Map<(ContractAddress, ContractAddress), u256>,

        // Metadata
        name: ByteArray,
        symbol: ByteArray,
        decimals: u8,

        // Access control
        minter: ContractAddress,

        // Privacy layer (V2) - prepared storage
        encrypted_balances: Map<ContractAddress, (felt252, felt252)>,
    }

    #[event]
    #[derive(Drop, starknet::Event)]
    pub enum Event {
        Transfer: Transfer,
        Approval: Approval,
        Mint: Mint,
        Burn: Burn,
    }

    #[derive(Drop, starknet::Event)]
    pub struct Transfer {
        pub from: ContractAddress,
        pub to: ContractAddress,
        pub value: u256,
    }

    #[derive(Drop, starknet::Event)]
    pub struct Approval {
        pub owner: ContractAddress,
        pub spender: ContractAddress,
        pub value: u256,
    }

    #[derive(Drop, starknet::Event)]
    pub struct Mint {
        pub to: ContractAddress,
        pub amount: u256,
    }

    #[derive(Drop, starknet::Event)]
    pub struct Burn {
        pub from: ContractAddress,
        pub amount: u256,
    }

    #[constructor]
    fn constructor(
        ref self: ContractState,
        minter: ContractAddress,
        name: ByteArray,
        symbol: ByteArray,
        decimals: u8,
    ) {
        self.minter.write(minter);
        self.name.write(name);
        self.symbol.write(symbol);
        self.decimals.write(decimals);
        self.total_supply.write(0);
    }

    #[abi(embed_v0)]
    impl PLSTImpl of super::IPLST<ContractState> {
        // ===== ERC-20 Metadata =====

        fn name(self: @ContractState) -> ByteArray {
            self.name.read()
        }

        fn symbol(self: @ContractState) -> ByteArray {
            self.symbol.read()
        }

        fn decimals(self: @ContractState) -> u8 {
            self.decimals.read()
        }

        fn total_supply(self: @ContractState) -> u256 {
            self.total_supply.read()
        }

        fn balance_of(self: @ContractState, account: ContractAddress) -> u256 {
            self.balances.read(account)
        }

        fn allowance(
            self: @ContractState, owner: ContractAddress, spender: ContractAddress
        ) -> u256 {
            self.allowances.read((owner, spender))
        }

        // ===== ERC-20 Transfers =====

        fn transfer(ref self: ContractState, recipient: ContractAddress, amount: u256) -> bool {
            let sender = get_caller_address();
            self._transfer(sender, recipient, amount);
            true
        }

        fn transfer_from(
            ref self: ContractState,
            sender: ContractAddress,
            recipient: ContractAddress,
            amount: u256
        ) -> bool {
            let caller = get_caller_address();
            let current_allowance = self.allowances.read((sender, caller));

            assert(current_allowance >= amount, 'Insufficient allowance');

            self.allowances.write((sender, caller), current_allowance - amount);
            self._transfer(sender, recipient, amount);
            true
        }

        fn approve(ref self: ContractState, spender: ContractAddress, amount: u256) -> bool {
            let owner = get_caller_address();
            self.allowances.write((owner, spender), amount);

            self.emit(Approval { owner, spender, value: amount });
            true
        }

        // ===== Minting & Burning (Restricted) =====

        fn mint(ref self: ContractState, to: ContractAddress, amount: u256) {
            let caller = get_caller_address();
            assert(caller == self.minter.read(), 'Only minter can mint');

            let current_balance = self.balances.read(to);
            self.balances.write(to, current_balance + amount);

            let current_supply = self.total_supply.read();
            self.total_supply.write(current_supply + amount);

            self.emit(Mint { to, amount });
            self.emit(Transfer {
                from: starknet::contract_address_const::<0>(),
                to,
                value: amount
            });
        }

        fn burn(ref self: ContractState, from: ContractAddress, amount: u256) {
            let caller = get_caller_address();
            assert(caller == self.minter.read(), 'Only minter can burn');

            let current_balance = self.balances.read(from);
            assert(current_balance >= amount, 'Insufficient balance to burn');

            self.balances.write(from, current_balance - amount);

            let current_supply = self.total_supply.read();
            self.total_supply.write(current_supply - amount);

            self.emit(Burn { from, amount });
            self.emit(Transfer {
                from,
                to: starknet::contract_address_const::<0>(),
                value: amount
            });
        }

        // ===== Privacy Extensions (V2 - Mock for now) =====

        fn encrypted_balance_of(
            self: @ContractState, account: ContractAddress
        ) -> (felt252, felt252) {
            // V2: Return real ElGamal encrypted balance
            // MVP: Return mock ciphertext (0, 0)
            self.encrypted_balances.read(account)
        }
    }

    // ===== Internal Functions =====

    #[generate_trait]
    impl InternalImpl of InternalTrait {
        fn _transfer(
            ref self: ContractState,
            sender: ContractAddress,
            recipient: ContractAddress,
            amount: u256
        ) {
            assert(!sender.is_zero(), 'Transfer from zero address');
            assert(!recipient.is_zero(), 'Transfer to zero address');

            let sender_balance = self.balances.read(sender);
            assert(sender_balance >= amount, 'Insufficient balance');

            self.balances.write(sender, sender_balance - amount);

            let recipient_balance = self.balances.read(recipient);
            self.balances.write(recipient, recipient_balance + amount);

            self.emit(Transfer { from: sender, to: recipient, value: amount });
        }
    }
}
