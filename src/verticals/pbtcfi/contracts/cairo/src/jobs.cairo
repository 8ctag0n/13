use starknet::ContractAddress;

// ============================================================================
// JobType Enum - Generic job types for FHE operations
// ============================================================================
#[derive(Drop, Serde, starknet::Store, PartialEq, Copy)]
pub enum JobType {
    LoanVerification,    // pBTCFi: verify encrypted BTC collateral
    BalanceUpdate,       // pLST: update encrypted token balance
    StakeProof,          // pLST: prove staking position
    TransferProof,       // Generic: prove valid transfer
    LiquidationCheck,    // pBTCFi: check liquidation threshold
}

// ============================================================================
// Generic Job Structure
// ============================================================================
#[derive(Drop, Serde, starknet::Store)]
pub struct Job {
    pub job_id: u256,
    pub job_type: JobType,
    pub creator: ContractAddress,
    pub payload_hash: felt252,        // Hash of job-specific data
    pub encrypted_c1: felt252,        // FHE ciphertext component 1
    pub encrypted_c2: felt252,        // FHE ciphertext component 2
    pub reward: u256,                 // Payment for completing job
    pub status: JobStatus,
    pub created_at: u64,
}

// ============================================================================
// Generic FHE Jobs Interface
// ============================================================================
#[starknet::interface]
pub trait IFheJobs<TContractState> {
    // Job creation (generic)
    fn create_job(
        ref self: TContractState,
        job_type: JobType,
        payload_hash: felt252,
        encrypted_c1: felt252,
        encrypted_c2: felt252,
        reward: u256,
    ) -> u256;

    fn get_job(self: @TContractState, job_id: u256) -> Job;

    // Prover lifecycle
    fn register_prover(ref self: TContractState);
    fn get_prover(self: @TContractState, prover: ContractAddress) -> Prover;
    fn is_prover_registered(self: @TContractState, prover: ContractAddress) -> bool;

    // Job operations
    fn claim_job(ref self: TContractState, job_id: u256);
    fn submit_result(ref self: TContractState, job_id: u256, result_hash: felt252);
    fn get_job_execution(self: @TContractState, job_id: u256) -> JobExecution;

    // Queries
    fn get_pending_jobs(self: @TContractState) -> Array<u256>;
    fn get_pending_jobs_by_type(self: @TContractState, job_type: JobType) -> Array<u256>;
    fn get_jobs_by_creator(self: @TContractState, creator: ContractAddress) -> Array<u256>;

    // Sprint 3: Multi-prover consensus
    fn enable_consensus(ref self: TContractState, job_id: u256, required_provers: u8, consensus_threshold: u8);
    fn get_consensus_data(self: @TContractState, job_id: u256) -> FheConsensusData;
    fn is_consensus_enabled(self: @TContractState, job_id: u256) -> bool;
}

// ============================================================================
// pBTCFi-specific Interface (loan operations only)
// For prover lifecycle and job operations, use IFheJobs interface
// ============================================================================
#[starknet::interface]
pub trait ILoanOperations<TContractState> {
    // Loan creation and queries
    fn create_loan(
        ref self: TContractState,
        borrower: ContractAddress,
        btc_commitment: felt252,
        btc_encrypted_c1: felt252,
        btc_encrypted_c2: felt252,
    ) -> u256;

    fn get_loan(self: @TContractState, loan_id: u256) -> Loan;
    fn get_loan_job_id(self: @TContractState, loan_id: u256) -> u256;
    fn get_pending_loan_jobs(self: @TContractState) -> Array<u256>;

    // Loan-specific job operations (uses job_id internally)
    fn claim_loan_job(ref self: TContractState, loan_id: u256);
    fn submit_loan_result(ref self: TContractState, loan_id: u256, result_hash: felt252);
    fn get_loan_job_execution(self: @TContractState, loan_id: u256) -> JobExecution;
}

#[derive(Drop, Serde, starknet::Store)]
pub struct Loan {
    pub borrower: ContractAddress,
    pub btc_commitment: felt252,
    pub btc_encrypted_c1: felt252,
    pub btc_encrypted_c2: felt252,
    pub status: LoanStatus,
    pub created_at: u64,
}

