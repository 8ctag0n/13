-- x402 Anti-Spam Payment Layer Tables
-- Creates tables for quotes and payment tokens

-- Table: x402_quotes
-- Stores price quotes before payment
CREATE TABLE IF NOT EXISTS x402_quotes (
    id SERIAL PRIMARY KEY,
    quote_id VARCHAR(36) NOT NULL UNIQUE,  -- UUID
    circuit_type SMALLINT NOT NULL,
    payer VARCHAR(44) NOT NULL,             -- Solana pubkey
    price_lamports BIGINT NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    used BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Index for quote lookup
CREATE INDEX IF NOT EXISTS idx_x402_quotes_quote_id ON x402_quotes(quote_id);
CREATE INDEX IF NOT EXISTS idx_x402_quotes_payer ON x402_quotes(payer);
CREATE INDEX IF NOT EXISTS idx_x402_quotes_expires ON x402_quotes(expires_at);

-- Table: x402_tokens
-- Stores payment tokens after successful payment
CREATE TABLE IF NOT EXISTS x402_tokens (
    id SERIAL PRIMARY KEY,
    token_id VARCHAR(36) NOT NULL UNIQUE,   -- UUID
    quote_id VARCHAR(36) NOT NULL,
    payer VARCHAR(44) NOT NULL,              -- Solana pubkey
    circuit_type SMALLINT NOT NULL,
    amount_paid BIGINT NOT NULL,
    tx_signature VARCHAR(88) NOT NULL,       -- Transaction signature
    expires_at TIMESTAMP NOT NULL,
    used BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Index for token lookup
CREATE INDEX IF NOT EXISTS idx_x402_tokens_token_id ON x402_tokens(token_id);
CREATE INDEX IF NOT EXISTS idx_x402_tokens_payer ON x402_tokens(payer);
CREATE INDEX IF NOT EXISTS idx_x402_tokens_expires ON x402_tokens(expires_at);
CREATE INDEX IF NOT EXISTS idx_x402_tokens_used ON x402_tokens(used);

-- Add token_id column to witnesses table for tracking
ALTER TABLE witnesses ADD COLUMN IF NOT EXISTS token_id VARCHAR(36);
CREATE INDEX IF NOT EXISTS idx_witnesses_token_id ON witnesses(token_id);
