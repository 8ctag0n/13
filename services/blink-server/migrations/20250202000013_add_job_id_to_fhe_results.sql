-- Add job_id column to fhe_results table
-- This allows 1:N relationship between jobs and FHE results (multi-prover consensus)

ALTER TABLE fhe_results ADD COLUMN IF NOT EXISTS job_id BIGINT;

-- Create index for fast lookup by job_id
CREATE INDEX IF NOT EXISTS idx_fhe_results_job_id ON fhe_results(job_id);

-- Add prover_pubkey to track which prover submitted each result
ALTER TABLE fhe_results ADD COLUMN IF NOT EXISTS prover_pubkey TEXT;
