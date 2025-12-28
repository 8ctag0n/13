/// pLST Token Module - Private Lending Stablecoin Token
/// Fungible asset representing debt in the lending system
module zyberlink::plst_token {
    use std::signer;
    use std::string::{Self, String};
    use std::option;
    use aptos_framework::fungible_asset::{Self, MintRef, TransferRef, BurnRef, Metadata};
    use aptos_framework::object::{Self, Object};
    use aptos_framework::primary_fungible_store;
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::account;

    /// Error codes
    const E_NOT_INITIALIZED: u64 = 1;
    const E_ALREADY_INITIALIZED: u64 = 2;
    const E_NOT_AUTHORIZED: u64 = 3;
    const E_INSUFFICIENT_BALANCE: u64 = 4;
    const E_INVALID_AMOUNT: u64 = 5;

    /// Token configuration
    struct TokenRefs has key {
        mint_ref: MintRef,
        burn_ref: BurnRef,
        transfer_ref: TransferRef,
        lending_manager: address,
        metadata: Object<Metadata>,
        mint_events: EventHandle<MintEvent>,
        burn_events: EventHandle<BurnEvent>,
    }

    /// Events
    struct MintEvent has drop, store {
        recipient: address,
        amount: u64,
        timestamp: u64,
    }

    struct BurnEvent has drop, store {
        from: address,
        amount: u64,
        timestamp: u64,
    }

    /// Initialize the pLST fungible asset
    /// @param admin - The admin signer who will control minting/burning
    public entry fun initialize(admin: &signer) {
        let admin_addr = signer::address_of(admin);
        assert!(!exists<TokenRefs>(admin_addr), E_ALREADY_INITIALIZED);

        // Create the fungible asset
        let constructor_ref = &object::create_named_object(admin, b"pLST");

        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            constructor_ref,
            option::none(),
            string::utf8(b"Private Lending Stablecoin Token"),
            string::utf8(b"pLST"),
            8, // decimals
            string::utf8(b"https://zyberlink.io/plst-icon.png"),
            string::utf8(b"https://zyberlink.io"),
        );

        // Get refs for future minting/burning
        let mint_ref = fungible_asset::generate_mint_ref(constructor_ref);
        let burn_ref = fungible_asset::generate_burn_ref(constructor_ref);
        let transfer_ref = fungible_asset::generate_transfer_ref(constructor_ref);
        let metadata = object::object_from_constructor_ref<Metadata>(constructor_ref);

        // Store refs
        move_to(admin, TokenRefs {
            mint_ref,
            burn_ref,
            transfer_ref,
            lending_manager: admin_addr, // Initially admin, will be updated
            metadata,
            mint_events: account::new_event_handle<MintEvent>(admin),
            burn_events: account::new_event_handle<BurnEvent>(admin),
        });
    }

    /// Set the lending manager address that can mint/burn
    /// @param admin - The admin signer
    /// @param lending_manager_addr - Address of the lending manager module
    public entry fun set_lending_manager(admin: &signer, lending_manager_addr: address) acquires TokenRefs {
        let admin_addr = signer::address_of(admin);
        assert!(exists<TokenRefs>(admin_addr), E_NOT_INITIALIZED);

        let refs = borrow_global_mut<TokenRefs>(admin_addr);
        refs.lending_manager = lending_manager_addr;
    }

    /// Mint pLST tokens to a recipient
    /// @param caller - The lending manager signer
    /// @param admin_addr - Address where TokenRefs is stored
    /// @param recipient - Address to receive tokens
    /// @param amount - Amount to mint
    public fun mint(
        caller: &signer,
        admin_addr: address,
        recipient: address,
        amount: u64,
    ) acquires TokenRefs {
        assert!(exists<TokenRefs>(admin_addr), E_NOT_INITIALIZED);
        assert!(amount > 0, E_INVALID_AMOUNT);

        let refs = borrow_global_mut<TokenRefs>(admin_addr);
        let caller_addr = signer::address_of(caller);
        assert!(caller_addr == refs.lending_manager, E_NOT_AUTHORIZED);

        // Mint to recipient's primary store
        let fa = fungible_asset::mint(&refs.mint_ref, amount);
        primary_fungible_store::deposit(recipient, fa);

        // Emit event
        event::emit_event(&mut refs.mint_events, MintEvent {
            recipient,
            amount,
            timestamp: aptos_framework::timestamp::now_seconds(),
        });
    }

    /// Burn pLST tokens from an account
    /// @param caller - The lending manager signer
    /// @param admin_addr - Address where TokenRefs is stored
    /// @param from - Address to burn tokens from
    /// @param amount - Amount to burn
    public fun burn(
        caller: &signer,
        admin_addr: address,
        from: address,
        amount: u64,
    ) acquires TokenRefs {
        assert!(exists<TokenRefs>(admin_addr), E_NOT_INITIALIZED);
        assert!(amount > 0, E_INVALID_AMOUNT);

        let refs = borrow_global_mut<TokenRefs>(admin_addr);
        let caller_addr = signer::address_of(caller);
        assert!(caller_addr == refs.lending_manager, E_NOT_AUTHORIZED);

        // Burn from primary store
        let fa = primary_fungible_store::withdraw(caller, refs.metadata, amount);
        fungible_asset::burn(&refs.burn_ref, fa);

        // Emit event
        event::emit_event(&mut refs.burn_events, BurnEvent {
            from,
            amount,
            timestamp: aptos_framework::timestamp::now_seconds(),
        });
    }

    /// Get balance of an account
    /// @param admin_addr - Address where TokenRefs is stored
    /// @param account - Account to check balance
    /// @return Balance amount
    public fun balance(admin_addr: address, account: address): u64 acquires TokenRefs {
        if (!exists<TokenRefs>(admin_addr)) {
            return 0
        };

        let refs = borrow_global<TokenRefs>(admin_addr);
        primary_fungible_store::balance(account, refs.metadata)
    }

    /// Get total supply of pLST
    /// @param admin_addr - Address where TokenRefs is stored
    /// @return Total supply
    public fun total_supply(admin_addr: address): u128 acquires TokenRefs {
        assert!(exists<TokenRefs>(admin_addr), E_NOT_INITIALIZED);

        let refs = borrow_global<TokenRefs>(admin_addr);
        option::destroy_some(fungible_asset::supply(refs.metadata))
    }

    /// Check if token is initialized
    /// @param admin_addr - Address to check
    /// @return True if initialized
    public fun is_initialized(admin_addr: address): bool {
        exists<TokenRefs>(admin_addr)
    }

    #[test_only]
    use aptos_framework::timestamp;

    #[test(admin = @zyberlink)]
    fun test_initialize(admin: &signer) {
        timestamp::set_time_has_started_for_testing(&account::create_account_for_test(@0x1));
        initialize(admin);
        assert!(is_initialized(signer::address_of(admin)), 0);
    }
}
