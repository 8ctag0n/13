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

// Mapping from loan_id to job_id (for backwards compatibility)
// This allows pBTCFi code to use loan_id while internally using job_id

#[starknet::contract]
pub mod PbtcfiJobs {
    use super::{Loan, LoanStatus, JobStatus, Prover, JobExecution, Job, JobType};
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

            // Sprint 1: Initialize prover with default values
            // TODO Sprint 2: Add parameters for stake, encryption_pubkey
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

        fn submit_result(ref self: ContractState, job_id: u256, result_hash: felt252) {
            let caller = get_caller_address();
            let timestamp = get_block_timestamp();

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
            job.status = JobStatus::Completed;
            self.jobs.entry(job_id).write(job);

            // Sprint 1: Update prover stats (jobs_completed, total_earnings)
            let mut prover = self.registered_provers.entry(caller).read();
            prover.jobs_completed += 1;
            prover.total_earnings += job.reward;
            self.registered_provers.entry(caller).write(prover);

            self.emit(JobCompleted {
                job_id,
                prover: caller,
                result_hash,
                timestamp,
            });
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
            job.status = JobStatus::Completed;
            self.jobs.entry(job_id).write(job);

            // Update loan status to Active
            let mut loan = self.loans.entry(loan_id).read();
            loan.status = LoanStatus::Active;
            self.loans.entry(loan_id).write(loan);

            // Sprint 1: Update prover stats (jobs_completed, total_earnings)
            let mut prover = self.registered_provers.entry(caller).read();
            prover.jobs_completed += 1;
            prover.total_earnings += job.reward;
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
