-- Table for tracking used nonces (anti-replay attack)
CREATE TABLE used_nonces (
    nonce TEXT PRIMARY KEY,
    used_at TIMESTAMP NOT NULL DEFAULT NOW()
);
