-- pBTCFi Sync State Table
-- Tracks the last synced block number for incremental event polling
-- Single-row table (enforced by constraint)

CREATE TABLE IF NOT EXISTS pbtcfi_sync_state (
    -- Single row identifier
    id INTEGER PRIMARY KEY DEFAULT 1,

    -- Last block number successfully synced from Starknet
    last_synced_block BIGINT NOT NULL DEFAULT 0,

    -- Last update timestamp
    updated_at TIMESTAMP DEFAULT NOW(),

    -- Ensure only one row exists
    CONSTRAINT single_row CHECK (id = 1)
);

-- Initialize with block 0
INSERT INTO pbtcfi_sync_state (id, last_synced_block)
VALUES (1, 0)
ON CONFLICT (id) DO NOTHING;

-- Comments for documentation
COMMENT ON TABLE pbtcfi_sync_state IS 'Tracks last synced Starknet block for pBTCFi event polling (single row table)';
COMMENT ON COLUMN pbtcfi_sync_state.last_synced_block IS 'Last Starknet block number successfully processed';
COMMENT ON COLUMN pbtcfi_sync_state.id IS 'Always 1 (single row enforced by CHECK constraint)';
