-- Create fhe_results table for storing FHE computation results
-- Similar to witnesses table - provers upload encrypted results, clients retrieve by commitment
CREATE TABLE fhe_results (
    id BIGSERIAL PRIMARY KEY,
    commitment TEXT NOT NULL UNIQUE,
    data BYTEA NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Index for fast lookup by commitment
CREATE INDEX idx_fhe_results_commitment ON fhe_results(commitment);
