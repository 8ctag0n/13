/// ZyberLink Marketplace Module - Global configuration and prover registry
/// Manages marketplace parameters, fees, and prover registration/reputation
module zyberlink::marketplace {
    use std::signer;
    use aptos_framework::account;
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::timestamp;
    use aptos_framework::event::{Self, EventHandle};
    use aptos_std::table::{Self, Table};

    /// Error codes
    const E_NOT_INITIALIZED: u64 = 1;
    const E_ALREADY_INITIALIZED: u64 = 2;
    const E_UNAUTHORIZED: u64 = 3;
    const E_PROVER_NOT_FOUND: u64 = 4;
    const E_PROVER_ALREADY_REGISTERED: u64 = 5;
    const E_INSUFFICIENT_STAKE: u64 = 6;
    const E_INVALID_FEE: u64 = 7;
    const E_MARKETPLACE_PAUSED: u64 = 8;
    const E_INSUFFICIENT_REPUTATION: u64 = 9;

    /// Reputation constants
    const MAX_REPUTATION: u32 = 1000;
    const INITIAL_REPUTATION: u32 = 1000;
    const REPUTATION_INCREASE_ON_SUCCESS: u32 = 10;
    const REPUTATION_DECREASE_ON_FAIL: u32 = 50;
    const REPUTATION_SLASH_PENALTY: u32 = 100;

    /// Default marketplace parameters
    const DEFAULT_FEE_BPS: u16 = 1000;        // 10%
    const DEFAULT_MIN_STAKE: u64 = 5_000_000_000; // 5 APT in octas
    const DEFAULT_MIN_REPUTATION: u32 = 500;
    const DEFAULT_JOB_TIMEOUT: u64 = 600;     // 10 minutes

    /// Prover account - stores reputation and stats
    struct ProverAccount has store, drop, copy {
        authority: address,
        stake_amount: u64,
        reputation_score: u32,
        total_jobs_completed: u64,
        total_jobs_failed: u64,
        avg_completion_time_secs: u32,
        is_active: bool,
        registration_timestamp: u64,
        total_earnings: u64,
        encryption_pubkey: vector<u8>, // X25519 public key (32 bytes)
    }

    /// Global marketplace configuration
    struct MarketplaceConfig has key {
        authority: address,
        fee_basis_points: u16,
        min_stake_amount: u64,
        min_reputation_score: u32,
        default_job_timeout_seconds: u64,
        protocol_fee_recipient: address,
        next_job_id: u64,
        total_provers: u64,
        total_jobs_created: u64,
        total_jobs_completed: u64,
        is_paused: bool,
        protocol_fees_collected: Coin<AptosCoin>,

        // Provers registry
        provers: Table<address, ProverAccount>,

        // Events
        prover_registered_events: EventHandle<ProverRegisteredEvent>,
        prover_slashed_events: EventHandle<ProverSlashedEvent>,
        config_updated_events: EventHandle<ConfigUpdatedEvent>,
    }

    /// Events
    struct ProverRegisteredEvent has drop, store {
        prover: address,
        stake_amount: u64,
        timestamp: u64,
    }

    struct ProverSlashedEvent has drop, store {
        prover: address,
        slash_amount: u64,
        reason: vector<u8>,
        new_reputation: u32,
    }

    struct ConfigUpdatedEvent has drop, store {
        updated_by: address,
        timestamp: u64,
    }

    /// Initialize marketplace (called once by deployer)
    public entry fun initialize(
        account: &signer,
        protocol_fee_recipient: address,
    ) {
        let account_addr = signer::address_of(account);
        assert!(!exists<MarketplaceConfig>(account_addr), E_ALREADY_INITIALIZED);

        move_to(account, MarketplaceConfig {
            authority: account_addr,
            fee_basis_points: DEFAULT_FEE_BPS,
            min_stake_amount: DEFAULT_MIN_STAKE,
            min_reputation_score: DEFAULT_MIN_REPUTATION,
            default_job_timeout_seconds: DEFAULT_JOB_TIMEOUT,
            protocol_fee_recipient,
            next_job_id: 0,
            total_provers: 0,
            total_jobs_created: 0,
            total_jobs_completed: 0,
            is_paused: false,
            protocol_fees_collected: coin::zero<AptosCoin>(),
            provers: table::new(),
            prover_registered_events: account::new_event_handle<ProverRegisteredEvent>(account),
            prover_slashed_events: account::new_event_handle<ProverSlashedEvent>(account),
            config_updated_events: account::new_event_handle<ConfigUpdatedEvent>(account),
        });
    }

