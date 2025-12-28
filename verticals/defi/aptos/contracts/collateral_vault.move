/// Collateral Vault Module - Generic custodian for lending collateral
/// Manages locked collateral for active loans across multiple coin types
module zyberlink::collateral_vault {
    use std::signer;
    use std::string::String;
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::account;
    use aptos_std::table::{Self, Table};
    use aptos_std::type_info;

    /// Error codes
    const E_NOT_INITIALIZED: u64 = 1;
    const E_ALREADY_INITIALIZED: u64 = 2;
    const E_NOT_AUTHORIZED: u64 = 3;
    const E_INSUFFICIENT_BALANCE: u64 = 4;
    const E_INVALID_AMOUNT: u64 = 5;
    const E_DEPOSIT_NOT_FOUND: u64 = 6;

    /// Deposit record for tracking locked collateral
    struct DepositRecord has store, drop, copy {
        user: address,
        loan_id: u64,
        amount: u64,
        deposited_at: u64,
    }

    /// Vault storage for a specific coin type
    struct VaultStore<phantom CoinType> has key {
        vault: Coin<CoinType>,
        deposits: Table<u64, DepositRecord>, // loan_id -> DepositRecord
        total_locked: u64,
        deposit_events: EventHandle<DepositEvent>,
        withdraw_events: EventHandle<WithdrawEvent>,
    }

    /// Vault access control
    struct VaultConfig has key {
        admin: address,
        lending_manager: address,
    }

    /// Events
    struct DepositEvent has drop, store {
        user: address,
        loan_id: u64,
        coin_type: String,
        amount: u64,
        timestamp: u64,
    }

    struct WithdrawEvent has drop, store {
        user: address,
        loan_id: u64,
        coin_type: String,
        amount: u64,
        timestamp: u64,
    }

    /// Initialize the vault with admin
    /// @param admin - The admin signer
    public entry fun initialize(admin: &signer) {
        let admin_addr = signer::address_of(admin);
        assert!(!exists<VaultConfig>(admin_addr), E_ALREADY_INITIALIZED);

        move_to(admin, VaultConfig {
            admin: admin_addr,
            lending_manager: admin_addr, // Initially admin, will be updated
        });
    }

    /// Set the lending manager address that can withdraw
    /// @param admin - The admin signer
    /// @param lending_manager_addr - Address of the lending manager module
    public entry fun set_lending_manager(admin: &signer, lending_manager_addr: address) acquires VaultConfig {
        let admin_addr = signer::address_of(admin);
        assert!(exists<VaultConfig>(admin_addr), E_NOT_INITIALIZED);

        let config = borrow_global_mut<VaultConfig>(admin_addr);
        assert!(admin_addr == config.admin, E_NOT_AUTHORIZED);
        config.lending_manager = lending_manager_addr;
    }

    /// Initialize vault for a specific coin type (public for admin setup)
    /// @param admin - The admin signer
    public entry fun initialize_vault_store<CoinType>(admin: &signer) acquires VaultConfig {
        let admin_addr = signer::address_of(admin);
        assert!(exists<VaultConfig>(admin_addr), E_NOT_INITIALIZED);

        let config = borrow_global<VaultConfig>(admin_addr);
        assert!(admin_addr == config.admin, E_NOT_AUTHORIZED);

        if (!exists<VaultStore<CoinType>>(admin_addr)) {
            move_to(admin, VaultStore<CoinType> {
                vault: coin::zero<CoinType>(),
                deposits: table::new(),
                total_locked: 0,
                deposit_events: account::new_event_handle<DepositEvent>(admin),
                withdraw_events: account::new_event_handle<WithdrawEvent>(admin),
            });
        }
    }

    /// Deposit collateral into the vault
    /// @param user - The user depositing collateral
    /// @param vault_admin - Address where vault is stored
    /// @param loan_id - Associated loan ID
    /// @param amount - Amount to deposit
    public fun deposit<CoinType>(
        user: &signer,
        vault_admin: address,
        loan_id: u64,
        amount: u64,
    ) acquires VaultStore {
        assert!(exists<VaultConfig>(vault_admin), E_NOT_INITIALIZED);
        assert!(amount > 0, E_INVALID_AMOUNT);

        let user_addr = signer::address_of(user);

        // Ensure vault store exists for this coin type
        if (!exists<VaultStore<CoinType>>(vault_admin)) {
            // This is a limitation - we need admin signer to initialize
            // In production, this should be called by admin beforehand
            abort E_NOT_INITIALIZED
        };

        let store = borrow_global_mut<VaultStore<CoinType>>(vault_admin);

        // Transfer coins from user to vault
        let coins = coin::withdraw<CoinType>(user, amount);
        coin::merge(&mut store.vault, coins);

        // Record deposit
        let record = DepositRecord {
            user: user_addr,
            loan_id,
            amount,
            deposited_at: aptos_framework::timestamp::now_seconds(),
        };

        if (table::contains(&store.deposits, loan_id)) {
            // Update existing deposit
            let existing = table::borrow_mut(&mut store.deposits, loan_id);
            existing.amount = existing.amount + amount;
        } else {
            // New deposit
            table::add(&mut store.deposits, loan_id, record);
        };

        store.total_locked = store.total_locked + amount;

        // Emit event
        event::emit_event(&mut store.deposit_events, DepositEvent {
            user: user_addr,
            loan_id,
            coin_type: type_info::type_name<CoinType>(),
            amount,
            timestamp: aptos_framework::timestamp::now_seconds(),
        });
    }

