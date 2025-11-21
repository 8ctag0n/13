-- Add wZEC payment support to temp_job_data table
-- Migration 007: Add payment_token_mint column

ALTER TABLE temp_job_data
ADD COLUMN payment_token_mint TEXT NULL,
ADD COLUMN payment_method TEXT NOT NULL DEFAULT 'SOL';

-- Add check constraint to ensure valid payment methods
ALTER TABLE temp_job_data
ADD CONSTRAINT check_payment_method
CHECK (payment_method IN ('SOL', 'wZEC'));

-- Add comment for documentation
COMMENT ON COLUMN temp_job_data.payment_token_mint IS 'SPL token mint address for token payments (e.g. wZEC). NULL for SOL payments.';
COMMENT ON COLUMN temp_job_data.payment_method IS 'Payment method: SOL (native) or wZEC (SPL token)';

-- Create index for querying by payment method
CREATE INDEX idx_temp_job_data_payment_method ON temp_job_data(payment_method);
