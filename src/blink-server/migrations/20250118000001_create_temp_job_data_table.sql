-- Create temp_job_data table for storing job data before on-chain confirmation
CREATE TABLE temp_job_data (
    id BIGSERIAL PRIMARY KEY,
    job_id BIGINT NOT NULL UNIQUE,
    creator_pubkey TEXT NOT NULL,
    encrypted_data BYTEA NOT NULL,
    server_key BYTEA NOT NULL,
    operation TEXT NOT NULL,
    operation_value SMALLINT NOT NULL,
    price_lamports BIGINT NOT NULL,
    required_provers SMALLINT NOT NULL,
    consensus_threshold SMALLINT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending_tx',
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP NOT NULL DEFAULT NOW() + INTERVAL '24 hours'
);
