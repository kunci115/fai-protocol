//! Storage layer for FAI Protocol
//! 
//! Handles content-addressed storage of AI models and metadata management.

use anyhow::{Result, anyhow};
use std::path::PathBuf;
use std::fs;
use std::sync::{Arc, Mutex};
use blake3::Hasher;
use serde::{Deserialize, Serialize};
use rusqlite::Connection;

/// Chunk size for large files (1MB)
const CHUNK_SIZE: usize = 1024 * 1024;

/// Manifest file structure for multi-chunk files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileManifest {
    /// Total file size in bytes
    pub total_size: u64,
    /// List of chunk hashes in order
    pub chunks: Vec<String>,
    /// Original file name (optional)
    pub filename: Option<String>,
}

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

/// Information about a commit for P2P transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    /// Commit hash
    pub hash: String,
    /// Commit message
    pub message: String,
    /// Timestamp as Unix epoch
    pub timestamp: i64,
    /// List of file hashes included in this commit
    pub file_hashes: Vec<String>,
}

/// Storage manager for AI models
#[derive(Clone)]
pub struct StorageManager {
    /// Root path to .fai directory
    root_path: PathBuf,
    /// SQLite database connection for metadata
    db: Arc<Mutex<Connection>>,
}

impl StorageManager {
    /// Create a new storage manager instance with the specified root path
    pub fn new(root: PathBuf) -> Result<Self> {
        // Ensure the .fai directory exists
        fs::create_dir_all(&root)?;
        
        // Initialize metadata database
        let db = Connection::open(root.join("metadata.db"))?;
        
        // Create models table
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
        
        // Create commits table for version control
        db.execute(
            "CREATE TABLE IF NOT EXISTS commits (
                hash TEXT PRIMARY KEY,
                message TEXT NOT NULL,
                timestamp INTEGER NOT NULL
            )",
            [],
        )?;
        
