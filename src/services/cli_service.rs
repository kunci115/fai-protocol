//! CLI service for FAI Protocol
//!
//! Handles command-line interface operations and user interactions

use anyhow::Result;
use std::path::Path;
use super::branch_service::BranchService;

/// CLI service for handling user commands
pub struct CliService {
    repo_path: std::path::PathBuf,
}

impl CliService {
    /// Create a new CLI service instance
    pub fn new<P: AsRef<Path>>(repo_path: P) -> Self {
        Self {
            repo_path: repo_path.as_ref().to_path_buf(),
        }
    }

    /// Get the repository path
    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }

    /// Check if repository is initialized
    pub fn check_repo_initialized(&self) -> Result<()> {
        if !self.repo_path.join(".fai").exists() {
            return Err(anyhow::anyhow!(
                "Not a FAI repository. Run 'fai init' first."
            ));
        }
        Ok(())
    }

    /// Initialize a repository
    pub fn init_repo(&self) -> Result<()> {
        if self.repo_path.join(".fai").exists() {
            return Err(anyhow::anyhow!("FAI repository already initialized"));
        }

        println!("Initializing FAI repository...");
        crate::FaiProtocol::init_at(&self.repo_path)?;
        println!("Initialized FAI repository in .fai/");
        Ok(())
    }

    /// Handle branch operations
    pub fn handle_branch_command(
        &self,
        branch_name: Option<String>,
        delete: bool,
        list: bool,
    ) -> Result<()> {
        self.check_repo_initialized()?;

        let branch_service = BranchService::from_repo_path(&self.repo_path.join(".fai"))?;

        if list {
            self.list_branches(&branch_service)?;
        } else if delete {
            self.delete_branch(&branch_service, branch_name)?;
        } else if let Some(name) = branch_name {
            self.create_branch(&branch_service, &name)?;
        } else {
            self.show_branch_help();
        }

        Ok(())
    }

    /// Handle checkout operations
    pub fn handle_checkout_command(&self, branch_name: &str) -> Result<()> {
        self.check_repo_initialized()?;

        let branch_service = BranchService::from_repo_path(&self.repo_path.join(".fai"))?;
        branch_service.checkout_branch(branch_name)?;
        println!("Switched to branch '{}'", branch_name);

        Ok(())
    }

    /// Handle commit amend operations
    pub fn handle_commit_amend(&self, message: Option<String>) -> Result<()> {
        self.check_repo_initialized()?;

        let fai = crate::FaiProtocol::new_at(&self.repo_path)?;
        let database = crate::database::DatabaseManager::new(&self.repo_path.join(".fai/db.sqlite"))?;

        // Get current HEAD commit
        let current_head = database.get_head_commit()?
            .ok_or_else(|| anyhow::anyhow!("No commits found"))?;

        // Get current commit details
        let commits = database.get_all_commits()?;
        let last_commit = commits.iter().find(|c| c.hash == current_head)
            .ok_or_else(|| anyhow::anyhow!("Last commit not found"))?;

        // Get current branch
        let current_branch = database.get_current_branch()?;

        // Use new message or keep the old one
        let commit_message = message.unwrap_or_else(|| last_commit.message.clone());

        println!("Amending last commit...");
        println!("Old message: {}", last_commit.message);
        println!("New message: {}", commit_message);

        // Get staged files (they should be the same as the last commit)
        let staged_files = database.get_staged_files()?;
        let has_staged_files = !staged_files.is_empty();
        let files_to_commit = if has_staged_files {
            staged_files
        } else {
            // If no staged files, get files from the last commit
            database.get_commit_files(&current_head)?
        };

        // Calculate new commit hash
        let new_hash = self.calculate_amended_hash(&commit_message, &current_branch, &files_to_commit, &last_commit)?;

        // Create new commit with same parents
        database.create_commit(
            &new_hash,
            &commit_message,
            &last_commit.parents,
            &files_to_commit,
            last_commit.is_merge,
        )?;

        // Update current branch HEAD
        database.update_branch_head(&current_branch, &new_hash)?;
        database.update_head(&new_hash)?;

        println!("Amended commit: {}", &new_hash[..8]);

        // Clear staging if there were staged files
        if has_staged_files {
            database.clear_staging()?;
        }

        Ok(())
    }

    /// Calculate hash for amended commit
    fn calculate_amended_hash(
        &self,
        message: &str,
        branch: &str,
        files: &[(String, String, u64)],
        last_commit: &crate::database::Commit,
    ) -> Result<String> {
        let mut hasher = blake3::Hasher::new();
        hasher.update(message.as_bytes());
        hasher.update(branch.as_bytes());
        for (path, hash, size) in files {
            hasher.update(path.as_bytes());
            hasher.update(hash.as_bytes());
            hasher.update(size.to_string().as_bytes());
        }

        // Include parents (same as last commit)
        for parent in &last_commit.parents {
            hasher.update(parent.as_bytes());
        }

        Ok(hasher.finalize().to_hex().to_string())
    }

    /// List branches with nice formatting
    fn list_branches(&self, branch_service: &BranchService) -> Result<()> {
        let branches = branch_service.list_branches()?;

        println!("Branches:");
        for branch in branches {
            println!("{}{}{} {}",
                branch.status_marker(),
                branch.name,
                if !branch.status_text().is_empty() { " " } else { "" },
                branch.status_text()
            );
            if !branch.is_empty {
                println!("    {}", branch.short_hash());
            }
        }

        Ok(())
    }

    /// Create a new branch
    fn create_branch(&self, branch_service: &BranchService, name: &str) -> Result<()> {
        let current_head = branch_service.database.get_head_commit()?
            .ok_or_else(|| anyhow::anyhow!("No commits found"))?;

        branch_service.create_branch(name, Some(&current_head))?;
        println!("Created branch '{}' pointing to {}", name, &current_head[..8]);
        Ok(())
    }

    /// Delete a branch
    fn delete_branch(&self, branch_service: &BranchService, branch_name: Option<String>) -> Result<()> {
        let name = branch_name.ok_or_else(|| anyhow::anyhow!("Branch name required for deletion"))?;
        branch_service.delete_branch(&name)?;
        println!("Deleted branch '{}'", name);
        Ok(())
    }

    /// Show branch command help
    fn show_branch_help(&self) {
        println!("Usage: fai branch [OPTIONS] [BRANCH_NAME]");
        println!("");
        println!("Options:");
        println!("  -l, --list     List all branches");
        println!("  -d, --delete    Delete a branch");
        println!("");
        println!("Arguments:");
        println!("  <BRANCH_NAME>  Name of the branch to create");
        println!("");
        println!("Examples:");
        println!("  fai branch feature-xyz    # Create a new branch");
        println!("  fai branch --list         # List all branches");
        println!("  fai branch --delete old   # Delete a branch");
    }
}