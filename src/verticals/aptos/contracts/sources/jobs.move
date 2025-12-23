/// ZyberLink Jobs Module - Complete job orchestration with FHE consensus
/// Manages ZK proof jobs (circuits 0-3) and FHE computation jobs (circuits 4-11)
module zyberlink::jobs {
    use std::signer;
    use std::vector;
    use std::option::{Self, Option};
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::timestamp;
    use aptos_framework::event::{Self, EventHandle};
    use aptos_std::table::{Self, Table};
    use zyberlink::marketplace;
    use zyberlink::zk_verifier;

    /// Error codes
    const E_NOT_INITIALIZED: u64 = 1;
    const E_ALREADY_INITIALIZED: u64 = 2;
    const E_JOB_NOT_FOUND: u64 = 3;
    const E_INVALID_STATUS: u64 = 4;
    const E_NOT_ASSIGNED: u64 = 5;
    const E_INVALID_PROOF: u64 = 6;
    const E_JOB_EXPIRED: u64 = 7;
    const E_INSUFFICIENT_PAYMENT: u64 = 8;
    const E_WORKER_NOT_REGISTERED: u64 = 9;
    const E_NOT_FHE_JOB: u64 = 10;
    const E_INVALID_CIRCUIT_TYPE: u64 = 11;
    const E_PROVER_ALREADY_CLAIMED: u64 = 12;
    const E_MAX_PROVERS_REACHED: u64 = 13;
    const E_INSUFFICIENT_FHE_RESULTS: u64 = 14;
    const E_ALREADY_FINALIZED: u64 = 15;
    const E_NOT_CREATOR: u64 = 16;
    const E_CONSENSUS_NOT_READY: u64 = 17;

    /// Job status constants
    const STATUS_PENDING: u8 = 0;
    const STATUS_CLAIMED: u8 = 1;
    const STATUS_COMPLETED: u8 = 2;
    const STATUS_FAILED: u8 = 3;
    const STATUS_CANCELLED: u8 = 4;

    /// Circuit type constants - ZK circuits (0-3)
    const CIRCUIT_ZCASH_ORCHARD: u8 = 0;
    const CIRCUIT_ZCASH_SAPLING: u8 = 1;
    const CIRCUIT_ANONYMOUS_VOTE: u8 = 2;
    const CIRCUIT_CREDENTIAL: u8 = 3;

    /// Circuit type constants - FHE operations (4-11)
    const CIRCUIT_FHE_ADD: u8 = 4;
    const CIRCUIT_FHE_MULTIPLY: u8 = 5;
    const CIRCUIT_FHE_SUM: u8 = 6;
    const CIRCUIT_FHE_THRESHOLD: u8 = 7;
    const CIRCUIT_FHE_RANGE_CHECK: u8 = 8;
    const CIRCUIT_FHE_AVERAGE: u8 = 9;
    const CIRCUIT_FHE_COUNT_IF: u8 = 10;
    const CIRCUIT_FHE_HISTOGRAM: u8 = 11;

    /// FHE constants
    const MAX_FHE_PROVERS: u64 = 5;
    const MIN_PAYMENT: u64 = 1_000_000; // 0.01 APT

    /// Job account - represents a compute task
    struct Job has store, drop, copy {
        id: u64,
        creator: address,
        prover: address,                    // For ZK: single prover; For FHE: first prover
        status: u8,
        circuit_type: u8,
        witness_hash: vector<u8>,           // Hash of encrypted witness (32 bytes)
        witness_size: u32,
        proof_hash: vector<u8>,             // Hash of proof once submitted (32 bytes)
        price: u64,
        created_at: u64,
        timeout_at: u64,
        claimed_at: u64,
        completed_at: u64,
        has_fhe_consensus: bool,            // True if this is an FHE job
    }

    /// FHE Consensus Data - for multi-prover consensus
    struct FheConsensusData has store, drop, copy {
        job_id: u64,
        operation_type: u8,
        operation_param1: u16,              // Threshold, operand, count, etc.
        operation_param2: u8,               // Min, flags, etc.
        operation_param3: u8,               // Max, etc.
        required_provers: u8,
        consensus_threshold: u8,            // Minimum matching results
        submission_timeout: u64,
        claimed_provers: vector<address>,   // Dynamic vector
        result_hashes: vector<vector<u8>>,  // Dynamic vector of 32-byte hashes
        result_submitted: vector<bool>,
        results_count: u8,
        consensus_hash: vector<u8>,         // 32 bytes when consensus reached
        finalized: bool,
    }

