-- Create index for cleanup queries on used_nonces
CREATE INDEX idx_used_at ON used_nonces(used_at);