#[derive(Drop, Serde, starknet::Store, PartialEq)]
pub enum LoanStatus {
    Pending,
    Active,
    Repaid,
    Liquidated,
}

#[derive(Drop, Serde, starknet::Store, PartialEq)]
pub enum JobStatus {
    Pending,
    Claimed,
    Completed,
}

/// Prover struct with multi-prover consensus support
///
/// # Sprint 1: Foundation
/// Added fields for tracking prover reputation and performance:
/// - stake: Amount staked by prover (for slashing on misbehavior)
/// - encryption_pubkey: Public key for encrypted job data
/// - jobs_completed/failed: Performance metrics
/// - total_earnings: Accumulated rewards
/// - is_active: Whether prover is currently accepting jobs
///
/// # Future (Sprint 2+):
/// These fields enable multi-prover consensus and reputation system
#[derive(Drop, Serde, starknet::Store)]
pub struct Prover {
    pub authority: ContractAddress,
    pub stake: u256,
    pub encryption_pubkey: felt252,
    pub jobs_completed: u64,
    pub jobs_failed: u64,
    pub total_earnings: u256,
    pub is_active: bool,
    pub registered_at: u64,
}

#[derive(Drop, Serde, starknet::Store)]
pub struct JobExecution {
    pub job_id: u256,
    pub prover: ContractAddress,
    pub status: JobStatus,
    pub result_hash: felt252,
    pub claimed_at: u64,
    pub completed_at: u64,
}

/// Sprint 3: Multi-Prover Consensus Data
/// Tracks consensus requirements and prover submissions for a job
#[derive(Drop, Copy, Serde, starknet::Store)]
pub struct FheConsensusData {
    pub job_id: u256,
    pub required_provers: u8,       // Total provers needed (e.g., 3)
    pub consensus_threshold: u8,    // Minimum provers that must agree (e.g., 2 for 2/3)
    pub claimed_count: u8,          // How many provers have claimed
    pub submitted_count: u8,        // How many results have been submitted
    pub consensus_reached: bool,    // Whether consensus has been achieved
    pub consensus_hash: felt252,    // The agreed-upon result hash
}

/// Sprint 3: Individual prover submission tracking
#[derive(Drop, Copy, Serde, starknet::Store)]
pub struct ProverSubmission {
    pub prover: ContractAddress,
    pub result_hash: felt252,
    pub submitted_at: u64,
    pub has_claimed: bool,
    pub has_submitted: bool,
}

// Mapping from loan_id to job_id (for backwards compatibility)
// This allows pBTCFi code to use loan_id while internally using job_id

#[starknet::contract]
pub mod PbtcfiJobs {
    use super::{Loan, LoanStatus, JobStatus, Prover, JobExecution, Job, JobType, FheConsensusData, ProverSubmission};
    use starknet::{ContractAddress, get_block_timestamp, get_caller_address};
    use starknet::storage::{Map, StoragePathEntry, StoragePointerReadAccess, StoragePointerWriteAccess};

    #[storage]
    struct Storage {
        // Generic job storage
        jobs: Map<u256, Job>,
        next_job_id: u256,
        job_executions: Map<u256, JobExecution>,
        registered_provers: Map<ContractAddress, Prover>,

        // pBTCFi-specific storage
        loans: Map<u256, Loan>,
        next_loan_id: u256,
        loan_to_job: Map<u256, u256>,  // loan_id -> job_id mapping

        // Creator tracking
        creator_jobs: Map<ContractAddress, Array<u256>>,
        creator_job_count: Map<ContractAddress, u256>,

        // Sprint 3: Multi-prover consensus storage
        fhe_consensus: Map<u256, FheConsensusData>,  // job_id -> consensus data
        job_prover_submissions: Map<(u256, ContractAddress), ProverSubmission>,  // (job_id, prover) -> submission
        job_result_counts: Map<(u256, felt252), u8>,  // (job_id, result_hash) -> count

        // Sprint 4: Prover tracking for reward splitting
        job_provers: Map<(u256, u8), ContractAddress>,  // (job_id, index) -> prover address
        job_prover_count: Map<u256, u8>,  // job_id -> count of provers who claimed
    }

