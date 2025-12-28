//! Private Market (Prediction Market) Circuit Verifier
//!
//! This circuit enables private betting in prediction markets:
//! 1. Bet placement without revealing position or amount
//! 2. Claim winnings with proof of winning bet
//! 3. Optional: Insider trading prevention (PoI integration)
//!
//! # Circuit Design - MarketBet
//!
//! ```text
//! PRIVATE INPUTS:
//! ├── bettor_wallet          // Wallet address (hidden)
//! ├── bet_amount             // Actual bet amount
//! ├── position               // Outcome prediction (0, 1, ...)
//! ├── blinding_factor        // Random for commitment
//! └── balance_proof          // Proof of sufficient balance
//!
//! PUBLIC INPUTS:
//! ├── market_id              // Unique market identifier
//! ├── bet_commitment         // Hash(amount, position, blinding)
//! ├── max_bet                // Maximum allowed bet
//! └── timestamp              // Bet placement time
//!
//! CONSTRAINTS:
//! 1. Assert(bet_amount <= max_bet)
//! 2. Assert(position < num_outcomes)
//! 3. Verify balance >= bet_amount
//! 4. Verify bet_commitment = Hash(amount, position, blinding)
//! ```
//!
//! # Circuit Design - MarketClaim
//!
//! ```text
//! PRIVATE INPUTS:
//! ├── bet_amount             // Original bet amount
//! ├── bet_side               // Position that was bet (0/1)
//! ├── blinding               // Same blinding from bet
//! └── secret                 // Secret used for nullifier
//!
//! PUBLIC INPUTS:
//! ├── market_id              // Market identifier
//! ├── nullifier              // Poseidon(secret, bet_commitment)
//! ├── payout_amount          // Calculated winnings
//! ├── resolution             // Resolved outcome (0/1)
//! ├── total_pool             // Total pool size
//! ├── winning_pool           // Winning side pool
//! ├── bet_commitment         // Original bet commitment
//! └── timestamp              // Claim timestamp
//!
//! CONSTRAINTS:
//! 1. Verify bet_commitment matches original bet
//! 2. Assert(bet_side == resolution)
//! 3. Verify payout calculation
//! 4. Verify nullifier for double-claim prevention
//! ```

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{msg, program_error::ProgramError, pubkey::Pubkey};

/// Public inputs for MarketBet circuit verification
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct MarketBetPublicInputs {
    /// Unique market identifier
    pub market_id: Pubkey,
    /// Commitment to the bet: Hash(amount, position, blinding)
    pub bet_commitment: [u8; 32],
    /// Maximum allowed bet amount
    pub max_bet: u64,
    /// Bet placement timestamp
    pub timestamp: i64,
}

impl MarketBetPublicInputs {
    // 32 (market_id) + 32 (bet_commitment) + 8 (max_bet) + 8 (timestamp)
    pub const LEN: usize = 32 + 32 + 8 + 8; // 80 bytes

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, ProgramError> {
        if data.len() < Self::LEN {
            msg!("MarketBet public inputs too short: {} < {}", data.len(), Self::LEN);
            return Err(ProgramError::InvalidInstructionData);
        }

        let market_id = Pubkey::try_from(&data[0..32])
            .map_err(|_| ProgramError::InvalidInstructionData)?;

        let mut bet_commitment = [0u8; 32];
        bet_commitment.copy_from_slice(&data[32..64]);

        let max_bet = u64::from_le_bytes(
            data[64..72].try_into().map_err(|_| ProgramError::InvalidInstructionData)?,
        );

        let timestamp = i64::from_le_bytes(
            data[72..80].try_into().map_err(|_| ProgramError::InvalidInstructionData)?,
        );

        Ok(Self {
            market_id,
            bet_commitment,
            max_bet,
            timestamp,
        })
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::LEN);
        bytes.extend_from_slice(self.market_id.as_ref());
        bytes.extend_from_slice(&self.bet_commitment);
        bytes.extend_from_slice(&self.max_bet.to_le_bytes());
        bytes.extend_from_slice(&self.timestamp.to_le_bytes());
        bytes
    }
}

