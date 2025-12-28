/// Lending Manager Module - Orchestrates private lending with FHE-verified LTV
/// Manages loan lifecycle from creation through FHE verification to repayment/liquidation
module zyberlink::lending_manager {
    use std::signer;
    use std::vector;
    use std::string::String;
    use aptos_framework::coin;
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::account;
    use aptos_framework::timestamp;
    use aptos_std::table::{Self, Table};
    use zyberlink::jobs;
    use zyberlink::collateral_vault;
    use zyberlink::plst_token;

    /// Error codes
    const E_NOT_INITIALIZED: u64 = 1;
    const E_ALREADY_INITIALIZED: u64 = 2;
    const E_LOAN_NOT_FOUND: u64 = 3;
    const E_INVALID_STATUS: u64 = 4;
    const E_NOT_BORROWER: u64 = 5;
    const E_INVALID_AMOUNT: u64 = 6;
    const E_NO_CONSENSUS: u64 = 7;
    const E_CONSENSUS_FAILED: u64 = 8;
    const E_ALREADY_ACTIVATED: u64 = 9;
    const E_NOT_ACTIVE: u64 = 10;
    const E_INSUFFICIENT_PLST: u64 = 11;
    const E_FHE_JOB_MISMATCH: u64 = 12;
    const E_INVALID_LTV: u64 = 13;

    /// Loan status constants
    const STATUS_PENDING: u8 = 0;      // Awaiting FHE verification
    const STATUS_ACTIVE: u8 = 1;       // LTV approved, pLST minted
    const STATUS_REPAID: u8 = 2;       // Loan fully repaid
    const STATUS_LIQUIDATED: u8 = 3;   // Collateral liquidated
    const STATUS_REJECTED: u8 = 4;     // LTV check failed

    /// FHE circuit type for LTV verification
    const FHE_LTV_CHECK: u8 = 7;

    /// Loan record
    struct Loan has store, drop, copy {
        loan_id: u64,
        borrower: address,
        coin_type_hash: String,         // Type name for generic CoinType tracking
        collateral_amount: u64,
        encrypted_c1: vector<u8>,       // Encrypted ciphertext 1
        encrypted_c2: vector<u8>,       // Encrypted ciphertext 2
        borrow_amount: u64,             // Amount of pLST to mint
        ltv_threshold: u16,             // Basis points (e.g., 7500 = 75%)
        fhe_job_id: u64,                // Associated FHE job for verification
        status: u8,
        created_at: u64,
        activated_at: u64,
        closed_at: u64,
    }

    /// Global lending registry
    struct LendingRegistry has key {
        loans: Table<u64, Loan>,
        next_loan_id: u64,
        jobs_module_addr: address,
        vault_admin: address,
        plst_admin: address,
        admin: address,
        total_loans: u64,
        active_loans: u64,
        loan_created_events: EventHandle<LoanCreatedEvent>,
        loan_activated_events: EventHandle<LoanActivatedEvent>,
        loan_repaid_events: EventHandle<LoanRepaidEvent>,
        loan_rejected_events: EventHandle<LoanRejectedEvent>,
    }

    /// Borrower's loan tracking
    struct BorrowerLoans has key {
        loan_ids: vector<u64>,
    }

    /// Events
    struct LoanCreatedEvent has drop, store {
        loan_id: u64,
        borrower: address,
        collateral_amount: u64,
        borrow_amount: u64,
        ltv_threshold: u16,
        fhe_job_id: u64,
        timestamp: u64,
    }

    struct LoanActivatedEvent has drop, store {
        loan_id: u64,
        borrower: address,
        borrow_amount: u64,
        timestamp: u64,
    }

    struct LoanRepaidEvent has drop, store {
        loan_id: u64,
        borrower: address,
        repaid_amount: u64,
        collateral_returned: u64,
        timestamp: u64,
    }

    struct LoanRejectedEvent has drop, store {
        loan_id: u64,
        borrower: address,
        reason: vector<u8>,
        timestamp: u64,
    }