    /// Withdraw collateral from the vault (only callable by lending manager)
    /// @param caller - The lending manager signer
    /// @param vault_admin - Address where vault is stored
    /// @param user - User to receive the withdrawal
    /// @param loan_id - Associated loan ID
    /// @param amount - Amount to withdraw
    public fun withdraw<CoinType>(
        caller: &signer,
        vault_admin: address,
        user: address,
        loan_id: u64,
        amount: u64,
    ) acquires VaultStore, VaultConfig {
        assert!(exists<VaultConfig>(vault_admin), E_NOT_INITIALIZED);
        assert!(exists<VaultStore<CoinType>>(vault_admin), E_NOT_INITIALIZED);
        assert!(amount > 0, E_INVALID_AMOUNT);

        // Verify caller is lending manager
        let config = borrow_global<VaultConfig>(vault_admin);
        let caller_addr = signer::address_of(caller);
        assert!(caller_addr == config.lending_manager, E_NOT_AUTHORIZED);

        let store = borrow_global_mut<VaultStore<CoinType>>(vault_admin);

        // Check deposit exists and has sufficient balance
        assert!(table::contains(&store.deposits, loan_id), E_DEPOSIT_NOT_FOUND);
        let deposit = table::borrow_mut(&mut store.deposits, loan_id);
        assert!(deposit.amount >= amount, E_INSUFFICIENT_BALANCE);
        assert!(deposit.user == user, E_NOT_AUTHORIZED);

        // Update deposit record
        deposit.amount = deposit.amount - amount;

        // Extract coins from vault and transfer to user
        let coins = coin::extract(&mut store.vault, amount);
        coin::deposit(user, coins);

        store.total_locked = store.total_locked - amount;

        // Emit event
        event::emit_event(&mut store.withdraw_events, WithdrawEvent {
            user,
            loan_id,
            coin_type: type_info::type_name<CoinType>(),
            amount,
            timestamp: aptos_framework::timestamp::now_seconds(),
        });

        // Clean up if deposit is fully withdrawn
        if (deposit.amount == 0) {
            table::remove(&mut store.deposits, loan_id);
        }
    }

    /// Get locked amount for a specific loan
    /// @param vault_admin - Address where vault is stored
    /// @param loan_id - Loan ID to query
    /// @return Locked amount
    public fun get_locked_amount<CoinType>(vault_admin: address, loan_id: u64): u64 acquires VaultStore {
        if (!exists<VaultStore<CoinType>>(vault_admin)) {
            return 0
        };

        let store = borrow_global<VaultStore<CoinType>>(vault_admin);
        if (!table::contains(&store.deposits, loan_id)) {
            return 0
        };

        table::borrow(&store.deposits, loan_id).amount
    }

    /// Get total locked collateral across all loans
    /// @param vault_admin - Address where vault is stored
    /// @return Total locked amount
    public fun get_total_locked<CoinType>(vault_admin: address): u64 acquires VaultStore {
        if (!exists<VaultStore<CoinType>>(vault_admin)) {
            return 0
        };

        borrow_global<VaultStore<CoinType>>(vault_admin).total_locked
    }

    /// Get deposit record for a loan
    /// @param vault_admin - Address where vault is stored
    /// @param loan_id - Loan ID to query
    /// @return Deposit record
    public fun get_deposit_record<CoinType>(vault_admin: address, loan_id: u64): DepositRecord acquires VaultStore {
        assert!(exists<VaultStore<CoinType>>(vault_admin), E_NOT_INITIALIZED);
        let store = borrow_global<VaultStore<CoinType>>(vault_admin);
        assert!(table::contains(&store.deposits, loan_id), E_DEPOSIT_NOT_FOUND);
        *table::borrow(&store.deposits, loan_id)
    }

    /// Check if vault is initialized
    /// @param vault_admin - Address to check
    /// @return True if initialized
    public fun is_initialized(vault_admin: address): bool {
        exists<VaultConfig>(vault_admin)
    }
}