        // Create commit_files table to track files in each commit
        db.execute(
            "CREATE TABLE IF NOT EXISTS commit_files (
                commit_hash TEXT NOT NULL,
                file_hash TEXT NOT NULL,
                PRIMARY KEY (commit_hash, file_hash),
                FOREIGN KEY (commit_hash) REFERENCES commits(hash)
            )",
            [],
        )?;
        
        // Create staging table for files to be committed
        db.execute(
            "CREATE TABLE IF NOT EXISTS staging (
                file_path TEXT PRIMARY KEY,
                file_hash TEXT NOT NULL,
                file_size INTEGER NOT NULL
            )",
            [],
        )?;
        
        Ok(Self { root_path: root, db: Arc::new(Mutex::new(db)) })
    }

    /// Store data and return its content hash
    /// 
    /// # Arguments
    /// * `data` - The data to store
    /// 
    /// # Returns
    /// The BLAKE3 hash of the stored data as a hex string
    /// For large files (>1MB), returns the manifest hash
    pub fn store(&self, data: &[u8]) -> Result<String> {
        println!("DEBUG: StorageManager::store called with {} bytes of data", data.len());
        println!("DEBUG: CHUNK_SIZE = {} bytes", CHUNK_SIZE);
        
        // Check if file needs to be chunked
        if data.len() > CHUNK_SIZE {
            let total_chunks = (data.len() + CHUNK_SIZE - 1) / CHUNK_SIZE;
            println!("SPLITTING: Large file detected ({} bytes > {} bytes)", data.len(), CHUNK_SIZE);
            println!("SPLITTING: Will create {} chunks of {} bytes each", total_chunks, CHUNK_SIZE);
            println!("Splitting file into {} chunks...", total_chunks);
            
            // Chunk the file
            let chunks = self.chunk_file(data)?;
            println!("SPLITTING: Created {} chunks total", chunks.len());
            
            // Store each chunk
            for (i, (chunk_hash, chunk_data)) in chunks.iter().enumerate() {
                println!("CHUNK {}: Storing chunk {}/{} (hash: {}, size: {} bytes)", 
                        i, i + 1, chunks.len(), &chunk_hash[..16], chunk_data.len());
                let stored_hash = self.store_single_object(chunk_data)?;
                println!("Stored chunk {}: {} ({} bytes)", i, chunk_hash, chunk_data.len());
                println!("CHUNK {}: Stored with hash: {}", i, &stored_hash[..16]);
            }
            
            // Create and store manifest
            println!("MANIFEST: Creating manifest for {} chunks", chunks.len());
            let manifest_hash = self.create_manifest(&chunks, None)?;
            println!("Created manifest: {}", manifest_hash);
            println!("MANIFEST: Created manifest with hash: {}", &manifest_hash[..16]);
            println!("MANIFEST: Stored large file successfully ({} chunks -> manifest)", chunks.len());
            
            Ok(manifest_hash)
        } else {
            // Small file - store as single object
            println!("SINGLE: Small file detected ({} bytes <= {} bytes)", data.len(), CHUNK_SIZE);
            println!("SINGLE: Storing as single object");
            let hash = self.store_single_object(data)?;
            println!("SINGLE: Stored with hash: {}", &hash[..16]);
            Ok(hash)
        }
    }

    /// Retrieve data by its content hash
    /// 
    /// # Arguments
    /// * `hash` - The BLAKE3 hash of the data to retrieve
    /// 
    /// # Returns
    /// The stored data as bytes
    pub fn retrieve(&self, hash: &str) -> Result<Vec<u8>> {
        println!("DEBUG: StorageManager::retrieve called with hash: {}", hash);
        
        if hash.len() < 2 {
            println!("DEBUG: Invalid hash length: {}", hash.len());
            return Err(anyhow!("Invalid hash length"));
        }
        
        let prefix = &hash[..2];
        let suffix = &hash[2..];
        let object_path = self.root_path.join("objects").join(prefix).join(suffix);
        
        println!("DEBUG: Looking for object at path: {:?}", object_path);
        println!("DEBUG: Object exists: {}", object_path.exists());
        
        match fs::read(&object_path) {
            Ok(data) => {
                println!("DEBUG: Successfully retrieved {} bytes for hash: {}", data.len(), hash);
                
                // Check if this is a manifest file (JSON)
                if let Ok(manifest_str) = std::str::from_utf8(&data) {
                    if manifest_str.trim_start().starts_with('{') {
                        println!("DEBUG: Detected manifest file, reconstructing from chunks");
                        return self.reconstruct_from_manifest(manifest_str);
                    }
                }
                
                // Regular file, return as-is
                Ok(data)
            },
            Err(e) => {
                println!("DEBUG: Failed to retrieve object {}: {}", hash, e);
                Err(anyhow!("Object not found: {}", hash))
            },
        }
    }

    /// Reconstruct file data from manifest
    /// 
    /// # Arguments
    /// * `manifest_str` - JSON manifest string
    /// 
    /// # Returns
    /// Reconstructed file data
    fn reconstruct_from_manifest(&self, manifest_str: &str) -> Result<Vec<u8>> {
        println!("DEBUG: Starting manifest reconstruction");
        println!("DEBUG: Manifest JSON: {}", manifest_str);
        
        let manifest: FileManifest = serde_json::from_str(manifest_str)?;
        println!("DEBUG: Parsed manifest: {} chunks, total size: {} bytes", 
                manifest.chunks.len(), manifest.total_size);
        
        let mut reconstructed_data = Vec::with_capacity(manifest.total_size as usize);
        println!("DEBUG: Allocated reconstruction buffer with capacity: {} bytes", manifest.total_size);
        
        for (i, chunk_hash) in manifest.chunks.iter().enumerate() {
            println!("DEBUG: Retrieving chunk {}/{} (hash: {})", i + 1, manifest.chunks.len(), &chunk_hash[..16]);
            
            let chunk_data = self.retrieve_single_chunk(chunk_hash)?;
            println!("DEBUG: Retrieved chunk {} (size: {} bytes)", i + 1, chunk_data.len());
            
            reconstructed_data.extend_from_slice(&chunk_data);
            println!("DEBUG: Reconstruction progress: {}/{} chunks, current size: {} bytes", 
                    i + 1, manifest.chunks.len(), reconstructed_data.len());
        }
        
        println!("DEBUG: Successfully reconstructed {} bytes from {} chunks", 
                reconstructed_data.len(), manifest.chunks.len());
        
        if reconstructed_data.len() != manifest.total_size as usize {
            println!("DEBUG: WARNING: Reconstructed size mismatch! Expected: {}, Got: {}", 
                    manifest.total_size, reconstructed_data.len());
        }
        
        Ok(reconstructed_data)
    }

    /// Retrieve a single chunk by hash
    /// 
    /// # Arguments
    /// * `hash` - The chunk hash
    /// 
    /// # Returns
    /// The chunk data
    fn retrieve_single_chunk(&self, hash: &str) -> Result<Vec<u8>> {
        if hash.len() < 2 {
            return Err(anyhow!("Invalid chunk hash length"));
        }
        
        let prefix = &hash[..2];
        let suffix = &hash[2..];
        let object_path = self.root_path.join("objects").join(prefix).join(suffix);
        
        match fs::read(&object_path) {
            Ok(data) => Ok(data),
            Err(e) => Err(anyhow!("Chunk not found: {} - {}", hash, e)),
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

    /// Chunk file data into smaller pieces
    /// 
    /// # Arguments
    /// * `data` - The file data to chunk
    /// 
    /// # Returns
    /// Vector of tuples containing (chunk_hash, chunk_data)
    fn chunk_file(&self, data: &[u8]) -> Result<Vec<(String, Vec<u8>)>> {
        let mut chunks = Vec::new();
        
        for (i, chunk_data) in data.chunks(CHUNK_SIZE).enumerate() {
            let mut hasher = Hasher::new();
            hasher.update(chunk_data);
            let hash = hasher.finalize().to_hex().to_string();
            println!("DEBUG: Created chunk {} ({} bytes, hash: {})", i, chunk_data.len(), &hash[..16]);
            chunks.push((hash, chunk_data.to_vec()));
        }
        
        println!("DEBUG: Chunked file into {} chunks", chunks.len());
        Ok(chunks)
    }

    /// Create a manifest file for chunks
    /// 
    /// # Arguments
    /// * `chunks` - Vector of chunk tuples (hash, data)
    /// * `filename` - Optional original filename
    /// 
    /// # Returns
    /// The manifest hash as a hex string
    fn create_manifest(&self, chunks: &[(String, Vec<u8>)], filename: Option<String>) -> Result<String> {
        let total_size: u64 = chunks.iter().map(|(_, data)| data.len() as u64).sum();
        let chunk_hashes: Vec<String> = chunks.iter().map(|(hash, _)| hash.clone()).collect();
        
        println!("MANIFEST: Building manifest with {} chunks", chunks.len());
        for (i, (hash, data)) in chunks.iter().enumerate() {
            println!("MANIFEST:   Chunk {} -> {} ({} bytes)", i, &hash[..16], data.len());
        }
        
        let manifest = FileManifest {
            total_size,
            chunks: chunk_hashes,
            filename,
        };
        
        // Serialize manifest to JSON
        let manifest_json = serde_json::to_string_pretty(&manifest)?;
        println!("MANIFEST: JSON size: {} bytes", manifest_json.len());
        println!("MANIFEST: Total file size: {} bytes ({:.2} MB)", total_size, total_size as f64 / 1_048_576.0);
        
        // Store manifest as a regular object
        self.store_single_object(manifest_json.as_bytes())
    }

    /// Store a single chunk/object
    /// 
    /// # Arguments
    /// * `hash` - The hash of the data
    /// * `data` - The data to store
    /// 
    /// # Returns
    /// Ok(()) if successful
    fn store_single_object(&self, data: &[u8]) -> Result<String> {
        // Compute BLAKE3 hash
        let mut hasher = Hasher::new();
        hasher.update(data);
        let hash = hasher.finalize().to_hex().to_string();
        
        println!("DEBUG: Storing object with hash: {}", hash);
        
        // Create directory structure: .fai/objects/[first-2-chars]/
        if hash.len() < 2 {
            return Err(anyhow!("Invalid hash length"));
        }
        
        let prefix = &hash[..2];
        let suffix = &hash[2..];
        let object_dir = self.root_path.join("objects").join(prefix);
        
        println!("DEBUG: Creating object directory: {:?}", object_dir);
        fs::create_dir_all(&object_dir)?;
        
        // Write data to: .fai/objects/[first-2-chars]/[rest-of-hash]
        let object_path = object_dir.join(suffix);
        
        println!("DEBUG: Object path: {:?}", object_path);
        println!("DEBUG: Object already exists: {}", object_path.exists());
        
        // Only write if file doesn't already exist (idempotent operation)
        if !object_path.exists() {
            println!("DEBUG: Writing {} bytes to object file", data.len());
            fs::write(&object_path, data)?;
            println!("DEBUG: Successfully wrote object file");
        } else {
            println!("DEBUG: Object file already exists, skipping write");
        }
        
        Ok(hash)
    }

    /// Store metadata for a model
    /// 
    /// # Arguments
    /// * `metadata` - The metadata to store
    /// 
    /// # Returns
    /// Ok(()) if successful, Err otherwise
    pub fn store_metadata(&self, metadata: &ModelMetadata) -> Result<()> {
        let conn = self.db.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO models (hash, name, version, size, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
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
    /// 
    /// # Arguments
    /// * `hash` - The BLAKE3 hash of the model
    /// 
    /// # Returns
    /// The metadata if found, None otherwise
    pub fn get_metadata(&self, hash: &str) -> Result<Option<ModelMetadata>> {
        let conn = self.db.lock().unwrap();
        let mut stmt = conn.prepare(
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

    /// Get all commits from the database
    /// 
    /// # Returns
    /// Vector of CommitInfo for all commits
    pub fn get_all_commits(&self) -> Result<Vec<CommitInfo>> {
        let conn = self.db.lock().unwrap();
        
        // Debug: Check what's in the database
        println!("DEBUG: get_all_commits called, checking database contents");
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM commits", [], |row| row.get(0))?;
        println!("DEBUG: Found {} commits in database", count);
        
        let mut stmt = conn.prepare(
            "SELECT c.hash, c.message, c.timestamp 
             FROM commits c 
             ORDER BY c.timestamp DESC"
        )?;
        
        let commit_iter = stmt.query_map([], |row| {
            Ok(CommitInfo {
                hash: row.get(0)?,
                message: row.get(1)?,
                timestamp: row.get(2)?,
                file_hashes: Vec::new(), // Will be filled separately
            })
        })?;
        
        let mut commits = Vec::new();
        for commit_result in commit_iter {
            let mut commit = commit_result?;
            
            // Get file hashes for this commit
            let mut file_stmt = conn.prepare(
                "SELECT file_hash FROM commit_files WHERE commit_hash = ?"
            )?;
            let file_hashes: Result<Vec<String>, _> = file_stmt
                .query_map([&commit.hash], |row| row.get(0))?
                .collect();
            commit.file_hashes = file_hashes?;
            
            commits.push(commit);
        }
        
        Ok(commits)
    }

    /// Get a specific commit by hash
    /// 
    /// # Arguments
    /// * `hash` - The commit hash
    /// 
    /// # Returns
    /// The CommitInfo if found, None otherwise
    pub fn get_commit(&self, hash: &str) -> Result<Option<CommitInfo>> {
        let conn = self.db.lock().unwrap();
        
        // First get basic commit info
        let mut stmt = conn.prepare(
            "SELECT hash, message, timestamp FROM commits WHERE hash = ?1"
        )?;
        
        let mut rows = stmt.query([hash])?;
        if let Some(row) = rows.next()? {
            let hash: String = row.get(0)?;
            let message: String = row.get(1)?;
            let timestamp: i64 = row.get(2)?;
            
            // Now get file hashes for this commit
            let mut file_stmt = conn.prepare(
                "SELECT file_hash FROM commit_files WHERE commit_hash = ?1"
            )?;
            
            let file_rows = file_stmt.query([hash])?;
            let mut file_hashes = Vec::new();
            while let Some(row) = file_rows.next()? {
                file_hashes.push(row.get(0)?);
            }
            
            Ok(Some(CommitInfo {
                hash,
                message,
                timestamp,
                file_hashes: file_hashes?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Save a commit received from a remote peer
    /// 
    /// # Arguments
    /// * `commit` - The commit information to save
    /// 
    /// # Returns
    /// Ok(()) if successful
    pub fn save_remote_commit(&self, commit: &CommitInfo) -> Result<()> {
        let mut conn = self.db.lock().unwrap();
        
        // Start transaction
        let tx = conn.transaction()?;
        
        // Insert commit (ignore if exists)
        tx.execute(
            "INSERT OR IGNORE INTO commits (hash, message, timestamp) VALUES (?1, ?2, ?3)",
            [&commit.hash, &commit.message, &commit.timestamp.to_string()],
        )?;
        
        // Insert file associations
        for file_hash in &commit.file_hashes {
            tx.execute(
                "INSERT OR IGNORE INTO commit_files (commit_hash, file_hash) VALUES (?1, ?2)",
                [&commit.hash, file_hash],
            )?;
        }
        
        // Commit transaction
        tx.commit()?;
        
        Ok(())
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