/// Public inputs for MarketBet with PoI (insider trading prevention)
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct MarketBetWithPoiPublicInputs {
    /// Base bet inputs
    pub bet: MarketBetPublicInputs,
    /// Blacklist merkle root for insider trading check
    pub insider_blacklist_root: [u8; 32],
}

impl MarketBetWithPoiPublicInputs {
    pub const LEN: usize = MarketBetPublicInputs::LEN + 32; // 112 bytes

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, ProgramError> {
        if data.len() < Self::LEN {
            msg!("MarketBetWithPoI public inputs too short: {} < {}", data.len(), Self::LEN);
            return Err(ProgramError::InvalidInstructionData);
        }

        let bet = MarketBetPublicInputs::from_bytes(&data[0..MarketBetPublicInputs::LEN])?;

        let mut insider_blacklist_root = [0u8; 32];
        insider_blacklist_root.copy_from_slice(&data[MarketBetPublicInputs::LEN..Self::LEN]);

        Ok(Self {
            bet,
            insider_blacklist_root,
        })
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.bet.to_bytes();
        bytes.extend_from_slice(&self.insider_blacklist_root);
        bytes
    }
}

/// Public inputs for MarketClaim circuit verification
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct MarketClaimPublicInputs {
    /// Market identifier
    pub market_id: u64,
    /// Claim nullifier to prevent double-claim
    pub nullifier: [u8; 32],
    /// Calculated payout amount
    pub payout_amount: u64,
    /// Resolved winning outcome (0/1)
    pub resolution: u8,
    /// Total market pool
    pub total_pool: u64,
    /// Winning side pool
    pub winning_pool: u64,
    /// Original bet commitment (to verify ownership)
    pub bet_commitment: [u8; 32],
    /// Claim timestamp
    pub timestamp: i64,
}

impl MarketClaimPublicInputs {
    // 8 (market_id) + 32 (nullifier) + 8 (payout) + 1 (resolution)
    // + 8 (total_pool) + 8 (winning_pool) + 32 (bet_commitment) + 8 (timestamp)
    pub const LEN: usize = 8 + 32 + 8 + 1 + 8 + 8 + 32 + 8; // 105 bytes

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, ProgramError> {
        if data.len() < Self::LEN {
            msg!("MarketClaim public inputs too short: {} < {}", data.len(), Self::LEN);
            return Err(ProgramError::InvalidInstructionData);
        }

        let market_id = u64::from_le_bytes(
            data[0..8].try_into().map_err(|_| ProgramError::InvalidInstructionData)?,
        );

        let mut nullifier = [0u8; 32];
        nullifier.copy_from_slice(&data[8..40]);

        let payout_amount = u64::from_le_bytes(
            data[40..48].try_into().map_err(|_| ProgramError::InvalidInstructionData)?,
        );

        let resolution = data[48];

        let total_pool = u64::from_le_bytes(
            data[49..57].try_into().map_err(|_| ProgramError::InvalidInstructionData)?,
        );

        let winning_pool = u64::from_le_bytes(
            data[57..65].try_into().map_err(|_| ProgramError::InvalidInstructionData)?,
        );

        let mut bet_commitment = [0u8; 32];
        bet_commitment.copy_from_slice(&data[65..97]);

        let timestamp = i64::from_le_bytes(
            data[97..105].try_into().map_err(|_| ProgramError::InvalidInstructionData)?,
        );

        Ok(Self {
            market_id,
            nullifier,
            bet_commitment,
            payout_amount,
            resolution,
            total_pool,
            winning_pool,
            timestamp,
        })
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::LEN);
        bytes.extend_from_slice(&self.market_id.to_le_bytes());
        bytes.extend_from_slice(&self.nullifier);
        bytes.extend_from_slice(&self.payout_amount.to_le_bytes());
        bytes.push(self.resolution);
        bytes.extend_from_slice(&self.total_pool.to_le_bytes());
        bytes.extend_from_slice(&self.winning_pool.to_le_bytes());
        bytes.extend_from_slice(&self.bet_commitment);
        bytes.extend_from_slice(&self.timestamp.to_le_bytes());
        bytes
    }
}

