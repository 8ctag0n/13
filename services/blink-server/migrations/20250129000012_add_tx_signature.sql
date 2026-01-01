-- Add tx_signature column to temp_job_data for tracking transaction signatures
ALTER TABLE temp_job_data
ADD COLUMN IF NOT EXISTS tx_signature TEXT;

-- Add tx_signature column to blockchain_jobs for tracking on-chain transaction signatures
ALTER TABLE blockchain_jobs
ADD COLUMN IF NOT EXISTS tx_signature TEXT;
