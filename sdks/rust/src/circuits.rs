use anyhow::{Context, Result};
use serde::Deserialize;
use std::{env, fs, path::PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CircuitId {
    ProofOfInnocence = 10,
    PrivateVote = 20,
    PrivateVoteWithPoI = 21,
    MarketBet = 30,
    MarketBetWithPoI = 31,
    MarketClaim = 32,
    PortfolioCompliance = 40,
    PortfolioNetWorth = 41,
}

#[derive(Debug, Clone)]
pub struct CircuitMetadata {
    pub id: CircuitId,
    pub name: &'static str,
    pub constraints: u64,
    pub public_inputs: &'static [&'static str],
    pub description: &'static str,
    pub vkey_filename: &'static str,
}

static CIRCUITS: &[CircuitMetadata] = &[
    CircuitMetadata {
        id: CircuitId::ProofOfInnocence,
        name: "ProofOfInnocence",
        constraints: 4_883,
        public_inputs: &["blacklist_root", "threshold", "timestamp"],
        description: "Prove wallet is NOT in blacklist",
        vkey_filename: "circuit_10_vkey.json",
    },
    CircuitMetadata {
        id: CircuitId::PrivateVote,
        name: "PrivateVote",
        constraints: 5_675,
        public_inputs: &["poll_id", "eligibility_root", "nullifier", "vote_commitment", "min_balance"],
        description: "Anonymous DAO voting with nullifier-based double-vote prevention",
        vkey_filename: "circuit_20_vkey.json",
    },
    CircuitMetadata {
        id: CircuitId::PrivateVoteWithPoI,
        name: "PrivateVoteWithPoI",
        constraints: 10_555,
        public_inputs: &[
            "poll_id",
            "eligibility_root",
            "nullifier",
            "vote_commitment",
            "min_balance",
            "blacklist_root",
        ],
        description: "Anonymous voting + conflict-of-interest blacklist proof",
        vkey_filename: "circuit_21_vkey.json",
    },
    CircuitMetadata {
        id: CircuitId::MarketBet,
        name: "MarketBet",
        constraints: 332,
        public_inputs: &["market_id", "bet_commitment", "max_bet", "timestamp"],
        description: "Private prediction market bet",
        vkey_filename: "circuit_30_vkey.json",
    },
    CircuitMetadata {
        id: CircuitId::MarketBetWithPoI,
        name: "MarketBetWithPoI",
        constraints: 5_212,
        public_inputs: &[
            "market_id",
            "bet_commitment",
            "max_bet",
            "timestamp",
            "insider_blacklist_root",
        ],
        description: "Private bet with insider-trading blacklist proof",
        vkey_filename: "circuit_31_vkey.json",
    },
    CircuitMetadata {
        id: CircuitId::MarketClaim,
        name: "MarketClaim",
        constraints: 510,
        public_inputs: &[
            "market_id",
            "winning_outcome",
            "bet_commitment",
            "claim_nullifier",
            "payout_amount",
        ],
        description: "Claim winnings proof with double-claim prevention",
        vkey_filename: "circuit_32_vkey.json",
    },
    CircuitMetadata {
        id: CircuitId::PortfolioCompliance,
        name: "PortfolioCompliance",
        constraints: 4_883,
        public_inputs: &["compliance_root", "threshold", "timestamp"],
        description: "Regulatory compliance proof (no exposure to sanctions list)",
        vkey_filename: "circuit_40_vkey.json",
    },
    CircuitMetadata {
        id: CircuitId::PortfolioNetWorth,
        name: "PortfolioNetWorth",
        constraints: 5_189,
        public_inputs: &["min_threshold", "price_oracle_root", "timestamp", "net_worth_commitment"],
        description: "Prove net worth above threshold without revealing amount",
        vkey_filename: "circuit_41_vkey.json",
    },
];

#[derive(Debug, Clone, Deserialize)]
pub struct VerificationKeyJson {
    pub protocol: String,
    pub curve: String,
    #[serde(rename = "vk_alpha_1")]
    pub vk_alpha_1: Vec<String>,
    #[serde(rename = "vk_beta_2")]
    pub vk_beta_2: Vec<Vec<String>>,
    #[serde(rename = "vk_gamma_2")]
    pub vk_gamma_2: Vec<Vec<String>>,
    #[serde(rename = "vk_delta_2")]
    pub vk_delta_2: Vec<Vec<String>>,
    #[serde(rename = "IC")]
    pub ic: Vec<Vec<String>>,
}

pub fn list_circuits() -> &'static [CircuitMetadata] {
    CIRCUITS
}

pub fn get_circuit(id: CircuitId) -> Option<&'static CircuitMetadata> {
    CIRCUITS.iter().find(|c| c.id == id)
}

pub fn vkey_path(id: CircuitId) -> PathBuf {
    let base = env::var("ZYBER_VKEY_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("circuits/verification_keys"));
    let filename = get_circuit(id)
        .map(|c| c.vkey_filename)
        .unwrap_or("unknown_vkey.json");
    base.join(filename)
}

pub fn load_vkey_json(id: CircuitId) -> Result<VerificationKeyJson> {
    let path = vkey_path(id);
    let data = fs::read_to_string(&path)
        .with_context(|| format!("failed to read vkey at {}", path.display()))?;
    let vkey: VerificationKeyJson =
        serde_json::from_str(&data).with_context(|| format!("failed to parse vkey {}", path.display()))?;
    Ok(vkey)
}
