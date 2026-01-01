-- Historical network metrics (cumulative, never decreases)
-- This tracks lifetime stats that persist even after data cleanup

CREATE TABLE IF NOT EXISTS network_metrics (
    id INTEGER PRIMARY KEY DEFAULT 1 CHECK (id = 1),  -- Singleton row
    total_data_processed_bytes BIGINT NOT NULL DEFAULT 0,
    total_jobs_processed BIGINT NOT NULL DEFAULT 0,
    total_fhe_computations BIGINT NOT NULL DEFAULT 0,
    total_zk_proofs BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Insert initial row
INSERT INTO network_metrics (id) VALUES (1) ON CONFLICT DO NOTHING;

-- Create trigger to update updated_at
CREATE OR REPLACE FUNCTION update_network_metrics_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER network_metrics_updated_at
    BEFORE UPDATE ON network_metrics
    FOR EACH ROW
    EXECUTE FUNCTION update_network_metrics_timestamp();
