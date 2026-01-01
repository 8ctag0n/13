-- Add expected_count column to temp_job_data table
-- This field stores the expected count for operations like Sum, Average, etc.
ALTER TABLE temp_job_data
ADD COLUMN expected_count SMALLINT;

-- No NOT NULL constraint because existing jobs don't have this value
-- SMALLINT is perfect for u16 (0-65535)