    /// Global jobs registry
    struct JobsRegistry has key {
        jobs: Table<u64, Job>,
        fhe_consensus: Table<u64, FheConsensusData>, // job_id -> FheConsensusData
        escrows: Table<u64, Coin<AptosCoin>>,        // job_id -> escrow coin
        marketplace_addr: address,

        // Events
        job_created_events: EventHandle<JobCreatedEvent>,
        job_claimed_events: EventHandle<JobClaimedEvent>,
        job_completed_events: EventHandle<JobCompletedEvent>,
        fhe_result_submitted_events: EventHandle<FheResultSubmittedEvent>,
        fhe_consensus_events: EventHandle<FheConsensusEvent>,
    }

    /// Events
    struct JobCreatedEvent has drop, store {
        job_id: u64,
        creator: address,
        reward: u64,
        circuit_type: u8,
        is_fhe: bool,
    }

    struct JobClaimedEvent has drop, store {
        job_id: u64,
        prover: address,
        claimed_at: u64,
    }

    struct JobCompletedEvent has drop, store {
        job_id: u64,
        prover: address,
        is_valid: bool,
        completed_at: u64,
    }

    struct FheResultSubmittedEvent has drop, store {
        job_id: u64,
        prover: address,
        result_hash: vector<u8>,
        results_count: u8,
    }

    struct FheConsensusEvent has drop, store {
        job_id: u64,
        consensus_achieved: bool,
        consensus_hash: vector<u8>,
        matching_provers: u8,
    }

    /// Initialize jobs registry
    public entry fun initialize(account: &signer, marketplace_addr: address) {
        let account_addr = signer::address_of(account);
        assert!(!exists<JobsRegistry>(account_addr), E_ALREADY_INITIALIZED);

        move_to(account, JobsRegistry {
            jobs: table::new(),
            fhe_consensus: table::new(),
            escrows: table::new(),
            marketplace_addr,
            job_created_events: event::new_event_handle<JobCreatedEvent>(account),
            job_claimed_events: event::new_event_handle<JobClaimedEvent>(account),
            job_completed_events: event::new_event_handle<JobCompletedEvent>(account),
            fhe_result_submitted_events: event::new_event_handle<FheResultSubmittedEvent>(account),
            fhe_consensus_events: event::new_event_handle<FheConsensusEvent>(account),
        });
    }

    /// Submit a new ZK proof job
    public entry fun submit_zk_job(
        user: &signer,
        registry_addr: address,
        witness_hash: vector<u8>,
        witness_size: u32,
        payment: u64,
        circuit_type: u8,
        timeout_seconds: u64,
    ) acquires JobsRegistry {
        assert!(exists<JobsRegistry>(registry_addr), E_NOT_INITIALIZED);
        assert!(payment >= MIN_PAYMENT, E_INSUFFICIENT_PAYMENT);
        assert!(circuit_type <= CIRCUIT_CREDENTIAL, E_INVALID_CIRCUIT_TYPE);

        let user_addr = signer::address_of(user);
        let registry = borrow_global_mut<JobsRegistry>(registry_addr);

        let job_id = marketplace::next_job_id(registry.marketplace_addr);

        // Withdraw payment and add to escrow
        let payment_coin = coin::withdraw<AptosCoin>(user, payment);
        table::add(&mut registry.escrows, job_id, payment_coin);

        let now = timestamp::now_seconds();
        let timeout = if (timeout_seconds == 0) {
            marketplace::get_default_timeout(registry.marketplace_addr)
        } else {
            timeout_seconds
        };

        let job = Job {
            id: job_id,
            creator: user_addr,
            prover: @0x0,
            status: STATUS_PENDING,
            circuit_type,
            witness_hash,
            witness_size,
            proof_hash: vector::empty(),
            price: payment,
            created_at: now,
            timeout_at: now + timeout,
            claimed_at: 0,
            completed_at: 0,
            has_fhe_consensus: false,
        };

        table::add(&mut registry.jobs, job_id, job);

        event::emit_event(&mut registry.job_created_events, JobCreatedEvent {
            job_id,
            creator: user_addr,
            reward: payment,
            circuit_type,
            is_fhe: false,
        });
    }

