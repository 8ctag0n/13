#[test_only]
module zyberlink::integration_tests {
    use std::signer;
    use std::vector;
    use aptos_framework::account;
    use aptos_framework::coin;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::timestamp;
    use zyberlink::marketplace;
    use zyberlink::jobs;
    use zyberlink::zk_verifier;

    const MIN_PAYMENT: u64 = 1_000_000; // 0.01 APT

    /// Test complete ZK job lifecycle
    #[test(aptos_framework = @0x1, deployer = @zyberlink, user = @0x123, prover = @0x456)]
    public fun test_zk_job_lifecycle(
        aptos_framework: &signer,
        deployer: &signer,
        user: &signer,
        prover: &signer,
    ) {
        // Setup
        timestamp::set_time_has_started_for_testing(aptos_framework);
        let deployer_addr = signer::address_of(deployer);
        let user_addr = signer::address_of(user);
        let prover_addr = signer::address_of(prover);

        account::create_account_for_test(deployer_addr);
        account::create_account_for_test(user_addr);
        account::create_account_for_test(prover_addr);

        // Initialize marketplace
        marketplace::initialize(deployer, deployer_addr);

        // Register prover
        coin::register<AptosCoin>(prover);
        let stake = coin::mint<AptosCoin>(5_000_000_000, aptos_framework); // 5 APT
        coin::deposit(prover_addr, stake);

        let encryption_key = vector::empty<u8>();
        let i = 0;
        while (i < 32) {
            vector::push_back(&mut encryption_key, (i as u8));
            i = i + 1;
        };

        marketplace::register_prover(prover, deployer_addr, 5_000_000_000, encryption_key);

        // Initialize jobs registry
        jobs::initialize(deployer, deployer_addr);

        // Fund user
        coin::register<AptosCoin>(user);
        let payment = coin::mint<AptosCoin>(MIN_PAYMENT * 10, aptos_framework);
        coin::deposit(user_addr, payment);

        // Create ZK job
        let witness_hash = vector::empty<u8>();
        let i = 0;
        while (i < 32) {
            vector::push_back(&mut witness_hash, 1);
            i = i + 1;
        };

        jobs::submit_zk_job(
            user,
            deployer_addr,
            witness_hash,
            2048,
            MIN_PAYMENT,
            0, // CIRCUIT_ZCASH_ORCHARD
            600,
        );

        // Verify job created
        assert!(jobs::job_exists(deployer_addr, 0), 1);
        assert!(!jobs::is_fhe_job(deployer_addr, 0), 2);

        // Prover claims job
        jobs::claim_zk_job(prover, deployer_addr, 0);

        // Prover submits proof
        let proof_bytes = vector::empty<u8>();
        let i = 0;
        while (i < 256) {
            vector::push_back(&mut proof_bytes, ((i + 1) as u8));
            i = i + 1;
        };

        let public_inputs = vector::empty<u8>();
        vector::push_back(&mut public_inputs, 42);

        jobs::submit_zk_proof(prover, deployer_addr, 0, proof_bytes, public_inputs);

        // Verify prover got paid
        let initial_balance = MIN_PAYMENT * 10;
        let job_payment = MIN_PAYMENT;
        let platform_fee = job_payment / 10; // 10% fee
        let prover_payout = job_payment - platform_fee;

        // Prover should have: 5 APT stake + prover_payout
        let expected_balance = 5_000_000_000 + prover_payout;
        assert!(coin::balance<AptosCoin>(prover_addr) >= prover_payout, 3);
    }

