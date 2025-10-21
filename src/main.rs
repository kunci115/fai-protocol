use clap::{Parser, Subcommand};
use anyhow::Result;
use std::path::Path;
use fai_protocol::FaiProtocol;

#[derive(Parser)]
#[command(name = "fai")]
#[command(about = "FAI Protocol - Decentralized version control for AI models")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new FAI repository
    Init,
    /// Add a model file to the repository
    Add { path: String },
    /// Commit changes with a message
    Commit { message: String },
    /// Show repository status
    Status,
    /// Show commit history
    Log,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => {
            // Check if already initialized
            if Path::new(".fai").exists() {
                return Err(anyhow::anyhow!("FAI repository already initialized"));
            }
            
            println!("Initializing FAI repository...");
            FaiProtocol::init()?;
            println!("Initialized FAI repository in .fai/");
        }
        Commands::Add { path } => {
            // Check if repository is initialized
            if !Path::new(".fai").exists() {
                return Err(anyhow::anyhow!("Not a FAI repository. Run 'fai init' first."));
            }
            
            println!("Adding {} to staging area...", path);
            
            // Initialize FAI protocol
            let fai = FaiProtocol::new()?;
            
            // Add file to staging
            let hash = fai.add_file(&path)?;
            
            println!("Added {} ({})", path, &hash[..8]);
        }
        Commands::Commit { message } => {
            // Check if repository is initialized
            if !Path::new(".fai").exists() {
                return Err(anyhow::anyhow!("Not a FAI repository. Run 'fai init' first."));
            }
            
            // Initialize FAI protocol
            let fai = FaiProtocol::new()?;
            
            // Create commit
            match fai.commit(&message) {
                Ok(hash) => {
                    println!("Created commit {}", &hash[..8]);
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Failed to create commit: {}", e));
                }
            }
        }
        Commands::Status => {
            // Check if repository is initialized
            if !Path::new(".fai").exists() {
                return Err(anyhow::anyhow!("Not a FAI repository. Run 'fai init' first."));
            }
            
            // Initialize FAI protocol
            let fai = FaiProtocol::new()?;
            
            // Get staged files
            let staged_files = fai.get_status()?;
            
            if staged_files.is_empty() {
                println!("No changes staged for commit");
            } else {
                println!("Changes to be committed:");
                println!();
                for (file_path, file_hash, file_size) in staged_files {
                    println!("  {} ({} - {} bytes)", file_path, &file_hash[..8], file_size);
                }
            }
        }
        Commands::Log => {
            // Check if repository is initialized
            if !Path::new(".fai").exists() {
                return Err(anyhow::anyhow!("Not a FAI repository. Run 'fai init' first."));
            }
            
            // Initialize FAI protocol
            let fai = FaiProtocol::new()?;
            
            // Get commit log
            let commits = fai.get_log()?;
            
            if commits.is_empty() {
                println!("No commits yet");
            } else {
                for commit in commits {
                    println!("commit {}", commit.hash);
                    println!("Date:   {}", commit.timestamp.format("%Y-%m-%d %H:%M:%S"));
                    println!();
                    println!("    {}", commit.message);
                    println!();
                }
            }
        }
    }

    Ok(())
}