    /// Register a new prover with stake
    public entry fun register_prover(
        prover: &signer,
        marketplace_addr: address,
        stake_amount: u64,
        encryption_pubkey: vector<u8>, // X25519 public key
    ) acquires MarketplaceConfig {
        assert!(exists<MarketplaceConfig>(marketplace_addr), E_NOT_INITIALIZED);

        let prover_addr = signer::address_of(prover);
        let marketplace = borrow_global_mut<MarketplaceConfig>(marketplace_addr);

        assert!(!marketplace.is_paused, E_MARKETPLACE_PAUSED);
        assert!(!table::contains(&marketplace.provers, prover_addr), E_PROVER_ALREADY_REGISTERED);
        assert!(stake_amount >= marketplace.min_stake_amount, E_INSUFFICIENT_STAKE);

        // Withdraw stake from prover and store in marketplace
        let stake_coin = coin::withdraw<AptosCoin>(prover, stake_amount);
        coin::merge(&mut marketplace.protocol_fees_collected, stake_coin);

        let now = timestamp::now_seconds();
        let prover_account = ProverAccount {
            authority: prover_addr,
            stake_amount,
            reputation_score: INITIAL_REPUTATION,
            total_jobs_completed: 0,
            total_jobs_failed: 0,
            avg_completion_time_secs: 0,
            is_active: true,
            registration_timestamp: now,
            total_earnings: 0,
            encryption_pubkey,
        };

        table::add(&mut marketplace.provers, prover_addr, prover_account);
        marketplace.total_provers = marketplace.total_provers + 1;

        event::emit_event(&mut marketplace.prover_registered_events, ProverRegisteredEvent {
            prover: prover_addr,
            stake_amount,
            timestamp: now,
        });
    }

    /// Update prover stats after successful job completion
    public fun update_prover_on_success(
        marketplace_addr: address,
        prover_addr: address,
        completion_time_secs: u32,
        payout: u64,
    ) acquires MarketplaceConfig {
        let marketplace = borrow_global_mut<MarketplaceConfig>(marketplace_addr);
        assert!(table::contains(&marketplace.provers, prover_addr), E_PROVER_NOT_FOUND);

        let prover = table::borrow_mut(&mut marketplace.provers, prover_addr);

        prover.total_jobs_completed = prover.total_jobs_completed + 1;
        prover.total_earnings = prover.total_earnings + payout;

        // Update rolling average completion time
        let total_jobs = prover.total_jobs_completed + prover.total_jobs_failed;
        if (total_jobs == 1) {
            prover.avg_completion_time_secs = completion_time_secs;
        } else {
            // Weighted average: 80% old, 20% new
            let old_avg = (prover.avg_completion_time_secs as u64);
            let new_time = (completion_time_secs as u64);
            let weighted = (old_avg * 4 + new_time) / 5;
            prover.avg_completion_time_secs = (weighted as u32);
        };

        // Increase reputation (max 1000)
        if (prover.reputation_score < MAX_REPUTATION) {
            let new_rep = prover.reputation_score + REPUTATION_INCREASE_ON_SUCCESS;
            prover.reputation_score = if (new_rep > MAX_REPUTATION) { MAX_REPUTATION } else { new_rep };
        };
    }