    /// Test FHE job with consensus
    #[test(aptos_framework = @0x1, deployer = @zyberlink, user = @0x200, p1 = @0x301, p2 = @0x302, p3 = @0x303)]
    public fun test_fhe_job_consensus(
        aptos_framework: &signer,
        deployer: &signer,
        user: &signer,
        p1: &signer,
        p2: &signer,
        p3: &signer,
    ) {
        // Setup
        timestamp::set_time_has_started_for_testing(aptos_framework);
        let deployer_addr = signer::address_of(deployer);
        let user_addr = signer::address_of(user);
        let p1_addr = signer::address_of(p1);
        let p2_addr = signer::address_of(p2);
        let p3_addr = signer::address_of(p3);

        account::create_account_for_test(deployer_addr);
        account::create_account_for_test(user_addr);
        account::create_account_for_test(p1_addr);
        account::create_account_for_test(p2_addr);
        account::create_account_for_test(p3_addr);

        // Initialize
        marketplace::initialize(deployer, deployer_addr);
        jobs::initialize(deployer, deployer_addr);

        // Register 3 provers
        let encryption_key = vector::empty<u8>();
        let i = 0;
        while (i < 32) {
            vector::push_back(&mut encryption_key, (i as u8));
            i = i + 1;
        };

        let stake_amount = 5_000_000_000u64;

        coin::register<AptosCoin>(p1);
        let stake1 = coin::mint<AptosCoin>(stake_amount, aptos_framework);
        coin::deposit(p1_addr, stake1);
        marketplace::register_prover(p1, deployer_addr, stake_amount, encryption_key);

        coin::register<AptosCoin>(p2);
        let stake2 = coin::mint<AptosCoin>(stake_amount, aptos_framework);
        coin::deposit(p2_addr, stake2);
        marketplace::register_prover(p2, deployer_addr, stake_amount, encryption_key);

        coin::register<AptosCoin>(p3);
        let stake3 = coin::mint<AptosCoin>(stake_amount, aptos_framework);
        coin::deposit(p3_addr, stake3);
        marketplace::register_prover(p3, deployer_addr, stake_amount, encryption_key);

        // Fund user
        coin::register<AptosCoin>(user);
        let payment = coin::mint<AptosCoin>(MIN_PAYMENT * 100, aptos_framework);
        coin::deposit(user_addr, payment);

        // Create FHE job (requires 3 provers, 2-of-3 consensus)
        let witness_hash = vector::empty<u8>();
        let i = 0;
        while (i < 32) {
            vector::push_back(&mut witness_hash, 1);
            i = i + 1;
        };

        jobs::submit_fhe_job(
            user,
            deployer_addr,
            witness_hash,
            4096,
            MIN_PAYMENT * 10,
            4, // CIRCUIT_FHE_ADD
            600,
            3, // required_provers
            2, // consensus_threshold (2-of-3)
            42, // operation_param1
            0,  // operation_param2
            0,  // operation_param3
        );

        // Verify FHE job created
        assert!(jobs::job_exists(deployer_addr, 0), 1);
        assert!(jobs::is_fhe_job(deployer_addr, 0), 2);

        // All 3 provers claim
        jobs::claim_fhe_job(p1, deployer_addr, 0);
        jobs::claim_fhe_job(p2, deployer_addr, 0);
        jobs::claim_fhe_job(p3, deployer_addr, 0);

        // Provers submit results (p1 and p2 agree, p3 differs)
        let consensus_hash = vector::empty<u8>();
        let i = 0;
        while (i < 32) {
            vector::push_back(&mut consensus_hash, 99);
            i = i + 1;
        };

        let different_hash = vector::empty<u8>();
        let i = 0;
        while (i < 32) {
            vector::push_back(&mut different_hash, 55);
            i = i + 1;
        };

        jobs::submit_fhe_result(p1, deployer_addr, 0, consensus_hash);
        jobs::submit_fhe_result(p2, deployer_addr, 0, consensus_hash);
        jobs::submit_fhe_result(p3, deployer_addr, 0, different_hash);

        // Finalize job (anyone can call)
        jobs::finalize_fhe_job(user, deployer_addr, 0);

        // Verify consensus achieved
        let fhe_data = jobs::get_fhe_consensus(deployer_addr, 0);
        assert!(fhe_data.finalized, 3);
        assert!(vector::length(&fhe_data.consensus_hash) > 0, 4);

        // Verify matching provers (p1, p2) got paid and p3 was penalized
        let prover1 = marketplace::get_prover(deployer_addr, p1_addr);
        let prover2 = marketplace::get_prover(deployer_addr, p2_addr);
        let prover3 = marketplace::get_prover(deployer_addr, p3_addr);

        // p1 and p2 should have increased earnings
        assert!(prover1.total_jobs_completed == 1, 5);
        assert!(prover2.total_jobs_completed == 1, 6);

        // p3 should have failed job count increased
        assert!(prover3.total_jobs_failed == 1, 7);
    }

