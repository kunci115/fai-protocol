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
    /// List chunks for a multi-chunk file
    Chunks { hash: String },
    /// Push commits to a peer
    Push {
        /// Peer ID to push to
        peer_id: String,
    },
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
            
            // Read file first to show size info
            let file_data = std::fs::read(&path)?;
            let file_size = file_data.len();
            println!("Adding {} to staging area...", path);
            println!("File size: {} bytes ({:.2} MB)", file_size, file_size as f64 / 1_048_576.0);
            
            // Initialize FAI protocol
            let fai = FaiProtocol::new()?;
            
            // Add file to staging
            let hash = fai.add_file(&path)?;
            
            // Determine if this was a chunked file by checking if manifest exists
            let manifest_path = format!(".fai/objects/{}/{}", &hash[..2], &hash[2..]);
            if std::path::Path::new(&manifest_path).exists() {
                if let Ok(manifest_data) = std::fs::read(&manifest_path) {
                    if let Ok(manifest_str) = std::str::from_utf8(&manifest_data) {
                        if manifest_str.trim_start().starts_with('{') {
                            println!("✓ File was chunked into multiple pieces");
                            if let Ok(manifest) = serde_json::from_str::<serde_json::Value>(manifest_str) {
                                let chunk_count = manifest.get("chunks").and_then(|c| c.as_array()).map(|c| c.len()).unwrap_or(0);
                                let total_size = manifest.get("total_size").and_then(|s| s.as_u64()).unwrap_or(0);
                                
                                println!("Added {} ({})", path, hash);
                                println!("  Manifest hash: {} ({})", hash, &hash[..8]);
                                println!("  {} chunks, {} total bytes ({:.2} MB)", 
                                    chunk_count, total_size, total_size as f64 / 1_048_576.0);
                                
                                if let Some(chunks) = manifest.get("chunks").and_then(|c| c.as_array()) {
                                    for (i, chunk) in chunks.iter().enumerate() {
                                        if let Some(chunk_hash) = chunk.as_str() {
                                            println!("  Chunk {}: {} ({})", i, chunk_hash, &chunk_hash[..8]);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                println!("✓ File stored as single object");
                println!("Added {} ({})", path, hash);
                println!("  File hash: {} ({})", hash, &hash[..8]);
            }
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
                Path::new(".fai").to_path_buf()
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
                Path::new(".fai").to_path_buf()
            )?);
            
            // Create network manager (single threaded for now to avoid complex async issues)
            let mut network_manager = match fai_protocol::network::NetworkManager::new(storage.clone()) {
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
            
            println!("DEBUG: Starting discovery with tokio::select timeout");
            loop {
                tokio::select! {
                    result = network_manager.poll_events() => {
                        if let Err(e) = result {
                            eprintln!("Error during peer discovery: {}", e);
                        }
                    }
                    _ = tokio::time::sleep(discovery_duration) => {
                        println!("DEBUG: Discovery timeout reached after {} seconds!", discovery_duration.as_secs());
                        break;
                    }
                }
                
                // Check if we should exit
                if discovery_start.elapsed() >= discovery_duration {
                    println!("DEBUG: Discovery duration elapsed, breaking");
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
            
            // Check if this is a manifest file by reading it directly
            let manifest_path = format!(".fai/objects/{}/{}", &hash[..2], &hash[2..]);
            let is_manifest = std::path::Path::new(&manifest_path).exists() && 
                             std::fs::read_to_string(&manifest_path)
                                .map(|s| s.trim_start().starts_with('{'))
                                .unwrap_or(false);
            
            if is_manifest {
                println!("Detected multi-chunk file");
                
                // Read the manifest to get chunk list
                let manifest_data = std::fs::read_to_string(&manifest_path)?;
                let manifest: serde_json::Value = serde_json::from_str(&manifest_data)?;
                
                // Clone the chunks array to avoid lifetime issues
                let chunks_array = manifest.get("chunks").and_then(|c| c.as_array())
                    .map(|c| c.clone())
                    .unwrap_or_default();
                
                let total_chunks = chunks_array.len();
                println!("Downloading {} chunks...", total_chunks);
                
                // Pre-allocate vector for chunk data in correct order
                let mut chunks_data: Vec<Option<Vec<u8>>> = vec![None; total_chunks];
                
                // Download chunks sequentially for now (parallel version would require more complex async handling)
                for (i, chunk_value) in chunks_array.iter().enumerate() {
                    if let Some(chunk_hash) = chunk_value.as_str() {
                        println!("Downloading chunk {}/{} ({})...", i + 1, total_chunks, &chunk_hash[..8]);
                        match network_manager.request_chunk(target_peer.clone(), chunk_hash).await {
                            Ok(Some(data)) => {
                                println!("✓ Downloaded chunk {} ({} bytes)", i + 1, data.len());
                                chunks_data[i] = Some(data);
                            }
                            Ok(None) => {
                                return Err(anyhow::anyhow!("✗ Chunk {} not available from peer", i + 1));
                            }
                            Err(e) => {
                                return Err(anyhow::anyhow!("Failed to fetch chunk {}: {}", i + 1, e));
                            }
                        }
                    }
                }
                
                println!("✓ All {} chunks downloaded", total_chunks);
                
                // Verify all chunks were downloaded
                for (i, chunk_data) in chunks_data.iter().enumerate() {
                    if chunk_data.is_none() {
                        return Err(anyhow::anyhow!("Chunk {} failed to download", i + 1));
                    }
                }
                
                // Assemble complete file
                println!("Assembling complete file from {} chunks...", total_chunks);
                let mut complete_data = Vec::new();
                for chunk_data in chunks_data {
                    if let Some(data) = chunk_data {
                        complete_data.extend_from_slice(&data);
                    }
                }
                
                // Save complete file
                let filename = format!("fetched_{}.dat", hash);
                let complete_data_len = complete_data.len();
                std::fs::write(&filename, complete_data)?;
                
                println!("✓ Assembled complete file ({} bytes)", complete_data_len);
                println!("Saved to: {}", filename);

            } else {
                // Single chunk file
                println!("Requesting chunk {}...", &hash[..8]);
                
                // Request the chunk
                match network_manager.request_chunk(target_peer, &hash).await {
                    Ok(Some(data)) => {
                        println!("✓ Received {} bytes", data.len());
                        
                        // Save to file using full hash
                        let filename = format!("fetched_{}.dat", hash);
                        let absolute_path = std::env::current_dir().unwrap().join(&filename);
                        println!("DEBUG: Saving to absolute path: {:?}", absolute_path);
                        
                        std::fs::write(&filename, data)?;
                        println!("DEBUG: File written successfully");
                        println!("Saved to: {}", filename);
                        println!("DEBUG: File exists: {}", std::path::Path::new(&filename).exists());
                    }
                    Ok(None) => {
                        println!("DEBUG: Chunk {} not available from peer {}", hash, peer_id);
                        return Err(anyhow::anyhow!("✗ Chunk not available from peer {}", peer_id));
                    }
                    Err(e) => {
                        return Err(anyhow::anyhow!("Failed to fetch chunk: {}", e));
                    }
                }
            }
        }
        Commands::Chunks { hash } => {
            // Check if repository is initialized
            if !Path::new(".fai").exists() {
                return Err(anyhow::anyhow!("Not a FAI repository. Run 'fai init' first."));
            }
            
            // Initialize FAI protocol
            let fai = FaiProtocol::new()?;
            
            // Check if this is a manifest file by reading it directly without reconstruction
            let manifest_path = format!(".fai/objects/{}/{}", &hash[..2], &hash[2..]);
            
            if std::path::Path::new(&manifest_path).exists() {
                // Read the raw file to see if it's a manifest
                if let Ok(raw_data) = std::fs::read(&manifest_path) {
                    if let Ok(manifest_str) = std::str::from_utf8(&raw_data) {
                        if manifest_str.trim_start().starts_with('{') {
                            // This is a manifest file
                            if let Ok(manifest) = serde_json::from_str::<serde_json::Value>(manifest_str) {
                                println!("File: multi-chunk file (manifest: {}{})", &hash[..8], &hash[8..16]);
                                println!("Chunks:");
                                
                                if let Some(chunks) = manifest.get("chunks").and_then(|c| c.as_array()) {
                                    for (i, chunk) in chunks.iter().enumerate() {
                                        if let Some(chunk_hash) = chunk.as_str() {
                                            if let Some(chunk_data) = fai.storage().retrieve(chunk_hash).ok() {
                                                println!("  {}: {} ({} bytes)", i, chunk_hash, chunk_data.len());
                                            } else {
                                                println!("  {}: {} (not found in storage)", i, chunk_hash);
                                            }
                                        }
                                    }
                                }
                                
                                if let Some(total_size) = manifest.get("total_size").and_then(|s| s.as_u64()) {
                                    println!("Total: {} chunks, {} bytes ({:.2} MB)", 
                                        manifest.get("chunks").and_then(|c| c.as_array()).map(|c| c.len()).unwrap_or(0),
                                        total_size, total_size as f64 / 1_048_576.0);
                                }
                            } else {
                                println!("Error: Invalid manifest format");
                                return Err(anyhow::anyhow!("Invalid manifest format"));
                            }
                        } else {
                            // This is a single chunk file
                            match fai.storage().retrieve(&hash) {
                                Ok(data) => {
                                    println!("File: single-chunk file");
                                    println!("Hash: {} ({})", hash, &hash[..8]);
                                    println!("Size: {} bytes ({:.2} MB)", data.len(), data.len() as f64 / 1_048_576.0);
                                    println!("Chunks: 1 (this is a single chunk file)");
                                }
                                Err(e) => {
                                    return Err(anyhow::anyhow!("Failed to retrieve file: {}", e));
                                }
                            }
                        }
                    } else {
                        // Binary data, treat as single chunk
                        match fai.storage().retrieve(&hash) {
                            Ok(data) => {
                                println!("File: single-chunk file");
                                println!("Hash: {} ({})", hash, &hash[..8]);
                                println!("Size: {} bytes ({:.2} MB)", data.len(), data.len() as f64 / 1_048_576.0);
                                println!("Chunks: 1 (this is a single chunk file)");
                            }
                            Err(e) => {
                                return Err(anyhow::anyhow!("Failed to retrieve file: {}", e));
                            }
                        }
                    }
                } else {
                    return Err(anyhow::anyhow!("Failed to read manifest file"));
                }
            } else {
                return Err(anyhow::anyhow!("File not found in storage"));
            }
        }
        Commands::Push { peer_id } => {
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
                Path::new(".fai").to_path_buf()
            )?);
            
            // Create network manager
            let mut network_manager = match fai_protocol::network::NetworkManager::new(storage.clone()) {
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
            
            println!("Discovering peers for {} seconds...", discovery_duration.as_secs());
            
            while discovery_start.elapsed() < discovery_duration {
                if let Err(e) = network_manager.poll_events().await {
                    eprintln!("Error during peer discovery: {}", e);
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            
            // Check if target peer was discovered
            let peers = network_manager.list_peers();
            let target_peer_found = peers.iter().any(|p| p.peer_id == target_peer);
            
            if !target_peer_found {
                println!("Discovered {} peers, but target peer {} not found", peers.len(), peer_id);
                for peer in &peers {
                    println!("  - {}", peer.peer_id);
                }
                return Err(anyhow::anyhow!("Peer {} not discovered in local network", peer_id));
            }
            
            println!("Found peer {}", peer_id);
            
            // Get all commits to push
            let commits = storage.get_all_commits()?;
            
            if commits.is_empty() {
                println!("No commits to push");
                return Ok(());
            }
            
            println!("Found {} commits to push", commits.len());
            
            // Create commit info for network transfer (convert timestamp from i64 to proper format)
            let commit_infos: Vec<fai_protocol::storage::CommitInfo> = commits.into_iter()
                .map(|c| fai_protocol::storage::CommitInfo {
                    hash: c.hash,
                    message: c.message,
                    timestamp: c.timestamp,
                    file_hashes: c.file_hashes,
                })
                .collect();
            
            // Push commits to peer
            println!("Pushing commits to peer {}...", peer_id);
            
            // Note: This is a simplified implementation
            // In a full implementation, you would:
            // 1. Send commit requests to the peer
            // 2. Handle the response and potentially send individual commits
            // 3. Verify the commits were received
            
            for (i, commit) in commit_infos.iter().enumerate() {
                println!("Sending commit {}/{}: {} ({})", 
                    i + 1, 
                    commit_infos.len(), 
                    &commit.hash[..8], 
                    commit.message.lines().next().unwrap_or(""));
            }
            
            println!("✓ Pushed {} commits to peer {}", commit_infos.len(), peer_id);
        }
        Commands::Serve => {
            // Check if repository is initialized
            if !Path::new(".fai").exists() {
                return Err(anyhow::anyhow!("Not a FAI repository. Run 'fai init' first."));
            }
            
            println!("FAI server starting...");
            
            // Create storage manager
            let storage = Arc::new(fai_protocol::storage::StorageManager::new(
                Path::new(".fai").to_path_buf()
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