    #[event]
    #[derive(Drop, starknet::Event)]
    pub enum Event {
        // Generic job events
        JobCreated: JobCreated,
        JobClaimed: JobClaimed,
        JobCompleted: JobCompleted,
        ProverRegistered: ProverRegistered,
        // pBTCFi-specific events
        LoanCreated: LoanCreated,
        // Sprint 3: Multi-prover consensus events
        ConsensusReached: ConsensusReached,
        ResultSubmitted: ResultSubmitted,
    }

    #[derive(Drop, starknet::Event)]
    pub struct JobCreated {
        #[key]
        pub job_id: u256,
        pub job_type: JobType,
        pub creator: ContractAddress,
        pub payload_hash: felt252,
        pub reward: u256,
        pub timestamp: u64,
    }

    #[derive(Drop, starknet::Event)]
    pub struct JobClaimed {
        #[key]
        pub job_id: u256,
        pub prover: ContractAddress,
        pub timestamp: u64,
    }

    #[derive(Drop, starknet::Event)]
    pub struct JobCompleted {
        #[key]
        pub job_id: u256,
        pub prover: ContractAddress,
        pub result_hash: felt252,
        pub timestamp: u64,
    }

    #[derive(Drop, starknet::Event)]
    pub struct ProverRegistered {
        #[key]
        pub prover: ContractAddress,
        pub timestamp: u64,
    }

    #[derive(Drop, starknet::Event)]
    pub struct LoanCreated {
        #[key]
        pub loan_id: u256,
        #[key]
        pub job_id: u256,
        pub borrower: ContractAddress,
        pub btc_commitment: felt252,
        pub timestamp: u64,
    }

    #[derive(Drop, starknet::Event)]
    pub struct ConsensusReached {
        #[key]
        pub job_id: u256,
        pub consensus_hash: felt252,
        pub agreeing_provers: u8,
        pub timestamp: u64,
    }

    #[derive(Drop, starknet::Event)]
    pub struct ResultSubmitted {
        #[key]
        pub job_id: u256,
        pub prover: ContractAddress,
        pub result_hash: felt252,
        pub submission_count: u8,
        pub timestamp: u64,
    }

    #[constructor]
    fn constructor(ref self: ContractState) {
        self.next_job_id.write(1);
        self.next_loan_id.write(1);
    }

    // ========================================================================
    // Generic FHE Jobs Implementation
    // ========================================================================
    #[abi(embed_v0)]
    impl FheJobsImpl of super::IFheJobs<ContractState> {
        fn create_job(
            ref self: ContractState,
            job_type: JobType,
            payload_hash: felt252,
            encrypted_c1: felt252,
            encrypted_c2: felt252,
            reward: u256,
        ) -> u256 {
            let caller = get_caller_address();
            let timestamp = get_block_timestamp();
            let job_id = self.next_job_id.read();

            let job = Job {
                job_id,
                job_type,
                creator: caller,
                payload_hash,
                encrypted_c1,
                encrypted_c2,
                reward,
                status: JobStatus::Pending,
                created_at: timestamp,
            };

            self.jobs.entry(job_id).write(job);
            self.next_job_id.write(job_id + 1);

            // Create initial job execution
            let zero_address: ContractAddress = starknet::contract_address_const::<0>();
            let job_execution = JobExecution {
                job_id,
                prover: zero_address,
                status: JobStatus::Pending,
                result_hash: 0,
                claimed_at: 0,
                completed_at: 0,
            };
            self.job_executions.entry(job_id).write(job_execution);

            // Track by creator
            let count = self.creator_job_count.entry(caller).read();
            self.creator_job_count.entry(caller).write(count + 1);

            self.emit(JobCreated {
                job_id,
                job_type,
                creator: caller,
                payload_hash,
                reward,
                timestamp,
            });

            job_id
        }

        fn get_job(self: @ContractState, job_id: u256) -> Job {
            self.jobs.entry(job_id).read()
        }

