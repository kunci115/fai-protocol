//! FAI Protocol - Decentralized version control system for AI models
//! 
//! This library provides the core functionality for managing AI model versions
//! in a decentralized manner.

pub mod storage;
pub mod database;

use std::path::PathBuf;
use anyhow::Result;

/// Main library interface for FAI Protocol
pub struct FaiProtocol {
    storage: storage::StorageManager,
    database: database::DatabaseManager,
}

impl FaiProtocol {
    /// Create a new FAI Protocol instance
    pub fn new() -> Result<Self> {
        let fai_path = PathBuf::from(".fai");
        let storage = storage::StorageManager::new(fai_path.clone())?;
        let database = database::DatabaseManager::new(&fai_path.join("db.sqlite"))?;
        Ok(Self { storage, database })
    }

    /// Initialize a new FAI repository
    pub fn init() -> Result<()> {
        let fai_path = PathBuf::from(".fai");
        
        // Create .fai directory structure
        std::fs::create_dir_all(&fai_path)?;
        std::fs::create_dir_all(fai_path.join("objects"))?;
        
        // Initialize storage (creates metadata database)
        let _storage = storage::StorageManager::new(fai_path.clone())?;
        
        // Initialize main database
        let _database = database::DatabaseManager::new(&fai_path.join("db.sqlite"))?;
        
        // Create .fai/HEAD file pointing to main branch
        std::fs::write(fai_path.join("HEAD"), "ref: refs/heads/main")?;
        
        Ok(())
    }

    /// Add a file to the staging area
    pub fn add_file(&self, file_path: &str) -> Result<String> {
        // Check if file exists
        if !std::path::Path::new(file_path).exists() {
            return Err(anyhow::anyhow!("File not found: {}", file_path));
        }

        // Read file content
        let content = std::fs::read(file_path)?;
        
        // Store in storage manager
        let hash = self.storage.store(&content)?;
        
        // Get file size
        let size = std::fs::metadata(file_path)?.len();
        
        // Add to staging area
        self.database.add_to_staging(file_path, &hash, size)?;
        
        Ok(hash)
    }

    /// Get repository status (staged files)
    pub fn get_status(&self) -> Result<Vec<(String, String, u64)>> {
        self.database.get_staged_files()
    }

    /// Get reference to the storage manager
    pub fn storage(&self) -> &storage::StorageManager {
        &self.storage
    }

    /// Get reference to the database manager
    pub fn database(&self) -> &database::DatabaseManager {
        &self.database
    }
}

/// Re-export commonly used types
pub use storage::{ModelMetadata, StorageManager};
pub use database::{DatabaseManager, Commit};
