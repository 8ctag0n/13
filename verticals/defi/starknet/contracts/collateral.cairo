#[starknet::interface]
pub trait ICollateral<TContractState> {
    fn submit_utxo_proof(ref self: TContractState, loan_id: u256, proof_hash: felt252) -> bool;
    fn get_collateral_status(self: @TContractState, loan_id: u256) -> CollateralStatus;
    fn calculate_ltv(self: @TContractState, loan_id: u256) -> u256;
}

#[derive(Drop, Serde, starknet::Store, PartialEq)]
pub enum CollateralStatus {
    Unverified,
    Verified,
    Insufficient,
}

#[starknet::contract]
pub mod PbtcfiCollateral {
    use super::CollateralStatus;
    use starknet::storage::{Map, StoragePathEntry, StoragePointerReadAccess, StoragePointerWriteAccess};

    #[storage]
    struct Storage {
        proof_hashes: Map<u256, felt252>,
        verified: Map<u256, bool>,
    }

    #[abi(embed_v0)]
    impl CollateralImpl of super::ICollateral<ContractState> {
        fn submit_utxo_proof(
            ref self: ContractState, loan_id: u256, proof_hash: felt252
        ) -> bool {
            // For MVP: Accept any proof (mock verification)
            // TODO: Call ZK verifier in Day 3
            self.proof_hashes.entry(loan_id).write(proof_hash);
            self.verified.entry(loan_id).write(true);
            true
        }

        fn get_collateral_status(self: @ContractState, loan_id: u256) -> CollateralStatus {
            if self.verified.entry(loan_id).read() {
                CollateralStatus::Verified
            } else {
                CollateralStatus::Unverified
            }
        }

        fn calculate_ltv(self: @ContractState, loan_id: u256) -> u256 {
            // Mock LTV calculation
            // TODO: Integrate with price oracle
            75 // 75% LTV
        }
    }
}