        fn register_prover(ref self: ContractState) {
            let caller = get_caller_address();
            let timestamp = get_block_timestamp();

            // Sprint 2: Validate prover is not already registered
            let existing = self.registered_provers.entry(caller).read();
            assert(existing.registered_at == 0, 'Prover already registered');

            // Initialize prover with default values
            let prover = Prover {
                authority: caller,
                stake: 0,  // Default: no stake required for MVP
                encryption_pubkey: 0,  // Default: no encryption for MVP
                jobs_completed: 0,
                jobs_failed: 0,
                total_earnings: 0,
                is_active: true,  // Active by default
                registered_at: timestamp,
            };

            self.registered_provers.entry(caller).write(prover);

            self.emit(ProverRegistered {
                prover: caller,
                timestamp,
            });
        }

        fn get_prover(self: @ContractState, prover: ContractAddress) -> Prover {
            self.registered_provers.entry(prover).read()
        }

        fn is_prover_registered(self: @ContractState, prover: ContractAddress) -> bool {
            let p = self.registered_provers.entry(prover).read();
            // Check if prover is registered and active
            p.registered_at > 0 && p.is_active
        }

        fn claim_job(ref self: ContractState, job_id: u256) {
            let caller = get_caller_address();
            let timestamp = get_block_timestamp();

            // Verify prover is registered
            let prover = self.registered_provers.entry(caller).read();
            assert(prover.registered_at > 0, 'Prover not registered');

            // Check if consensus is enabled for this job
            let mut consensus = self.fhe_consensus.entry(job_id).read();
            let consensus_enabled = consensus.required_provers > 0;

            if consensus_enabled {
                // Multi-prover mode: allow multiple claims up to required_provers
                assert(consensus.claimed_count < consensus.required_provers, 'All provers claimed');

                // Check if this prover already claimed
                let existing_submission = self.job_prover_submissions.entry((job_id, caller)).read();
                assert(!existing_submission.has_claimed, 'Already claimed');

                // Record prover claim
                let submission = ProverSubmission {
                    prover: caller,
                    result_hash: 0,
                    submitted_at: 0,
                    has_claimed: true,
                    has_submitted: false,
                };
                self.job_prover_submissions.entry((job_id, caller)).write(submission);

                // Sprint 4: Track prover in array for reward splitting
                let current_count = self.job_prover_count.entry(job_id).read();
                self.job_provers.entry((job_id, current_count)).write(caller);
                self.job_prover_count.entry(job_id).write(current_count + 1);

                // Increment claimed count
                consensus.claimed_count += 1;
                self.fhe_consensus.entry(job_id).write(consensus);

                // Only change job status to Claimed when all required provers have claimed
                if consensus.claimed_count == consensus.required_provers {
                    let mut job = self.jobs.entry(job_id).read();
                    job.status = JobStatus::Claimed;
                    self.jobs.entry(job_id).write(job);
                }
            } else {
                // Single-prover mode (original behavior)
                let mut job_exec = self.job_executions.entry(job_id).read();
                assert(job_exec.status == JobStatus::Pending, 'Job not available');

                // Update job execution
                job_exec.prover = caller;
                job_exec.status = JobStatus::Claimed;
                job_exec.claimed_at = timestamp;

                self.job_executions.entry(job_id).write(job_exec);

                // Update job status
                let mut job = self.jobs.entry(job_id).read();
                job.status = JobStatus::Claimed;
                self.jobs.entry(job_id).write(job);
            }

            self.emit(JobClaimed {
                job_id,
                prover: caller,
                timestamp,
            });
        }