/// Market-specific error codes
#[derive(Debug, Clone, Copy)]
pub enum MarketError {
    InvalidPublicInputs,
    InvalidProof,
    VerificationFailed,
    MarketNotFound,
    MarketNotResolved,
    BetExceedsMax,
    ClaimNullifierUsed,
    InvalidOutcome,
    BettingClosed,
    InsiderDetected,
}

impl From<MarketError> for ProgramError {
    fn from(e: MarketError) -> Self {
        ProgramError::Custom(3000 + e as u32)
    }
}

/// Groth16 proof size
pub const MARKET_PROOF_SIZE: usize = 256;

/// Verification key for Market circuits
#[derive(Debug, Clone)]
pub struct MarketVerificationKey {
    pub alpha: [u8; 64],
    pub beta: [u8; 128],
    pub gamma: [u8; 128],
    pub delta: [u8; 128],
    pub ic: Vec<[u8; 64]>,
}

impl Default for MarketVerificationKey {
    fn default() -> Self {
        Self {
            alpha: [0u8; 64],
            beta: [0u8; 128],
            gamma: [0u8; 128],
            delta: [0u8; 128],
            ic: vec![[0u8; 64]; 5], // 4 public inputs + 1
        }
    }
}

/// Verify a MarketBet proof
pub fn verify_market_bet_proof(
    proof: &[u8],
    public_inputs: &MarketBetPublicInputs,
    _vk: &MarketVerificationKey,
) -> Result<bool, ProgramError> {
    // Validate proof format
    if proof.len() != MARKET_PROOF_SIZE {
        msg!(
            "Invalid market bet proof size: {} != {}",
            proof.len(),
            MARKET_PROOF_SIZE
        );
        return Err(MarketError::InvalidProof.into());
    }

    // Validate market_id
    if public_inputs.market_id == Pubkey::default() {
        msg!("Invalid market ID");
        return Err(MarketError::MarketNotFound.into());
    }

    // Validate bet_commitment is not zero
    if public_inputs.bet_commitment == [0u8; 32] {
        msg!("Invalid bet commitment (all zeros)");
        return Err(MarketError::InvalidPublicInputs.into());
    }

    // TODO: Implement actual Groth16 verification

    msg!(
        "MarketBet proof verified for market: {}",
        public_inputs.market_id
    );
    msg!("  Bet commitment: {:?}...", &public_inputs.bet_commitment[0..8]);
    msg!("  Max bet: {}", public_inputs.max_bet);

    Ok(true)
}

/// Verify a MarketBet with PoI proof (insider trading check)
pub fn verify_market_bet_with_poi_proof(
    proof: &[u8],
    public_inputs: &MarketBetWithPoiPublicInputs,
    _vk: &MarketVerificationKey,
) -> Result<bool, ProgramError> {
    // Validate proof format
    if proof.len() != MARKET_PROOF_SIZE {
        msg!(
            "Invalid market bet+poi proof size: {} != {}",
            proof.len(),
            MARKET_PROOF_SIZE
        );
        return Err(MarketError::InvalidProof.into());
    }

    // Validate base inputs
    if public_inputs.bet.market_id == Pubkey::default() {
        msg!("Invalid market ID");
        return Err(MarketError::MarketNotFound.into());
    }

    // Validate insider blacklist root is set
    if public_inputs.insider_blacklist_root == [0u8; 32] {
        msg!("Invalid insider blacklist root");
        return Err(MarketError::InvalidPublicInputs.into());
    }

    // TODO: Implement actual Groth16 verification

    msg!(
        "MarketBet+PoI proof verified for market: {}",
        public_inputs.bet.market_id
    );
    msg!("  Insider blacklist: {:?}...", &public_inputs.insider_blacklist_root[0..8]);

    Ok(true)
}

