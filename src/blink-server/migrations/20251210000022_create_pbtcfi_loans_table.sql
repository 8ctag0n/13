-- pBTCFi Loans Table
-- Stores loan state (both public and encrypted private data)
-- Privacy: commitments + ciphertexts stored, never raw values

CREATE TABLE IF NOT EXISTS pbtcfi_loans (
    -- Loan lifecycle identifier (u256 from Cairo as hex string)
    loan_id VARCHAR(64) PRIMARY KEY,

    -- Borrower (Starknet ContractAddress)
    borrower VARCHAR(66) NOT NULL,

    -- Status tracking
    status VARCHAR(30) NOT NULL DEFAULT 'pending',

    -- PRIVATE STORAGE: Commitments (Pedersen hashes)
    btc_commitment VARCHAR(64) NOT NULL,           -- Commitment of BTC amount
    plst_commitment VARCHAR(64),                    -- Commitment of PLST borrowed (NULL until activated)
    ltv_commitment VARCHAR(64),                     -- Commitment of LTV ratio (NULL until activated)

    -- PRIVATE STORAGE: Encrypted values (ElGamal ciphertext tuples)
    btc_encrypted_c1 VARCHAR(64) NOT NULL,          -- ElGamal BTC amount part 1 (felt252)
    btc_encrypted_c2 VARCHAR(64) NOT NULL,          -- ElGamal BTC amount part 2 (felt252)
    plst_encrypted_c1 VARCHAR(64),                  -- ElGamal PLST borrowed part 1
    plst_encrypted_c2 VARCHAR(64),                  -- ElGamal PLST borrowed part 2

    -- Collateral proof hash
    collateral_hash VARCHAR(64),                    -- Hash of UTXO proof (NULL until registered)

    -- Timestamps (Unix timestamps from Cairo contracts)
    created_at BIGINT NOT NULL,                     -- When loan was created
    activated_at BIGINT,                            -- When loan was activated
    repaid_at BIGINT,                               -- When loan was fully repaid
    liquidated_at BIGINT,                           -- When loan was liquidated

    -- Sync metadata
    synced_at TIMESTAMP DEFAULT NOW(),              -- Last sync from Starknet

    -- Constraints
    CONSTRAINT valid_loan_status CHECK (status IN
        ('pending', 'collateral_registered', 'active', 'repaid', 'liquidated')
    )
);

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_pbtcfi_loans_borrower ON pbtcfi_loans(borrower);
CREATE INDEX IF NOT EXISTS idx_pbtcfi_loans_status ON pbtcfi_loans(status);
CREATE INDEX IF NOT EXISTS idx_pbtcfi_loans_created ON pbtcfi_loans(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_pbtcfi_loans_synced ON pbtcfi_loans(synced_at DESC);

-- Comments for documentation
COMMENT ON TABLE pbtcfi_loans IS 'pBTCFi loan records with encrypted private data (commitments + ciphertexts)';
COMMENT ON COLUMN pbtcfi_loans.loan_id IS 'Unique loan identifier from Cairo contract (u256 as hex)';
COMMENT ON COLUMN pbtcfi_loans.borrower IS 'Starknet ContractAddress of borrower (0x + 64 hex chars)';
COMMENT ON COLUMN pbtcfi_loans.btc_commitment IS 'Pedersen commitment of BTC collateral amount (64 hex chars)';
COMMENT ON COLUMN pbtcfi_loans.btc_encrypted_c1 IS 'ElGamal-encrypted BTC amount part 1 (felt252 as hex)';
COMMENT ON COLUMN pbtcfi_loans.btc_encrypted_c2 IS 'ElGamal-encrypted BTC amount part 2 (felt252 as hex)';
COMMENT ON COLUMN pbtcfi_loans.status IS 'Current loan status: pending → collateral_registered → active → repaid/liquidated';
