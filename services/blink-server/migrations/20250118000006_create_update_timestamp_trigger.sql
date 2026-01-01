-- Trigger to update updated_at on temp_job_data changes
CREATE TRIGGER update_temp_job_data_updated_at
    BEFORE UPDATE ON temp_job_data
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
