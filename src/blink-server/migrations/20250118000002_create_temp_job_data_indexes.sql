-- Create indexes for better query performance on temp_job_data
CREATE INDEX idx_job_id ON temp_job_data(job_id);
CREATE INDEX idx_creator ON temp_job_data(creator_pubkey);
CREATE INDEX idx_status ON temp_job_data(status);
CREATE INDEX idx_expires_at ON temp_job_data(expires_at);
