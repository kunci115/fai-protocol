use anyhow::Result;
use clap::{Parser, Subcommand};
use fai_protocol::FaiProtocol;
use libp2p::PeerId;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "fai")]
#[command(about = "FAI Protocol - Distributed version control for large files")]
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
        message: String,
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
    /// Pull commits and files from a peer
    Pull {
        /// Peer ID to pull from
        peer_id: String,
        /// Optional specific commit hash to pull (pulls all if not specified)
        commit_hash: Option<String>,
    },
    /// Clone an entire repository from a peer
    Clone {
        /// Peer ID to clone from
        peer_id: String,
        /// Optional target directory (defaults to current directory)
        directory: Option<String>,
    },
    /// Compare two commits or versions
    Diff {
        /// First commit hash
        hash1: String,
        /// Second commit hash
        hash2: String,
    },
    /// Generate shell completion script
    Completion {
        /// Shell type (bash, fish, zsh, powershell, elvish)
        shell: clap_complete::Shell,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle completion commands
    match &cli.command {
        Commands::Completion { shell } => {
            use clap::CommandFactory;
            use std::io;

            let mut cmd = Cli::command();
            let name = "fai";

            match shell {
                clap_complete::Shell::Bash => {
                    clap_complete::generate(clap_complete::shells::Bash, &mut cmd, name, &mut io::stdout());
                }
                clap_complete::Shell::Fish => {
                    clap_complete::generate(clap_complete::shells::Fish, &mut cmd, name, &mut io::stdout());
                }
                clap_complete::Shell::Zsh => {
                    clap_complete::generate(clap_complete::shells::Zsh, &mut cmd, name, &mut io::stdout());
                }
                clap_complete::Shell::PowerShell => {
                    clap_complete::generate(clap_complete::shells::PowerShell, &mut cmd, name, &mut io::stdout());
                }
                clap_complete::Shell::Elvish => {
                    clap_complete::generate(clap_complete::shells::Elvish, &mut cmd, name, &mut io::stdout());
                }
                _ => {
                    eprintln!("Shell not supported for completion generation");
                    return Ok(());
                }
            }
            return Ok(());
        }
        _ => {}
    }

    match cli.command {
        Commands::Completion { .. } => {
            // Already handled above
            unreachable!();
        }
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
                return Err(anyhow::anyhow!(
                    "Not a FAI repository. Run 'fai init' first."
                ));
            }

            // Read file first to show size info
            let file_data = std::fs::read(&path)?;
            let file_size = file_data.len();
            println!("Adding {} to staging area...", path);
            println!(
                "File size: {} bytes ({:.2} MB)",
                file_size,
                file_size as f64 / 1_048_576.0
            );

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
                            if let Ok(manifest) =
                                serde_json::from_str::<serde_json::Value>(manifest_str)
                            {
                                let chunk_count = manifest
                                    .get("chunks")
                                    .and_then(|c| c.as_array())
                                    .map(|c| c.len())
                                    .unwrap_or(0);
                                let total_size = manifest
                                    .get("total_size")
                                    .and_then(|s| s.as_u64())
                                    .unwrap_or(0);

                                println!("Added {} ({})", path, hash);
                                println!("  Manifest hash: {} ({})", hash, &hash[..8]);
                                println!(
                                    "  {} chunks, {} total bytes ({:.2} MB)",
                                    chunk_count,
                                    total_size,
                                    total_size as f64 / 1_048_576.0
                                );

                                if let Some(chunks) =
                                    manifest.get("chunks").and_then(|c| c.as_array())
                                {
                                    for (i, chunk) in chunks.iter().enumerate() {
                                        if let Some(chunk_hash) = chunk.as_str() {
                                            println!(
                                                "  Chunk {}: {} ({})",
                                                i,
                                                chunk_hash,
                                                &chunk_hash[..8]
                                            );
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
                return Err(anyhow::anyhow!(
                    "Not a FAI repository. Run 'fai init' first."
                ));
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
                return Err(anyhow::anyhow!(
                    "Not a FAI repository. Run 'fai init' first."
                ));
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
                    println!(
                        "  {} ({} - {} bytes)",
                        file_path,
                        &file_hash[..8],
                        file_size
                    );
                }
            }
        }
        Commands::Log => {
            // Check if repository is initialized
            if !Path::new(".fai").exists() {
                return Err(anyhow::anyhow!(
                    "Not a FAI repository. Run 'fai init' first."
                ));
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
                Path::new(".fai").to_path_buf(),
            )?);

            // Create database manager
            let database = fai_protocol::database::DatabaseManager::new(
                &Path::new(".fai").join("db.sqlite")
            )?;

            // Create network manager
            let mut network_manager = match fai_protocol::network::NetworkManager::new(storage.clone(), database) {
                Ok(nm) => nm,
                Err(e) => {
                    return Err(anyhow::anyhow!("Failed to create network manager: {}", e));
                }
            };

            // Start the network manager
            if let Err(e) = network_manager.start().await {
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
                return Err(anyhow::anyhow!(
                    "Not a FAI repository. Run 'fai init' first."
                ));
            }

            // Parse peer ID
            let target_peer = PeerId::from_str(&peer_id)
                .map_err(|_| anyhow::anyhow!("Invalid peer ID format: {}", peer_id))?;

            println!("Discovering peers...");

            // Create storage manager
            let storage = Arc::new(fai_protocol::storage::StorageManager::new(
                Path::new(".fai").to_path_buf(),
            )?);

            // Create database manager
            let database = fai_protocol::database::DatabaseManager::new(
                &Path::new(".fai").join("db.sqlite")
            )?;

            // Create network manager (single threaded for now to avoid complex async issues)
            let mut network_manager =
                match fai_protocol::network::NetworkManager::new(storage.clone(), database) {
                    Ok(nm) => nm,
                    Err(e) => {
                        return Err(anyhow::anyhow!("Failed to create network manager: {}", e));
                    }
                };

            // Start the network manager
            if let Err(e) = network_manager.start().await {
                return Err(anyhow::anyhow!("Failed to start network manager: {}", e));
            }

            println!("Local peer ID: {}", network_manager.local_peer_id());

            // Load peers from shared files for test discovery
            if let Ok(loaded) = network_manager.load_peers_from_files() {
                println!("Loaded {} peers from shared files", loaded);
            }

            // Discover peers for 10 seconds
            let discovery_duration = std::time::Duration::from_secs(10);

            println!(
                "DEBUG: Starting peer discovery for {} seconds...",
                discovery_duration.as_secs()
            );
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

                // Check if we should exit - just use the timeout from tokio::select
                // This condition is no longer needed since we're using tokio::select!
            }

            println!(
                "DEBUG: Discovery time elapsed ({} seconds), checking results...",
                discovery_duration.as_secs()
            );
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
                println!(
                    "Discovered {} peers, but target peer {} not found",
                    peers.len(),
                    peer_id
                );
                for peer in &peers {
                    println!("  - {}", peer.peer_id);
                }
                return Err(anyhow::anyhow!(
                    "Peer {} not discovered in local network",
                    peer_id
                ));
            }

            println!("Found peer {}", peer_id);

            // Check if this is a manifest file by reading it directly
            let manifest_path = format!(".fai/objects/{}/{}", &hash[..2], &hash[2..]);
            let is_manifest = std::path::Path::new(&manifest_path).exists()
                && std::fs::read_to_string(&manifest_path)
                    .map(|s| s.trim_start().starts_with('{'))
                    .unwrap_or(false);

            if is_manifest {
                println!("Detected multi-chunk file");

                // Read the manifest to get chunk list
                let manifest_data = std::fs::read_to_string(&manifest_path)?;
                let manifest: serde_json::Value = serde_json::from_str(&manifest_data)?;

                // Clone the chunks array to avoid lifetime issues
                let chunks_array = manifest
                    .get("chunks")
                    .and_then(|c| c.as_array())
                    .map(|c| c.clone())
                    .unwrap_or_default();

                let total_chunks = chunks_array.len();
                println!("Downloading {} chunks...", total_chunks);

                // Pre-allocate vector for chunk data in correct order
                let mut chunks_data: Vec<Option<Vec<u8>>> = vec![None; total_chunks];

                // Download chunks sequentially for now (parallel version would require more complex async handling)
                for (i, chunk_value) in chunks_array.iter().enumerate() {
                    if let Some(chunk_hash) = chunk_value.as_str() {
                        println!(
                            "Downloading chunk {}/{} ({})...",
                            i + 1,
                            total_chunks,
                            &chunk_hash[..8]
                        );
                        match network_manager
                            .request_chunk(target_peer.clone(), chunk_hash)
                            .await
                        {
                            Ok(Some(data)) => {
                                println!("✓ Downloaded chunk {} ({} bytes)", i + 1, data.len());
                                chunks_data[i] = Some(data);
                            }
                            Ok(None) => {
                                return Err(anyhow::anyhow!(
                                    "✗ Chunk {} not available from peer",
                                    i + 1
                                ));
                            }
                            Err(e) => {
                                return Err(anyhow::anyhow!(
                                    "Failed to fetch chunk {}: {}",
                                    i + 1,
                                    e
                                ));
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
                        println!(
                            "DEBUG: File exists: {}",
                            std::path::Path::new(&filename).exists()
                        );
                    }
                    Ok(None) => {
                        println!("DEBUG: Chunk {} not available from peer {}", hash, peer_id);
                        return Err(anyhow::anyhow!(
                            "✗ Chunk not available from peer {}",
                            peer_id
                        ));
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
                return Err(anyhow::anyhow!(
                    "Not a FAI repository. Run 'fai init' first."
                ));
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
                            if let Ok(manifest) =
                                serde_json::from_str::<serde_json::Value>(manifest_str)
                            {
                                println!(
                                    "File: multi-chunk file (manifest: {}{})",
                                    &hash[..8],
                                    &hash[8..16]
                                );
                                println!("Chunks:");

                                if let Some(chunks) =
                                    manifest.get("chunks").and_then(|c| c.as_array())
                                {
                                    for (i, chunk) in chunks.iter().enumerate() {
                                        if let Some(chunk_hash) = chunk.as_str() {
                                            if let Some(chunk_data) =
                                                fai.storage().retrieve(chunk_hash).ok()
                                            {
                                                println!(
                                                    "  {}: {} ({} bytes)",
                                                    i,
                                                    chunk_hash,
                                                    chunk_data.len()
                                                );
                                            } else {
                                                println!(
                                                    "  {}: {} (not found in storage)",
                                                    i, chunk_hash
                                                );
                                            }
                                        }
                                    }
                                }

                                if let Some(total_size) =
                                    manifest.get("total_size").and_then(|s| s.as_u64())
                                {
                                    println!(
                                        "Total: {} chunks, {} bytes ({:.2} MB)",
                                        manifest
                                            .get("chunks")
                                            .and_then(|c| c.as_array())
                                            .map(|c| c.len())
                                            .unwrap_or(0),
                                        total_size,
                                        total_size as f64 / 1_048_576.0
                                    );
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
                                    println!(
                                        "Size: {} bytes ({:.2} MB)",
                                        data.len(),
                                        data.len() as f64 / 1_048_576.0
                                    );
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
                                println!(
                                    "Size: {} bytes ({:.2} MB)",
                                    data.len(),
                                    data.len() as f64 / 1_048_576.0
                                );
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
                return Err(anyhow::anyhow!(
                    "Not a FAI repository. Run 'fai init' first."
                ));
            }

            // Parse peer ID
            let target_peer = PeerId::from_str(&peer_id)
                .map_err(|_| anyhow::anyhow!("Invalid peer ID format: {}", peer_id))?;

            println!("Discovering peers...");
            println!("DEBUG: About to create storage manager");

            // Create storage manager
            let storage = Arc::new(fai_protocol::storage::StorageManager::new(
                Path::new(".fai").to_path_buf(),
            )?);
            println!("DEBUG: Storage manager created");

            // Create database manager
            let database = fai_protocol::database::DatabaseManager::new(
                &Path::new(".fai").join("db.sqlite")
            )?;
            println!("DEBUG: Database manager created");

            // Create network manager
            println!("DEBUG: About to create network manager");
            let mut network_manager =
                match fai_protocol::network::NetworkManager::new(storage.clone(), database) {
                    Ok(nm) => nm,
                    Err(e) => {
                        return Err(anyhow::anyhow!("Failed to create network manager: {}", e));
                    }
                };
            println!("DEBUG: Network manager created");

            // Start the network manager
            println!("DEBUG: About to start network manager");
            if let Err(e) = network_manager.start().await {
                return Err(anyhow::anyhow!("Failed to start network manager: {}", e));
            }
            println!("DEBUG: Network manager started");

            println!("Local peer ID: {}", network_manager.local_peer_id());

            // Load peers from shared files for test discovery
            if let Ok(loaded) = network_manager.load_peers_from_files() {
                println!("Loaded {} peers from shared files", loaded);
            }

            // Discover peers for 10 seconds
            let discovery_duration = std::time::Duration::from_secs(10);

            println!(
                "Discovering peers for {} seconds...",
                discovery_duration.as_secs()
            );

            // Simplified discovery loop
            // while discovery_start.elapsed() < discovery_duration {
            //     if let Err(e) = network_manager.poll_events().await {
            //         eprintln!("Error during peer discovery: {}", e);
            //     }
            //     tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            // }

            // Check if target peer was discovered
            let peers = network_manager.list_peers();
            let target_peer_found = peers.iter().any(|p| p.peer_id == target_peer);

            if !target_peer_found {
                println!(
                    "Discovered {} peers, but target peer {} not found",
                    peers.len(),
                    peer_id
                );
                for peer in &peers {
                    println!("  - {}", peer.peer_id);
                }
                return Err(anyhow::anyhow!(
                    "Peer {} not discovered in local network",
                    peer_id
                ));
            }

            println!("Found peer {}", peer_id);

            // Check if target peer was discovered
            let peers = network_manager.list_peers();
            let target_peer_found = peers.iter().any(|p| p.peer_id == target_peer);

            if !target_peer_found {
                println!(
                    "Discovered {} peers, but target peer {} not found",
                    peers.len(),
                    peer_id
                );
                for peer in &peers {
                    println!("  - {}", peer.peer_id);
                }
                return Err(anyhow::anyhow!(
                    "Peer {} not discovered in local network",
                    peer_id
                ));
            }

            // Get all commits to push
            let commits = storage.get_all_commits()?;

            if commits.is_empty() {
                println!("No commits to push");
                return Ok(());
            }

            println!("Found {} commits to push", commits.len());

            // Create commit info for network transfer (convert timestamp from i64 to proper format)
            let commit_infos: Vec<fai_protocol::storage::CommitInfo> = commits
                .into_iter()
                .map(|c| fai_protocol::storage::CommitInfo {
                    hash: c.hash,
                    message: c.message,
                    timestamp: c.timestamp,
                    file_hashes: c.file_hashes,
                })
                .collect();

            // Push commits to peer
            println!("Pushing commits to peer {}...", peer_id);
            println!("DEBUG: About to call network_manager.send_commits");

            // Actually send commits to the peer
            match network_manager
                .send_commits(target_peer.clone(), commit_infos.clone())
                .await
            {
                Ok(_) => {
                    println!(
                        "✓ Successfully pushed {} commits to peer {}",
                        commit_infos.len(),
                        peer_id
                    );
                }
                Err(e) => {
                    eprintln!("Failed to push commits: {}", e);
                    return Err(anyhow::anyhow!("Push failed: {}", e));
                }
            }
        }
        Commands::Pull {
            peer_id,
            commit_hash,
        } => {
            // Check if repository is initialized
            if !Path::new(".fai").exists() {
                return Err(anyhow::anyhow!(
                    "Not a FAI repository. Run 'fai init' first."
                ));
            }

            // Parse peer ID
            let target_peer = PeerId::from_str(&peer_id)
                .map_err(|_| anyhow::anyhow!("Invalid peer ID format: {}", peer_id))?;

            println!("Pulling commits from peer {}...", peer_id);

            // Create storage manager
            let storage = Arc::new(fai_protocol::storage::StorageManager::new(
                Path::new(".fai").to_path_buf(),
            )?);

            // Create database manager
            let database = fai_protocol::database::DatabaseManager::new(
                &Path::new(".fai").join("db.sqlite")
            )?;

            // Create network manager
            let mut network_manager =
                match fai_protocol::network::NetworkManager::new(storage.clone(), database) {
                    Ok(nm) => nm,
                    Err(e) => {
                        return Err(anyhow::anyhow!("Failed to create network manager: {}", e));
                    }
                };

            // Start the network manager
            if let Err(e) = network_manager.start().await {
                return Err(anyhow::anyhow!("Failed to start network manager: {}", e));
            }

            println!("Local peer ID: {}", network_manager.local_peer_id());

            // Load peers from shared files for test discovery
            if let Ok(loaded) = network_manager.load_peers_from_files() {
                println!("Loaded {} peers from shared files", loaded);
            }

            // Discover peers for 10 seconds
            let discovery_duration = std::time::Duration::from_secs(10);

            println!(
                "Discovering peers for {} seconds...",
                discovery_duration.as_secs()
            );

            // Simplified discovery loop
            // while discovery_start.elapsed() < discovery_duration {
            //     if let Err(e) = network_manager.poll_events().await {
            //         eprintln!("Error during peer discovery: {}", e);
            //     }
            //     tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            // }

            // Check if target peer was discovered
            let peers = network_manager.list_peers();
            let target_peer_found = peers.iter().any(|p| p.peer_id == target_peer);

            if !target_peer_found {
                println!(
                    "Discovered {} peers, but target peer {} not found",
                    peers.len(),
                    peer_id
                );
                for peer in &peers {
                    println!("  - {}", peer.peer_id);
                }
                return Err(anyhow::anyhow!(
                    "Peer {} not discovered in local network",
                    peer_id
                ));
            }

            println!("Found peer {}", peer_id);

            // Request commits from peer
            println!("Requesting commits from peer {}...", peer_id);
            println!("DEBUG: About to call network_manager.request_commits");
            let commits = network_manager
                .request_commits(target_peer.clone(), commit_hash.clone())
                .await?;
            println!("DEBUG: request_commits returned");

            if commits.is_empty() {
                println!("No new commits to pull");
                return Ok(());
            }

            println!("Found {} commits to pull", commits.len());

            // For each commit, pull the files
            for commit in &commits {
                println!("Pulling commit: {} - {}", &commit.hash[..8], commit.message);

                // Download all files referenced in this commit
                for file_hash in &commit.file_hashes {
                    println!("  Fetching file {}...", &file_hash[..8]);

                    // Check if we already have this file
                    if storage.retrieve(file_hash).is_ok() {
                        println!("  ✓ Already have file {}", &file_hash[..8]);
                        continue;
                    }

                    // Download the file (reuse fetch logic)
                    match network_manager
                        .request_chunk(target_peer.clone(), file_hash)
                        .await
                    {
                        Ok(Some(data)) => {
                            storage.store(&data)?;
                            println!(
                                "  ✓ Downloaded file {} ({} bytes)",
                                &file_hash[..8],
                                data.len()
                            );
                        }
                        Ok(None) => {
                            println!("  ✗ File {} not available", &file_hash[..8]);
                        }
                        Err(e) => {
                            println!("  ✗ Failed to download file {}: {}", &file_hash[..8], e);
                        }
                    }
                }

                // Save the commit to local database
                storage.save_remote_commit(commit)?;
                println!("✓ Pulled commit: {}", &commit.hash[..8]);
            }

            println!("✓ Pull complete! Pulled {} commits", commits.len());
        }
        Commands::Clone { peer_id, directory } => {
            println!("Cloning repository from peer {}...", peer_id);

            // Parse peer ID
            let target_peer = PeerId::from_str(&peer_id)
                .map_err(|_| anyhow::anyhow!("Invalid peer ID format: {}", peer_id))?;

            // Determine target directory
            let target_dir = directory.unwrap_or_else(|| ".".to_string());
            let repo_path = std::path::Path::new(&target_dir).join(".fai");

            // Check if repo already exists
            if repo_path.exists() {
                return Err(anyhow::anyhow!(
                    "Repository already exists at {}",
                    repo_path.display()
                ));
            }

            // Create target directory if it doesn't exist
            if target_dir != "." {
                std::fs::create_dir_all(&target_dir)?;
                println!("Created target directory: {}", target_dir);
            }

            // Initialize new repository in target directory
            println!("Initializing repository in {}...", target_dir);

            // Create the .fai directory structure
            let fai_path = repo_path.clone();
            std::fs::create_dir_all(fai_path.join("objects"))?;

            // Create storage manager
            let storage = Arc::new(fai_protocol::storage::StorageManager::new(
                fai_path.to_path_buf(),
            )?);

            // Create database manager
            let database = fai_protocol::database::DatabaseManager::new(
                &fai_path.join("db.sqlite")
            )?;

            // Initialize network
            let mut network_manager =
                match fai_protocol::network::NetworkManager::new(storage.clone(), database) {
                    Ok(nm) => nm,
                    Err(e) => {
                        return Err(anyhow::anyhow!("Failed to create network manager: {}", e));
                    }
                };

            network_manager.start().await?;

            println!("Local peer ID: {}", network_manager.local_peer_id());

            // Load peers from shared files for test discovery
            if let Ok(loaded) = network_manager.load_peers_from_files() {
                println!("Loaded {} peers from shared files", loaded);
            }

            // Discover peer
            println!("Discovering peer...");

            // Simplified discovery loop
            // while discovery_start.elapsed() < discovery_duration {
            //     if let Err(e) = network_manager.poll_events().await {
            //         eprintln!("Error during peer discovery: {}", e);
            //     }
            //     tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            // }

            // Check if target peer was discovered
            let peers = network_manager.list_peers();
            let target_peer_found = peers.iter().any(|p| p.peer_id == target_peer);

            if !target_peer_found {
                println!(
                    "Discovered {} peers, but target peer {} not found",
                    peers.len(),
                    peer_id
                );
                for peer in &peers {
                    println!("  - {}", peer.peer_id);
                }
                return Err(anyhow::anyhow!(
                    "Peer {} not discovered in local network",
                    peer_id
                ));
            }

            println!("Found peer {}", peer_id);

            // Request ALL commits from peer
            println!("Fetching commit history...");
            let commits = network_manager
                .request_commits(target_peer.clone(), None)
                .await?;

            if commits.is_empty() {
                println!("⚠️  Peer has no commits");
                return Ok(());
            }

            println!("Found {} commits to clone", commits.len());

            // Collect all unique file hashes across all commits
            let mut all_file_hashes: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            for commit in &commits {
                for file_hash in &commit.file_hashes {
                    all_file_hashes.insert(file_hash.clone());
                }
            }

            println!("Downloading {} unique files...", all_file_hashes.len());

            // Download all files
            let mut downloaded = 0;
            for file_hash in &all_file_hashes {
                print!(
                    "  Downloading file {}/{} ({})... ",
                    downloaded + 1,
                    all_file_hashes.len(),
                    &file_hash[..8]
                );

                match network_manager
                    .request_chunk(target_peer.clone(), file_hash)
                    .await
                {
                    Ok(Some(data)) => {
                        storage.store(&data)?;
                        println!("✓ {} bytes", data.len());
                        downloaded += 1;
                    }
                    Ok(None) => {
                        println!("✗ Not available");
                    }
                    Err(e) => {
                        println!("✗ Failed: {}", e);
                    }
                }
            }

            println!(
                "✓ Downloaded {}/{} files",
                downloaded,
                all_file_hashes.len()
            );

            // Save all commits to local database
            println!("Importing commit history...");
            for (i, commit) in commits.iter().enumerate() {
                storage.save_remote_commit(commit)?;
                println!(
                    "  Imported commit {}/{}: {} - {}",
                    i + 1,
                    commits.len(),
                    &commit.hash[..8],
                    commit.message
                );
            }

            println!("\n✓ Clone complete!");
            println!("  Repository: {}", repo_path.display());
            println!("  Commits: {}", commits.len());
            println!("  Files: {}", downloaded);
        }
        Commands::Diff { hash1, hash2 } => {
            // Check if repository is initialized
            if !Path::new(".fai").exists() {
                return Err(anyhow::anyhow!(
                    "Not a FAI repository. Run 'fai init' first."
                ));
            }

            println!("Comparing versions...");
            println!("  Version 1: {}", &hash1[..8]);
            println!("  Version 2: {}", &hash2[..8]);
            println!();

            // Create storage and database managers
            let storage =
                fai_protocol::storage::StorageManager::new(Path::new(".fai").to_path_buf())?;
            let database = fai_protocol::database::DatabaseManager::new(
                &Path::new(".fai").join("db.sqlite")
            )?;

            // Get both commits from database (try short hash matching if full hash not found)
            let db_commit1 = if let Some(commit) = database.get_commit(&hash1)? {
                commit
            } else {
                // Try to find commit by short hash prefix
                let commits = database.get_commit_history(None)?;
                let matching_commits: Vec<_> = commits.iter()
                    .filter(|c| c.hash.starts_with(&hash1))
                    .collect();
                if matching_commits.len() == 1 {
                    matching_commits[0].clone()
                } else {
                    return Err(anyhow::anyhow!("Commit not found: {} (found {} matches)", hash1, matching_commits.len()));
                }
            };

            let db_commit2 = if let Some(commit) = database.get_commit(&hash2)? {
                commit
            } else {
                // Try to find commit by short hash prefix
                let commits = database.get_commit_history(None)?;
                let matching_commits: Vec<_> = commits.iter()
                    .filter(|c| c.hash.starts_with(&hash2))
                    .collect();
                if matching_commits.len() == 1 {
                    matching_commits[0].clone()
                } else {
                    return Err(anyhow::anyhow!("Commit not found: {} (found {} matches)", hash2, matching_commits.len()));
                }
            };

            // Get file hashes for both commits (use full hashes from found commits)
            let files1 = database.get_commit_files(&db_commit1.hash)?;
            let files2 = database.get_commit_files(&db_commit2.hash)?;

            println!("Commit 1: {}", db_commit1.message);
            println!(
                "  Date: {}",
                db_commit1.timestamp.format("%Y-%m-%d %H:%M:%S")
            );
            println!("  Files: {}", files1.len());

            println!();

            println!("Commit 2: {}", db_commit2.message);
            println!(
                "  Date: {}",
                db_commit2.timestamp.format("%Y-%m-%d %H:%M:%S")
            );
            println!("  Files: {}", files2.len());

            println!();
            println!("=== Changes ===");

            // Convert to HashSets for comparison (use only file hashes)
            let files1_hashes: std::collections::HashSet<_> = files1.iter()
                .map(|(_, hash, _)| hash)
                .collect();
            let files2_hashes: std::collections::HashSet<_> = files2.iter()
                .map(|(_, hash, _)| hash)
                .collect();

            // Files only in commit1 (removed in commit2)
            let removed: Vec<_> = files1_hashes.difference(&files2_hashes).collect();

            // Files only in commit2 (added in commit2)
            let added: Vec<_> = files2_hashes.difference(&files1_hashes).collect();

            // Files in both (unchanged)
            let unchanged: Vec<_> = files1_hashes.intersection(&files2_hashes).collect();

            if !removed.is_empty() {
                println!("\n❌ Removed files ({}):", removed.len());
                for file_hash in &removed {
                    // Try to get file size
                    if let Ok(data) = storage.retrieve(file_hash) {
                        println!("  - {} ({} bytes)", &file_hash[..8], data.len());
                    } else {
                        println!("  - {}", &file_hash[..8]);
                    }
                }
            }

            if !added.is_empty() {
                println!("\n✅ Added files ({}):", added.len());
                for file_hash in &added {
                    // Try to get file size
                    if let Ok(data) = storage.retrieve(file_hash) {
                        println!("  + {} ({} bytes)", &file_hash[..8], data.len());
                    } else {
                        println!("  + {}", &file_hash[..8]);
                    }
                }
            }

            if !unchanged.is_empty() {
                println!("\n⚪ Unchanged files ({}):", unchanged.len());
                for file_hash in unchanged.iter().take(5) {
                    println!("  = {}", &file_hash[..8]);
                }
                if unchanged.len() > 5 {
                    println!("  ... and {} more", unchanged.len() - 5);
                }
            }

            // Summary
            println!();
            println!("=== Summary ===");
            println!("  Added:     {} files", added.len());
            println!("  Removed:   {} files", removed.len());
            println!("  Unchanged: {} files", unchanged.len());

            // Calculate total size change
            let mut size_change: i64 = 0;
            for file_hash in &added {
                if let Ok(data) = storage.retrieve(file_hash) {
                    size_change += data.len() as i64;
                }
            }
            for file_hash in &removed {
                if let Ok(data) = storage.retrieve(file_hash) {
                    size_change -= data.len() as i64;
                }
            }

            if size_change > 0 {
                println!(
                    "  Size:      +{} bytes ({:.2} MB)",
                    size_change,
                    size_change as f64 / 1_048_576.0
                );
            } else if size_change < 0 {
                println!(
                    "  Size:      {} bytes ({:.2} MB)",
                    size_change,
                    size_change as f64 / 1_048_576.0
                );
            } else {
                println!("  Size:      No change");
            }
        }
        Commands::Serve => {
            // Check if repository is initialized
            if !Path::new(".fai").exists() {
                return Err(anyhow::anyhow!(
                    "Not a FAI repository. Run 'fai init' first."
                ));
            }

            println!("FAI server starting...");

            // Create storage manager
            let storage = Arc::new(fai_protocol::storage::StorageManager::new(
                Path::new(".fai").to_path_buf(),
            )?);

            // Create database manager
            let database = fai_protocol::database::DatabaseManager::new(
                &Path::new(".fai").join("db.sqlite")
            )?;

            // Create network manager
            let mut network_manager = match fai_protocol::network::NetworkManager::new(storage.clone(), database) {
                Ok(nm) => nm,
                Err(e) => {
                    return Err(anyhow::anyhow!("Failed to create network manager: {}", e));
                }
            };

            // Start the network manager
            if let Err(e) = network_manager.start().await {
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
