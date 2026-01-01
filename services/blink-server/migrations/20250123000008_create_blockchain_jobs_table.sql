-- Tabla para jobs sincronizados desde la blockchain
-- Almacena el snapshot completo de JobAccount on-chain

CREATE TABLE IF NOT EXISTS blockchain_jobs (
    -- Primary identifier from on-chain JobAccount
    job_id BIGINT PRIMARY KEY,

    -- Account pubkey (PDA address)
    pubkey TEXT NOT NULL,

    -- Core job fields
    creator_pubkey TEXT NOT NULL,
    prover_pubkey TEXT,

    -- Status tracking
    status TEXT NOT NULL,
    circuit_type TEXT NOT NULL,

    -- Pricing
    price_lamports BIGINT NOT NULL,

    -- Timestamps (converted from i64 Unix timestamps)
    created_at TIMESTAMP NOT NULL,
    claimed_at TIMESTAMP,
    completed_at TIMESTAMP,
    timeout_at TIMESTAMP NOT NULL,

    -- FHE-specific fields (nullable for ZK jobs)
    required_provers SMALLINT,
    consensus_threshold SMALLINT,
    fhe_operation TEXT,

    -- Sync metadata
    synced_at TIMESTAMP DEFAULT NOW(),

    -- Constraints
    CONSTRAINT valid_status CHECK (status IN ('pending', 'claimed', 'completed', 'failed', 'cancelled'))
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_blockchain_jobs_status ON blockchain_jobs(status);
CREATE INDEX IF NOT EXISTS idx_blockchain_jobs_created ON blockchain_jobs(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_blockchain_jobs_synced ON blockchain_jobs(synced_at DESC);
CREATE INDEX IF NOT EXISTS idx_blockchain_jobs_creator ON blockchain_jobs(creator_pubkey);

-- Comment for documentation
COMMENT ON TABLE blockchain_jobs IS 'Jobs synchronized from Solana blockchain via periodic polling';
