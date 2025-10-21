//! FAI Protocol - Decentralized version control system for AI models
//! 
//! This library provides the core functionality for managing AI model versions
//! in a decentralized manner.

pub mod storage;

/// Main library interface for FAI Protocol
pub struct FaiProtocol {
    storage: storage::StorageManager,
}

impl FaiProtocol {
    /// Create a new FAI Protocol instance
    pub fn new() -> anyhow::Result<Self> {
        let storage = storage::StorageManager::new(std::path::PathBuf::from(".fai"))?;
        Ok(Self { storage })
    }

    /// Get reference to the storage manager
    pub fn storage(&self) -> &storage::StorageManager {
        &self.storage
    }
}

/// Re-export commonly used types
pub use storage::{ModelMetadata, StorageManager};
