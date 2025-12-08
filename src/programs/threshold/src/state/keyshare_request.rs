//! Key share request state

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Seeds for key share request PDA
pub const KEYSHARE_REQUEST_SEED: &[u8] = b"keyshare_request";

/// Timeout for key share requests (5 minutes)
pub const REQUEST_TIMEOUT_SECONDS: i64 = 300;

/// Minimum responses required to reconstruct threshold key (3-of-5)
pub const MIN_RESPONSES_REQUIRED: u8 = 3;

/// Maximum IPFS CID length
pub const MAX_CID_LENGTH: usize = 128;

/// Status of a key share request
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum RequestStatus {
    /// Waiting for validator responses
    Pending,

    /// Sufficient responses received, prover can reconstruct key
    Ready,

    /// Request timed out without sufficient responses
    Expired,

    /// Prover successfully used the key shares
    Completed,
}

impl Default for RequestStatus {
    fn default() -> Self {
        RequestStatus::Pending
    }
}

/// Key share request account
///
/// Created when a prover needs to decrypt witness data for proof generation.
/// Validators respond with their key shares, and once 3+ responses are received,
/// the prover can reconstruct the decryption key off-chain.
///
/// PDA: ["keyshare_request", job_id]
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct KeyShareRequest {
    /// ZK job that needs witness decryption
    pub job_id: Pubkey,

    /// Prover requesting the key shares
    pub prover: Pubkey,

    /// IPFS CID of the encrypted witness data
    pub encrypted_witness_cid: String,

    /// Number of validator responses received
    pub responses_received: u8,

    /// Current status of the request
    pub status: RequestStatus,

    /// Request creation timestamp
    pub created_at: i64,

    /// Request expiration timestamp
    pub expires_at: i64,

    /// Bump seed for PDA
    pub bump: u8,
}

impl KeyShareRequest {
    /// Size: 32 + 32 + (4 + 128) + 1 + 1 + 8 + 8 + 1 = 215 bytes
    /// Allocate 256 for future expansion
    pub const SIZE: usize = 256;

    pub fn new(
        job_id: Pubkey,
        prover: Pubkey,
        encrypted_witness_cid: String,
        created_at: i64,
        bump: u8,
    ) -> Self {
        Self {
            job_id,
            prover,
            encrypted_witness_cid,
            responses_received: 0,
            status: RequestStatus::Pending,
            created_at,
            expires_at: created_at + REQUEST_TIMEOUT_SECONDS,
            bump,
        }
    }

    /// Check if request has expired
    pub fn is_expired(&self, current_time: i64) -> bool {
        current_time >= self.expires_at
    }

    /// Check if request has sufficient responses
    pub fn has_sufficient_responses(&self) -> bool {
        self.responses_received >= MIN_RESPONSES_REQUIRED
    }

    /// Record a new validator response
    pub fn record_response(&mut self) {
        self.responses_received += 1;

        // Update status to Ready if we have enough responses
        if self.has_sufficient_responses() && self.status == RequestStatus::Pending {
            self.status = RequestStatus::Ready;
        }
    }

    /// Mark request as completed
    pub fn mark_completed(&mut self) {
        self.status = RequestStatus::Completed;
    }

    /// Mark request as expired
    pub fn mark_expired(&mut self) {
        if self.status == RequestStatus::Pending {
            self.status = RequestStatus::Expired;
        }
    }

    /// Check if request can accept new responses
    pub fn can_accept_responses(&self, current_time: i64) -> bool {
        matches!(self.status, RequestStatus::Pending | RequestStatus::Ready)
            && !self.is_expired(current_time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyshare_request_lifecycle() {
        let job_id = Pubkey::new_unique();
        let prover = Pubkey::new_unique();
        let cid = "QmTest123".to_string();
        let created_at = 1000;

        let mut request = KeyShareRequest::new(
            job_id,
            prover,
            cid.clone(),
            created_at,
            255,
        );

        assert_eq!(request.status, RequestStatus::Pending);
        assert_eq!(request.responses_received, 0);
        assert!(!request.has_sufficient_responses());

        // Add responses
        request.record_response();
        assert_eq!(request.responses_received, 1);
        assert_eq!(request.status, RequestStatus::Pending);

        request.record_response();
        assert_eq!(request.responses_received, 2);
        assert_eq!(request.status, RequestStatus::Pending);

        request.record_response();
        assert_eq!(request.responses_received, 3);
        assert_eq!(request.status, RequestStatus::Ready);
        assert!(request.has_sufficient_responses());

        // Mark completed
        request.mark_completed();
        assert_eq!(request.status, RequestStatus::Completed);
    }

    #[test]
    fn test_request_expiration() {
        let request = KeyShareRequest::new(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            "QmTest".to_string(),
            1000,
            255,
        );

        assert!(!request.is_expired(1000));
        assert!(!request.is_expired(1299));
        assert!(request.is_expired(1300));
        assert!(request.is_expired(2000));
    }
}
