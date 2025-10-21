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
            
            println!("Committing with message: {}", message);
            // TODO: Implement commit functionality
            println!("Commit functionality not yet implemented");
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

    Ok(())
}
