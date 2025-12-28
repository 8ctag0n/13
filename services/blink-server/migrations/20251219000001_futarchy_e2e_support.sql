-- Migration: Futarchy E2E Anti-Spam Support
-- Adds fields needed for ciphertext validation flow and TX tracking

-- =============================================================================
-- Extend futarchy_ciphertexts table for anti-spam tracking
-- =============================================================================

-- Add server_key_hash reference for validation
ALTER TABLE futarchy_ciphertexts
    ADD COLUMN IF NOT EXISTS server_key_hash VARCHAR(64);

-- Add TX tracking fields
ALTER TABLE futarchy_ciphertexts
    ADD COLUMN IF NOT EXISTS tx_signature VARCHAR(88),
    ADD COLUMN IF NOT EXISTS slot BIGINT,
    ADD COLUMN IF NOT EXISTS status VARCHAR(20) DEFAULT 'pending';

-- Add validation status constraint
ALTER TABLE futarchy_ciphertexts
    DROP CONSTRAINT IF EXISTS valid_ciphertext_status;
ALTER TABLE futarchy_ciphertexts
    ADD CONSTRAINT valid_ciphertext_status
    CHECK (status IN ('pending', 'confirmed', 'failed'));

-- Add comment explaining new fields
COMMENT ON COLUMN futarchy_ciphertexts.server_key_hash IS
    'SHA256 hash of the server public key used for FHE validation';
COMMENT ON COLUMN futarchy_ciphertexts.tx_signature IS
    'Solana transaction signature when ciphertext was submitted on-chain';
COMMENT ON COLUMN futarchy_ciphertexts.status IS
    'Status: pending (awaiting TX), confirmed (TX confirmed), failed (TX failed)';

-- =============================================================================
-- Extend futarchy_positions table for on-chain tracking
-- =============================================================================

-- Add Position PDA tracking (on-chain account address)
ALTER TABLE futarchy_positions
    ADD COLUMN IF NOT EXISTS position_pda VARCHAR(44);

-- Add index for Position PDA lookups
CREATE INDEX IF NOT EXISTS idx_futarchy_positions_pda
    ON futarchy_positions(position_pda) WHERE position_pda IS NOT NULL;

-- Add comment
COMMENT ON COLUMN futarchy_positions.position_pda IS
    'On-chain Position account PDA (Program Derived Address)';

-- =============================================================================
-- Create anti_spam_validations table (optional tracking table)
-- =============================================================================

-- This table tracks the validation flow for debugging/auditing
CREATE TABLE IF NOT EXISTS futarchy_anti_spam_validations (
    id SERIAL PRIMARY KEY,

    -- Ciphertext being validated
    ciphertext_hash VARCHAR(64) NOT NULL REFERENCES futarchy_ciphertexts(hash),

    -- User info
    user_pubkey VARCHAR(44) NOT NULL,

    -- Server key used for validation
    server_key_hash VARCHAR(64) NOT NULL,

    -- Validation result
    is_valid BOOLEAN,
    validation_error TEXT,

    -- TX info (once submitted)
    tx_signature VARCHAR(88),
    position_pda VARCHAR(44),

    -- Timing
    validated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    submitted_at TIMESTAMPTZ,
    confirmed_at TIMESTAMPTZ,

    -- Status
    status VARCHAR(20) NOT NULL DEFAULT 'validated',

    CONSTRAINT valid_anti_spam_status CHECK (
        status IN ('validated', 'tx_submitted', 'tx_confirmed', 'tx_failed', 'rejected')
    )
);

-- Indexes for anti_spam_validations
CREATE INDEX IF NOT EXISTS idx_futarchy_anti_spam_ciphertext
    ON futarchy_anti_spam_validations(ciphertext_hash);
CREATE INDEX IF NOT EXISTS idx_futarchy_anti_spam_user
    ON futarchy_anti_spam_validations(user_pubkey);
CREATE INDEX IF NOT EXISTS idx_futarchy_anti_spam_status
    ON futarchy_anti_spam_validations(status);
CREATE INDEX IF NOT EXISTS idx_futarchy_anti_spam_tx
    ON futarchy_anti_spam_validations(tx_signature)
    WHERE tx_signature IS NOT NULL;

COMMENT ON TABLE futarchy_anti_spam_validations IS
    'Tracks the anti-spam validation flow: ciphertext validation → TX submission → confirmation';

-- =============================================================================
-- Update existing ciphertexts to have default status
-- =============================================================================

-- Set existing ciphertexts to 'confirmed' (backward compatibility)
UPDATE futarchy_ciphertexts
SET status = 'confirmed'
WHERE status IS NULL;

-- Make status NOT NULL now that we have defaults
ALTER TABLE futarchy_ciphertexts
    ALTER COLUMN status SET NOT NULL;

-- =============================================================================
-- Add index for ciphertext hash lookups (performance)
-- =============================================================================

-- The hash column is already PRIMARY KEY, so it's indexed by default
-- But we add a specific index for server_key_hash queries
CREATE INDEX IF NOT EXISTS idx_futarchy_ciphertexts_server_key
    ON futarchy_ciphertexts(server_key_hash)
    WHERE server_key_hash IS NOT NULL;

-- Add composite index for prover lookups: (status, created_at)
CREATE INDEX IF NOT EXISTS idx_futarchy_ciphertexts_status_created
    ON futarchy_ciphertexts(status, created_at DESC);

-- =============================================================================
-- Summary
-- =============================================================================

-- This migration adds:
-- 1. server_key_hash to futarchy_ciphertexts (for validation tracking)
-- 2. tx_signature, slot, status to futarchy_ciphertexts (TX lifecycle)
-- 3. position_pda to futarchy_positions (on-chain account reference)
-- 4. futarchy_anti_spam_validations table (optional audit trail)
-- 5. Indexes for efficient lookups
