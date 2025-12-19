use solana_client::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use solana_sdk::instruction::Instruction;
use std::sync::Arc;

use crate::{
    accounts::*,
    error::Result,
    instructions::*,
    queries::*,
    state::*,
};

pub struct FutarchyClient {
    rpc_client: Arc<RpcClient>,
    program_id: Pubkey,
}

impl FutarchyClient {
    pub fn new(rpc_url: &str, program_id: Pubkey) -> Self {
        Self {
            rpc_client: Arc::new(RpcClient::new(rpc_url.to_string())),
            program_id,
        }
    }

    pub fn with_client(rpc_client: Arc<RpcClient>, program_id: Pubkey) -> Self {
        Self {
            rpc_client,
            program_id,
        }
    }

    pub fn program_id(&self) -> &Pubkey {
        &self.program_id
    }

    pub fn rpc_client(&self) -> &RpcClient {
        &self.rpc_client
    }

    pub fn create_market(
        &self,
        authority: &Pubkey,
        market_id: u64,
        question_hash: [u8; 32],
        oracle: &Pubkey,
        end_time: i64,
        max_bet: u64,
    ) -> Result<Instruction> {
        build_create_market_ix(
            &self.program_id,
            authority,
            market_id,
            question_hash,
            oracle,
            end_time,
            max_bet,
        )
    }

    pub fn place_bet(
        &self,
        bettor: &Pubkey,
        market_id: u64,
        bet_commitment: [u8; 32],
        proof: Vec<u8>,
        public_inputs: Vec<u8>,
        amount: u64,
        circuit_type: u8,
        ciphertext_hash: Option<[u8; 32]>,
        encrypted_bet_amount: Option<Vec<u8>>,
        side: Option<bool>,
        zk_generator_program: &Pubkey,
        fhe_accounts: Option<FheAccounts>,
    ) -> Result<Instruction> {
        build_place_bet_ix(
            &self.program_id,
            bettor,
            market_id,
            bet_commitment,
            proof,
            public_inputs,
            amount,
            circuit_type,
            ciphertext_hash,
            encrypted_bet_amount,
            side,
            zk_generator_program,
            fhe_accounts,
        )
    }

    pub fn settle_market(
        &self,
        oracle: &Pubkey,
        market_id: u64,
        outcome: bool,
    ) -> Result<Instruction> {
        build_settle_market_ix(&self.program_id, oracle, market_id, outcome)
    }

    pub fn claim_payout(
        &self,
        user: &Pubkey,
        market_id: u64,
        claim_nullifier: [u8; 32],
        proof: Vec<u8>,
        public_inputs: Vec<u8>,
        payout_amount: u64,
        zk_generator_program: &Pubkey,
        include_position: bool,
    ) -> Result<Instruction> {
        build_claim_payout_ix(
            &self.program_id,
            user,
            market_id,
            claim_nullifier,
            proof,
            public_inputs,
            payout_amount,
            zk_generator_program,
            include_position,
        )
    }

    pub fn update_pool(
        &self,
        updater: &Pubkey,
        market_id: u64,
        fhe_job_id: u64,
        encrypted_result: Vec<u8>,
        side: bool,
        fhe_job_account: &Pubkey,
        fhe_consensus_account: &Pubkey,
    ) -> Result<Instruction> {
        build_update_pool_ix(
            &self.program_id,
            updater,
            market_id,
            fhe_job_id,
            encrypted_result,
            side,
            fhe_job_account,
            fhe_consensus_account,
        )
    }

    pub fn register_user(
        &self,
        user: &Pubkey,
        proof: Vec<u8>,
        public_inputs: Vec<u8>,
        blacklist_root: [u8; 32],
        blacklist_version: u32,
        zk_generator_program: &Pubkey,
    ) -> Result<Instruction> {
        build_register_user_ix(
            &self.program_id,
            user,
            proof,
            public_inputs,
            blacklist_root,
            blacklist_version,
            zk_generator_program,
        )
    }

