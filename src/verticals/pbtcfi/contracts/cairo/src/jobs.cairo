use starknet::ContractAddress;

#[starknet::interface]
pub trait IJobSubmission<TContractState> {
    fn create_loan(
        ref self: TContractState,
        borrower: ContractAddress,
        btc_amount: u256,
        collateral_hash: felt252
    ) -> u256;

    fn get_loan(self: @TContractState, loan_id: u256) -> Loan;
}

#[derive(Drop, Serde, starknet::Store)]
pub struct Loan {
    pub borrower: ContractAddress,
    pub btc_amount: u256,
    pub collateral_hash: felt252,
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

    #[constructor]
    fn constructor(ref self: ContractState) {
        self.next_loan_id.write(1);
    }

    #[abi(embed_v0)]
    impl JobSubmissionImpl of super::IJobSubmission<ContractState> {
        fn create_loan(
            ref self: ContractState,
            borrower: ContractAddress,
            btc_amount: u256,
            collateral_hash: felt252
        ) -> u256 {
            let loan_id = self.next_loan_id.read();

            let loan = Loan {
                borrower,
                btc_amount,
                collateral_hash,
                status: LoanStatus::Pending,
                created_at: get_block_timestamp(),
            };

            self.loans.entry(loan_id).write(loan);
            self.next_loan_id.write(loan_id + 1);

            loan_id
        }

        fn get_loan(self: @ContractState, loan_id: u256) -> Loan {
            self.loans.entry(loan_id).read()
        }
    }
}