    /// Update prover stats after job failure
    public fun update_prover_on_failure(
        marketplace_addr: address,
        prover_addr: address,
    ) acquires MarketplaceConfig {
        let marketplace = borrow_global_mut<MarketplaceConfig>(marketplace_addr);
        assert!(table::contains(&marketplace.provers, prover_addr), E_PROVER_NOT_FOUND);

        let prover = table::borrow_mut(&mut marketplace.provers, prover_addr);

        prover.total_jobs_failed = prover.total_jobs_failed + 1;

        // Reduce reputation
        if (prover.reputation_score > REPUTATION_DECREASE_ON_FAIL) {
            prover.reputation_score = prover.reputation_score - REPUTATION_DECREASE_ON_FAIL;
        } else {
            prover.reputation_score = 0;
        };
    }

    /// Slash a prover for misbehavior (authority only)
    public entry fun slash_prover(
        authority: &signer,
        marketplace_addr: address,
        prover_addr: address,
        slash_amount: u64,
        reason: vector<u8>,
    ) acquires MarketplaceConfig {
        assert!(exists<MarketplaceConfig>(marketplace_addr), E_NOT_INITIALIZED);

        let authority_addr = signer::address_of(authority);
        let marketplace = borrow_global_mut<MarketplaceConfig>(marketplace_addr);

        assert!(authority_addr == marketplace.authority, E_UNAUTHORIZED);
        assert!(table::contains(&marketplace.provers, prover_addr), E_PROVER_NOT_FOUND);

        let prover = table::borrow_mut(&mut marketplace.provers, prover_addr);

        // Reduce stake
        if (slash_amount >= prover.stake_amount) {
            prover.stake_amount = 0;
            prover.is_active = false;
        } else {
            prover.stake_amount = prover.stake_amount - slash_amount;
            if (prover.stake_amount < marketplace.min_stake_amount) {
                prover.is_active = false;
            };
        };

        // Reduce reputation
        if (prover.reputation_score > REPUTATION_SLASH_PENALTY) {
            prover.reputation_score = prover.reputation_score - REPUTATION_SLASH_PENALTY;
        } else {
            prover.reputation_score = 0;
        };

        prover.total_jobs_failed = prover.total_jobs_failed + 1;

        event::emit_event(&mut marketplace.prover_slashed_events, ProverSlashedEvent {
            prover: prover_addr,
            slash_amount,
            reason,
            new_reputation: prover.reputation_score,
        });
    }

    /// Check if prover can claim jobs
    public fun can_claim_jobs(
        marketplace_addr: address,
        prover_addr: address,
    ): bool acquires MarketplaceConfig {
        let marketplace = borrow_global<MarketplaceConfig>(marketplace_addr);

        if (!table::contains(&marketplace.provers, prover_addr)) {
            return false
        };

        let prover = table::borrow(&marketplace.provers, prover_addr);

        prover.is_active &&
        prover.reputation_score >= marketplace.min_reputation_score &&
        prover.stake_amount >= marketplace.min_stake_amount
    }

    /// Calculate platform fee
    public fun calculate_platform_fee(
        marketplace_addr: address,
        price: u64,
    ): u64 acquires MarketplaceConfig {
        let marketplace = borrow_global<MarketplaceConfig>(marketplace_addr);
        (price * (marketplace.fee_basis_points as u64)) / 10000
    }

    /// Calculate prover payout (price minus fee)
    public fun calculate_prover_payout(
        marketplace_addr: address,
        price: u64,
    ): u64 acquires MarketplaceConfig {
        price - calculate_platform_fee(marketplace_addr, price)
    }

    /// Get next job ID and increment counter
    public fun next_job_id(marketplace_addr: address): u64 acquires MarketplaceConfig {
        let marketplace = borrow_global_mut<MarketplaceConfig>(marketplace_addr);
        let id = marketplace.next_job_id;
        marketplace.next_job_id = id + 1;
        marketplace.total_jobs_created = marketplace.total_jobs_created + 1;
        id
    }

    /// Increment completed jobs counter
    public fun increment_completed_jobs(marketplace_addr: address) acquires MarketplaceConfig {
        let marketplace = borrow_global_mut<MarketplaceConfig>(marketplace_addr);
        marketplace.total_jobs_completed = marketplace.total_jobs_completed + 1;
    }

