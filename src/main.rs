use clap::{Parser, Subcommand};
use anyhow::Result;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use libp2p::PeerId;
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
    Commit { 
        /// Commit message
        #[arg(short, long)]
        message: String 
    },
    /// Show repository status
    Status,
    /// Show commit history
    Log,
    /// Discover and list network peers
    Peers,
    /// Fetch a chunk of data from a peer
    Fetch {
        /// Peer ID to fetch from
        peer_id: String,
        /// Hash of the data to fetch
        hash: String,
    },
    /// Start server to serve chunks to other peers
    Serve,
}

#[tokio::main]
async fn main() -> Result<()> {
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
        Commands::Peers => {
            println!("Discovering peers on local network...");
            
            // Create storage manager
            let storage = Arc::new(fai_protocol::storage::StorageManager::new(
                Path::new(".fai").join("storage")
            )?);
            
            // Create network manager
            let mut network_manager = match fai_protocol::network::NetworkManager::new(storage) {
                Ok(nm) => nm,
                Err(e) => {
                    return Err(anyhow::anyhow!("Failed to create network manager: {}", e));
                }
            };
            
            // Start the network manager
            if let Err(e) = network_manager.start() {
                return Err(anyhow::anyhow!("Failed to start network manager: {}", e));
            }
            
            // Discover peers for 5 seconds
            let start_time = std::time::Instant::now();
            let discovery_duration = std::time::Duration::from_secs(5);
            
            println!("Local peer ID: {}", network_manager.local_peer_id());
            println!();
            
            while start_time.elapsed() < discovery_duration {
                if let Err(e) = network_manager.poll_events().await {
                    eprintln!("Error during peer discovery: {}", e);
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            
            // List discovered peers
            let peers = network_manager.list_peers();
            
            if peers.is_empty() {
                println!("No peers discovered");
            } else {
                for peer in &peers {
                    println!("Peer ID: {}", peer.peer_id);
                    
                    if !peer.addresses.is_empty() {
                        println!("Addresses:");
                        for addr in &peer.addresses {
                            println!("  {}", addr);
                        }
                    }
                    
                    let last_seen = peer.last_seen.elapsed().unwrap_or_default();
                    let seconds_ago = last_seen.as_secs();
                    if seconds_ago < 60 {
                        println!("Last seen: {} seconds ago", seconds_ago);
                    } else if seconds_ago < 3600 {
                        println!("Last seen: {} minutes ago", seconds_ago / 60);
                    } else {
                        println!("Last seen: {} hours ago", seconds_ago / 3600);
                    }
                    
                    println!();
                }
            }
            
            println!("Found {} peer(s)", peers.len());
        }
        Commands::Fetch { peer_id, hash } => {
            // Check if repository is initialized
            if !Path::new(".fai").exists() {
                return Err(anyhow::anyhow!("Not a FAI repository. Run 'fai init' first."));
            }
            
            // Parse peer ID
            let target_peer = PeerId::from_str(&peer_id)
                .map_err(|_| anyhow::anyhow!("Invalid peer ID format: {}", peer_id))?;
            
            println!("Discovering peers...");
            
            // Create storage manager
            let storage = Arc::new(fai_protocol::storage::StorageManager::new(
                Path::new(".fai").join("storage")
            )?);
            
            // Create network manager
            let mut network_manager = match fai_protocol::network::NetworkManager::new(storage) {
                Ok(nm) => nm,
                Err(e) => {
                    return Err(anyhow::anyhow!("Failed to create network manager: {}", e));
                }
            };
            
            // Start the network manager
            if let Err(e) = network_manager.start() {
                return Err(anyhow::anyhow!("Failed to start network manager: {}", e));
            }
            
            println!("Local peer ID: {}", network_manager.local_peer_id());
            
            // Discover peers for 10 seconds
            let discovery_start = std::time::Instant::now();
            let discovery_duration = std::time::Duration::from_secs(10);
            
            println!("DEBUG: Starting peer discovery for {} seconds...", discovery_duration.as_secs());
            println!("DEBUG: Target peer: {}", target_peer);
            
            println!("DEBUG: Starting discovery loop for {} seconds", discovery_duration.as_secs());
            while discovery_start.elapsed() < discovery_duration {
                if let Err(e) = network_manager.poll_events().await {
                    eprintln!("Error during peer discovery: {}", e);
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                
                // Check elapsed time more frequently to ensure we exit
                if discovery_start.elapsed() >= discovery_duration {
                    println!("DEBUG: Discovery timeout reached!");
                    break;
                }
            }
            
            println!("DEBUG: Discovery time elapsed ({} seconds), checking results...", discovery_duration.as_secs());
            println!("DEBUG: About to check discovered peers...");
            
            // Check if target peer was discovered
            let peers = network_manager.list_peers();
            println!("DEBUG: Discovery complete");
            println!("DEBUG: Discovered {} peers", peers.len());
            for peer in &peers {
                println!("DEBUG: Peer: {}", peer.peer_id);
            }
            
            println!("DEBUG: Looking for target peer: {}", target_peer);
            let target_peer_found = peers.iter().any(|p| p.peer_id == target_peer);
            println!("DEBUG: Target peer found: {}", target_peer_found);
            
            if !target_peer_found {
                println!("Discovered {} peers, but target peer {} not found", peers.len(), peer_id);
                for peer in &peers {
                    println!("  - {}", peer.peer_id);
                }
                return Err(anyhow::anyhow!("Peer {} not discovered in local network", peer_id));
            }
            
            println!("Found peer {}", peer_id);
            println!("Requesting chunk {}...", &hash[..8]);
            
            // Request the chunk
            match network_manager.request_chunk(target_peer, &hash).await {
                Ok(Some(data)) => {
                    println!("✓ Received {} bytes", data.len());
                    
                    // Save to file
                    let filename = format!("fetched_{}.dat", &hash[..8]);
                    std::fs::write(&filename, data)?;
                    
                    println!("Saved to: {}", filename);
                }
                Ok(None) => {
                    return Err(anyhow::anyhow!("✗ Chunk not available from peer {}", peer_id));
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Failed to fetch chunk: {}", e));
                }
            }
        }
        Commands::Serve => {
            // Check if repository is initialized
            if !Path::new(".fai").exists() {
                return Err(anyhow::anyhow!("Not a FAI repository. Run 'fai init' first."));
            }
            
            println!("FAI server starting...");
            
            // Create storage manager
            let storage = Arc::new(fai_protocol::storage::StorageManager::new(
                Path::new(".fai").join("storage")
            )?);
            
            // Create network manager
            let mut network_manager = match fai_protocol::network::NetworkManager::new(storage) {
                Ok(nm) => nm,
                Err(e) => {
                    return Err(anyhow::anyhow!("Failed to create network manager: {}", e));
                }
            };
            
            // Start the network manager
            if let Err(e) = network_manager.start() {
                return Err(anyhow::anyhow!("Failed to start network manager: {}", e));
            }
            
            println!("FAI server started");
            println!("Local peer ID: {}", network_manager.local_peer_id());
            println!("Ready to serve chunks...");
            println!("Press Ctrl+C to stop");
            
            // Run event loop indefinitely
            loop {
                if let Err(e) = network_manager.poll_events().await {
                    eprintln!("Error during event polling: {}", e);
                }
                
                // Small delay to prevent busy-waiting
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
    }

    Ok(())
}