    /// Initialize the lending manager
    /// @param admin - The admin signer
    /// @param jobs_module_addr - Address of the jobs module
    /// @param vault_admin - Address of the vault admin
    /// @param plst_admin - Address of the pLST token admin
    public entry fun initialize(
        admin: &signer,
        jobs_module_addr: address,
        vault_admin: address,
        plst_admin: address,
    ) {
        let admin_addr = signer::address_of(admin);
        assert!(!exists<LendingRegistry>(admin_addr), E_ALREADY_INITIALIZED);

        move_to(admin, LendingRegistry {
            loans: table::new(),
            next_loan_id: 1,
            jobs_module_addr,
            vault_admin,
            plst_admin,
            admin: admin_addr,
            total_loans: 0,
            active_loans: 0,
            loan_created_events: account::new_event_handle<LoanCreatedEvent>(admin),
            loan_activated_events: account::new_event_handle<LoanActivatedEvent>(admin),
            loan_repaid_events: account::new_event_handle<LoanRepaidEvent>(admin),
            loan_rejected_events: account::new_event_handle<LoanRejectedEvent>(admin),
        });
    }

    /// Create a new loan and submit FHE job for LTV verification
    /// @param borrower - The borrower signer
    /// @param registry_addr - Address where LendingRegistry is stored
    /// @param collateral_amount - Amount of collateral to lock
    /// @param encrypted_c1 - First encrypted ciphertext component
    /// @param encrypted_c2 - Second encrypted ciphertext component
    /// @param borrow_amount - Amount of pLST to borrow
    /// @param ltv_threshold - LTV threshold in basis points
    /// @param required_provers - Number of FHE provers required
    /// @param consensus_threshold - Minimum matching results for consensus
    /// @return loan_id
    public fun create_loan<CoinType>(
        borrower: &signer,
        registry_addr: address,
        collateral_amount: u64,
        encrypted_c1: vector<u8>,
        encrypted_c2: vector<u8>,
        borrow_amount: u64,
        ltv_threshold: u16,
        required_provers: u8,
        consensus_threshold: u8,
    ): u64 acquires LendingRegistry, BorrowerLoans {
        assert!(exists<LendingRegistry>(registry_addr), E_NOT_INITIALIZED);
        assert!(collateral_amount > 0, E_INVALID_AMOUNT);
        assert!(borrow_amount > 0, E_INVALID_AMOUNT);
        assert!(ltv_threshold > 0 && ltv_threshold <= 10000, E_INVALID_LTV);

        let borrower_addr = signer::address_of(borrower);
        let registry = borrow_global_mut<LendingRegistry>(registry_addr);

        // Deposit collateral to vault
        collateral_vault::deposit<CoinType>(
            borrower,
            registry.vault_admin,
            registry.next_loan_id,
            collateral_amount,
        );

        // Prepare encrypted witness (c1 || c2)
        let encrypted_witness = encrypted_c1;
        vector::append(&mut encrypted_witness, encrypted_c2);

        // Submit FHE job for LTV verification
        // Calculate witness hash (simplified - in production use proper hash)
        let witness_hash = encrypted_witness;
        let witness_size = (vector::length(&encrypted_witness) as u32);

        jobs::submit_fhe_job(
            borrower,
            registry.jobs_module_addr,
            witness_hash,
            witness_size,
            1_000_000,                         // payment: 0.01 APT
            FHE_LTV_CHECK,
            3600,                              // timeout: 1 hour
            required_provers,
            consensus_threshold,
            (registry.next_loan_id as u16),    // operation_param1: loan_id
            (ltv_threshold as u8),             // operation_param2: threshold (truncated for demo)
            0,                                 // operation_param3: unused
        );

        // Get the FHE job ID (it's the last created job)
        // Note: In production, submit_fhe_job should return the job_id
        let fhe_job_id = registry.next_loan_id; // Simplified: assuming 1:1 mapping

        // Create loan record
        let loan = Loan {
            loan_id: registry.next_loan_id,
            borrower: borrower_addr,
            coin_type_hash: aptos_std::type_info::type_name<CoinType>(),
            collateral_amount,
            encrypted_c1,
            encrypted_c2,
            borrow_amount,
            ltv_threshold,
            fhe_job_id,
            status: STATUS_PENDING,
            created_at: timestamp::now_seconds(),
            activated_at: 0,
            closed_at: 0,
        };

        let loan_id = registry.next_loan_id;
        table::add(&mut registry.loans, loan_id, loan);
        registry.next_loan_id = registry.next_loan_id + 1;
        registry.total_loans = registry.total_loans + 1;

        // Track borrower's loans
        if (!exists<BorrowerLoans>(borrower_addr)) {
            move_to(borrower, BorrowerLoans {
                loan_ids: vector::empty(),
            });
        };
        let borrower_loans = borrow_global_mut<BorrowerLoans>(borrower_addr);
        vector::push_back(&mut borrower_loans.loan_ids, loan_id);

        // Emit event
        event::emit_event(&mut registry.loan_created_events, LoanCreatedEvent {
            loan_id,
            borrower: borrower_addr,
            collateral_amount,
            borrow_amount,
            ltv_threshold,
            fhe_job_id,
            timestamp: timestamp::now_seconds(),
        });

        loan_id
    }

