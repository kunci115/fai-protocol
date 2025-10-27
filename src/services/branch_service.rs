//! Branch management service for FAI Protocol
//!
//! Provides Git-like branching functionality including:
//! - Creating and deleting branches
//! - Switching between branches
//! - Listing branches
//! - Managing branch references

use anyhow::Result;
use std::path::Path;

/// Branch management service
pub struct BranchService {
    pub database: crate::database::DatabaseManager,
}

impl BranchService {
    /// Create a new branch service instance
    pub fn new(database: crate::database::DatabaseManager) -> Self {
        Self { database }
    }

    /// Initialize the branch service from repository path
    pub fn from_repo_path(repo_path: &Path) -> Result<Self> {
        let database = crate::database::DatabaseManager::new(&repo_path.join("db.sqlite"))?;
        Ok(Self::new(database))
    }

    /// Create a new branch
    ///
    /// # Arguments
    /// * `name` - Branch name
    /// * `commit_hash` - Commit hash to point branch to (optional, defaults to HEAD)
    pub fn create_branch(&self, name: &str, commit_hash: Option<&str>) -> Result<()> {
        // Check if branch already exists
        if self.database.branch_exists(name)? {
            return Err(anyhow::anyhow!("Branch '{}' already exists", name));
        }

        // Get commit hash (use HEAD if not provided)
        let target_hash = if let Some(hash) = commit_hash {
            hash.to_string()
        } else {
            self.database.get_head_commit()?
                .ok_or_else(|| anyhow::anyhow!("No commits found"))?
        };

        self.database.create_branch(name, &target_hash)?;
        Ok(())
    }

    /// Delete a branch
    ///
    /// # Arguments
    /// * `name` - Branch name to delete
    ///
    /// Note: Cannot delete the current branch
    pub fn delete_branch(&self, name: &str) -> Result<()> {
        self.database.delete_branch(name)?;
        Ok(())
    }

    /// List all branches
    ///
    /// # Returns
    /// Vector of branch information
    pub fn list_branches(&self) -> Result<Vec<BranchInfo>> {
        let branches = self.database.list_branches()?;
        let current_branch = self.get_current_branch().unwrap_or_else(|_| "detached".to_string());

        let mut branch_infos = Vec::new();
        for (name, head_commit) in branches {
            let is_current = name == current_branch;
            let is_empty = head_commit == "0000000000000000000000000000000000000000";

            branch_infos.push(BranchInfo {
                name,
                head_commit,
                is_current,
                is_empty,
            });
        }

        Ok(branch_infos)
    }

    /// Switch to a branch
    ///
    /// # Arguments
    /// * `name` - Branch name to switch to
    pub fn checkout_branch(&self, name: &str) -> Result<()> {
        // Check if branch exists
        if !self.database.branch_exists(name)? {
            return Err(anyhow::anyhow!("Branch '{}' does not exist", name));
        }

        // Switch to the branch
        let ref_name = format!("refs/heads/{}", name);
        self.database.set_current_ref(&ref_name)?;
        Ok(())
    }

    /// Get current branch name
    ///
    /// # Returns
    /// Current branch name
    pub fn get_current_branch(&self) -> Result<String> {
        self.database.get_current_branch()
    }

    /// Get the head commit of a branch
    ///
    /// # Arguments
    /// * `name` - Branch name
    ///
    /// # Returns
    /// Commit hash if branch exists
    pub fn get_branch_head(&self, name: &str) -> Result<Option<String>> {
        self.database.get_branch_head(name)
    }

    /// Update the head commit of a branch
    ///
    /// # Arguments
    /// * `name` - Branch name
    /// * `commit_hash` - New commit hash
    pub fn update_branch_head(&self, name: &str, commit_hash: &str) -> Result<()> {
        self.database.update_branch_head(name, commit_hash)?;
        Ok(())
    }
}

/// Branch information
#[derive(Debug, Clone)]
pub struct BranchInfo {
    /// Branch name
    pub name: String,
    /// Head commit hash
    pub head_commit: String,
    /// Whether this is the current branch
    pub is_current: bool,
    /// Whether the branch has no commits
    pub is_empty: bool,
}

impl BranchInfo {
    /// Get a short hash for display
    pub fn short_hash(&self) -> &str {
        if self.head_commit.len() >= 8 {
            &self.head_commit[..8]
        } else {
            &self.head_commit
        }
    }

    /// Get display status marker
    pub fn status_marker(&self) -> &'static str {
        if self.is_current { "* " } else { "  " }
    }

    /// Get display status text
    pub fn status_text(&self) -> &'static str {
        if self.is_empty { "(no commits)" } else { "" }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::database::DatabaseManager;

    fn create_test_branch_service() -> (BranchService, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let database = DatabaseManager::new(&db_path).unwrap();
        let branch_service = BranchService::new(database);
        (branch_service, temp_dir)
    }

    #[test]
    fn test_create_branch() {
        let (service, _temp_dir) = create_test_branch_service();

        // This test would need a commit to be created first
        // For now, we'll just test the branch existence check
        assert!(!service.database.branch_exists("test").unwrap());
    }
}