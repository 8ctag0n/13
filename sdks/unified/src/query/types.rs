use solana_sdk::pubkey::Pubkey;
pub use zyberlink_jobs::JobCommon;
pub use zyberlink_types::JobStatus;

/// Unified representation of a job from any generator program
#[derive(Debug, Clone)]
pub enum UnifiedJob {
    Zk {
        program_id: Pubkey,
        address: Pubkey,
        common: JobCommon,
        circuit_type: u8,
    },
    Fhe {
        program_id: Pubkey,
        address: Pubkey,
        common: JobCommon,
        circuit_type: u8,
        consensus_bump: u8,
    },
    #[allow(dead_code)]
    Legacy {
        program_id: Pubkey,
        address: Pubkey,
    },
}

impl UnifiedJob {
    pub fn common(&self) -> &JobCommon {
        match self {
            Self::Zk { common, .. } => common,
            Self::Fhe { common, .. } => common,
            Self::Legacy { .. } => todo!("Legacy job support"),
        }
    }

    pub fn id(&self) -> u64 {
        self.common().id
    }

    pub fn status(&self) -> JobStatus {
        self.common().status
    }

    pub fn address(&self) -> &Pubkey {
        match self {
            Self::Zk { address, .. } => address,
            Self::Fhe { address, .. } => address,
            Self::Legacy { address, .. } => address,
        }
    }

    pub fn program_id(&self) -> &Pubkey {
        match self {
            Self::Zk { program_id, .. } => program_id,
            Self::Fhe { program_id, .. } => program_id,
            Self::Legacy { program_id, .. } => program_id,
        }
    }

    pub fn circuit_type(&self) -> u8 {
        match self {
            Self::Zk { circuit_type, .. } => *circuit_type,
            Self::Fhe { circuit_type, .. } => *circuit_type,
            Self::Legacy { .. } => todo!("Get circuit_type from legacy JobAccount"),
        }
    }

    pub fn is_zk(&self) -> bool {
        matches!(self, Self::Zk { .. })
    }

    pub fn is_fhe(&self) -> bool {
        matches!(self, Self::Fhe { .. })
    }

    pub fn can_claim(&self) -> bool {
        self.common().can_claim()
    }

    pub fn is_expired(&self, current_time: i64) -> bool {
        self.common().is_expired(current_time)
    }

    pub fn creator(&self) -> &Pubkey {
        &self.common().creator
    }

    pub fn prover(&self) -> Option<&Pubkey> {
        self.common().prover.as_ref()
    }

    pub fn price_lamports(&self) -> u64 {
        self.common().price_lamports
    }

    pub fn witness_hash(&self) -> &[u8; 32] {
        &self.common().witness_hash
    }

    pub fn proof_hash(&self) -> Option<&[u8; 32]> {
        self.common().proof_hash.as_ref()
    }
}
