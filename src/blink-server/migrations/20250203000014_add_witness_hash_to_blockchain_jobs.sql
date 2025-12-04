-- Add witness_hash column to blockchain_jobs for relating jobs to witnesses
-- This enables smart cleanup: delete witness data when job completes/expires

ALTER TABLE blockchain_jobs
ADD COLUMN IF NOT EXISTS witness_hash TEXT;

-- Index for fast lookup when cleaning up witnesses
CREATE INDEX IF NOT EXISTS idx_blockchain_jobs_witness_hash
ON blockchain_jobs(witness_hash);

COMMENT ON COLUMN blockchain_jobs.witness_hash IS 'Blake2s-256 hash of witness data, matches witnesses.commitment';
