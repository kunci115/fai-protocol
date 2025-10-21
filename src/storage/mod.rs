//! Storage layer for FAI Protocol
//! 
//! Handles content-addressed storage of AI models and metadata management.

use anyhow::{Result, anyhow};
use std::path::PathBuf;
use std::fs;
use blake3::Hasher;
use serde::{Deserialize, Serialize};

/// Metadata for a stored AI model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    /// Content hash using BLAKE3
    pub hash: String,
    /// Model name/identifier
    pub name: String,
    /// Model version
    pub version: String,
    /// Size in bytes
    pub size: u64,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Storage manager for AI models
pub struct StorageManager {
    /// Root path to .fai directory
    root_path: PathBuf,
}

impl StorageManager {
    /// Create a new storage manager instance with the specified root path
    pub fn new(root: PathBuf) -> Result<Self> {
        // Ensure the .fai directory exists
        fs::create_dir_all(&root)?;
        
        Ok(Self { root_path: root })
    }

    /// Store data and return its content hash
    /// 
    /// # Arguments
    /// * `data` - The data to store
    /// 
    /// # Returns
    /// The BLAKE3 hash of the stored data as a hex string
    pub fn store(&self, data: &[u8]) -> Result<String> {
        // Compute BLAKE3 hash
        let mut hasher = Hasher::new();
        hasher.update(data);
        let hash = hasher.finalize().to_hex().to_string();
        
        // Create directory structure: .fai/objects/[first-2-chars]/
        if hash.len() < 2 {
            return Err(anyhow!("Invalid hash length"));
        }
        
        let prefix = &hash[..2];
        let suffix = &hash[2..];
        let object_dir = self.root_path.join("objects").join(prefix);
        fs::create_dir_all(&object_dir)?;
        
        // Write data to: .fai/objects/[first-2-chars]/[rest-of-hash]
        let object_path = object_dir.join(suffix);
        
        // Only write if file doesn't already exist (idempotent operation)
        if !object_path.exists() {
            fs::write(&object_path, data)?;
        }
        
        Ok(hash)
    }

    /// Retrieve data by its content hash
    /// 
    /// # Arguments
    /// * `hash` - The BLAKE3 hash of the data to retrieve
    /// 
    /// # Returns
    /// The stored data as bytes
    pub fn retrieve(&self, hash: &str) -> Result<Vec<u8>> {
        if hash.len() < 2 {
            return Err(anyhow!("Invalid hash length"));
        }
        
        let prefix = &hash[..2];
        let suffix = &hash[2..];
        let object_path = self.root_path.join("objects").join(prefix).join(suffix);
        
        match fs::read(&object_path) {
            Ok(data) => Ok(data),
            Err(_) => Err(anyhow!("Object not found: {}", hash)),
        }
    }

    /// Check if a hash exists in storage
    /// 
    /// # Arguments
    /// * `hash` - The BLAKE3 hash to check
    /// 
    /// # Returns
    /// true if the hash exists, false otherwise
    pub fn exists(&self, hash: &str) -> bool {
        if hash.len() < 2 {
            return false;
        }
        
        let prefix = &hash[..2];
        let suffix = &hash[2..];
        let object_path = self.root_path.join("objects").join(prefix).join(suffix);
        
        object_path.exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_temp_storage() -> (StorageManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = StorageManager::new(temp_dir.path().to_path_buf()).unwrap();
        (storage, temp_dir)
    }

    #[test]
    fn test_store_and_retrieve() {
        let (storage, _temp_dir) = create_temp_storage();
        let data = b"Hello, FAI Protocol!";
        
        let hash = storage.store(data).unwrap();
        let retrieved = storage.retrieve(&hash).unwrap();
        
        assert_eq!(data.to_vec(), retrieved);
    }

    #[test]
    fn test_store_twice_same_hash() {
        let (storage, _temp_dir) = create_temp_storage();
        let data = b"Test data for idempotency";
        
        let hash1 = storage.store(data).unwrap();
        let hash2 = storage.store(data).unwrap();
        
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_exists() {
        let (storage, _temp_dir) = create_temp_storage();
        let data = b"Existence test data";
        
        let hash = storage.store(data).unwrap();
        
        assert!(storage.exists(&hash));
        assert!(!storage.exists("nonexistenthash123456789"));
    }

    #[test]
    fn test_retrieve_nonexistent() {
        let (storage, _temp_dir) = create_temp_storage();
        
        let result = storage.retrieve("nonexistenthash123456789");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_hash_length() {
        let (storage, _temp_dir) = create_temp_storage();
        
        let result = storage.retrieve("");
        assert!(result.is_err());
        
        let result = storage.retrieve("a");
        assert!(result.is_err());
        
        assert!(!storage.exists(""));
        assert!(!storage.exists("a"));
    }
}