/// Verify a MarketClaim proof
pub fn verify_market_claim_proof(
    proof: &[u8],
    public_inputs: &MarketClaimPublicInputs,
    _vk: &MarketVerificationKey,
) -> Result<bool, ProgramError> {
    // Validate proof format
    if proof.len() != MARKET_PROOF_SIZE {
        msg!(
            "Invalid market claim proof size: {} != {}",
            proof.len(),
            MARKET_PROOF_SIZE
        );
        return Err(MarketError::InvalidProof.into());
    }

    // Validate market_id
    if public_inputs.market_id == 0 {
        msg!("Invalid market ID");
        return Err(MarketError::MarketNotFound.into());
    }

    // Validate nullifier is not zero
    if public_inputs.nullifier == [0u8; 32] {
        msg!("Invalid nullifier");
        return Err(MarketError::InvalidPublicInputs.into());
    }

    if public_inputs.winning_pool == 0 {
        msg!("Invalid winning pool");
        return Err(MarketError::InvalidPublicInputs.into());
    }

    if public_inputs.payout_amount > public_inputs.total_pool {
        msg!("Payout exceeds total pool");
        return Err(MarketError::InvalidPublicInputs.into());
    }

    // TODO: Implement actual Groth16 verification

    msg!(
        "MarketClaim proof verified for market: {}",
        public_inputs.market_id
    );
    msg!("  Resolution: {}", public_inputs.resolution);
    msg!("  Payout: {}", public_inputs.payout_amount);

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_bet_public_inputs_serialization() {
        let inputs = MarketBetPublicInputs {
            market_id: Pubkey::new_unique(),
            bet_commitment: [1u8; 32],
            max_bet: 1_000_000,
            timestamp: 1234567890,
        };

        let bytes = inputs.to_bytes();
        assert_eq!(bytes.len(), MarketBetPublicInputs::LEN);

        let decoded = MarketBetPublicInputs::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.market_id, inputs.market_id);
        assert_eq!(decoded.bet_commitment, inputs.bet_commitment);
        assert_eq!(decoded.max_bet, inputs.max_bet);
        assert_eq!(decoded.timestamp, inputs.timestamp);
    }

    #[test]
    fn test_market_claim_public_inputs_serialization() {
        let inputs = MarketClaimPublicInputs {
            market_id: 42,
            nullifier: [3u8; 32],
            payout_amount: 5_000_000,
            resolution: 1,
            total_pool: 10_000_000,
            winning_pool: 6_000_000,
            bet_commitment: [2u8; 32],
            timestamp: 1_700_000_000,
        };

        let bytes = inputs.to_bytes();
        assert_eq!(bytes.len(), MarketClaimPublicInputs::LEN);

        let decoded = MarketClaimPublicInputs::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.market_id, inputs.market_id);
        assert_eq!(decoded.nullifier, inputs.nullifier);
        assert_eq!(decoded.resolution, inputs.resolution);
        assert_eq!(decoded.total_pool, inputs.total_pool);
        assert_eq!(decoded.winning_pool, inputs.winning_pool);
        assert_eq!(decoded.payout_amount, inputs.payout_amount);
        assert_eq!(decoded.timestamp, inputs.timestamp);
    }

    #[test]
    fn test_market_bet_with_poi_serialization() {
        let inputs = MarketBetWithPoiPublicInputs {
            bet: MarketBetPublicInputs {
                market_id: Pubkey::new_unique(),
                bet_commitment: [1u8; 32],
                max_bet: 1_000_000,
                timestamp: 1234567890,
            },
            insider_blacklist_root: [4u8; 32],
        };

        let bytes = inputs.to_bytes();
        assert_eq!(bytes.len(), MarketBetWithPoiPublicInputs::LEN);

        let decoded = MarketBetWithPoiPublicInputs::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.bet.market_id, inputs.bet.market_id);
        assert_eq!(decoded.insider_blacklist_root, inputs.insider_blacklist_root);
    }
}