    /// Activate loan after FHE consensus is reached (admin/lending manager only)
    /// @param admin - The lending manager admin signer
    /// @param loan_id - Loan ID to activate
    public entry fun activate_loan(
        admin: &signer,
        loan_id: u64,
    ) acquires LendingRegistry {
        let admin_addr = signer::address_of(admin);
        assert!(exists<LendingRegistry>(admin_addr), E_NOT_INITIALIZED);

        let registry = borrow_global_mut<LendingRegistry>(admin_addr);
        assert!(table::contains(&registry.loans, loan_id), E_LOAN_NOT_FOUND);

        let loan = table::borrow_mut(&mut registry.loans, loan_id);
        assert!(loan.status == STATUS_PENDING, E_INVALID_STATUS);

        // Check FHE consensus using public accessors
        assert!(jobs::is_fhe_finalized(registry.jobs_module_addr, loan.fhe_job_id), E_NO_CONSENSUS);

        // Get consensus hash
        let consensus_hash = jobs::get_fhe_consensus_hash(registry.jobs_module_addr, loan.fhe_job_id);

        // Check if consensus was positive (LTV check passed)
        // Note: In production, parse the consensus_hash to determine pass/fail
        // For now, we check if consensus hash is non-empty
        if (vector::length(&consensus_hash) > 0) {
            // LTV check passed - activate loan
            loan.status = STATUS_ACTIVE;
            loan.activated_at = timestamp::now_seconds();
            registry.active_loans = registry.active_loans + 1;

            // Mint pLST to borrower
            plst_token::mint(
                admin,
                registry.plst_admin,
                loan.borrower,
                loan.borrow_amount,
            );

            // Emit event
            event::emit_event(&mut registry.loan_activated_events, LoanActivatedEvent {
                loan_id,
                borrower: loan.borrower,
                borrow_amount: loan.borrow_amount,
                timestamp: timestamp::now_seconds(),
            });
        } else {
            // LTV check failed - reject loan
            loan.status = STATUS_REJECTED;
            loan.closed_at = timestamp::now_seconds();

            // Emit event
            event::emit_event(&mut registry.loan_rejected_events, LoanRejectedEvent {
                loan_id,
                borrower: loan.borrower,
                reason: b"LTV threshold not met",
                timestamp: timestamp::now_seconds(),
            });
        }
    }

    /// Repay loan and retrieve collateral (requires admin to execute withdrawal)
    /// Step 1: Borrower burns pLST
    /// @param borrower - The borrower signer
    /// @param registry_addr - Address where LendingRegistry is stored
    /// @param loan_id - Loan ID to repay
    public entry fun repay_loan_step1_burn(
        borrower: &signer,
        registry_addr: address,
        loan_id: u64,
    ) acquires LendingRegistry {
        assert!(exists<LendingRegistry>(registry_addr), E_NOT_INITIALIZED);

        let registry = borrow_global_mut<LendingRegistry>(registry_addr);
        assert!(table::contains(&registry.loans, loan_id), E_LOAN_NOT_FOUND);

        let loan = table::borrow_mut(&mut registry.loans, loan_id);
        let borrower_addr = signer::address_of(borrower);
        assert!(loan.borrower == borrower_addr, E_NOT_BORROWER);
        assert!(loan.status == STATUS_ACTIVE, E_NOT_ACTIVE);

        // Burn pLST from borrower
        plst_token::burn(
            borrower,
            registry.plst_admin,
            borrower_addr,
            loan.borrow_amount,
        );

        // Mark as ready for repayment (admin needs to release collateral)
        // For now, we'll mark as REPAID here and admin releases in step 2
        loan.status = STATUS_REPAID;
        loan.closed_at = timestamp::now_seconds();
        registry.active_loans = registry.active_loans - 1;
    }

