-- Futarchy Markets Tables
-- Stores prediction market state and FHE ciphertexts for encrypted betting pools

-- =============================================================================
-- Markets Table
-- =============================================================================

CREATE TABLE IF NOT EXISTS futarchy_markets (
    -- Market identifier (Solana PDA pubkey as base58 string)
    id VARCHAR(44) PRIMARY KEY,

    -- Market question/proposal (hashed on-chain, stored plaintext here for display)
    question TEXT NOT NULL,
    question_hash VARCHAR(64) NOT NULL,

    -- Creator (Solana pubkey)
    creator VARCHAR(44) NOT NULL,

    -- Market configuration
    oracle VARCHAR(44) NOT NULL,
    resolution_window_secs BIGINT NOT NULL DEFAULT 86400,
    max_bet_lamports BIGINT NOT NULL DEFAULT 1000000000,

    -- Pool totals (encrypted on-chain, these are plaintext aggregates for UI)
    yes_pool_lamports BIGINT NOT NULL DEFAULT 0,
    no_pool_lamports BIGINT NOT NULL DEFAULT 0,

    -- Status
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    outcome BOOLEAN,  -- NULL = unresolved, true = YES won, false = NO won

    -- Timestamps (with timezone for chrono compatibility)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ends_at TIMESTAMPTZ,
    settled_at TIMESTAMPTZ,

    -- On-chain references
    tx_signature VARCHAR(88),
    slot BIGINT,

    CONSTRAINT valid_market_status CHECK (status IN ('active', 'resolved', 'cancelled'))
);

-- =============================================================================
-- Ciphertexts Table (FHE encrypted values)
-- =============================================================================

CREATE TABLE IF NOT EXISTS futarchy_ciphertexts (
    -- SHA256 hash of ciphertext bytes (hex string)
    hash VARCHAR(64) PRIMARY KEY,

    -- The encrypted data (FheUint64 serialized, ~500KB each)
    ciphertext BYTEA NOT NULL,

    -- Type: 'bet' for individual bets, 'pool' for aggregated pool
    ciphertext_type VARCHAR(10) NOT NULL DEFAULT 'bet',

    -- Associated market (nullable for standalone ciphertexts)
    market_id VARCHAR(44) REFERENCES futarchy_markets(id),

    -- For pool ciphertexts: which side (YES=true, NO=false)
    side BOOLEAN,

    -- Version for pool updates (incremented each time pool is updated)
    version INTEGER NOT NULL DEFAULT 0,

    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by VARCHAR(44),  -- Who uploaded this ciphertext

    CONSTRAINT valid_ciphertext_type CHECK (ciphertext_type IN ('bet', 'pool'))
);

-- =============================================================================
-- Positions Table (user bets)
-- =============================================================================

CREATE TABLE IF NOT EXISTS futarchy_positions (
    id SERIAL PRIMARY KEY,

    -- Market reference
    market_id VARCHAR(44) NOT NULL REFERENCES futarchy_markets(id),

    -- Bettor (Solana pubkey)
    bettor VARCHAR(44) NOT NULL,

    -- Bet details
    side BOOLEAN NOT NULL,  -- true = YES, false = NO
    amount_lamports BIGINT NOT NULL,

    -- If encrypted bet was used
    encrypted_amount_hash VARCHAR(64) REFERENCES futarchy_ciphertexts(hash),

    -- On-chain reference
    tx_signature VARCHAR(88),
    slot BIGINT,

    -- Status
    status VARCHAR(20) NOT NULL DEFAULT 'pending',

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT valid_position_status CHECK (status IN ('pending', 'confirmed', 'settled', 'cancelled')),
    CONSTRAINT unique_bettor_market_side UNIQUE (market_id, bettor, side)
);

-- =============================================================================
-- FHE Jobs Table (pool update jobs for prover-node)
-- =============================================================================

CREATE TABLE IF NOT EXISTS futarchy_fhe_jobs (
    id SERIAL PRIMARY KEY,

    -- Job reference (matches on-chain job if created)
    job_id BIGINT,

    -- Market and side being updated
    market_id VARCHAR(44) NOT NULL REFERENCES futarchy_markets(id),
    side BOOLEAN NOT NULL,

    -- Input ciphertexts
    pool_ciphertext_hash VARCHAR(64) NOT NULL,
    bet_ciphertext_hash VARCHAR(64) NOT NULL,

    -- Output ciphertext (after prover computes pool + bet)
    result_ciphertext_hash VARCHAR(64),

    -- Status
    status VARCHAR(20) NOT NULL DEFAULT 'pending',

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ,

    CONSTRAINT valid_fhe_job_status CHECK (status IN ('pending', 'processing', 'completed', 'failed'))
);

-- =============================================================================
-- Indexes
-- =============================================================================

CREATE INDEX IF NOT EXISTS idx_futarchy_markets_creator ON futarchy_markets(creator);
CREATE INDEX IF NOT EXISTS idx_futarchy_markets_status ON futarchy_markets(status);
CREATE INDEX IF NOT EXISTS idx_futarchy_markets_created ON futarchy_markets(created_at DESC);

CREATE INDEX IF NOT EXISTS idx_futarchy_ciphertexts_market ON futarchy_ciphertexts(market_id);
CREATE INDEX IF NOT EXISTS idx_futarchy_ciphertexts_type ON futarchy_ciphertexts(ciphertext_type);
CREATE INDEX IF NOT EXISTS idx_futarchy_ciphertexts_pool ON futarchy_ciphertexts(market_id, side, version DESC)
    WHERE ciphertext_type = 'pool';

CREATE INDEX IF NOT EXISTS idx_futarchy_positions_market ON futarchy_positions(market_id);
CREATE INDEX IF NOT EXISTS idx_futarchy_positions_bettor ON futarchy_positions(bettor);
CREATE INDEX IF NOT EXISTS idx_futarchy_positions_market_side ON futarchy_positions(market_id, side);

CREATE INDEX IF NOT EXISTS idx_futarchy_fhe_jobs_market ON futarchy_fhe_jobs(market_id);
CREATE INDEX IF NOT EXISTS idx_futarchy_fhe_jobs_status ON futarchy_fhe_jobs(status);

-- =============================================================================
-- Comments
-- =============================================================================

COMMENT ON TABLE futarchy_markets IS 'Futarchy prediction markets with FHE-encrypted betting pools';
COMMENT ON TABLE futarchy_ciphertexts IS 'FHE ciphertexts (FheUint64) for encrypted bets and pool totals';
COMMENT ON TABLE futarchy_positions IS 'User betting positions in futarchy markets';
COMMENT ON TABLE futarchy_fhe_jobs IS 'FHE computation jobs for pool updates (processed by prover-node)';
