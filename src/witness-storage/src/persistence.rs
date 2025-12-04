use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Disk-based persistence layer for witness data and FHE results
pub struct DiskPersistence {
    base_path: PathBuf,
}

impl DiskPersistence {
    /// Create a new DiskPersistence instance with the given base path
    ///
    /// This will create the necessary directory structure if it doesn't exist:
    /// - {base_path}/witnesses/
    /// - {base_path}/fhe_results/
    pub fn new(base_path: impl AsRef<Path>) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();

        // Create directories if they don't exist
        fs::create_dir_all(base_path.join("witnesses"))
            .context("Failed to create witnesses directory")?;
        fs::create_dir_all(base_path.join("fhe_results"))
            .context("Failed to create fhe_results directory")?;

        Ok(Self { base_path })
    }

    /// Save witness data to disk
    ///
    /// The file is stored at: {base_path}/witnesses/{hex(commitment)}
    pub fn save_witness(&self, commitment: &[u8; 32], data: &[u8]) -> Result<()> {
        let path = self
            .base_path
            .join("witnesses")
            .join(hex::encode(commitment));
        fs::write(path, data).context("Failed to save witness to disk")
    }

    /// Load witness data from disk
    ///
    /// Returns None if the witness doesn't exist on disk
    pub fn load_witness(&self, commitment: &[u8; 32]) -> Result<Option<Vec<u8>>> {
        let path = self
            .base_path
            .join("witnesses")
            .join(hex::encode(commitment));
        if path.exists() {
            Ok(Some(
                fs::read(path).context("Failed to read witness from disk")?,
            ))
        } else {
            Ok(None)
        }
    }

    /// Save FHE result to disk
    ///
    /// The file is stored at: {base_path}/fhe_results/{hex(commitment)}
    pub fn save_fhe_result(&self, commitment: &[u8; 32], data: &[u8]) -> Result<()> {
        let path = self
            .base_path
            .join("fhe_results")
            .join(hex::encode(commitment));
        fs::write(path, data).context("Failed to save FHE result to disk")
    }

    /// Load FHE result from disk
    ///
    /// Returns None if the FHE result doesn't exist on disk
    pub fn load_fhe_result(&self, commitment: &[u8; 32]) -> Result<Option<Vec<u8>>> {
        let path = self
            .base_path
            .join("fhe_results")
            .join(hex::encode(commitment));
        if path.exists() {
            Ok(Some(
                fs::read(path).context("Failed to read FHE result from disk")?,
            ))
        } else {
            Ok(None)
        }
    }

    /// Delete witness from disk
    pub fn delete_witness(&self, commitment: &[u8; 32]) -> Result<()> {
        let path = self
            .base_path
            .join("witnesses")
            .join(hex::encode(commitment));
        if path.exists() {
            fs::remove_file(path).context("Failed to delete witness from disk")?;
        }
        Ok(())
    }

    /// Delete FHE result from disk
    pub fn delete_fhe_result(&self, commitment: &[u8; 32]) -> Result<()> {
        let path = self
            .base_path
            .join("fhe_results")
            .join(hex::encode(commitment));
        if path.exists() {
            fs::remove_file(path).context("Failed to delete FHE result from disk")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_save_load_witness() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = DiskPersistence::new(temp_dir.path()).unwrap();

        let commitment = [1u8; 32];
        let data = b"test witness data";

        persistence.save_witness(&commitment, data).unwrap();
        let loaded = persistence.load_witness(&commitment).unwrap().unwrap();

        assert_eq!(loaded, data);
    }

    #[test]
    fn test_load_nonexistent_witness() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = DiskPersistence::new(temp_dir.path()).unwrap();

        let commitment = [1u8; 32];
        let loaded = persistence.load_witness(&commitment).unwrap();

        assert!(loaded.is_none());
    }

    #[test]
    fn test_save_load_fhe_result() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = DiskPersistence::new(temp_dir.path()).unwrap();

        let commitment = [2u8; 32];
        let data = b"encrypted FHE result";

        persistence.save_fhe_result(&commitment, data).unwrap();
        let loaded = persistence.load_fhe_result(&commitment).unwrap().unwrap();

        assert_eq!(loaded, data);
    }

    #[test]
    fn test_delete_witness() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = DiskPersistence::new(temp_dir.path()).unwrap();

        let commitment = [3u8; 32];
        let data = b"test data";

        persistence.save_witness(&commitment, data).unwrap();
        assert!(persistence.load_witness(&commitment).unwrap().is_some());

        persistence.delete_witness(&commitment).unwrap();
        assert!(persistence.load_witness(&commitment).unwrap().is_none());
    }

    #[test]
    fn test_delete_fhe_result() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = DiskPersistence::new(temp_dir.path()).unwrap();

        let commitment = [4u8; 32];
        let data = b"test FHE data";

        persistence.save_fhe_result(&commitment, data).unwrap();
        assert!(persistence.load_fhe_result(&commitment).unwrap().is_some());

        persistence.delete_fhe_result(&commitment).unwrap();
        assert!(persistence.load_fhe_result(&commitment).unwrap().is_none());
    }

    #[test]
    fn test_directory_structure() {
        let temp_dir = TempDir::new().unwrap();
        let _persistence = DiskPersistence::new(temp_dir.path()).unwrap();

        let witnesses_dir = temp_dir.path().join("witnesses");
        let fhe_results_dir = temp_dir.path().join("fhe_results");

        assert!(witnesses_dir.exists());
        assert!(witnesses_dir.is_dir());
        assert!(fhe_results_dir.exists());
        assert!(fhe_results_dir.is_dir());
    }
}
