-- ZK Jobs Table
-- Stores Zero-Knowledge proof jobs (circuit_type 10-49)
-- Separate from blockchain_jobs for cleaner separation of concerns

CREATE TABLE IF NOT EXISTS zk_jobs (
    -- Primary identifier
    id SERIAL PRIMARY KEY,
    job_id BIGINT NOT NULL UNIQUE,

    -- Creator info
    creator_pubkey VARCHAR(44) NOT NULL,

    -- Circuit info (10-49 for ZK)
    -- 10-19: Core (PoI, PoR)
    -- 20-29: Voting
    -- 30-39: Market
    -- 40-49: Portfolio
    circuit_type SMALLINT NOT NULL CHECK (circuit_type >= 10 AND circuit_type <= 49),

    -- Witness and proof data
    witness_commitment VARCHAR(64) NOT NULL,  -- Blake2s256 hash (32 bytes = 64 hex)
    public_inputs JSONB NOT NULL DEFAULT '[]',
    proof_hash VARCHAR(64),                   -- Set when proof is submitted

    -- Status tracking
    -- pending_tx: Job created, waiting for on-chain confirmation
    -- active: On-chain confirmed, waiting for prover
    -- proving: Prover claimed, generating proof
    -- completed: Proof verified on-chain
    -- failed: Job failed (timeout, invalid proof, etc)
    status VARCHAR(20) NOT NULL DEFAULT 'pending_tx'
        CHECK (status IN ('pending_tx', 'active', 'proving', 'completed', 'failed')),

    -- Payment (x402 integration)
    x402_token_id VARCHAR(36),
    price_lamports BIGINT NOT NULL DEFAULT 0,

    -- Transaction signatures
    create_tx_signature VARCHAR(88),
    confirm_tx_signature VARCHAR(88),

    -- Prover info (set when claimed)
    prover_pubkey VARCHAR(44),
    claimed_at TIMESTAMP,

    -- Timing
    timeout_seconds INTEGER NOT NULL DEFAULT 3600,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMP
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_zk_jobs_job_id ON zk_jobs(job_id);
CREATE INDEX IF NOT EXISTS idx_zk_jobs_status ON zk_jobs(status);
CREATE INDEX IF NOT EXISTS idx_zk_jobs_creator ON zk_jobs(creator_pubkey);
CREATE INDEX IF NOT EXISTS idx_zk_jobs_circuit_type ON zk_jobs(circuit_type);
CREATE INDEX IF NOT EXISTS idx_zk_jobs_created_at ON zk_jobs(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_zk_jobs_x402_token ON zk_jobs(x402_token_id);

-- Trigger for updated_at
CREATE OR REPLACE TRIGGER update_zk_jobs_updated_at
    BEFORE UPDATE ON zk_jobs
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Comments
COMMENT ON TABLE zk_jobs IS 'Zero-Knowledge proof jobs (circuit_type 10-49)';
COMMENT ON COLUMN zk_jobs.circuit_type IS '10-19: Core, 20-29: Voting, 30-39: Market, 40-49: Portfolio';
COMMENT ON COLUMN zk_jobs.witness_commitment IS 'Blake2s256 hash of witness data (64 hex chars)';
COMMENT ON COLUMN zk_jobs.public_inputs IS 'JSON array of public inputs for the circuit';
