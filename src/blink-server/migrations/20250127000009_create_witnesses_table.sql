-- Create witnesses table for storing encrypted witness data
-- The commitment is a Blake2b hash of the witness bytes
CREATE TABLE witnesses (
    id BIGSERIAL PRIMARY KEY,
    commitment TEXT NOT NULL UNIQUE,
    data BYTEA NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Index for fast lookup by commitment
CREATE INDEX idx_witnesses_commitment ON witnesses(commitment);