        fn submit_result(ref self: ContractState, job_id: u256, result_hash: felt252) {
            let caller = get_caller_address();
            let timestamp = get_block_timestamp();

            // Check if consensus is enabled for this job
            let mut consensus = self.fhe_consensus.entry(job_id).read();
            let consensus_enabled = consensus.required_provers > 0;

            if consensus_enabled {
                // Multi-prover consensus mode
                assert(!consensus.consensus_reached, 'Consensus already reached');

                // Verify prover has claimed this job
                let mut submission = self.job_prover_submissions.entry((job_id, caller)).read();
                assert(submission.has_claimed, 'Job not claimed by prover');
                assert(!submission.has_submitted, 'Result already submitted');

                // Record submission
                submission.result_hash = result_hash;
                submission.submitted_at = timestamp;
                submission.has_submitted = true;
                self.job_prover_submissions.entry((job_id, caller)).write(submission);

                // Increment result count for this hash
                let current_count = self.job_result_counts.entry((job_id, result_hash)).read();
                let new_count = current_count + 1;
                self.job_result_counts.entry((job_id, result_hash)).write(new_count);

                // Increment submitted count
                consensus.submitted_count += 1;

                self.emit(ResultSubmitted {
                    job_id,
                    prover: caller,
                    result_hash,
                    submission_count: consensus.submitted_count,
                    timestamp,
                });

                // Check for consensus
                if new_count >= consensus.consensus_threshold {
                    // Consensus reached!
                    consensus.consensus_reached = true;
                    consensus.consensus_hash = result_hash;
                    self.fhe_consensus.entry(job_id).write(consensus);

                    // Complete the job
                    let mut job = self.jobs.entry(job_id).read();
                    let job_reward = job.reward;
                    job.status = JobStatus::Completed;
                    self.jobs.entry(job_id).write(job);

                    // Update job execution with consensus result
                    let mut job_exec = self.job_executions.entry(job_id).read();
                    job_exec.result_hash = result_hash;
                    job_exec.status = JobStatus::Completed;
                    job_exec.completed_at = timestamp;
                    self.job_executions.entry(job_id).write(job_exec);

                    // Emit consensus event
                    self.emit(ConsensusReached {
                        job_id,
                        consensus_hash: result_hash,
                        agreeing_provers: new_count,
                        timestamp,
                    });

                    // Sprint 4: Distribute reward among provers who submitted correct hash
                    let prover_count = self.job_prover_count.entry(job_id).read();
                    let reward_per_prover = job_reward / new_count.into();

                    let mut i: u8 = 0;
                    loop {
                        if i >= prover_count {
                            break;
                        }

                        let prover_addr = self.job_provers.entry((job_id, i)).read();
                        let prover_submission = self.job_prover_submissions.entry((job_id, prover_addr)).read();

                        // Only reward provers who submitted the winning hash
                        if prover_submission.has_submitted && prover_submission.result_hash == result_hash {
                            let mut prover = self.registered_provers.entry(prover_addr).read();
                            prover.jobs_completed += 1;
                            prover.total_earnings += reward_per_prover;
                            self.registered_provers.entry(prover_addr).write(prover);
                        } else if prover_submission.has_submitted {
                            // Prover submitted wrong hash - count as failed
                            let mut prover = self.registered_provers.entry(prover_addr).read();
                            prover.jobs_failed += 1;
                            self.registered_provers.entry(prover_addr).write(prover);
                        }

                        i += 1;
                    };

                    self.emit(JobCompleted {
                        job_id,
                        prover: caller,
                        result_hash,
                        timestamp,
                    });
                } else {
                    // No consensus yet, just save the consensus state
                    self.fhe_consensus.entry(job_id).write(consensus);
                }
            } else {
                // Single-prover mode (original behavior)
                let mut job_exec = self.job_executions.entry(job_id).read();
                assert(job_exec.status == JobStatus::Claimed, 'Job not claimed');
                assert(job_exec.prover == caller, 'Not job owner');

                // MVP: Auto-finalize on submit
                job_exec.result_hash = result_hash;
                job_exec.status = JobStatus::Completed;
                job_exec.completed_at = timestamp;

                self.job_executions.entry(job_id).write(job_exec);

                // Update job status
                let mut job = self.jobs.entry(job_id).read();
                let job_reward = job.reward;  // Read reward before moving job
                job.status = JobStatus::Completed;
                self.jobs.entry(job_id).write(job);

                // Sprint 1: Update prover stats (jobs_completed, total_earnings)
                let mut prover = self.registered_provers.entry(caller).read();
                prover.jobs_completed += 1;
                prover.total_earnings += job_reward;
                self.registered_provers.entry(caller).write(prover);

                self.emit(JobCompleted {
                    job_id,
                    prover: caller,
                    result_hash,
                    timestamp,
                });
            }
        }

        fn get_job_execution(self: @ContractState, job_id: u256) -> JobExecution {
            self.job_executions.entry(job_id).read()
        }

        fn get_pending_jobs(self: @ContractState) -> Array<u256> {
            let mut pending_jobs = ArrayTrait::new();
            let total_jobs = self.next_job_id.read();

            let mut job_id: u256 = 1;
            loop {
                if job_id >= total_jobs {
                    break;
                }

                let job = self.jobs.entry(job_id).read();
                if job.status == JobStatus::Pending {
                    pending_jobs.append(job_id);
                }

                job_id += 1;
            };

            pending_jobs
        }