    /// Submit a new FHE computation job
    public entry fun submit_fhe_job(
        user: &signer,
        registry_addr: address,
        witness_hash: vector<u8>,
        witness_size: u32,
        payment: u64,
        circuit_type: u8,
        timeout_seconds: u64,
        required_provers: u8,
        consensus_threshold: u8,
        operation_param1: u16,
        operation_param2: u8,
        operation_param3: u8,
    ) acquires JobsRegistry {
        assert!(exists<JobsRegistry>(registry_addr), E_NOT_INITIALIZED);
        assert!(payment >= MIN_PAYMENT, E_INSUFFICIENT_PAYMENT);
        assert!(circuit_type >= CIRCUIT_FHE_ADD && circuit_type <= CIRCUIT_FHE_HISTOGRAM, E_INVALID_CIRCUIT_TYPE);
        assert!((required_provers as u64) <= MAX_FHE_PROVERS, E_MAX_PROVERS_REACHED);

        let user_addr = signer::address_of(user);
        let registry = borrow_global_mut<JobsRegistry>(registry_addr);

        let job_id = marketplace::next_job_id(registry.marketplace_addr);

        // Withdraw payment and add to escrow
        let payment_coin = coin::withdraw<AptosCoin>(user, payment);
        table::add(&mut registry.escrows, job_id, payment_coin);

        let now = timestamp::now_seconds();
        let timeout = if (timeout_seconds == 0) {
            marketplace::get_default_timeout(registry.marketplace_addr)
        } else {
            timeout_seconds
        };

        let job = Job {
            id: job_id,
            creator: user_addr,
            prover: @0x0,
            status: STATUS_PENDING,
            circuit_type,
            witness_hash,
            witness_size,
            proof_hash: vector::empty(),
            price: payment,
            created_at: now,
            timeout_at: now + timeout,
            claimed_at: 0,
            completed_at: 0,
            has_fhe_consensus: true,
        };

        table::add(&mut registry.jobs, job_id, job);

        // Create FHE consensus data
        let fhe_data = FheConsensusData {
            job_id,
            operation_type: circuit_type,
            operation_param1,
            operation_param2,
            operation_param3,
            required_provers,
            consensus_threshold,
            submission_timeout: now + timeout,
            claimed_provers: vector::empty(),
            result_hashes: vector::empty(),
            result_submitted: vector::empty(),
            results_count: 0,
            consensus_hash: vector::empty(),
            finalized: false,
        };

        table::add(&mut registry.fhe_consensus, job_id, fhe_data);

        event::emit_event(&mut registry.job_created_events, JobCreatedEvent {
            job_id,
            creator: user_addr,
            reward: payment,
            circuit_type,
            is_fhe: true,
        });
    }

    /// Claim a ZK job
    public entry fun claim_zk_job(
        prover: &signer,
        registry_addr: address,
        job_id: u64,
    ) acquires JobsRegistry {
        assert!(exists<JobsRegistry>(registry_addr), E_NOT_INITIALIZED);

        let prover_addr = signer::address_of(prover);
        let registry = borrow_global_mut<JobsRegistry>(registry_addr);

        assert!(table::contains(&registry.jobs, job_id), E_JOB_NOT_FOUND);

        // Check prover can claim jobs
        assert!(
            marketplace::can_claim_jobs(registry.marketplace_addr, prover_addr),
            E_WORKER_NOT_REGISTERED
        );

        let job = table::borrow_mut(&mut registry.jobs, job_id);
        assert!(job.status == STATUS_PENDING, E_INVALID_STATUS);
        assert!(!job.has_fhe_consensus, E_NOT_FHE_JOB);

        let now = timestamp::now_seconds();
        assert!(now < job.timeout_at, E_JOB_EXPIRED);

        job.status = STATUS_CLAIMED;
        job.prover = prover_addr;
        job.claimed_at = now;

        event::emit_event(&mut registry.job_claimed_events, JobClaimedEvent {
            job_id,
            prover: prover_addr,
            claimed_at: now,
        });
    }

