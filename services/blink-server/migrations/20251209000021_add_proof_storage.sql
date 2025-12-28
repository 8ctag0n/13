-- Add proof storage columns for hybrid verification system
-- Allows users to download full proof for local verification with snarkjs

ALTER TABLE attestations
    ADD COLUMN proof_json JSONB,
    ADD COLUMN retention_expires_at TIMESTAMP DEFAULT (NOW() + INTERVAL '30 days');

-- Index for cleanup service to efficiently find expired proofs
CREATE INDEX idx_attestations_retention ON attestations(retention_expires_at)
    WHERE proof_json IS NOT NULL;

COMMENT ON COLUMN attestations.proof_json IS 'Full ZK proof JSON for trustless verification (snarkjs format). Cleared after retention period.';
COMMENT ON COLUMN attestations.retention_expires_at IS 'When proof_json will be cleared by cleanup service. Default 30 days from creation.';