    /// Update marketplace config (authority only)
    public entry fun update_config(
        authority: &signer,
        marketplace_addr: address,
        fee_basis_points: u16,
        min_stake_amount: u64,
        min_reputation_score: u32,
        default_job_timeout_seconds: u64,
    ) acquires MarketplaceConfig {
        assert!(exists<MarketplaceConfig>(marketplace_addr), E_NOT_INITIALIZED);

        let authority_addr = signer::address_of(authority);
        let marketplace = borrow_global_mut<MarketplaceConfig>(marketplace_addr);

        assert!(authority_addr == marketplace.authority, E_UNAUTHORIZED);
        assert!(fee_basis_points <= 10000, E_INVALID_FEE); // Max 100%

        marketplace.fee_basis_points = fee_basis_points;
        marketplace.min_stake_amount = min_stake_amount;
        marketplace.min_reputation_score = min_reputation_score;
        marketplace.default_job_timeout_seconds = default_job_timeout_seconds;

        event::emit_event(&mut marketplace.config_updated_events, ConfigUpdatedEvent {
            updated_by: authority_addr,
            timestamp: timestamp::now_seconds(),
        });
    }

    /// Pause/unpause marketplace (authority only)
    public entry fun set_paused(
        authority: &signer,
        marketplace_addr: address,
        paused: bool,
    ) acquires MarketplaceConfig {
        assert!(exists<MarketplaceConfig>(marketplace_addr), E_NOT_INITIALIZED);

        let authority_addr = signer::address_of(authority);
        let marketplace = borrow_global_mut<MarketplaceConfig>(marketplace_addr);

        assert!(authority_addr == marketplace.authority, E_UNAUTHORIZED);
        marketplace.is_paused = paused;
    }

    /// Add protocol fees to collection
    public fun collect_protocol_fee(
        marketplace_addr: address,
        fee_coin: Coin<AptosCoin>,
    ) acquires MarketplaceConfig {
        let marketplace = borrow_global_mut<MarketplaceConfig>(marketplace_addr);
        coin::merge(&mut marketplace.protocol_fees_collected, fee_coin);
    }

    /// View functions
    #[view]
    public fun get_prover(marketplace_addr: address, prover_addr: address): ProverAccount acquires MarketplaceConfig {
        let marketplace = borrow_global<MarketplaceConfig>(marketplace_addr);
        assert!(table::contains(&marketplace.provers, prover_addr), E_PROVER_NOT_FOUND);
        *table::borrow(&marketplace.provers, prover_addr)
    }

    #[view]
    public fun get_total_provers(marketplace_addr: address): u64 acquires MarketplaceConfig {
        borrow_global<MarketplaceConfig>(marketplace_addr).total_provers
    }

    #[view]
    public fun get_total_jobs_created(marketplace_addr: address): u64 acquires MarketplaceConfig {
        borrow_global<MarketplaceConfig>(marketplace_addr).total_jobs_created
    }

    #[view]
    public fun is_paused(marketplace_addr: address): bool acquires MarketplaceConfig {
        borrow_global<MarketplaceConfig>(marketplace_addr).is_paused
    }

    #[view]
    public fun prover_exists(marketplace_addr: address, prover_addr: address): bool acquires MarketplaceConfig {
        if (!exists<MarketplaceConfig>(marketplace_addr)) {
            return false
        };
        let marketplace = borrow_global<MarketplaceConfig>(marketplace_addr);
        table::contains(&marketplace.provers, prover_addr)
    }

    #[view]
    public fun get_fee_basis_points(marketplace_addr: address): u16 acquires MarketplaceConfig {
        borrow_global<MarketplaceConfig>(marketplace_addr).fee_basis_points
    }

    #[view]
    public fun get_default_timeout(marketplace_addr: address): u64 acquires MarketplaceConfig {
        borrow_global<MarketplaceConfig>(marketplace_addr).default_job_timeout_seconds
    }
}
