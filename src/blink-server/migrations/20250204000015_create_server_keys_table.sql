-- Create server_keys table for storing pre-uploaded TFHE server keys
-- The hash is a Blake2s256 hash of the server key bytes (hex-encoded)
-- Server keys are ~117MB and are uploaded separately to avoid timeouts
CREATE TABLE IF NOT EXISTS server_keys (
    id BIGSERIAL PRIMARY KEY,
    hash TEXT NOT NULL UNIQUE,
    data BYTEA NOT NULL,
    size_bytes BIGINT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Index for fast lookup by hash
CREATE INDEX IF NOT EXISTS idx_server_keys_hash ON server_keys(hash);

-- Comment for documentation
COMMENT ON TABLE server_keys IS 'Pre-uploaded TFHE server keys, indexed by Blake2s256 hash';
COMMENT ON COLUMN server_keys.hash IS 'Blake2s256 hash of server key bytes (hex-encoded)';
COMMENT ON COLUMN server_keys.data IS 'Raw server key bytes (bincode-serialized tfhe::ServerKey)';
COMMENT ON COLUMN server_keys.size_bytes IS 'Size of data in bytes for quick reference';