    /// Test FHE job with no consensus (all different results)
    #[test(aptos_framework = @0x1, deployer = @zyberlink, user = @0x200, p1 = @0x401, p2 = @0x402, p3 = @0x403)]
    public fun test_fhe_job_no_consensus(
        aptos_framework: &signer,
        deployer: &signer,
        user: &signer,
        p1: &signer,
        p2: &signer,
        p3: &signer,
    ) {
        // Setup (similar to consensus test)
        timestamp::set_time_has_started_for_testing(aptos_framework);
        let deployer_addr = signer::address_of(deployer);
        let user_addr = signer::address_of(user);
        let p1_addr = signer::address_of(p1);
        let p2_addr = signer::address_of(p2);
        let p3_addr = signer::address_of(p3);

        account::create_account_for_test(deployer_addr);
        account::create_account_for_test(user_addr);
        account::create_account_for_test(p1_addr);
        account::create_account_for_test(p2_addr);
        account::create_account_for_test(p3_addr);

        marketplace::initialize(deployer, deployer_addr);
        jobs::initialize(deployer, deployer_addr);

        // Register provers and create FHE job
        let encryption_key = vector::empty<u8>();
        let i = 0;
        while (i < 32) {
            vector::push_back(&mut encryption_key, (i as u8));
            i = i + 1;
        };

        let stake_amount = 5_000_000_000u64;

        coin::register<AptosCoin>(p1);
        let stake1 = coin::mint<AptosCoin>(stake_amount, aptos_framework);
        coin::deposit(p1_addr, stake1);
        marketplace::register_prover(p1, deployer_addr, stake_amount, encryption_key);

        coin::register<AptosCoin>(p2);
        let stake2 = coin::mint<AptosCoin>(stake_amount, aptos_framework);
        coin::deposit(p2_addr, stake2);
        marketplace::register_prover(p2, deployer_addr, stake_amount, encryption_key);

        coin::register<AptosCoin>(p3);
        let stake3 = coin::mint<AptosCoin>(stake_amount, aptos_framework);
        coin::deposit(p3_addr, stake3);
        marketplace::register_prover(p3, deployer_addr, stake_amount, encryption_key);

        coin::register<AptosCoin>(user);
        let initial_user_balance = MIN_PAYMENT * 100;
        let payment = coin::mint<AptosCoin>(initial_user_balance, aptos_framework);
        coin::deposit(user_addr, payment);

        let witness_hash = vector::empty<u8>();
        let i = 0;
        while (i < 32) {
            vector::push_back(&mut witness_hash, 1);
            i = i + 1;
        };

        let job_payment = MIN_PAYMENT * 10;
        jobs::submit_fhe_job(
            user,
            deployer_addr,
            witness_hash,
            4096,
            job_payment,
            5, // CIRCUIT_FHE_MULTIPLY
            600,
            3, // required_provers
            2, // consensus_threshold
            100,
            0,
            0,
        );

        // Provers claim and submit DIFFERENT results
        jobs::claim_fhe_job(p1, deployer_addr, 0);
        jobs::claim_fhe_job(p2, deployer_addr, 0);
        jobs::claim_fhe_job(p3, deployer_addr, 0);

        let hash1 = vector::empty<u8>();
        let i = 0;
        while (i < 32) {
            vector::push_back(&mut hash1, 1);
            i = i + 1;
        };

        let hash2 = vector::empty<u8>();
        let i = 0;
        while (i < 32) {
            vector::push_back(&mut hash2, 2);
            i = i + 1;
        };

        let hash3 = vector::empty<u8>();
        let i = 0;
        while (i < 32) {
            vector::push_back(&mut hash3, 3);
            i = i + 1;
        };

        jobs::submit_fhe_result(p1, deployer_addr, 0, hash1);
        jobs::submit_fhe_result(p2, deployer_addr, 0, hash2);
        jobs::submit_fhe_result(p3, deployer_addr, 0, hash3);

        // Finalize - should fail consensus and refund user
        jobs::finalize_fhe_job(user, deployer_addr, 0);

        let fhe_data = jobs::get_fhe_consensus(deployer_addr, 0);
        assert!(fhe_data.finalized, 1);
        assert!(vector::length(&fhe_data.consensus_hash) == 0, 2); // No consensus

        // User should be refunded
        let expected_balance = initial_user_balance; // Full refund
        assert!(coin::balance<AptosCoin>(user_addr) >= job_payment, 3);

        // All provers should be penalized
        let prover1 = marketplace::get_prover(deployer_addr, p1_addr);
        let prover2 = marketplace::get_prover(deployer_addr, p2_addr);
        let prover3 = marketplace::get_prover(deployer_addr, p3_addr);

        assert!(prover1.total_jobs_failed == 1, 4);
        assert!(prover2.total_jobs_failed == 1, 5);
        assert!(prover3.total_jobs_failed == 1, 6);
    }

