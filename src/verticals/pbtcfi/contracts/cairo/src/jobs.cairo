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

#[starknet::contract]
pub mod PbtcfiJobs {
    use super::{Loan, LoanStatus};
    use starknet::{ContractAddress, get_block_timestamp};
    use starknet::storage::{Map, StoragePathEntry, StoragePointerReadAccess, StoragePointerWriteAccess};

    #[storage]
    struct Storage {
        loans: Map<u256, Loan>,
        next_loan_id: u256,
    }

    #[event]
    #[derive(Drop, starknet::Event)]
    pub enum Event {
        LoanCreated: LoanCreated,
    }

    /// Emitted when a new loan is created
    /// All fields go to event.data for pbtcfi_sync.rs compatibility
    /// Order: [loan_id, borrower, btc_commitment, btc_encrypted_c1, btc_encrypted_c2, timestamp]
    /// Note: loan_id as felt252 for single-slot serialization (u256 would use 2 slots)
    #[derive(Drop, starknet::Event)]
    pub struct LoanCreated {
        pub loan_id: felt252,
        pub borrower: ContractAddress,
        pub btc_commitment: felt252,
        pub btc_encrypted_c1: felt252,
        pub btc_encrypted_c2: felt252,
        pub timestamp: u64,
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

            // Emit event for off-chain sync
            // Cast loan_id to felt252 for single-slot serialization
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
    }
}
