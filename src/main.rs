use clap::{Parser, Subcommand};
use anyhow::Result;
use std::fs;
use std::path::Path;
use fai_protocol::storage::{StorageManager, ModelMetadata};

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

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => {
            println!("Initializing FAI repository...");
            
            // Create .fai directory structure
            fs::create_dir_all(".fai/objects")?;
            fs::create_dir_all(".fai/refs")?;
            
            // Initialize storage manager (this creates the database)
            let _storage = StorageManager::new(std::path::PathBuf::from(".fai"))?;
            
            println!("Initialized FAI repository in .fai/");
        }
        Commands::Add { path } => {
            println!("Adding model: {}", path);
            
            // Check if file exists
            if !Path::new(&path).exists() {
                return Err(anyhow::anyhow!("File not found: {}", path));
            }
            
            // Initialize storage and store the model
            let storage = StorageManager::new(std::path::PathBuf::from(".fai"))?;
            let file_content = fs::read(&path)?;
            let hash = storage.store(&file_content)?;
            
            // Extract model name from path
            let model_name = Path::new(&path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            
            // Create metadata
            let metadata = ModelMetadata {
                hash: hash.clone(),
                name: model_name.to_string(),
                version: "1.0.0".to_string(),
                size: fs::metadata(&path)?.len(),
                created_at: chrono::Utc::now(),
            };
            
            // Store metadata
            storage.store_metadata(&metadata)?;
            
            println!("Added model {} with hash: {}", model_name, hash);
        }
        Commands::Commit { message } => {
            println!("Committing with message: {}", message);
            // TODO: Implement commit functionality
        }
        Commands::Status => {
            println!("Repository status:");
            // TODO: Implement status display
        }
    }

    Ok(())
}