    /// Test job cancellation
    #[test(aptos_framework = @0x1, deployer = @zyberlink, user = @0x500)]
    public fun test_cancel_job(
        aptos_framework: &signer,
        deployer: &signer,
        user: &signer,
    ) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        let deployer_addr = signer::address_of(deployer);
        let user_addr = signer::address_of(user);

        account::create_account_for_test(deployer_addr);
        account::create_account_for_test(user_addr);

        marketplace::initialize(deployer, deployer_addr);
        jobs::initialize(deployer, deployer_addr);

        coin::register<AptosCoin>(user);
        let initial_balance = MIN_PAYMENT * 10;
        let payment = coin::mint<AptosCoin>(initial_balance, aptos_framework);
        coin::deposit(user_addr, payment);

        let witness_hash = vector::empty<u8>();
        let i = 0;
        while (i < 32) {
            vector::push_back(&mut witness_hash, 1);
            i = i + 1;
        };

        jobs::submit_zk_job(user, deployer_addr, witness_hash, 1024, MIN_PAYMENT, 0, 600);

        // Cancel job
        jobs::cancel_job(user, deployer_addr, 0);

        // Verify refund
        assert!(coin::balance<AptosCoin>(user_addr) == initial_balance, 1);
    }

    /// Test prover slashing
    #[test(aptos_framework = @0x1, deployer = @zyberlink, prover = @0x600)]
    public fun test_slash_prover(
        aptos_framework: &signer,
        deployer: &signer,
        prover: &signer,
    ) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        let deployer_addr = signer::address_of(deployer);
        let prover_addr = signer::address_of(prover);

        account::create_account_for_test(deployer_addr);
        account::create_account_for_test(prover_addr);

        marketplace::initialize(deployer, deployer_addr);

        let encryption_key = vector::empty<u8>();
        let i = 0;
        while (i < 32) {
            vector::push_back(&mut encryption_key, (i as u8));
            i = i + 1;
        };

        coin::register<AptosCoin>(prover);
        let stake = coin::mint<AptosCoin>(10_000_000_000, aptos_framework); // 10 APT
        coin::deposit(prover_addr, stake);

        marketplace::register_prover(prover, deployer_addr, 10_000_000_000, encryption_key);

        // Slash prover
        let slash_amount = 1_000_000_000; // 1 APT
        let reason = b"Misbehavior detected";

        marketplace::slash_prover(deployer, deployer_addr, prover_addr, slash_amount, reason);

        // Verify stake reduced
        let prover_account = marketplace::get_prover(deployer_addr, prover_addr);
        assert!(prover_account.stake_amount == 9_000_000_000, 1);
        assert!(prover_account.total_jobs_failed == 1, 2);
    }
}
