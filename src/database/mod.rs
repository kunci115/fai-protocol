//! Database layer for FAI Protocol
//!
//! Handles SQLite database operations for commits, staging, and file tracking.

use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::path::Path;

/// Represents a commit in the FAI repository
#[derive(Debug, Clone)]
pub struct Commit {
    /// Commit hash
    pub hash: String,
    /// Commit message
    pub message: String,
    /// Commit timestamp
    pub timestamp: DateTime<Utc>,
    /// Whether this is a merge commit
    pub is_merge: bool,
    /// Parent commit hashes (empty for initial commit, multiple for merge commits)
    pub parents: Vec<String>,
}

/// Represents a branch in the FAI repository
#[derive(Debug, Clone)]
pub struct Branch {
    /// Branch name
    pub name: String,
    /// Current commit hash this branch points to
    pub commit_hash: String,
    /// When this branch was created
    pub created_at: DateTime<Utc>,
    /// When this branch was last updated
    pub last_updated: DateTime<Utc>,
}

/// Represents the current HEAD state
#[derive(Debug, Clone)]
pub struct HeadState {
    /// Current branch name
    pub branch_name: String,
    /// Current commit hash
    pub commit_hash: String,
    /// Whether HEAD is detached (pointing directly to commit rather than branch)
    pub is_detached: bool,
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

    /// Get access to the database connection (for network module usage)
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Initialize the database schema
    ///
    /// Creates the necessary tables if they don't exist
    fn init_schema(&self) -> Result<()> {
        // Enable foreign key support
        self.conn.execute("PRAGMA foreign_keys = ON", [])?;

        // Create commits table (updated to support multiple parents for merges)
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS commits (
                hash TEXT PRIMARY KEY,
                message TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                is_merge BOOLEAN NOT NULL DEFAULT FALSE
            )",
            [],
        )?;