    pub fn create_market_with_governance(
        &self,
        authority: &Pubkey,
        market_id: u64,
        question_hash: [u8; 32],
        oracle: &Pubkey,
        end_time: i64,
        max_bet: u64,
        executable_action: ExecutableAction,
        execution_threshold: u8,
        timelock_duration: i64,
    ) -> Result<Instruction> {
        build_create_market_with_governance_ix(
            &self.program_id,
            authority,
            market_id,
            question_hash,
            oracle,
            end_time,
            max_bet,
            executable_action,
            execution_threshold,
            timelock_duration,
        )
    }

    pub fn cancel_market(&self, authority: &Pubkey, market_id: u64) -> Result<Instruction> {
        build_cancel_market_ix(&self.program_id, authority, market_id)
    }

    pub fn deposit_to_market(
        &self,
        user: &Pubkey,
        market_id: u64,
        amount: u64,
    ) -> Result<Instruction> {
        build_deposit_to_market_ix(&self.program_id, user, market_id, amount)
    }

    pub fn withdraw_from_escrow(
        &self,
        user: &Pubkey,
        market_id: u64,
        amount: u64,
    ) -> Result<Instruction> {
        build_withdraw_from_escrow_ix(&self.program_id, user, market_id, amount)
    }

    pub async fn get_market(&self, market_id: u64) -> Result<Market> {
        query_market(&self.rpc_client, &self.program_id, market_id).await
    }

    pub async fn get_position(&self, market_id: u64, user: &Pubkey) -> Result<Position> {
        query_position(&self.rpc_client, &self.program_id, market_id, user).await
    }

    pub async fn get_all_markets(&self) -> Result<Vec<(Pubkey, Market)>> {
        query_all_markets(&self.rpc_client, &self.program_id).await
    }

    pub async fn get_pending_fhe_jobs(&self) -> Result<Vec<FutarchyFheJob>> {
        find_pending_fhe_jobs(&self.rpc_client, &self.program_id).await
    }

    pub async fn get_pool_ciphertext(&self, market_id: u64, side: bool) -> Result<Vec<u8>> {
        get_pool_ciphertext(&self.rpc_client, &self.program_id, market_id, side).await
    }

    pub async fn get_user_eligibility(&self, user: &Pubkey) -> Result<Option<UserEligibility>> {
        query_user_eligibility(&self.rpc_client, &self.program_id, user).await
    }

    pub async fn get_user_escrow(
        &self,
        user: &Pubkey,
        market_id: u64,
    ) -> Result<Option<UserEscrow>> {
        query_user_escrow(&self.rpc_client, &self.program_id, user, market_id).await
    }

    pub async fn get_active_markets(&self) -> Result<Vec<(Pubkey, Market)>> {
        query_active_markets(&self.rpc_client, &self.program_id).await
    }

    pub async fn get_settled_markets(&self) -> Result<Vec<(Pubkey, Market)>> {
        query_settled_markets(&self.rpc_client, &self.program_id).await
    }

    pub fn find_market_pda(&self, market_id: u64) -> PdaDerivation {
        find_market_pda(&self.program_id, market_id)
    }

    pub fn find_position_pda(&self, market_id: u64, user: &Pubkey) -> PdaDerivation {
        find_position_pda(&self.program_id, market_id, user)
    }

    pub fn find_escrow_pda(&self, market_id: u64) -> PdaDerivation {
        find_escrow_pda(&self.program_id, market_id)
    }

    pub fn find_user_eligibility_pda(&self, user: &Pubkey) -> PdaDerivation {
        find_user_eligibility_pda(&self.program_id, user)
    }

    pub fn find_user_escrow_pda(&self, user: &Pubkey, market_id: u64) -> PdaDerivation {
        find_user_escrow_pda(&self.program_id, user, market_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let program_id = Pubkey::new_unique();
        let client = FutarchyClient::new("http://localhost:8899", program_id);

        assert_eq!(client.program_id(), &program_id);
    }

    #[test]
    fn test_instruction_building() {
        let program_id = Pubkey::new_unique();
        let client = FutarchyClient::new("http://localhost:8899", program_id);

        let authority = Pubkey::new_unique();
        let oracle = Pubkey::new_unique();

        let ix = client
            .create_market(&authority, 1, [0u8; 32], &oracle, 1234567890, 1_000_000_000)
            .unwrap();

        assert_eq!(ix.program_id, program_id);
    }
}
