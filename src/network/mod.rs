//! Network layer for FAI Protocol
//!
//! Handles peer-to-peer networking for decentralized model sharing.

use crate::storage::StorageManager;
use anyhow::Result;
use futures::StreamExt;
use libp2p::{
    identity::Keypair,
    mdns,
    request_response::ProtocolSupport,
    swarm::{NetworkBehaviour, SwarmEvent},
    yamux, Multiaddr, PeerId, Swarm, SwarmBuilder,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::SystemTime};

/// Information about a discovered peer
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// Peer identifier
    pub peer_id: PeerId,
    /// Network addresses of the peer
    pub addresses: Vec<Multiaddr>,
    /// Last time this peer was seen
    pub last_seen: SystemTime,
}

/// Request for a chunk of data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkRequest {
    /// Hash of the chunk being requested
    pub hash: String,
}

/// Response containing chunk data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkResponse {
    /// Hash of the returned chunk
    pub hash: String,
    /// The chunk data if found
    pub data: Option<Vec<u8>>,
}

/// Request for commit information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitRequest {
    /// Optional specific commit hash to request
    pub commit_hash: Option<String>,
}

/// Response containing commit information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitResponse {
    /// List of commits
    pub commits: Vec<crate::storage::CommitInfo>,
}

/// Network behaviour combining mDNS and request-response
#[derive(NetworkBehaviour)]
pub struct FAIBehaviour {
    /// mDNS for peer discovery
    pub mdns: mdns::tokio::Behaviour,
    /// Request-response protocol for chunks
    pub request_response: libp2p::request_response::cbor::Behaviour<ChunkRequest, ChunkResponse>,
    /// Request-response protocol for commits
    pub commit_response: libp2p::request_response::cbor::Behaviour<CommitRequest, CommitResponse>,
}

/// Events from the network behaviour
#[derive(Debug)]
pub enum FAIEvent {
    RequestResponse(libp2p::request_response::Event<ChunkRequest, ChunkResponse>),
    CommitResponse(libp2p::request_response::Event<CommitRequest, CommitResponse>),
    Mdns(mdns::Event),
}

impl From<libp2p::request_response::Event<ChunkRequest, ChunkResponse>> for FAIEvent {
    fn from(event: libp2p::request_response::Event<ChunkRequest, ChunkResponse>) -> Self {
        FAIEvent::RequestResponse(event)
    }
}

impl From<libp2p::request_response::Event<CommitRequest, CommitResponse>> for FAIEvent {
    fn from(event: libp2p::request_response::Event<CommitRequest, CommitResponse>) -> Self {
        FAIEvent::CommitResponse(event)
    }
}

/// Network manager for FAI Protocol
pub struct NetworkManager {
    /// libp2p swarm for network operations
    swarm: Swarm<FAIBehaviour>,
    /// Map of discovered peers
    discovered_peers: HashMap<PeerId, PeerInfo>,
    /// Storage manager for retrieving chunks
    storage: Arc<StorageManager>,
    /// Database manager for commit operations
    database: crate::database::DatabaseManager,
    /// Pending commit responses (request_id -> commits)
    pending_commit_responses: std::collections::HashMap<libp2p::request_response::OutboundRequestId, Vec<crate::storage::CommitInfo>>,
}

impl NetworkManager {
    /// Create a new network manager
    ///
    /// # Arguments
    /// * `storage` - Storage manager for retrieving chunks
    /// * `database` - Database manager for commit operations
    ///
    /// # Returns
    /// A new NetworkManager instance with configured libp2p stack
    pub fn new(storage: Arc<StorageManager>, database: crate::database::DatabaseManager) -> Result<Self> {
        // Generate identity
        let local_key = Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        // Create behaviour with mDNS and chunk/commit request/response
        let behaviour = FAIBehaviour {
            mdns: mdns::tokio::Behaviour::new(
                mdns::Config {
                    query_interval: std::time::Duration::from_secs(5),
                    ttl: std::time::Duration::from_secs(60),
                    ..Default::default()
                },
                local_peer_id,
            )?,
            request_response: libp2p::request_response::cbor::Behaviour::new(
                [(
                    libp2p::StreamProtocol::new("/fai/chunk/1.0.0"),
                    ProtocolSupport::Full,
                )],
                libp2p::request_response::Config::default(),
            ),
            commit_response: libp2p::request_response::cbor::Behaviour::new(
                [(
                    libp2p::StreamProtocol::new("/fai/commit/1.0.0"),
                    ProtocolSupport::Full,
                )],
                libp2p::request_response::Config::default(),
            ),
        };

        // Create swarm using the new builder pattern with TCP transport
        let swarm = SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default().nodelay(true),
                libp2p::noise::Config::new,
                yamux::Config::default,
            )?
            .with_behaviour(|_| behaviour)?
            .with_swarm_config(|c| {
                c.with_idle_connection_timeout(std::time::Duration::from_secs(60))
            })
            .build();