        // Create commit_parents table for supporting multiple parents (merge commits)
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS commit_parents (
                commit_hash TEXT NOT NULL,
                parent_hash TEXT NOT NULL,
                parent_order INTEGER NOT NULL,
                PRIMARY KEY (commit_hash, parent_order),
                FOREIGN KEY (commit_hash) REFERENCES commits(hash) ON DELETE CASCADE,
                FOREIGN KEY (parent_hash) REFERENCES commits(hash) ON DELETE CASCADE
            )",
            [],
        )?;

        // Create branches table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS branches (
                name TEXT PRIMARY KEY,
                commit_hash TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                last_updated INTEGER NOT NULL,
                FOREIGN KEY (commit_hash) REFERENCES commits(hash) ON DELETE CASCADE
            )",
            [],
        )?;

        // Create HEAD tracking table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS head (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                branch_name TEXT NOT NULL,
                commit_hash TEXT NOT NULL,
                FOREIGN KEY (branch_name) REFERENCES branches(name) ON DELETE CASCADE,
                FOREIGN KEY (commit_hash) REFERENCES commits(hash) ON DELETE CASCADE
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
        let mut stmt = self
            .conn
            .prepare("SELECT file_path, file_hash, file_size FROM staging ORDER BY file_path")?;

        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;

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
    /// * `parents` - List of parent commit hashes (empty for initial, multiple for merge)
    /// * `files` - List of files included in this commit
    /// * `is_merge` - Whether this is a merge commit
    pub fn create_commit(
        &self,
        hash: &str,
        message: &str,
        parents: &[&str],
        files: &[(String, String, u64)],
        is_merge: bool,
    ) -> Result<()> {
        // Validate inputs
        if hash.is_empty() {
            return Err(anyhow::anyhow!("Commit hash cannot be empty"));
        }
        if message.trim().is_empty() {
            return Err(anyhow::anyhow!("Commit message cannot be empty"));
        }

        // Insert commit with current timestamp in milliseconds for uniqueness
        let timestamp = Utc::now().timestamp_millis();

        // Check if commit already exists
        let existing_count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM commits WHERE hash = ?1",
                [hash],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if existing_count > 0 {
            println!("DEBUG: Commit {} already exists, skipping insertion", hash);
            return Ok(());
        }

        println!(
            "DEBUG: Creating commit: hash={}, message={}, timestamp={}, is_merge={}, parents={:?}",
            hash, message, timestamp, is_merge, parents
        );

        // Insert commit
        match self.conn.execute(
            "INSERT INTO commits (hash, message, timestamp, is_merge) VALUES (?1, ?2, ?3, ?4)",
            params![hash, message, timestamp, is_merge],
        ) {
            Ok(rows) => {
                println!(
                    "DEBUG: Successfully inserted commit, rows affected: {}",
                    rows
                );
            }
            Err(e) => {
                println!("DEBUG: Failed to insert commit: {}", e);
                return Err(anyhow::anyhow!("Failed to insert commit: {}", e));
            }
        }

        // Insert parent relationships
        for (index, parent_hash) in parents.iter().enumerate() {
            match self.conn.execute(
                "INSERT INTO commit_parents (commit_hash, parent_hash, parent_order) VALUES (?1, ?2, ?3)",
                params![hash, parent_hash, index as i32],
            ) {
                Ok(_) => {
                    println!("DEBUG: Successfully inserted parent relationship: {} -> {}", hash, parent_hash);
                }
                Err(e) => {
                    println!("DEBUG: Failed to insert parent relationship: {}", e);
                    return Err(anyhow::anyhow!("Failed to insert parent relationship: {}", e));
                }
            }
        }

        // Insert commit files
        for (file_path, file_hash, file_size) in files {
            println!(
                "DEBUG: Inserting commit file: path={}, hash={}, size={}",
                file_path, file_hash, file_size
            );
            match self.conn.execute(
                "INSERT INTO commit_files (commit_hash, file_path, file_hash, file_size) VALUES (?1, ?2, ?3, ?4)",
                params![hash, file_path, file_hash, file_size],
            ) {
                Ok(rows) => {
                    println!("DEBUG: Successfully inserted commit file, rows affected: {}", rows);
                }
                Err(e) => {
                    println!("DEBUG: Failed to insert commit file: {}", e);
                    // Continue with other files even if one fails
                }
            }
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
        let mut stmt = self
            .conn
            .prepare("SELECT hash, message, timestamp, is_merge FROM commits WHERE hash = ?1")?;

        let mut rows = stmt.query([hash])?;
        if let Some(row) = rows.next()? {
            let commit_hash: String = row.get(0)?;
            let message: String = row.get(1)?;
            let timestamp: i64 = row.get(2)?;
            let is_merge: bool = row.get(3)?;

            // Get parent commits
            let mut parent_stmt = self.conn.prepare(
                "SELECT parent_hash FROM commit_parents WHERE commit_hash = ?1 ORDER BY parent_order"
            )?;
            let parent_rows = parent_stmt.query([&commit_hash])?;

            let mut parents = Vec::new();
            for parent_row in parent_rows.map(|row| row.unwrap()) {
                parents.push(parent_row.get(0)?);
            }

            Ok(Some(Commit {
                hash: commit_hash,
                message,
                timestamp: DateTime::from_timestamp_millis(timestamp).unwrap_or_default(),
                is_merge,
                parents,
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

        let rows = stmt.query_map([hash], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;

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
        let mut stmt = self
            .conn
            .prepare("SELECT hash FROM commits ORDER BY timestamp DESC, hash DESC LIMIT 1")?;

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
            "SELECT hash, message, timestamp, parent_hash FROM commits ORDER BY timestamp DESC"
                .to_string()
        };

        let mut stmt = self.conn.prepare(&query)?;

        let rows = stmt.query_map([], |row| {
            Ok(Commit {
                hash: row.get(0)?,
                message: row.get(1)?,
                timestamp: DateTime::from_timestamp_millis(row.get(2)?).unwrap_or_default(),
                parent_hash: row.get(3)?,
            })
        })?;

        let mut commits = Vec::new();
        for row in rows {
            commits.push(row?);
        }

        Ok(commits)
    }

    // Branch Management Methods

    /// Create a new branch pointing to a specific commit
    ///
    /// # Arguments
    /// * `name` - Branch name
    /// * `commit_hash` - Commit hash to point to
    pub fn create_branch(&self, name: &str, commit_hash: &str) -> Result<()> {
        // Validate inputs
        if name.is_empty() {
            return Err(anyhow::anyhow!("Branch name cannot be empty"));
        }
        if commit_hash.is_empty() {
            return Err(anyhow::anyhow!("Commit hash cannot be empty"));
        }

        // Check if commit exists
        let commit_exists = self
            .conn
            .query_row("SELECT COUNT(*) FROM commits WHERE hash = ?1", [commit_hash], |row| row.get(0))
            .unwrap_or(0);

        if commit_exists == 0 {
            return Err(anyhow::anyhow!("Commit {} does not exist", commit_hash));
        }

        // Create branch
        let timestamp = Utc::now().timestamp_millis();
        self.conn.execute(
            "INSERT OR REPLACE INTO branches (name, commit_hash, created_at, last_updated) VALUES (?1, ?2, ?3, ?3)",
            params![name, commit_hash, timestamp],
        )?;

        Ok(())
    }

    /// Get all branches
    ///
    /// # Returns
    /// Vector of all branches
    pub fn get_branches(&self) -> Result<Vec<Branch>> {
        let mut stmt = self
            .conn
            .prepare("SELECT name, commit_hash, created_at, last_updated FROM branches ORDER BY name")?;

        let rows = stmt.query_map([], |row| {
            Ok(Branch {
                name: row.get(0)?,
                commit_hash: row.get(1)?,
                created_at: DateTime::from_timestamp_millis(row.get(2)?).unwrap_or_default(),
                last_updated: DateTime::from_timestamp_millis(row.get(3)?).unwrap_or_default(),
            })
        })?;

        let mut branches = Vec::new();
        for row in rows {
            branches.push(row?);
        }

        Ok(branches)
    }

    /// Get branch by name
    ///
    /// # Arguments
    /// * `name` - Branch name
    ///
    /// # Returns
    /// The branch if found, None otherwise
    pub fn get_branch(&self, name: &str) -> Result<Option<Branch>> {
        let mut stmt = self
            .conn
            .prepare("SELECT name, commit_hash, created_at, last_updated FROM branches WHERE name = ?1")?;

        let mut rows = stmt.query([name])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Branch {
                name: row.get(0)?,
                commit_hash: row.get(1)?,
                created_at: DateTime::from_timestamp_millis(row.get(2)?).unwrap_or_default(),
                last_updated: DateTime::from_timestamp_millis(row.get(3)?).unwrap_or_default(),
            }))
        } else {
            Ok(None)
        }
    }

    /// Update branch to point to a new commit
    ///
    /// # Arguments
    /// * `name` - Branch name
    /// * `commit_hash` - New commit hash
    pub fn update_branch(&self, name: &str, commit_hash: &str) -> Result<()> {
        // Validate inputs
        if name.is_empty() {
            return Err(anyhow::anyhow!("Branch name cannot be empty"));
        }
        if commit_hash.is_empty() {
            return Err(anyhow::anyhow!("Commit hash cannot be empty"));
        }

        // Check if commit exists
        let commit_exists = self
            .conn
            .query_row("SELECT COUNT(*) FROM commits WHERE hash = ?1", [commit_hash], |row| row.get(0))
            .unwrap_or(0);

        if commit_exists == 0 {
            return Err(anyhow::anyhow!("Commit {} does not exist", commit_hash));
        }

        // Update branch
        let timestamp = Utc::now().timestamp_millis();
        self.conn.execute(
            "UPDATE branches SET commit_hash = ?1, last_updated = ?2 WHERE name = ?3",
            params![commit_hash, timestamp, name],
        )?;

        Ok(())
    }

    /// Delete a branch
    ///
    /// # Arguments
    /// * `name` - Branch name to delete
    pub fn delete_branch(&self, name: &str) -> Result<()> {
        // Cannot delete current branch
        if let Some(head) = self.get_head()? {
            if head.branch_name == name {
                return Err(anyhow::anyhow!("Cannot delete currently checked out branch"));
            }
        }

        self.conn.execute("DELETE FROM branches WHERE name = ?1", [name])?;
        Ok(())
    }

    /// Get current HEAD state
    ///
    /// # Returns
    /// Current HEAD state
    pub fn get_head(&self) -> Result<Option<HeadState>> {
        let mut stmt = self
            .conn
            .prepare("SELECT branch_name, commit_hash FROM head WHERE id = 1")?;

        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            Ok(Some(HeadState {
                branch_name: row.get(0)?,
                commit_hash: row.get(1)?,
                is_detached: false, // TODO: Implement detached HEAD
            }))
        } else {
            Ok(None)
        }
    }

    /// Set current HEAD to a branch
    ///
    /// # Arguments
    /// * `branch_name` - Branch name to check out
    pub fn set_head(&self, branch_name: &str) -> Result<()> {
        // Check if branch exists
        let branch = self.get_branch(branch_name)?;
        if branch.is_none() {
            return Err(anyhow::anyhow!("Branch '{}' does not exist", branch_name));
        }

        let branch = branch.unwrap();
        let timestamp = Utc::now().timestamp_millis();

        // Update or insert HEAD
        self.conn.execute(
            "INSERT OR REPLACE INTO head (id, branch_name, commit_hash) VALUES (1, ?1, ?2)",
            params![branch_name, branch.commit_hash],
        )?;

        Ok(())
    }

    /// Initialize default branch (main) and HEAD if they don't exist
    pub fn initialize_default_branch(&self) -> Result<()> {
        // Check if any commits exist
        let latest_commit: Option<String> = self
            .conn
            .query_row(
                "SELECT hash FROM commits ORDER BY timestamp DESC, hash DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .ok();

        if let Some(commit_hash) = latest_commit {
            let timestamp = Utc::now().timestamp_millis();

            // Create default 'main' branch if it doesn't exist
            self.conn.execute(
                "INSERT OR IGNORE INTO branches (name, commit_hash, created_at, last_updated) VALUES ('main', ?1, ?2, ?2)",
                params![commit_hash, timestamp],
            )?;

            // Set HEAD to main branch if not set
            self.conn.execute(
                "INSERT OR IGNORE INTO head (id, branch_name, commit_hash) VALUES (1, 'main', ?1)",
                params![commit_hash],
            )?;
        }

        Ok(())
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
        db.create_commit("commit1", "Initial commit", None, &files)
            .unwrap();

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
        db.create_commit("commit2", "Second commit", Some("commit1"), &files2)
            .unwrap();

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