    /// Claim an FHE job (multiple provers can claim)
    public entry fun claim_fhe_job(
        prover: &signer,
        registry_addr: address,
        job_id: u64,
    ) acquires JobsRegistry {
        assert!(exists<JobsRegistry>(registry_addr), E_NOT_INITIALIZED);

        let prover_addr = signer::address_of(prover);
        let registry = borrow_global_mut<JobsRegistry>(registry_addr);

        assert!(table::contains(&registry.jobs, job_id), E_JOB_NOT_FOUND);
        assert!(table::contains(&registry.fhe_consensus, job_id), E_NOT_FHE_JOB);

        // Check prover can claim jobs
        assert!(
            marketplace::can_claim_jobs(registry.marketplace_addr, prover_addr),
            E_WORKER_NOT_REGISTERED
        );

        let job = table::borrow_mut(&mut registry.jobs, job_id);
        assert!(job.has_fhe_consensus, E_NOT_FHE_JOB);

        let now = timestamp::now_seconds();
        assert!(now < job.timeout_at, E_JOB_EXPIRED);

        let fhe_data = table::borrow_mut(&mut registry.fhe_consensus, job_id);
        assert!(vector::length(&fhe_data.claimed_provers) < (fhe_data.required_provers as u64), E_MAX_PROVERS_REACHED);

        // Check if prover already claimed
        let i = 0;
        let len = vector::length(&fhe_data.claimed_provers);
        while (i < len) {
            assert!(*vector::borrow(&fhe_data.claimed_provers, i) != prover_addr, E_PROVER_ALREADY_CLAIMED);
            i = i + 1;
        };

        // Add prover to claimed list
        vector::push_back(&mut fhe_data.claimed_provers, prover_addr);
        vector::push_back(&mut fhe_data.result_submitted, false);

        // If first prover, update job status
        if (vector::length(&fhe_data.claimed_provers) == 1) {
            job.status = STATUS_CLAIMED;
            job.prover = prover_addr;
            job.claimed_at = now;
        };

        event::emit_event(&mut registry.job_claimed_events, JobClaimedEvent {
            job_id,
            prover: prover_addr,
            claimed_at: now,
        });
    }

    /// Submit ZK proof result
    public entry fun submit_zk_proof(
        prover: &signer,
        registry_addr: address,
        job_id: u64,
        proof_bytes: vector<u8>,
        public_inputs: vector<u8>,
    ) acquires JobsRegistry {
        assert!(exists<JobsRegistry>(registry_addr), E_NOT_INITIALIZED);

        let prover_addr = signer::address_of(prover);
        let registry = borrow_global_mut<JobsRegistry>(registry_addr);

        assert!(table::contains(&registry.jobs, job_id), E_JOB_NOT_FOUND);

        let job = table::borrow_mut(&mut registry.jobs, job_id);
        assert!(job.status == STATUS_CLAIMED, E_INVALID_STATUS);
        assert!(job.prover == prover_addr, E_NOT_ASSIGNED);
        assert!(!job.has_fhe_consensus, E_NOT_FHE_JOB);

        let now = timestamp::now_seconds();
        assert!(now < job.timeout_at, E_JOB_EXPIRED);

        // Verify ZK proof
        let is_valid = zk_verifier::verify_groth16(
            job.circuit_type,
            proof_bytes,
            public_inputs,
        );

        assert!(is_valid, E_INVALID_PROOF);

        // Mark job as completed
        job.status = STATUS_COMPLETED;
        job.proof_hash = public_inputs; // Store public inputs as proof hash
        job.completed_at = now;

        // Calculate payments
        let total_reward = job.price;
        let platform_fee = marketplace::calculate_platform_fee(registry.marketplace_addr, total_reward);
        let prover_payout = total_reward - platform_fee;

        // Withdraw from escrow
        let escrow = table::remove(&mut registry.escrows, job_id);
        let fee_coin = coin::extract(&mut escrow, platform_fee);
        let payout_coin = escrow;

        // Pay platform fee
        marketplace::collect_protocol_fee(registry.marketplace_addr, fee_coin);

        // Pay prover
        coin::deposit(prover_addr, payout_coin);

        // Update prover stats
        let completion_time = (now - job.claimed_at) as u32;
        marketplace::update_prover_on_success(
            registry.marketplace_addr,
            prover_addr,
            completion_time,
            prover_payout,
        );

        marketplace::increment_completed_jobs(registry.marketplace_addr);

        event::emit_event(&mut registry.job_completed_events, JobCompletedEvent {
            job_id,
            prover: prover_addr,
            is_valid: true,
            completed_at: now,
        });
    }