        fn get_pending_jobs_by_type(self: @ContractState, job_type: JobType) -> Array<u256> {
            let mut pending_jobs = ArrayTrait::new();
            let total_jobs = self.next_job_id.read();

            let mut job_id: u256 = 1;
            loop {
                if job_id >= total_jobs {
                    break;
                }

                let job = self.jobs.entry(job_id).read();
                if job.status == JobStatus::Pending && job.job_type == job_type {
                    pending_jobs.append(job_id);
                }

                job_id += 1;
            };

            pending_jobs
        }

        fn get_jobs_by_creator(self: @ContractState, creator: ContractAddress) -> Array<u256> {
            let mut creator_jobs = ArrayTrait::new();
            let total_jobs = self.next_job_id.read();

            let mut job_id: u256 = 1;
            loop {
                if job_id >= total_jobs {
                    break;
                }

                let job = self.jobs.entry(job_id).read();
                if job.creator == creator {
                    creator_jobs.append(job_id);
                }

                job_id += 1;
            };

            creator_jobs
        }

        // Sprint 3: Multi-prover consensus functions
        fn enable_consensus(ref self: ContractState, job_id: u256, required_provers: u8, consensus_threshold: u8) {
            let caller = get_caller_address();

            // Verify job exists and caller is creator
            let job = self.jobs.entry(job_id).read();
            assert(job.job_id == job_id, 'Job does not exist');
            assert(job.creator == caller, 'Not job creator');
            assert(job.status == JobStatus::Pending, 'Job must be pending');

            // Validate consensus parameters
            assert(required_provers > 0, 'Need at least 1 prover');
            assert(consensus_threshold > 0, 'Threshold must be > 0');
            assert(consensus_threshold <= required_provers, 'Threshold > required');

            // Initialize consensus data
            let consensus = FheConsensusData {
                job_id,
                required_provers,
                consensus_threshold,
                claimed_count: 0,
                submitted_count: 0,
                consensus_reached: false,
                consensus_hash: 0,
            };

            self.fhe_consensus.entry(job_id).write(consensus);
        }

        fn get_consensus_data(self: @ContractState, job_id: u256) -> FheConsensusData {
            self.fhe_consensus.entry(job_id).read()
        }

        fn is_consensus_enabled(self: @ContractState, job_id: u256) -> bool {
            let consensus = self.fhe_consensus.entry(job_id).read();
            consensus.required_provers > 0
        }
    }

    // ========================================================================
    // pBTCFi Loan Operations Implementation
    // ========================================================================
    #[abi(embed_v0)]
    impl LoanOperationsImpl of super::ILoanOperations<ContractState> {
        fn create_loan(
            ref self: ContractState,
            borrower: ContractAddress,
            btc_commitment: felt252,
            btc_encrypted_c1: felt252,
            btc_encrypted_c2: felt252,
        ) -> u256 {
            let loan_id = self.next_loan_id.read();
            let timestamp = get_block_timestamp();
            let caller = get_caller_address();

            // Create loan record
            let loan = Loan {
                borrower,
                btc_commitment,
                btc_encrypted_c1,
                btc_encrypted_c2,
                status: LoanStatus::Pending,
                created_at: timestamp,
            };

            self.loans.entry(loan_id).write(loan);
            self.next_loan_id.write(loan_id + 1);

            // Create generic job for this loan
            let job_id = self.next_job_id.read();
            let job = Job {
                job_id,
                job_type: JobType::LoanVerification,
                creator: caller,
                payload_hash: btc_commitment,  // Use commitment as payload hash
                encrypted_c1: btc_encrypted_c1,
                encrypted_c2: btc_encrypted_c2,
                reward: 0,  // pBTCFi-specific reward logic can be added
                status: JobStatus::Pending,
                created_at: timestamp,
            };

            self.jobs.entry(job_id).write(job);
            self.next_job_id.write(job_id + 1);

            // Map loan to job
            self.loan_to_job.entry(loan_id).write(job_id);

            // Create job execution
            let zero_address: ContractAddress = starknet::contract_address_const::<0>();
            let job_execution = JobExecution {
                job_id,
                prover: zero_address,
                status: JobStatus::Pending,
                result_hash: 0,
                claimed_at: 0,
                completed_at: 0,
            };
            self.job_executions.entry(job_id).write(job_execution);

            self.emit(LoanCreated {
                loan_id,
                job_id,
                borrower,
                btc_commitment,
                timestamp,
            });

            loan_id
        }

