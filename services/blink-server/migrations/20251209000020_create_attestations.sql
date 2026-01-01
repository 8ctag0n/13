-- Create attestations table for storing ZK proof verification witnesses
CREATE TABLE attestations (
    id BIGSERIAL PRIMARY KEY,
    job_id BIGINT NOT NULL REFERENCES zk_jobs(job_id) ON DELETE CASCADE,
    circuit_type SMALLINT NOT NULL,
    witness JSONB NOT NULL,
    vk_hash VARCHAR(64) NOT NULL,
    verification_result BOOLEAN NOT NULL,
    verification_time_ms INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    used_for_dispute BOOLEAN DEFAULT FALSE,
    dispute_tx_signature VARCHAR(88)
);

-- Indexes for efficient queries
CREATE INDEX idx_attestations_job_id ON attestations(job_id);
CREATE INDEX idx_attestations_vk_hash ON attestations(vk_hash);
CREATE INDEX idx_attestations_circuit_type ON attestations(circuit_type);
CREATE INDEX idx_attestations_verification_result ON attestations(verification_result);
CREATE INDEX idx_attestations_used_for_dispute ON attestations(used_for_dispute) WHERE used_for_dispute = TRUE;

-- Add comment for table
COMMENT ON TABLE attestations IS 'ZK proof verification attestations for off-chain verification and potential on-chain disputes';
COMMENT ON COLUMN attestations.witness IS 'JSONB containing proof points, public inputs, and verification result for regenerating verification';
COMMENT ON COLUMN attestations.vk_hash IS 'Keccak256 hash of verification key JSON for identifying which VK was used';
COMMENT ON COLUMN attestations.used_for_dispute IS 'Flag indicating if this attestation was submitted in an on-chain dispute';
