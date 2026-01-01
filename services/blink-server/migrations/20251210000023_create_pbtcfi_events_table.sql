-- pBTCFi Events Table
-- Stores off-chain events emitted by Starknet contracts
-- Used for syncing loan state and auditing

CREATE TABLE IF NOT EXISTS pbtcfi_events (
    -- Event tracking
    id BIGSERIAL PRIMARY KEY,

    -- Event classification
    event_type VARCHAR(30) NOT NULL,               -- 'LoanCreated', 'LoanActivated', etc.
    loan_id VARCHAR(66) NOT NULL,                  -- Reference to loan (0x + 64 hex, not FK for out-of-order processing)

    -- Block information
    block_number BIGINT NOT NULL,                  -- Starknet block number
    transaction_hash VARCHAR(66) NOT NULL,         -- Transaction hash (0x + 64 hex)
    event_index INTEGER NOT NULL,                  -- Index of event within transaction

    -- Event data (polymorphic JSON for different event types)
    event_data JSONB NOT NULL,                     -- Full event payload as JSON

    -- Timestamps
    timestamp BIGINT NOT NULL,                     -- Unix timestamp from event
    synced_at TIMESTAMP DEFAULT NOW(),             -- When synced to database

    -- Unique constraint: one event per transaction+index
    UNIQUE(transaction_hash, event_index)
);

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_pbtcfi_events_loan_id ON pbtcfi_events(loan_id);
CREATE INDEX IF NOT EXISTS idx_pbtcfi_events_type ON pbtcfi_events(event_type);
CREATE INDEX IF NOT EXISTS idx_pbtcfi_events_block ON pbtcfi_events(block_number DESC);
CREATE INDEX IF NOT EXISTS idx_pbtcfi_events_tx_hash ON pbtcfi_events(transaction_hash);
CREATE INDEX IF NOT EXISTS idx_pbtcfi_events_synced ON pbtcfi_events(synced_at DESC);

-- Comments for documentation
COMMENT ON TABLE pbtcfi_events IS 'Events emitted by pBTCFi Cairo contracts for state synchronization';
COMMENT ON COLUMN pbtcfi_events.event_type IS 'Type of event: LoanCreated, CollateralRegistered, LoanActivated, LoanRepaid, LoanLiquidated';
COMMENT ON COLUMN pbtcfi_events.event_data IS 'JSON-serialized event payload (structure depends on event_type)';
COMMENT ON COLUMN pbtcfi_events.event_index IS 'Index of event within transaction (for uniqueness)';