    /// Step 2: Admin releases collateral back to borrower
    /// @param admin - The lending manager admin signer
    /// @param loan_id - Loan ID to release collateral
    public entry fun repay_loan_step2_release<CoinType>(
        admin: &signer,
        loan_id: u64,
    ) acquires LendingRegistry {
        let admin_addr = signer::address_of(admin);
        assert!(exists<LendingRegistry>(admin_addr), E_NOT_INITIALIZED);

        let registry = borrow_global_mut<LendingRegistry>(admin_addr);
        assert!(table::contains(&registry.loans, loan_id), E_LOAN_NOT_FOUND);

        let loan = table::borrow(&registry.loans, loan_id);
        assert!(loan.status == STATUS_REPAID, E_INVALID_STATUS);

        // Verify coin type matches
        let coin_type_hash = aptos_std::type_info::type_name<CoinType>();
        assert!(loan.coin_type_hash == coin_type_hash, E_INVALID_STATUS);

        // Return collateral from vault
        collateral_vault::withdraw<CoinType>(
            admin,
            registry.vault_admin,
            loan.borrower,
            loan_id,
            loan.collateral_amount,
        );

        // Emit event
        event::emit_event(&mut registry.loan_repaid_events, LoanRepaidEvent {
            loan_id,
            borrower: loan.borrower,
            repaid_amount: loan.borrow_amount,
            collateral_returned: loan.collateral_amount,
            timestamp: timestamp::now_seconds(),
        });
    }

    /// Get loan details
    /// @param registry_addr - Address where LendingRegistry is stored
    /// @param loan_id - Loan ID to query
    /// @return Loan record
    public fun get_loan(registry_addr: address, loan_id: u64): Loan acquires LendingRegistry {
        assert!(exists<LendingRegistry>(registry_addr), E_NOT_INITIALIZED);
        let registry = borrow_global<LendingRegistry>(registry_addr);
        assert!(table::contains(&registry.loans, loan_id), E_LOAN_NOT_FOUND);
        *table::borrow(&registry.loans, loan_id)
    }

    /// Get all loan IDs for a borrower
    /// @param borrower - Borrower address
    /// @return Vector of loan IDs
    public fun get_borrower_loans(borrower: address): vector<u64> acquires BorrowerLoans {
        if (!exists<BorrowerLoans>(borrower)) {
            return vector::empty()
        };
        *&borrow_global<BorrowerLoans>(borrower).loan_ids
    }

    /// Get total number of loans
    /// @param registry_addr - Address where LendingRegistry is stored
    /// @return Total loans count
    public fun get_total_loans(registry_addr: address): u64 acquires LendingRegistry {
        if (!exists<LendingRegistry>(registry_addr)) {
            return 0
        };
        borrow_global<LendingRegistry>(registry_addr).total_loans
    }

    /// Get number of active loans
    /// @param registry_addr - Address where LendingRegistry is stored
    /// @return Active loans count
    public fun get_active_loans(registry_addr: address): u64 acquires LendingRegistry {
        if (!exists<LendingRegistry>(registry_addr)) {
            return 0
        };
        borrow_global<LendingRegistry>(registry_addr).active_loans
    }

    /// Check if loan exists
    /// @param registry_addr - Address where LendingRegistry is stored
    /// @param loan_id - Loan ID to check
    /// @return True if loan exists
    public fun loan_exists(registry_addr: address, loan_id: u64): bool acquires LendingRegistry {
        if (!exists<LendingRegistry>(registry_addr)) {
            return false
        };
        let registry = borrow_global<LendingRegistry>(registry_addr);
        table::contains(&registry.loans, loan_id)
    }
}
