-- Add FHE processing columns to pbtcfi_loans
-- Enables tracking of FHE verification jobs for each loan

-- FHE job status tracking
ALTER TABLE pbtcfi_loans
ADD COLUMN IF NOT EXISTS fhe_status VARCHAR(30) DEFAULT NULL;

ALTER TABLE pbtcfi_loans
ADD COLUMN IF NOT EXISTS fhe_prover_id VARCHAR(64) DEFAULT NULL;

ALTER TABLE pbtcfi_loans
ADD COLUMN IF NOT EXISTS fhe_error TEXT DEFAULT NULL;

-- FHE job timestamps
ALTER TABLE pbtcfi_loans
ADD COLUMN IF NOT EXISTS fhe_created_at TIMESTAMP DEFAULT NULL;

ALTER TABLE pbtcfi_loans
ADD COLUMN IF NOT EXISTS fhe_started_at TIMESTAMP DEFAULT NULL;

ALTER TABLE pbtcfi_loans
ADD COLUMN IF NOT EXISTS fhe_completed_at TIMESTAMP DEFAULT NULL;

-- Index for FHE job queries
CREATE INDEX IF NOT EXISTS idx_pbtcfi_loans_fhe_status ON pbtcfi_loans(fhe_status);
CREATE INDEX IF NOT EXISTS idx_pbtcfi_loans_fhe_created ON pbtcfi_loans(fhe_created_at);

-- Update status constraint to include FHE statuses
ALTER TABLE pbtcfi_loans DROP CONSTRAINT IF EXISTS valid_loan_status;
ALTER TABLE pbtcfi_loans
ADD CONSTRAINT valid_loan_status CHECK (status IN
    ('pending', 'collateral_registered', 'active', 'repaid', 'liquidated')
);

-- Comments
COMMENT ON COLUMN pbtcfi_loans.fhe_status IS 'FHE job status: fhe_pending, fhe_processing, fhe_completed, fhe_failed';
COMMENT ON COLUMN pbtcfi_loans.fhe_prover_id IS 'ID of prover processing the FHE verification';
COMMENT ON COLUMN pbtcfi_loans.fhe_error IS 'Error message if FHE processing failed';
