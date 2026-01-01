-- Create provers table for syncing on-chain ProverAccount data
-- This enables real-time tracking of active provers in the network

CREATE TABLE IF NOT EXISTS provers (
    -- PDA address (unique identifier from blockchain)
    pubkey TEXT PRIMARY KEY,

    -- Prover identity
    authority_pubkey TEXT NOT NULL,

    -- State from ProverAccount
    stake_amount BIGINT NOT NULL DEFAULT 0,
    reputation_score INTEGER NOT NULL DEFAULT 1000,
    is_active BOOLEAN NOT NULL DEFAULT true,

    -- Performance stats
    total_jobs_completed BIGINT NOT NULL DEFAULT 0,
    total_jobs_failed BIGINT NOT NULL DEFAULT 0,
    avg_completion_time_secs INTEGER NOT NULL DEFAULT 0,
    total_earnings_lamports BIGINT NOT NULL DEFAULT 0,

    -- Encryption key (hex encoded)
    encryption_pubkey TEXT,

    -- Timestamps
    registration_timestamp TIMESTAMP NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMP DEFAULT NOW(),
    synced_at TIMESTAMP DEFAULT NOW(),

    -- Constraints
    CONSTRAINT valid_reputation CHECK (reputation_score >= 0 AND reputation_score <= 1000)
);

-- Indexes for common queries
CREATE INDEX idx_provers_active ON provers(is_active) WHERE is_active = true;
CREATE INDEX idx_provers_authority ON provers(authority_pubkey);
CREATE INDEX idx_provers_reputation ON provers(reputation_score DESC);
CREATE INDEX idx_provers_synced ON provers(synced_at DESC);
