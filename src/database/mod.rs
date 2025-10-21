//! Database layer for FAI Protocol
//! 
//! Handles SQLite database operations for commits, staging, and file tracking.

use anyhow::Result;
use rusqlite::{Connection, params};
use std::path::Path;
use chrono::{DateTime, Utc};

/// Represents a commit in the FAI repository
#[derive(Debug, Clone)]
pub struct Commit {
    /// Commit hash
    pub hash: String,
    /// Commit message
    pub message: String,
    /// Commit timestamp
    pub timestamp: DateTime<Utc>,
    /// Parent commit hash (None for initial commit)
    pub parent_hash: Option<String>,
}

/// Database manager for FAI Protocol
pub struct DatabaseManager {
    /// SQLite database connection
    conn: Connection,
}

impl DatabaseManager {
    /// Create a new database manager with the specified database path
    /// 
    /// # Arguments
    /// * `db_path` - Path to the SQLite database file
    /// 
    /// # Returns
    /// A new DatabaseManager instance
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Initialize the database schema
    /// 
    /// Creates the necessary tables if they don't exist
    fn init_schema(&self) -> Result<()> {
        // Create commits table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS commits (
                hash TEXT PRIMARY KEY,
                message TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                parent_hash TEXT,
                FOREIGN KEY (parent_hash) REFERENCES commits(hash)
            )",
            [],
        )?;

        // Create commit_files table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS commit_files (
                commit_hash TEXT NOT NULL,
                file_path TEXT NOT NULL,
                file_hash TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                PRIMARY KEY (commit_hash, file_path),
                FOREIGN KEY (commit_hash) REFERENCES commits(hash) ON DELETE CASCADE
            )",
            [],
        )?;

        // Create staging table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS staging (
                file_path TEXT PRIMARY KEY,
                file_hash TEXT NOT NULL,
                file_size INTEGER NOT NULL
            )",
            [],
        )?;

        Ok(())
    }

    /// Add a file to the staging area
    /// 
    /// # Arguments
    /// * `path` - File path relative to repository root
    /// * `hash` - Content hash of the file
    /// * `size` - File size in bytes
    pub fn add_to_staging(&self, path: &str, hash: &str, size: u64) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO staging (file_path, file_hash, file_size) VALUES (?1, ?2, ?3)",
            params![path, hash, size],
        )?;
        Ok(())
    }

    /// Get all staged files
    /// 
    /// # Returns
    /// Vector of tuples containing (file_path, file_hash, file_size)
    pub fn get_staged_files(&self) -> Result<Vec<(String, String, u64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT file_path, file_hash, file_size FROM staging ORDER BY file_path"
        )?;
        
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
            ))
        })?;

        let mut files = Vec::new();
        for row in rows {
            files.push(row?);
        }
        
        Ok(files)
    }

    /// Clear all files from the staging area
    pub fn clear_staging(&self) -> Result<()> {
        self.conn.execute("DELETE FROM staging", [])?;
        Ok(())
    }

    /// Create a new commit
    /// 
    /// # Arguments
    /// * `hash` - Commit hash
    /// * `message` - Commit message
    /// * `parent` - Optional parent commit hash
    /// * `files` - List of files included in this commit
    pub fn create_commit(
        &self,
        hash: &str,
        message: &str,
        parent: Option<&str>,
        files: &[(String, String, u64)],
    ) -> Result<()> {
        // Insert commit with current timestamp in milliseconds for uniqueness
        let timestamp = Utc::now().timestamp_millis();
        self.conn.execute(
            "INSERT INTO commits (hash, message, timestamp, parent_hash) VALUES (?1, ?2, ?3, ?4)",
            params![hash, message, timestamp, parent],
        )?;

        // Insert commit files
        for (file_path, file_hash, file_size) in files {
            self.conn.execute(
                "INSERT INTO commit_files (commit_hash, file_path, file_hash, file_size) VALUES (?1, ?2, ?3, ?4)",
                params![hash, file_path, file_hash, file_size],
            )?;
        }

        Ok(())
    }

    /// Get commit information by hash
    /// 
    /// # Arguments
    /// * `hash` - Commit hash
    /// 
    /// # Returns
    /// The commit if found, None otherwise
    pub fn get_commit(&self, hash: &str) -> Result<Option<Commit>> {
        let mut stmt = self.conn.prepare(
            "SELECT hash, message, timestamp, parent_hash FROM commits WHERE hash = ?1"
        )?;
        
        let mut rows = stmt.query([hash])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Commit {
                hash: row.get(0)?,
                message: row.get(1)?,
                timestamp: DateTime::from_timestamp(row.get(2)?, 0).unwrap_or_default(),
                parent_hash: row.get(3)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get all files associated with a commit
    /// 
    /// # Arguments
    /// * `hash` - Commit hash
    /// 
    /// # Returns
    /// Vector of tuples containing (file_path, file_hash, file_size)
    pub fn get_commit_files(&self, hash: &str) -> Result<Vec<(String, String, u64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT file_path, file_hash, file_size FROM commit_files WHERE commit_hash = ?1 ORDER BY file_path"
        )?;
        
        let rows = stmt.query_map([hash], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
            ))
        })?;

        let mut files = Vec::new();
        for row in rows {
            files.push(row?);
        }
        
        Ok(files)
    }

    /// Get the latest commit (HEAD)
    /// 
    /// # Returns
    /// The latest commit hash if any commits exist
    pub fn get_head(&self) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT hash FROM commits ORDER BY timestamp DESC, hash DESC LIMIT 1"
        )?;
        
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    /// Get commit history
    /// 
    /// # Arguments
    /// * `limit` - Maximum number of commits to return (None for all)
    /// 
    /// # Returns
    /// Vector of commits ordered by timestamp (newest first)
    pub fn get_commit_history(&self, limit: Option<i32>) -> Result<Vec<Commit>> {
        let query = if let Some(limit) = limit {
            format!("SELECT hash, message, timestamp, parent_hash FROM commits ORDER BY timestamp DESC LIMIT {}", limit)
        } else {
            "SELECT hash, message, timestamp, parent_hash FROM commits ORDER BY timestamp DESC".to_string()
        };
        
        let mut stmt = self.conn.prepare(&query)?;
        
        let rows = stmt.query_map([], |row| {
            Ok(Commit {
                hash: row.get(0)?,
                message: row.get(1)?,
                timestamp: DateTime::from_timestamp(row.get(2)?, 0).unwrap_or_default(),
                parent_hash: row.get(3)?,
            })
        })?;

        let mut commits = Vec::new();
        for row in rows {
            commits.push(row?);
        }
        
        Ok(commits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_temp_database() -> (DatabaseManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = DatabaseManager::new(&db_path).unwrap();
        (db, temp_dir)
    }

    #[test]
    fn test_staging_operations() {
        let (db, _temp_dir) = create_temp_database();
        
        // Test adding to staging
        db.add_to_staging("test.txt", "hash123", 100).unwrap();
        db.add_to_staging("model.onnx", "hash456", 2048).unwrap();
        
        // Test getting staged files
        let staged = db.get_staged_files().unwrap();
        assert_eq!(staged.len(), 2);
        // Check that both files are present (order guaranteed by ORDER BY file_path)
        assert!(staged.contains(&("model.onnx".to_string(), "hash456".to_string(), 2048)));
        assert!(staged.contains(&("test.txt".to_string(), "hash123".to_string(), 100)));
        
        // Test clearing staging
        db.clear_staging().unwrap();
        let staged = db.get_staged_files().unwrap();
        assert_eq!(staged.len(), 0);
    }

    #[test]
    fn test_commit_operations() {
        let (db, _temp_dir) = create_temp_database();
        
        // Create initial commit
        let files = vec![
            ("file1.txt".to_string(), "hash1".to_string(), 100),
            ("file2.txt".to_string(), "hash2".to_string(), 200),
        ];
        db.create_commit("commit1", "Initial commit", None, &files).unwrap();
        
        // Test getting commit
        let commit = db.get_commit("commit1").unwrap();
        assert!(commit.is_some());
        let commit = commit.unwrap();
        assert_eq!(commit.hash, "commit1");
        assert_eq!(commit.message, "Initial commit");
        assert_eq!(commit.parent_hash, None);
        
        // Test getting commit files
        let commit_files = db.get_commit_files("commit1").unwrap();
        assert_eq!(commit_files.len(), 2);
        // Check that both files are present (order guaranteed by ORDER BY file_path)
        assert!(commit_files.contains(&("file1.txt".to_string(), "hash1".to_string(), 100)));
        assert!(commit_files.contains(&("file2.txt".to_string(), "hash2".to_string(), 200)));
        
        // Test HEAD
        let head = db.get_head().unwrap();
        assert_eq!(head, Some("commit1".to_string()));
        
        // Create second commit with parent (add small delay to ensure different timestamp)
        std::thread::sleep(std::time::Duration::from_millis(10));
        let files2 = vec![
            ("file1.txt".to_string(), "hash1_updated".to_string(), 150),
            ("file3.txt".to_string(), "hash3".to_string(), 300),
        ];
        db.create_commit("commit2", "Second commit", Some("commit1"), &files2).unwrap();
        
        // Test HEAD updated
        let head = db.get_head().unwrap();
        println!("Debug: HEAD = {:?}", head);
        println!("Debug: Expected = {:?}", Some("commit2".to_string()));
        assert_eq!(head, Some("commit2".to_string()));
        
        // Test commit history
        let history = db.get_commit_history(None).unwrap();
        assert_eq!(history.len(), 2);
        // Check that both commits are present (order guaranteed by ORDER BY timestamp DESC)
        assert!(history.iter().any(|c| c.hash == "commit1"));
        assert!(history.iter().any(|c| c.hash == "commit2"));
        // Most recent commit should be first
        assert_eq!(history[0].hash, "commit2");
    }
}
