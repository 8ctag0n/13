#[starknet::interface]
pub trait ILiquidation<TContractState> {
    fn check_liquidation(self: @TContractState, loan_id: u256) -> bool;
    fn execute_liquidation(ref self: TContractState, loan_id: u256) -> bool;
}

#[starknet::contract]
pub mod PbtcfiLiquidation {
    use starknet::storage::{Map, StoragePathEntry, StoragePointerReadAccess, StoragePointerWriteAccess};

    #[storage]
    struct Storage {
        liquidated: Map<u256, bool>,
    }

    #[abi(embed_v0)]
    impl LiquidationImpl of super::ILiquidation<ContractState> {
        fn check_liquidation(self: @ContractState, loan_id: u256) -> bool {
            // Mock: Check if LTV > 80%
            // TODO: Integrate with collateral.calculate_ltv()
            false
        }

        fn execute_liquidation(ref self: ContractState, loan_id: u256) -> bool {
            // Mark as liquidated
            self.liquidated.entry(loan_id).write(true);
            true
        }
    }
}
