-- pBTCFi Starknet Callbacks Table
-- Stores pending callbacks to execute on Starknet after FHE job completion
-- These are processed by a separate relayer or manually via starkli

CREATE TABLE IF NOT EXISTS pbtcfi_callbacks (
    id SERIAL PRIMARY KEY,

    -- Loan reference (0x + 64 hex = 66 chars)
    loan_id VARCHAR(66) NOT NULL REFERENCES pbtcfi_loans(loan_id),

    -- Callback type
    callback_type VARCHAR(50) NOT NULL,  -- 'mint_encrypted', 'update_balance', etc.

    -- Target contract (pLST address)
    target_contract VARCHAR(66) NOT NULL,

    -- Function to call
    function_name VARCHAR(100) NOT NULL,

    -- Call arguments (JSON array of felt252 values)
    call_args JSONB NOT NULL,

    -- Status tracking
    status VARCHAR(30) NOT NULL DEFAULT 'pending',

    -- Execution details
    tx_hash VARCHAR(66),           -- Starknet transaction hash when executed
    executed_at TIMESTAMP,
    error_message TEXT,

    -- Retry tracking
    retry_count INTEGER DEFAULT 0,
    last_retry_at TIMESTAMP,

    -- Timestamps
    created_at TIMESTAMP DEFAULT NOW(),

    -- Constraints
    CONSTRAINT valid_callback_status CHECK (status IN
        ('pending', 'processing', 'executed', 'failed')
    )
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_pbtcfi_callbacks_status ON pbtcfi_callbacks(status);
CREATE INDEX IF NOT EXISTS idx_pbtcfi_callbacks_loan ON pbtcfi_callbacks(loan_id);
CREATE INDEX IF NOT EXISTS idx_pbtcfi_callbacks_created ON pbtcfi_callbacks(created_at);

-- Comments
COMMENT ON TABLE pbtcfi_callbacks IS 'Pending Starknet callbacks for pBTCFi FHE job completions';
COMMENT ON COLUMN pbtcfi_callbacks.callback_type IS 'Type of callback: mint_encrypted, update_balance, etc.';
COMMENT ON COLUMN pbtcfi_callbacks.call_args IS 'JSON array of felt252 arguments for the Starknet call';