    /// Submit FHE computation result
    public entry fun submit_fhe_result(
        prover: &signer,
        registry_addr: address,
        job_id: u64,
        result_hash: vector<u8>,
    ) acquires JobsRegistry {
        assert!(exists<JobsRegistry>(registry_addr), E_NOT_INITIALIZED);

        let prover_addr = signer::address_of(prover);
        let registry = borrow_global_mut<JobsRegistry>(registry_addr);

        assert!(table::contains(&registry.jobs, job_id), E_JOB_NOT_FOUND);
        assert!(table::contains(&registry.fhe_consensus, job_id), E_NOT_FHE_JOB);

        let job = table::borrow(&registry.jobs, job_id);
        assert!(job.has_fhe_consensus, E_NOT_FHE_JOB);

        let now = timestamp::now_seconds();
        assert!(now < job.timeout_at, E_JOB_EXPIRED);

        let fhe_data = table::borrow_mut(&mut registry.fhe_consensus, job_id);

        // Find prover index
        let prover_idx = 0;
        let found = false;
        let len = vector::length(&fhe_data.claimed_provers);
        while (prover_idx < len) {
            if (*vector::borrow(&fhe_data.claimed_provers, prover_idx) == prover_addr) {
                found = true;
                break
            };
            prover_idx = prover_idx + 1;
        };

        assert!(found, E_NOT_ASSIGNED);
        assert!(!*vector::borrow(&fhe_data.result_submitted, prover_idx), E_ALREADY_FINALIZED);

        // Add result
        vector::push_back(&mut fhe_data.result_hashes, result_hash);
        *vector::borrow_mut(&mut fhe_data.result_submitted, prover_idx) = true;
        fhe_data.results_count = fhe_data.results_count + 1;

        event::emit_event(&mut registry.fhe_result_submitted_events, FheResultSubmittedEvent {
            job_id,
            prover: prover_addr,
            result_hash,
            results_count: fhe_data.results_count,
        });
    }

    /// Finalize FHE job after all results submitted
    public entry fun finalize_fhe_job(
        _caller: &signer, // Anyone can finalize
        registry_addr: address,
        job_id: u64,
    ) acquires JobsRegistry {
        assert!(exists<JobsRegistry>(registry_addr), E_NOT_INITIALIZED);

        let registry = borrow_global_mut<JobsRegistry>(registry_addr);

        assert!(table::contains(&registry.jobs, job_id), E_JOB_NOT_FOUND);
        assert!(table::contains(&registry.fhe_consensus, job_id), E_NOT_FHE_JOB);

        let job = table::borrow_mut(&mut registry.jobs, job_id);
        let fhe_data = table::borrow_mut(&mut registry.fhe_consensus, job_id);

        assert!(!fhe_data.finalized, E_ALREADY_FINALIZED);
        assert!(fhe_data.results_count >= fhe_data.required_provers, E_INSUFFICIENT_FHE_RESULTS);

        // Check consensus
        let consensus_result = check_consensus(fhe_data);

        if (vector::length(&consensus_result) > 0) {
            // Consensus achieved
            job.status = STATUS_COMPLETED;
            job.proof_hash = consensus_result;
            job.completed_at = timestamp::now_seconds();
            fhe_data.consensus_hash = consensus_result;
            fhe_data.finalized = true;

            // Count matching provers
            let matching_count = 0u8;
            let i = 0;
            let len = vector::length(&fhe_data.result_hashes);
            while (i < len) {
                if (*vector::borrow(&fhe_data.result_hashes, i) == consensus_result) {
                    matching_count = matching_count + 1;
                };
                i = i + 1;
            };

            // Distribute payments to matching provers
            let total_reward = job.price;
            let platform_fee = marketplace::calculate_platform_fee(registry.marketplace_addr, total_reward);
            let prover_payout_total = total_reward - platform_fee;
            let payout_per_prover = if (matching_count > 0) {
                prover_payout_total / (matching_count as u64)
            } else {
                0
            };

            // Withdraw from escrow
            let escrow = table::remove(&mut registry.escrows, job_id);
            let fee_coin = coin::extract(&mut escrow, platform_fee);
            marketplace::collect_protocol_fee(registry.marketplace_addr, fee_coin);

            // Pay matching provers and penalize mismatching
            let i = 0;
            let len = vector::length(&fhe_data.claimed_provers);
            while (i < len) {
                let prover_addr = *vector::borrow(&fhe_data.claimed_provers, i);
                let result_hash = vector::borrow(&fhe_data.result_hashes, i);

                if (*result_hash == consensus_result) {
                    // Pay matching prover
                    let payout = coin::extract(&mut escrow, payout_per_prover);
                    coin::deposit(prover_addr, payout);

                    marketplace::update_prover_on_success(
                        registry.marketplace_addr,
                        prover_addr,
                        60, // Placeholder completion time
                        payout_per_prover,
                    );
                } else {
                    // Penalize mismatching prover
                    marketplace::update_prover_on_failure(registry.marketplace_addr, prover_addr);
                };

                i = i + 1;
            };

            // Return any remaining escrow to creator (rounding errors)
            if (coin::value(&escrow) > 0) {
                coin::deposit(job.creator, escrow);
            } else {
                coin::destroy_zero(escrow);
            };

            marketplace::increment_completed_jobs(registry.marketplace_addr);

            event::emit_event(&mut registry.fhe_consensus_events, FheConsensusEvent {
                job_id,
                consensus_achieved: true,
                consensus_hash: consensus_result,
                matching_provers: matching_count,
            });
        } else {
            // No consensus - refund creator and penalize all provers
            job.status = STATUS_FAILED;
            fhe_data.finalized = true;

            let escrow = table::remove(&mut registry.escrows, job_id);
            coin::deposit(job.creator, escrow);

            // Penalize all provers
            let i = 0;
            let len = vector::length(&fhe_data.claimed_provers);
            while (i < len) {
                let prover_addr = *vector::borrow(&fhe_data.claimed_provers, i);
                marketplace::update_prover_on_failure(registry.marketplace_addr, prover_addr);
                i = i + 1;
            };

            event::emit_event(&mut registry.fhe_consensus_events, FheConsensusEvent {
                job_id,
                consensus_achieved: false,
                consensus_hash: vector::empty(),
                matching_provers: 0,
            });
        };
    }