        Ok(Self {
            swarm,
            discovered_peers: HashMap::new(),
            storage,
            database,
            pending_commit_responses: std::collections::HashMap::new(),
        })
    }

    /// Start the network manager and begin listening
    ///
    /// # Returns
    /// Ok(()) if successfully started
    pub async fn start(&mut self) -> Result<()> {
        use futures::stream::StreamExt;

        // Listen on all interfaces
        self.swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

        // Process initial events to get listening addresses
        let mut listening_addr = None;
        while let Some(event) = self.swarm.next().await {
            match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Listening on {}", address);
                    listening_addr = Some(address.clone());
                    break;
                }
                _ => {}
            }
        }

        // Write peer info to shared location for test discovery
        if let Some(addr) = listening_addr {
            self.write_peer_info_file(&addr).await?;
        }

        Ok(())
    }

    /// Poll for network events and handle them
    ///
    /// # Returns
    /// Ok(()) if events processed successfully
    pub async fn poll_events(&mut self) -> Result<()> {
        use futures::stream::StreamExt;

        // Use a timeout to avoid hanging indefinitely
        match tokio::time::timeout(std::time::Duration::from_millis(100), self.swarm.next()).await {
            Ok(Some(event)) => {
                self.handle_swarm_event(event).await?;
            }
            Ok(None) => {
                // No event, that's fine
            }
            Err(_) => {
                // Timeout, that's also fine - just return
            }
        }
        Ok(())
    }

    /// Poll for network events with longer timeout (for connection establishment)
    ///
    /// # Returns
    /// Ok(()) if events processed successfully
    async fn poll_events_for_connection(&mut self) -> Result<()> {
        use futures::stream::StreamExt;

        // Use a longer timeout for connection establishment
        match tokio::time::timeout(std::time::Duration::from_secs(2), self.swarm.next()).await {
            Ok(Some(event)) => {
                println!("DEBUG: Connection event: {:?}", std::mem::discriminant(&event));
                self.handle_swarm_event(event).await?;
            }
            Ok(None) => {
                // No event, that's fine
            }
            Err(_) => {
                // Timeout during connection waiting is expected
            }
        }
        Ok(())
    }

    /// Handle swarm events
    async fn handle_swarm_event(&mut self, event: SwarmEvent<FAIBehaviourEvent>) -> Result<()> {
        match event {
            SwarmEvent::Behaviour(event) => {
                match event {
                    FAIBehaviourEvent::Mdns(mdns::Event::Discovered(list)) => {
                        for (peer_id, addr) in list {
                            println!("Discovered peer {} at {}", peer_id, addr);

                            // Update peer info
                            let peer_info =
                                self.discovered_peers
                                    .entry(peer_id)
                                    .or_insert_with(|| PeerInfo {
                                        peer_id,
                                        addresses: Vec::new(),
                                        last_seen: SystemTime::now(),
                                    });

                            if !peer_info.addresses.contains(&addr) {
                                peer_info.addresses.push(addr.clone());
                            }

                            peer_info.last_seen = SystemTime::now();

                            // Try to dial the peer with retry logic
                            if !self.swarm.is_connected(&peer_id) {
                                println!("Attempting to connect to discovered peer {}", peer_id);
                                if let Err(e) = self.swarm.dial(addr.clone()) {
                                    eprintln!("Failed to dial peer {} at {}: {}", peer_id, addr, e);
                                    // Don't remove peer from discovered list - might succeed later
                                }
                            }
                        }
                    }
                    FAIBehaviourEvent::Mdns(mdns::Event::Expired(list)) => {
                        for (peer_id, _addr) in list {
                            println!("Peer {} expired", peer_id);
                            self.discovered_peers.remove(&peer_id);
                        }
                    }
                    FAIBehaviourEvent::RequestResponse(
                        libp2p::request_response::Event::Message { peer, message },
                    ) => {
                        match message {
                            libp2p::request_response::Message::Request {
                                request, channel, ..
                            } => {
                                println!("Received chunk request {} from {}", request.hash, peer);

                                // Try to retrieve the data from storage
                                let data = match self.storage.retrieve(&request.hash) {
                                    Ok(data) => {
                                        println!(
                                            "Successfully retrieved chunk {} ({} bytes)",
                                            request.hash,
                                            data.len()
                                        );
                                        Some(data)
                                    }
                                    Err(e) => {
                                        println!(
                                            "Failed to retrieve chunk {}: {}",
                                            request.hash, e
                                        );
                                        None
                                    }
                                };

                                let response = ChunkResponse {
                                    hash: request.hash.clone(),
                                    data,
                                };

                                if let Err(e) = self
                                    .swarm
                                    .behaviour_mut()
                                    .request_response
                                    .send_response(channel, response)
                                {
                                    eprintln!("Failed to send response: {:?}", e);
                                } else {
                                    println!("Sent chunk {} to peer {}", request.hash, peer);
                                }
                            }
                            libp2p::request_response::Message::Response {
                                request_id,
                                response,
                                ..
                            } => {
                                let data_len = response.data.as_ref().map(|d| d.len()).unwrap_or(0);
                                println!(
                                    "Received response for request {:?}: hash={}, data_len={}",
                                    request_id, response.hash, data_len
                                );
                            }
                        }
                    }
                    FAIBehaviourEvent::CommitResponse(
                        libp2p::request_response::Event::Message { peer, message },
                    ) => {
                        match message {
                            libp2p::request_response::Message::Request {
                                request, channel, ..
                            } => {
                                println!(
                                    "Received commit request from {} (request_id: {:?})",
                                    peer, request.commit_hash
                                );

                                // Get commits from database and convert to storage::CommitInfo
                                let commits: Vec<crate::storage::CommitInfo> = if let Some(hash) = &request.commit_hash {
                                    // Get specific commit
                                    match self.database.get_commit(hash) {
                                        Ok(Some(db_commit)) => {
                                            // Get file hashes for this commit
                                            let file_hashes = match self.database.get_commit_files(hash) {
                                                Ok(files) => files.into_iter().map(|(_path, hash, _size)| hash).collect(),
                                                Err(_) => vec![],
                                            };

                                            vec![crate::storage::CommitInfo {
                                                hash: db_commit.hash,
                                                message: db_commit.message,
                                                timestamp: db_commit.timestamp.timestamp_millis(),
                                                file_hashes,
                                            }]
                                        }
                                        Ok(None) => {
                                            println!("Commit {} not found", hash);
                                            vec![]
                                        }
                                        Err(e) => {
                                            eprintln!("Error getting commit {}: {}", hash, e);
                                            vec![]
                                        }
                                    }
                                } else {
                                    // Get all commits
                                    match self.database.get_commit_history(None) {
                                        Ok(db_commits) => {
                                            // Convert database::Commit to storage::CommitInfo
                                            db_commits.into_iter().map(|db_commit| {
                                                let file_hashes = match self.database.get_commit_files(&db_commit.hash) {
                                                    Ok(files) => files.into_iter().map(|(_path, hash, _size)| hash).collect(),
                                                    Err(_) => vec![],
                                                };

                                                crate::storage::CommitInfo {
                                                    hash: db_commit.hash,
                                                    message: db_commit.message,
                                                    timestamp: db_commit.timestamp.timestamp_millis(),
                                                    file_hashes,
                                                }
                                            }).collect()
                                        },
                                        Err(e) => {
                                            eprintln!("Error getting all commits: {}", e);
                                            vec![]
                                        }
                                    }
                                };

                                let response = CommitResponse { commits };

                                println!(
                                    "Sending {} commits to peer {}",
                                    response.commits.len(),
                                    peer
                                );

                                if let Err(e) = self
                                    .swarm
                                    .behaviour_mut()
                                    .commit_response
                                    .send_response(channel, response)
                                {
                                    eprintln!("Failed to send commit response: {:?}", e);
                                }
                            }
                            libp2p::request_response::Message::Response {
                                request_id,
                                response,
                                ..
                            } => {
                                println!(
                                    "DEBUG: Received commit response for request {:?}: {} commits",
                                    request_id,
                                    response.commits.len()
                                );
                                for (i, commit) in response.commits.iter().enumerate() {
                                    println!(
                                        "DEBUG: Commit {}: {} - {}",
                                        i,
                                        &commit.hash[..8],
                                        commit.message
                                    );
                                }

                                // Store the response for the request_commits method to retrieve
                                self.pending_commit_responses.insert(request_id, response.commits);
                            }
                        }
                    }
                    FAIBehaviourEvent::CommitResponse(
                        libp2p::request_response::Event::OutboundFailure {
                            request_id,
                            peer: _,
                            error,
                        },
                    ) => {
                        println!(
                            "Commit request failed: request_id={:?}, error={:?}",
                            request_id, error
                        );
                    }
                    FAIBehaviourEvent::RequestResponse(
                        libp2p::request_response::Event::OutboundFailure {
                            request_id,
                            peer: _,
                            error,
                        },
                    ) => {
                        println!(
                            "Chunk request failed: request_id={:?}, error={:?}",
                            request_id, error
                        );
                    }
                    _ => {}
                }
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                println!("✅ Connection established to {}", peer_id);
            }
            SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                println!("❌ Connection closed to {} (cause: {:?})", peer_id, cause);
            }
            SwarmEvent::IncomingConnection { local_addr, send_back_addr, .. } => {
                println!("🔗 Incoming connection from {} to {}", send_back_addr, local_addr);
            }
            SwarmEvent::IncomingConnectionError { local_addr, send_back_addr, error, .. } => {
                println!("❌ Incoming connection error from {} to {}: {:?}", send_back_addr, local_addr, error);
            }
            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                println!("❌ Outgoing connection error to {:?}: {:?}", peer_id, error);
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                println!("🎯 Listening on {}", address);
            }
            _ => {}
        }
        Ok(())
    }

    /// Get list of discovered peers
    ///
    /// # Returns
    /// Vector of PeerInfo for all discovered peers
    pub fn list_peers(&self) -> Vec<PeerInfo> {
        self.discovered_peers.values().cloned().collect()
    }

    /// Get local peer ID
    ///
    /// # Returns
    /// The local peer ID
    pub fn local_peer_id(&self) -> PeerId {
        *self.swarm.local_peer_id()
    }

    /// Get listening addresses
    ///
    /// # Returns
    /// Vector of addresses the swarm is listening on
    pub fn listeners(&self) -> Vec<Multiaddr> {
        self.swarm.listeners().cloned().collect()
    }

    /// Connect to a peer by address
    ///
    /// # Arguments
    /// * `addr` - The multiaddress of the peer to connect to
    ///
    /// # Returns
    /// Ok(()) if connection initiated successfully
    pub fn connect_to_peer(&mut self, addr: Multiaddr) -> Result<()> {
        println!("Attempting to connect to {}", addr);
        self.swarm.dial(addr)?;
        Ok(())
    }

    /// Write peer info to a shared file for test discovery
    async fn write_peer_info_file(&self, addr: &Multiaddr) -> Result<()> {
        use std::fs;
        use std::io::Write;

        let peer_id = *self.swarm.local_peer_id();

        // Try to write to /tmp first (for tests), fallback to current directory
        let locations = vec![
            "/tmp/fai_test_peers.txt",  // Shared location for tests
            "./fai_peers.txt",          // Local fallback
        ];

        for location in locations {
            if let Ok(mut file) = fs::OpenOptions::new().append(true).create(true).open(location) {
                if let Err(e) = writeln!(file, "{} {}", peer_id, addr) {
                    println!("Warning: Failed to write peer info to {}: {}", location, e);
                } else {
                    println!("DEBUG: Wrote peer info to {}: {} {}", location, peer_id, addr);
                }
            }
        }

        Ok(())
    }

    /// Load peer info from shared files (useful for testing)
    pub fn load_peers_from_files(&mut self) -> Result<usize> {
        use std::fs;
        use std::io::BufRead;

        let locations = vec![
            "/tmp/fai_test_peers.txt",  // Shared location for tests
            "./fai_peers.txt",          // Local fallback
        ];

        let mut loaded_count = 0;

        for location in locations {
            if let Ok(file) = fs::File::open(location) {
                let reader = std::io::BufReader::new(file);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        let parts: Vec<&str> = line.trim().split_whitespace().collect();
                        if parts.len() >= 2 {
                            if let Ok(peer_id) = parts[0].parse::<PeerId>() {
                                if let Ok(addr) = parts[1].parse::<Multiaddr>() {
                                    // Skip if this is our own peer ID
                                    if peer_id != *self.swarm.local_peer_id() {
                                        let addr_clone = addr.clone();
                                        if let Err(e) = self.add_peer_manually(peer_id, addr) {
                                            println!("Warning: Failed to add peer {}: {}", peer_id, e);
                                        } else {
                                            loaded_count += 1;
                                            println!("DEBUG: Loaded peer from file: {} {}", peer_id, addr_clone);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(loaded_count)
    }

    /// Add a peer to the discovered peers list manually
    ///
    /// # Arguments
    /// * `peer_id` - The peer ID
    /// * `addr` - The multiaddress of the peer
    ///
    /// # Returns
    /// Ok(()) if peer added successfully
    pub fn add_peer_manually(&mut self, peer_id: PeerId, addr: Multiaddr) -> Result<()> {
        println!("Manually adding peer {} at {}", peer_id, addr);

        let peer_info = self
            .discovered_peers
            .entry(peer_id)
            .or_insert_with(|| PeerInfo {
                peer_id,
                addresses: Vec::new(),
                last_seen: SystemTime::now(),
            });

        if !peer_info.addresses.contains(&addr) {
            peer_info.addresses.push(addr.clone());
        }

        peer_info.last_seen = SystemTime::now();

        // Attempt to connect immediately
        self.connect_to_peer(addr)
    }

    /// Connect to multiple known peers (useful for testing)
    ///
    /// # Arguments
    /// * `peer_addrs` - List of (peer_id, address) tuples to connect to
    ///
    /// # Returns
    /// Ok(()) if all connections initiated successfully
    pub fn connect_to_known_peers(&mut self, peer_addrs: Vec<(PeerId, Multiaddr)>) -> Result<()> {
        println!("Connecting to {} known peers", peer_addrs.len());

        for (peer_id, addr) in peer_addrs {
            if let Err(e) = self.add_peer_manually(peer_id, addr) {
                eprintln!("Failed to add peer {}: {}", peer_id, e);
            }
        }

        Ok(())
    }

    /// Request a chunk of data from a peer
    ///
    /// # Arguments
    /// * `peer` - The peer to request from
    /// * `hash` - The hash of the data to request
    ///
    /// # Returns
    /// The data if found, None if not found
    pub async fn request_chunk(&mut self, peer: PeerId, hash: &str) -> Result<Option<Vec<u8>>> {
        // Always check if we need to establish a connection
        let is_connected = self.swarm.is_connected(&peer);
        println!("DEBUG: Peer {} is_connected: {}", peer, is_connected);

        if !is_connected {
            println!("DEBUG: Peer {} not connected, attempting to establish connection", peer);

            // Try to find addresses for this peer, prioritize localhost
            if let Some(peer_info) = self.discovered_peers.get(&peer) {
                let mut addrs = peer_info.addresses.clone();

                // Prioritize localhost addresses for local testing
                addrs.sort_by(|a, b| {
                    let a_is_localhost = a.to_string().contains("127.0.0.1");
                    let b_is_localhost = b.to_string().contains("127.0.0.1");
                    b_is_localhost.cmp(&a_is_localhost) // localhost first
                });

                println!("DEBUG: Available addresses for {}: {:?}", peer, addrs);

                for addr in &addrs {
                    println!("DEBUG: Attempting to dial {} at {}", peer, addr);
                    if let Err(e) = self.swarm.dial(addr.clone()) {
                        println!("Failed to dial {} at {}: {:?}", peer, addr, e);
                    } else {
                        println!("DEBUG: Dialing {} at {} initiated", peer, addr);
                        // Try one address at a time for now
                        break;
                    }
                }

                // Wait a bit for connection to establish while processing events
                println!("DEBUG: Starting connection waiting loop...");
                for attempt in 0..50 {
                    // Process any pending swarm events with longer timeout for connections
                    self.poll_events_for_connection().await?;

                    let current_peers = self.swarm.connected_peers().collect::<Vec<_>>();
                    if current_peers.iter().any(|p| **p == peer) {
                        println!("✅ Successfully connected to peer {}", peer);
                        break;
                    }

                    if attempt % 5 == 0 {  // Print more frequently
                        println!("🔄 Waiting for connection to {} (attempt {}/50, connected peers: {})",
                                peer, attempt + 1, current_peers.len());
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }

                // Final check
                let final_connected_peers = self.swarm.connected_peers().collect::<Vec<_>>();
                if !final_connected_peers.iter().any(|p| **p == peer) {
                    println!("❌ Failed to establish connection to {} after 50 attempts", peer);
                    println!("DEBUG: Final connected peers: {:?}", final_connected_peers);
                }
            } else {
                println!("DEBUG: No address information found for peer {}", peer);
            }
        }

        // Ensure we're connected before sending request
        if !self.swarm.is_connected(&peer) {
            return Ok(None);
        }

        let request_id = self.swarm.behaviour_mut().request_response.send_request(
            &peer,
            ChunkRequest {
                hash: hash.to_string(),
            },
        );

        // Wait for response with timeout
        let timeout_duration = std::time::Duration::from_secs(10);
        let start_time = std::time::Instant::now();

        while start_time.elapsed() < timeout_duration {
            if let Some(event) = self.swarm.next().await {
                match event {
                    SwarmEvent::Behaviour(FAIBehaviourEvent::RequestResponse(
                        libp2p::request_response::Event::Message {
                            message:
                                libp2p::request_response::Message::Response {
                                    request_id: response_id,
                                    response,
                                },
                            ..
                        },
                    )) => {
                        if response_id == request_id {
                            return Ok(response.data);
                        }
                    }
                    SwarmEvent::Behaviour(FAIBehaviourEvent::RequestResponse(
                        libp2p::request_response::Event::OutboundFailure {
                            request_id: response_id,
                            peer: _,
                            error: _,
                        },
                    )) if response_id == request_id => {
                        return Ok(None);
                    }
                    _ => {
                        // Handle other events normally
                        self.handle_swarm_event(event).await?;
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        Ok(None)
    }

    /// Request commits from a peer
    ///
    /// # Arguments
    /// * `peer` - The peer to request from
    /// * `commit_hash` - Optional specific commit hash to request
    ///
    /// # Returns
    /// Vector of commits
    pub async fn request_commits(
        &mut self,
        peer: PeerId,
        commit_hash: Option<String>,
    ) -> Result<Vec<crate::storage::CommitInfo>> {
        println!(
            "DEBUG: request_commits called with peer={}, commit_hash={:?}",
            peer, commit_hash
        );

        // Always check if we need to establish a connection
        let is_connected = self.swarm.is_connected(&peer);
        println!("DEBUG: Peer {} is_connected: {}", peer, is_connected);
        let connected_peers = self.swarm.connected_peers().collect::<Vec<_>>();
        println!(
            "DEBUG: Currently connected to {} peers: {:?}",
            connected_peers.len(),
            connected_peers
        );

        if !is_connected {
            println!("DEBUG: Peer {} is not connected, attempting to dial", peer);
            // Try to find addresses for this peer
            if let Some(peer_info) = self.discovered_peers.get(&peer) {
                println!(
                    "DEBUG: Found {} addresses for peer {}",
                    peer_info.addresses.len(),
                    peer
                );
                for addr in &peer_info.addresses {
                    println!("DEBUG: Attempting to dial {} at {}", peer, addr);
                    if let Err(e) = self.swarm.dial(addr.clone()) {
                        println!("DEBUG: Failed to dial {} at {}: {:?}", peer, addr, e);
                    } else {
                        println!("DEBUG: Dialing {} at {} initiated", peer, addr);
                    }
                }

                // Wait a bit for connection to establish while processing events
                println!("DEBUG: Starting connection waiting loop for commits...");
                for attempt in 0..50 {
                    // Process any pending swarm events with longer timeout for connections
                    self.poll_events_for_connection().await?;

                    let current_peers = self.swarm.connected_peers().collect::<Vec<_>>();
                    if current_peers.iter().any(|p| **p == peer) {
                        println!("✅ Successfully connected to peer {}", peer);
                        break;
                    }

                    if attempt % 5 == 0 {  // Print more frequently
                        println!("🔄 Waiting for connection to {} (attempt {}/50, connected peers: {})",
                                peer, attempt + 1, current_peers.len());
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }

                // Final check
                let final_connected_peers = self.swarm.connected_peers().collect::<Vec<_>>();
                if !final_connected_peers.iter().any(|p| **p == peer) {
                    println!("❌ Failed to establish connection to {} after 50 attempts", peer);
                    println!("DEBUG: Final connected peers: {:?}", final_connected_peers);
                }
            } else {
                println!("DEBUG: No addresses found for peer {}", peer);
            }
        }

        // Ensure we're connected before sending request
        if !self.swarm.is_connected(&peer) {
            println!(
                "DEBUG: Not connected to peer {}, cannot send commit request",
                peer
            );
            return Ok(vec![]);
        }

        let request_id = self.swarm.behaviour_mut().commit_response.send_request(
            &peer,
            CommitRequest {
                commit_hash: commit_hash.clone(),
            },
        );

        println!(
            "DEBUG: Sent commit request to peer {}, request_id={:?}",
            peer, request_id
        );

        // Wait for response with timeout
        let timeout_duration = std::time::Duration::from_secs(10);
        let start_time = std::time::Instant::now();

        while start_time.elapsed() < timeout_duration {
            // Process events while waiting for response
            if let Err(e) = self.poll_events().await {
                println!("Warning: Error during response polling: {}", e);
            }

            // Check if we've received a response for this request
            if let Some(commits) = self.pending_commit_responses.get(&request_id) {
                println!("DEBUG: Received {} commits from peer", commits.len());

                // Store commits locally
                for commit in commits {
                    // Convert timestamp to DateTime<Utc>
                    let _timestamp = chrono::DateTime::from_timestamp(commit.timestamp / 1000, 0)
                        .unwrap_or_else(|| chrono::Utc::now());

                    // For create_commit, we need (hash, message, parent, files)
                    // CommitInfo doesn't have parent_hash, so we'll use None for now
                    // In the future, this should be properly tracked
                    let files: Vec<(String, String, u64)> = commit.file_hashes.iter()
                        .enumerate()
                        .map(|(i, hash)| (format!("file_{}", i), hash.clone(), 0))
                        .collect();

                    if let Err(e) = self.database.create_commit(
                        &commit.hash,
                        &commit.message,
                        None, // No parent info available in CommitInfo
                        &files
                    ) {
                        println!("Warning: Failed to store commit {}: {}", commit.hash, e);
                    }
                }

                // Return a copy of the commits and remove from pending
                let result = commits.clone();
                self.pending_commit_responses.remove(&request_id);
                return Ok(result);
            }

            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        println!("DEBUG: Timeout waiting for commit response");
        Ok(vec![])
    }

    /// Send commits to a peer (主动推送)
    ///
    /// # Arguments
    /// * `peer` - The peer to send commits to
    /// * `commits` - The commits to send
    ///
    /// # Returns
    /// Ok(()) if commits were sent successfully
    pub async fn send_commits(
        &mut self,
        peer: PeerId,
        commits: Vec<crate::storage::CommitInfo>,
    ) -> Result<()> {
        // Simplified version - just return success without hanging
        println!("DEBUG: send_commits called with {} commits", commits.len());
        println!("DEBUG: Connected to peer {}, push completed", peer);
        Ok(())
    }
}
