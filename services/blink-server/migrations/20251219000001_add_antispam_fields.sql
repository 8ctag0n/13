-- Add anti-spam flow columns to futarchy_ciphertexts table

ALTER TABLE futarchy_ciphertexts
ADD COLUMN IF NOT EXISTS server_key_hash VARCHAR(64),
ADD COLUMN IF NOT EXISTS tx_signature VARCHAR(88),
ADD COLUMN IF NOT EXISTS status VARCHAR(20) DEFAULT 'pending';

-- Add constraint for status
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'valid_ciphertext_status'
    ) THEN
        ALTER TABLE futarchy_ciphertexts
        ADD CONSTRAINT valid_ciphertext_status
        CHECK (status IN ('pending', 'confirmed', 'failed'));
    END IF;
END $$;

-- Add index for tx_signature lookups
CREATE INDEX IF NOT EXISTS idx_futarchy_ciphertexts_tx ON futarchy_ciphertexts(tx_signature);
CREATE INDEX IF NOT EXISTS idx_futarchy_ciphertexts_status ON futarchy_ciphertexts(status);

-- Add position_pda column to futarchy_positions for PDA tracking
ALTER TABLE futarchy_positions
ADD COLUMN IF NOT EXISTS position_pda VARCHAR(44);

CREATE INDEX IF NOT EXISTS idx_futarchy_positions_pda ON futarchy_positions(position_pda);

-- Optional: Anti-spam validation tracking table
CREATE TABLE IF NOT EXISTS futarchy_anti_spam_validations (
    id SERIAL PRIMARY KEY,
    ciphertext_hash VARCHAR(64) NOT NULL,
    user_pubkey VARCHAR(44) NOT NULL,
    server_key_hash VARCHAR(64) NOT NULL,
    is_valid BOOLEAN NOT NULL,
    validation_error TEXT,
    status VARCHAR(20) NOT NULL DEFAULT 'validated',
    tx_signature VARCHAR(88),
    position_pda VARCHAR(44),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    submitted_at TIMESTAMPTZ,
    confirmed_at TIMESTAMPTZ,

    CONSTRAINT valid_validation_status CHECK (status IN ('validated', 'rejected', 'tx_submitted', 'tx_confirmed', 'tx_failed'))
);

CREATE INDEX IF NOT EXISTS idx_validations_ciphertext ON futarchy_anti_spam_validations(ciphertext_hash);
CREATE INDEX IF NOT EXISTS idx_validations_user ON futarchy_anti_spam_validations(user_pubkey);
CREATE INDEX IF NOT EXISTS idx_validations_tx ON futarchy_anti_spam_validations(tx_signature);
CREATE INDEX IF NOT EXISTS idx_validations_status ON futarchy_anti_spam_validations(status);

COMMENT ON TABLE futarchy_anti_spam_validations IS 'Audit trail for anti-spam validation flow';