    /// Cancel a pending job (creator only)
    public entry fun cancel_job(
        creator: &signer,
        registry_addr: address,
        job_id: u64,
    ) acquires JobsRegistry {
        assert!(exists<JobsRegistry>(registry_addr), E_NOT_INITIALIZED);

        let creator_addr = signer::address_of(creator);
        let registry = borrow_global_mut<JobsRegistry>(registry_addr);

        assert!(table::contains(&registry.jobs, job_id), E_JOB_NOT_FOUND);

        let job = table::borrow_mut(&mut registry.jobs, job_id);
        assert!(job.creator == creator_addr, E_NOT_CREATOR);
        assert!(job.status == STATUS_PENDING, E_INVALID_STATUS);

        job.status = STATUS_CANCELLED;

        // Refund payment
        let escrow = table::remove(&mut registry.escrows, job_id);
        coin::deposit(creator_addr, escrow);
    }

    /// Helper: Check FHE consensus
    fun check_consensus(fhe_data: &FheConsensusData): vector<u8> {
        if (fhe_data.results_count < fhe_data.consensus_threshold) {
            return vector::empty()
        };

        let best_hash = vector::empty<u8>();
        let best_count = 0u8;

        let i = 0;
        let len = vector::length(&fhe_data.result_hashes);
        while (i < len) {
            let hash = vector::borrow(&fhe_data.result_hashes, i);
            let count = 0u8;

            // Count matching hashes
            let j = 0;
            while (j < len) {
                if (vector::borrow(&fhe_data.result_hashes, j) == hash) {
                    count = count + 1;
                };
                j = j + 1;
            };

            if (count > best_count) {
                best_count = count;
                best_hash = *hash;
            };

            i = i + 1;
        };

        if (best_count >= fhe_data.consensus_threshold) {
            best_hash
        } else {
            vector::empty()
        }
    }

    /// View functions
    #[view]
    public fun get_job(registry_addr: address, job_id: u64): Job acquires JobsRegistry {
        assert!(exists<JobsRegistry>(registry_addr), E_NOT_INITIALIZED);
        let registry = borrow_global<JobsRegistry>(registry_addr);
        assert!(table::contains(&registry.jobs, job_id), E_JOB_NOT_FOUND);
        *table::borrow(&registry.jobs, job_id)
    }

    #[view]
    public fun get_fhe_consensus(registry_addr: address, job_id: u64): FheConsensusData acquires JobsRegistry {
        assert!(exists<JobsRegistry>(registry_addr), E_NOT_INITIALIZED);
        let registry = borrow_global<JobsRegistry>(registry_addr);
        assert!(table::contains(&registry.fhe_consensus, job_id), E_NOT_FHE_JOB);
        *table::borrow(&registry.fhe_consensus, job_id)
    }

    #[view]
    public fun job_exists(registry_addr: address, job_id: u64): bool acquires JobsRegistry {
        if (!exists<JobsRegistry>(registry_addr)) {
            return false
        };
        let registry = borrow_global<JobsRegistry>(registry_addr);
        table::contains(&registry.jobs, job_id)
    }

    #[view]
    public fun is_fhe_job(registry_addr: address, job_id: u64): bool acquires JobsRegistry {
        let job = get_job(registry_addr, job_id);
        job.has_fhe_consensus
    }
}
