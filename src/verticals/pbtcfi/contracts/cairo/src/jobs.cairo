use starknet::ContractAddress;

#[starknet::interface]
pub trait IJobSubmission<TContractState> {
    fn create_loan(
        ref self: TContractState,
        borrower: ContractAddress,
        btc_commitment: felt252,
        btc_encrypted_c1: felt252,
        btc_encrypted_c2: felt252,
    ) -> u256;

    fn get_loan(self: @TContractState, loan_id: u256) -> Loan;

    // Job lifecycle functions
    fn register_prover(ref self: TContractState);
    fn claim_job(ref self: TContractState, loan_id: u256);
    fn submit_result(ref self: TContractState, loan_id: u256, result_hash: felt252);
    fn get_job_execution(self: @TContractState, loan_id: u256) -> JobExecution;
    fn get_pending_jobs(self: @TContractState) -> Array<u256>;
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

#[derive(Drop, Serde, starknet::Store)]
pub struct Prover {
    pub address: ContractAddress,
    pub registered_at: u64,
}

#[derive(Drop, Serde, starknet::Store)]
pub struct JobExecution {
    pub loan_id: u256,
    pub prover: ContractAddress,
    pub status: JobStatus,
    pub result_hash: felt252,
    pub claimed_at: u64,
    pub completed_at: u64,
}

#[starknet::contract]
pub mod PbtcfiJobs {
    use super::{Loan, LoanStatus, JobStatus, Prover, JobExecution};
    use starknet::{ContractAddress, get_block_timestamp, get_caller_address};
    use starknet::storage::{Map, StoragePathEntry, StoragePointerReadAccess, StoragePointerWriteAccess};

    #[storage]
    struct Storage {
        loans: Map<u256, Loan>,
        next_loan_id: u256,
        registered_provers: Map<ContractAddress, Prover>,
        job_executions: Map<u256, JobExecution>,
    }

    #[event]
    #[derive(Drop, starknet::Event)]
    pub enum Event {
        LoanCreated: LoanCreated,
        ProverRegistered: ProverRegistered,
        JobClaimed: JobClaimed,
        JobCompleted: JobCompleted,
    }

    #[derive(Drop, starknet::Event)]
    pub struct LoanCreated {
        pub loan_id: felt252,
        pub borrower: ContractAddress,
        pub btc_commitment: felt252,
        pub btc_encrypted_c1: felt252,
        pub btc_encrypted_c2: felt252,
        pub timestamp: u64,
    }

    #[derive(Drop, starknet::Event)]
    pub struct ProverRegistered {
        pub prover: ContractAddress,
    }

    #[derive(Drop, starknet::Event)]
    pub struct JobClaimed {
        pub loan_id: felt252,
        pub prover: ContractAddress,
    }

    #[derive(Drop, starknet::Event)]
    pub struct JobCompleted {
        pub loan_id: felt252,
        pub prover: ContractAddress,
        pub result_hash: felt252,
    }

    #[constructor]
    fn constructor(ref self: ContractState) {
        self.next_loan_id.write(1);
    }

    #[abi(embed_v0)]
    impl JobSubmissionImpl of super::IJobSubmission<ContractState> {
        fn create_loan(
            ref self: ContractState,
            borrower: ContractAddress,
            btc_commitment: felt252,
            btc_encrypted_c1: felt252,
            btc_encrypted_c2: felt252,
        ) -> u256 {
            let loan_id = self.next_loan_id.read();
            let timestamp = get_block_timestamp();

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

            // Create initial job execution in Pending state
            let zero_address: ContractAddress = starknet::contract_address_const::<0>();
            let job_execution = JobExecution {
                loan_id,
                prover: zero_address,
                status: JobStatus::Pending,
                result_hash: 0,
                claimed_at: 0,
                completed_at: 0,
            };
            self.job_executions.entry(loan_id).write(job_execution);

            self.emit(LoanCreated {
                loan_id: loan_id.try_into().unwrap(),
                borrower,
                btc_commitment,
                btc_encrypted_c1,
                btc_encrypted_c2,
                timestamp,
            });

            loan_id
        }

        fn get_loan(self: @ContractState, loan_id: u256) -> Loan {
            self.loans.entry(loan_id).read()
        }

        fn register_prover(ref self: ContractState) {
            let caller = get_caller_address();
            let timestamp = get_block_timestamp();

            let prover = Prover {
                address: caller,
                registered_at: timestamp,
            };

            self.registered_provers.entry(caller).write(prover);

            self.emit(ProverRegistered {
                prover: caller,
            });
        }

        fn claim_job(ref self: ContractState, loan_id: u256) {
            let caller = get_caller_address();
            let timestamp = get_block_timestamp();

            // Verify prover is registered
            let prover = self.registered_provers.entry(caller).read();
            assert(prover.registered_at > 0, 'Prover not registered');

            // Get current job execution
            let mut job = self.job_executions.entry(loan_id).read();

            // Verify job is in Pending status
            assert(job.status == JobStatus::Pending, 'Job not available');

            // Update job execution
            job.prover = caller;
            job.status = JobStatus::Claimed;
            job.claimed_at = timestamp;

            self.job_executions.entry(loan_id).write(job);

            self.emit(JobClaimed {
                loan_id: loan_id.try_into().unwrap(),
                prover: caller,
            });
        }

        fn submit_result(ref self: ContractState, loan_id: u256, result_hash: felt252) {
            let caller = get_caller_address();
            let timestamp = get_block_timestamp();

            // Get current job execution
            let mut job = self.job_executions.entry(loan_id).read();

            // Verify job is claimed by this prover
            assert(job.status == JobStatus::Claimed, 'Job not claimed');
            assert(job.prover == caller, 'Not job owner');

            // MVP: Auto-finalize on submit
            job.result_hash = result_hash;
            job.status = JobStatus::Completed;
            job.completed_at = timestamp;

            self.job_executions.entry(loan_id).write(job);

            // Update loan status to Active
            let mut loan = self.loans.entry(loan_id).read();
            loan.status = LoanStatus::Active;
            self.loans.entry(loan_id).write(loan);

            self.emit(JobCompleted {
                loan_id: loan_id.try_into().unwrap(),
                prover: caller,
                result_hash,
            });
        }

        fn get_job_execution(self: @ContractState, loan_id: u256) -> JobExecution {
            self.job_executions.entry(loan_id).read()
        }

        fn get_pending_jobs(self: @ContractState) -> Array<u256> {
            let mut pending_jobs = ArrayTrait::new();
            let total_loans = self.next_loan_id.read();

            let mut loan_id: u256 = 1;
            loop {
                if loan_id >= total_loans {
                    break;
                }

                let job = self.job_executions.entry(loan_id).read();
                if job.status == JobStatus::Pending {
                    pending_jobs.append(loan_id);
                }

                loan_id += 1;
            };

            pending_jobs
        }
    }
}
