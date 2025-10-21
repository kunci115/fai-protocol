//! Storage layer for FAI Protocol
//! 
//! Handles content-addressed storage of AI models and metadata management.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use blake3::Hasher;
use rusqlite::Connection;

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
    db: Connection,
    storage_dir: PathBuf,
}

impl StorageManager {
    /// Create a new storage manager instance
    pub fn new() -> Result<Self> {
        let storage_dir = PathBuf::from(".fai/objects");
        fs::create_dir_all(&storage_dir)?;
        
        let db = Connection::open(".fai/metadata.db")?;
        db.execute(
            "CREATE TABLE IF NOT EXISTS models (
                hash TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                size INTEGER NOT NULL,
                created_at TEXT NOT NULL
            )",
            [],
        )?;
        
        Ok(Self { db, storage_dir })
    }

    /// Store a model file and return its content hash
    pub fn store<P: AsRef<Path>>(&self, path: P) -> Result<String> {
        let content = fs::read(path)?;
        let mut hasher = Hasher::new();
        hasher.update(&content);
        let hash = hasher.finalize().to_hex().to_string();
        
        let object_path = self.storage_dir.join(&hash);
        if !object_path.exists() {
            fs::write(object_path, content)?;
        }
        
        Ok(hash)
    }

    /// Retrieve a model by its content hash
    pub fn retrieve(&self, hash: &str) -> Result<Vec<u8>> {
        let object_path = self.storage_dir.join(hash);
        Ok(fs::read(object_path)?)
    }

    /// Store metadata for a model
    pub fn store_metadata(&self, metadata: &ModelMetadata) -> Result<()> {
        self.db.execute(
            "INSERT INTO models (hash, name, version, size, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            [
                &metadata.hash,
                &metadata.name,
                &metadata.version,
                &metadata.size.to_string(),
                &metadata.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Retrieve metadata for a model
    pub fn get_metadata(&self, hash: &str) -> Result<Option<ModelMetadata>> {
        let mut stmt = self.db.prepare(
            "SELECT hash, name, version, size, created_at FROM models WHERE hash = ?1"
        )?;
        
        let mut rows = stmt.query([hash])?;
        if let Some(row) = rows.next()? {
            Ok(Some(ModelMetadata {
                hash: row.get(0)?,
                name: row.get(1)?,
                version: row.get(2)?,
                size: row.get(3)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)?.with_timezone(&chrono::Utc),
            }))
        } else {
            Ok(None)
        }
    }
}
