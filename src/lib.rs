//! FAI Protocol - Distributed version control system for large files
//!
//! This library provides the core functionality for managing large file versions
//! in a decentralized manner. Perfect for game assets, video footage, scientific
//! datasets, AI models, and any files that are too large for traditional version
//! control systems.

pub mod database;
pub mod network;
pub mod storage;
pub mod services;

use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};

/// Information about a commit for display purposes
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommitInfo {
    /// Commit hash
    pub hash: String,
    /// Commit message
    pub message: String,
    /// Commit timestamp
    pub timestamp: DateTime<Utc>,
    /// Parent commit hashes (empty for initial commit, multiple for merge commits)
    pub parents: Vec<String>,
    /// Whether this is a merge commit
    pub is_merge: bool,
}

/// Main library interface for FAI Protocol
pub struct FaiProtocol {
    storage: storage::StorageManager,
    database: database::DatabaseManager,
    fai_path: PathBuf,
}

impl FaiProtocol {
    /// Create a new FAI Protocol instance
    pub fn new() -> Result<Self> {
        let fai_path = PathBuf::from(".fai");
        Self::new_at(&fai_path)
    }

    /// Create a new FAI Protocol instance at a specific path
    pub fn new_at<P: AsRef<Path>>(path: P) -> Result<Self> {
        let fai_path = path.as_ref().to_path_buf();
        let storage = storage::StorageManager::new(fai_path.clone())?;
        let database = database::DatabaseManager::new(&fai_path.join("db.sqlite"))?;
        Ok(Self {
            storage,
            database,
            fai_path,
        })
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

    /// Initialize a new FAI repository at a specific path
    pub fn init_at<P: AsRef<Path>>(path: P) -> Result<()> {
        let fai_path = path.as_ref().to_path_buf();

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
        println!(
            "DEBUG: Read {} bytes from file: {}",
            content.len(),
            file_path
        );

        // Store in storage manager FIRST (this actually writes to .fai/objects/)
        let hash = self.storage.store(&content)?;
        println!(
            "DEBUG: Stored file content with hash: {} ({} bytes)",
            hash,
            content.len()
        );

        // Get file size
        let size = std::fs::metadata(file_path)?.len();

        // THEN add to staging area with the hash returned by storage
        self.database.add_to_staging(file_path, &hash, size)?;
        println!("DEBUG: Added file to staging: {} -> {}", file_path, hash);

        // Debug: verify the file was actually stored
        if !self.storage.exists(&hash) {
            return Err(anyhow::anyhow!(
                "Failed to store file: {} with hash: {}",
                file_path,
                hash
            ));
        }
        println!("DEBUG: Verified file exists in storage: {}", hash);

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

    /// Create a commit from staged files
    pub fn commit(&self, message: &str) -> Result<String> {
        // Get staged files
        let staged_files = self.database.get_staged_files()?;

        if staged_files.is_empty() {
            return Err(anyhow::anyhow!("Nothing to commit"));
        }

        // Read current HEAD
        let parent_hash = self.get_head()?;

        // Generate commit hash
        let commit_data = format!(
            "{}{}{:?}",
            Utc::now().timestamp_millis(),
            message,
            staged_files
        );
        let mut hasher = blake3::Hasher::new();
        hasher.update(commit_data.as_bytes());
        let commit_hash = hasher.finalize().to_hex().to_string();

        // Create commit in database
        let parents = match parent_hash {
            Some(p) => vec![p],
            None => vec![],
        };
        self.database.create_commit(
            &commit_hash,
            message,
            &parents,
            &staged_files,
            false, // Not a merge commit
        )?;

        // Update HEAD file
        std::fs::write(self.fai_path.join("HEAD"), &commit_hash)?;

        // Clear staging area
        self.database.clear_staging()?;

        Ok(commit_hash)
    }

    /// Get commit log
    pub fn get_log(&self) -> Result<Vec<CommitInfo>> {
        let commits = self.database.get_commit_history(None)?;
        Ok(commits
            .into_iter()
            .map(|c| CommitInfo {
                hash: c.hash,
                message: c.message,
                timestamp: c.timestamp,
                parents: c.parents,
                is_merge: c.is_merge,
            })
            .collect())
    }

    /// Read current HEAD commit hash
    fn get_head(&self) -> Result<Option<String>> {
        let head_path = self.fai_path.join("HEAD");
        if head_path.exists() {
            let content = std::fs::read_to_string(&head_path)?;
            // Handle both direct hash and ref: refs/heads/main format
            if content.starts_with("ref:") {
                // For now, return None for branch refs (not implemented yet)
                Ok(None)
            } else {
                Ok(Some(content.trim().to_string()))
            }
        } else {
            Ok(None)
        }
    }

    /// Get the current HEAD commit hash
    pub fn get_head_commit(&self) -> Result<Option<String>> {
        self.get_head()
    }

    /// Update HEAD to point to a specific commit
    pub fn update_head(&self, commit_hash: &str) -> Result<()> {
        let head_path = self.fai_path.join("HEAD");
        std::fs::write(&head_path, commit_hash)?;
        Ok(())
    }

    /// Get all commits in the repository
    pub fn get_all_commits(&self) -> Result<Vec<Commit>> {
        self.database.get_all_commits()
    }

    /// Get files included in a commit
    pub fn get_commit_files(&self, commit_hash: &str) -> Result<Vec<(String, String, u64)>> {
        self.database.get_commit_files(commit_hash)
    }
}

pub use database::{Commit, DatabaseManager};
/// Re-export commonly used types
pub use storage::{ModelMetadata, StorageManager};