        fn get_loan(self: @ContractState, loan_id: u256) -> Loan {
            self.loans.entry(loan_id).read()
        }

        fn get_loan_job_id(self: @ContractState, loan_id: u256) -> u256 {
            self.loan_to_job.entry(loan_id).read()
        }

        fn get_pending_loan_jobs(self: @ContractState) -> Array<u256> {
            let mut pending_loans = ArrayTrait::new();
            let total_loans = self.next_loan_id.read();

            let mut loan_id: u256 = 1;
            loop {
                if loan_id >= total_loans {
                    break;
                }

                let job_id = self.loan_to_job.entry(loan_id).read();
                if job_id > 0 {
                    let job = self.jobs.entry(job_id).read();
                    if job.status == JobStatus::Pending {
                        pending_loans.append(loan_id);
                    }
                }

                loan_id += 1;
            };

            pending_loans
        }

        fn claim_loan_job(ref self: ContractState, loan_id: u256) {
            let caller = get_caller_address();
            let timestamp = get_block_timestamp();

            // Get the job_id for this loan
            let job_id = self.loan_to_job.entry(loan_id).read();
            assert(job_id > 0, 'Loan has no job');

            // Verify prover is registered
            let prover = self.registered_provers.entry(caller).read();
            assert(prover.registered_at > 0, 'Prover not registered');

            // Get current job execution
            let mut job_exec = self.job_executions.entry(job_id).read();
            assert(job_exec.status == JobStatus::Pending, 'Job not available');

            // Update job execution
            job_exec.prover = caller;
            job_exec.status = JobStatus::Claimed;
            job_exec.claimed_at = timestamp;

            self.job_executions.entry(job_id).write(job_exec);

            // Update job status
            let mut job = self.jobs.entry(job_id).read();
            job.status = JobStatus::Claimed;
            self.jobs.entry(job_id).write(job);

            self.emit(JobClaimed {
                job_id,
                prover: caller,
                timestamp,
            });
        }

        fn submit_loan_result(ref self: ContractState, loan_id: u256, result_hash: felt252) {
            let caller = get_caller_address();
            let timestamp = get_block_timestamp();

            // Get the job_id for this loan
            let job_id = self.loan_to_job.entry(loan_id).read();
            assert(job_id > 0, 'Loan has no job');

            // Get current job execution
            let mut job_exec = self.job_executions.entry(job_id).read();
            assert(job_exec.status == JobStatus::Claimed, 'Job not claimed');
            assert(job_exec.prover == caller, 'Not job owner');

            // MVP: Auto-finalize on submit
            job_exec.result_hash = result_hash;
            job_exec.status = JobStatus::Completed;
            job_exec.completed_at = timestamp;

            self.job_executions.entry(job_id).write(job_exec);

            // Update job status
            let mut job = self.jobs.entry(job_id).read();
            let job_reward = job.reward;  // Read reward before moving job
            job.status = JobStatus::Completed;
            self.jobs.entry(job_id).write(job);

            // Update loan status to Active
            let mut loan = self.loans.entry(loan_id).read();
            loan.status = LoanStatus::Active;
            self.loans.entry(loan_id).write(loan);

            // Sprint 1: Update prover stats (jobs_completed, total_earnings)
            let mut prover = self.registered_provers.entry(caller).read();
            prover.jobs_completed += 1;
            prover.total_earnings += job_reward;
            self.registered_provers.entry(caller).write(prover);

            self.emit(JobCompleted {
                job_id,
                prover: caller,
                result_hash,
                timestamp,
            });
        }

        fn get_loan_job_execution(self: @ContractState, loan_id: u256) -> JobExecution {
            let job_id = self.loan_to_job.entry(loan_id).read();
            self.job_executions.entry(job_id).read()
        }
    }
}
