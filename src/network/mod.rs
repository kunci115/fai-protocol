//! Network layer for FAI Protocol
//! 
//! Handles peer-to-peer networking for decentralized model sharing.

use anyhow::Result;
use libp2p::{
    Swarm, SwarmBuilder,
    identity::Keypair,
    mdns,
    yamux,
    Multiaddr,
    PeerId,
    swarm::{NetworkBehaviour, SwarmEvent},
    request_response::ProtocolSupport,
};
use serde::{Serialize, Deserialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{SystemTime, Duration},
};
use crate::storage::StorageManager;

/// Information about a discovered peer
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// Peer identifier
    pub peer_id: PeerId,
    /// Network addresses of the peer
    pub addresses: Vec<Multiaddr>,
    /// When this peer was last seen
    pub last_seen: SystemTime,
}

/// Request for a chunk of data from a peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkRequest {
    /// The file hash to request
    pub hash: String,
}

/// Response containing the requested chunk data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkResponse {
    /// The file hash this response is for
    pub hash: String,
    /// The data if found, None if not found
    pub data: Option<Vec<u8>>,
}

/// Request for commits from a peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitRequest {
    /// Optional commit hash (None = get all commits)
    pub commit_hash: Option<String>,
}

/// Response containing commit information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitResponse {
    /// List of commits
    pub commits: Vec<crate::storage::CommitInfo>,
}

/// Simple network behavior for peer discovery and chunk requests
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "FAIEvent")]
pub struct FAIBehaviour {
    /// mDNS for local peer discovery
    mdns: mdns::tokio::Behaviour,
    /// Request-response protocol for chunk requests
    request_response: libp2p::request_response::cbor::Behaviour<ChunkRequest, ChunkResponse>,
    /// Request-response protocol for commit requests
    commit_response: libp2p::request_response::cbor::Behaviour<CommitRequest, CommitResponse>,
}

/// Network events
#[derive(Debug)]
pub enum FAIEvent {
    Mdns(mdns::Event),
    RequestResponse(libp2p::request_response::Event<ChunkRequest, ChunkResponse>),
    CommitResponse(libp2p::request_response::Event<CommitRequest, CommitResponse>),
}

impl From<mdns::Event> for FAIEvent {
    fn from(event: mdns::Event) -> Self {
        FAIEvent::Mdns(event)
    }
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
}

impl NetworkManager {
    /// Create a new network manager
    /// 
    /// # Arguments
    /// * `storage` - Storage manager for retrieving chunks
    /// 
    /// # Returns
    /// A new NetworkManager instance with configured libp2p stack
    pub fn new(storage: Arc<StorageManager>) -> Result<Self> {
        // Generate identity
        let local_key = Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        // Create behavior with mDNS and chunk/commit request/response
        let behaviour = FAIBehaviour {
            mdns: mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)?,
            request_response: libp2p::request_response::cbor::Behaviour::new(
                [(libp2p::StreamProtocol::new("/fai/chunk/1.0.0"), ProtocolSupport::Full)],
                libp2p::request_response::Config::default(),
            ),
            commit_response: libp2p::request_response::cbor::Behaviour::new(
                [(libp2p::StreamProtocol::new("/fai/commit/1.0.0"), ProtocolSupport::Full)],
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
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        Ok(Self {
            swarm,
            discovered_peers: HashMap::new(),
            storage,
        })
    }

    /// Start the network manager and begin listening
    /// 
    /// # Returns
    /// Ok(()) if successfully started
    pub fn start(&mut self) -> Result<()> {
        // Listen on a random TCP port
        let listening_addr: Multiaddr = "/ip4/0.0.0.0/tcp/0".parse()?;
        self.swarm.listen_on(listening_addr)?;

        println!("FAI Protocol network started");
        println!("Local peer ID: {}", self.swarm.local_peer_id());
        
        Ok(())
    }

    /// Process network events
    /// 
    /// Should be called in a loop to handle incoming events
    pub async fn poll_events(&mut self) -> Result<()> {
        use futures::stream::StreamExt;
        
        if let Some(event) = self.swarm.next().await {
            println!("DEBUG: Network event: {:?}", event);
            match event {
                SwarmEvent::Behaviour(FAIEvent::Mdns(mdns::Event::Discovered(list))) => {
                    for (peer_id, addr) in list {
                        println!("Discovered peer {} at {}", peer_id, addr);
                        
                        // Update peer info
                        let peer_info = self.discovered_peers.entry(peer_id).or_insert_with(|| PeerInfo {
                            peer_id,
                            addresses: Vec::new(),
                            last_seen: SystemTime::now(),
                        });
                        
                        if !peer_info.addresses.contains(&addr) {
                            peer_info.addresses.push(addr.clone());
                        }
                        
                        peer_info.last_seen = SystemTime::now();
                        
                        // Try to dial the peer
                        if let Err(e) = self.swarm.dial(addr) {
                            eprintln!("Failed to dial peer {}: {}", peer_id, e);
                        }
                    }
                }
                SwarmEvent::Behaviour(FAIEvent::Mdns(mdns::Event::Expired(list))) => {
                    for (peer_id, _addr) in list {
                        println!("Peer {} expired", peer_id);
                        self.discovered_peers.remove(&peer_id);
                    }
                }
                SwarmEvent::Behaviour(FAIEvent::RequestResponse(
                    libp2p::request_response::Event::Message { 
                        peer, 
                        message 
                    }
                )) => {
                    match message {
                        libp2p::request_response::Message::Request { 
                            request, 
                            channel, 
                            request_id,
                            ..
                        } => {
                            println!("DEBUG: Received chunk request {} from {} (request_id: {:?})", request.hash, peer, request_id);
                            
                            // Try to retrieve the data from storage
                            println!("DEBUG: Server attempting to retrieve chunk: {}", request.hash);
                            let data = match self.storage.retrieve(&request.hash) {
                                Ok(data) => {
                                    println!("DEBUG: Server successfully retrieved chunk {} ({} bytes)", request.hash, data.len());
                                    println!("DEBUG: Server sending chunk response with data");
                                    Some(data)
                                }
                                Err(e) => {
                                    println!("DEBUG: Server failed to retrieve chunk {}: {}", request.hash, e);
                                    println!("DEBUG: Server sending chunk response with no data");
                                    None
                                }
                            };
                            
                            let response = ChunkResponse {
                                hash: request.hash.clone(),
                                data,
                            };
                            
                            // Copy values before moving response
                            let hash_copy = response.hash.clone();
                            let data_len = response.data.as_ref().map(|d| d.len()).unwrap_or(0);
                            
                            if let Err(e) = self.swarm.behaviour_mut().request_response.send_response(
                                channel,
                                response
                            ) {
                                eprintln!("Failed to send response: {:?}", e);
                            } else {
                                if data_len > 0 {
                                    println!("Sent chunk {} ({} bytes) to peer {}", hash_copy, data_len, peer);
                                } else {
                                    println!("Sent chunk {} (not found) to peer {}", hash_copy, peer);
                                }
                            }
                        }
                        libp2p::request_response::Message::Response { 
                            request_id, 
                            response,
                            ..
                        } => {
                            let data_len = response.data.as_ref().map(|d| d.len()).unwrap_or(0);
                            println!("DEBUG: Received response for request {:?}: hash={}, data_len={}, data_present={}", 
                                request_id, response.hash, data_len, response.data.is_some());
                            if let Some(ref data) = response.data {
                                println!("DEBUG: Response data preview: {:?}", &data[..data.len().min(20)]);
                            }
                        }
                    }
                }
                SwarmEvent::Behaviour(FAIEvent::CommitResponse(
                    libp2p::request_response::Event::Message { 
                        peer, 
                        message 
                    }
                )) => {
                    match message {
                        libp2p::request_response::Message::Request { 
                            request, 
                            channel, 
                            request_id,
                            ..
                        } => {
                            println!("DEBUG: Received commit request from {} (request_id: {:?})", peer, request_id);
                            
                            // Get commits from storage
                            let commits = if let Some(hash) = &request.commit_hash {
                                // Get specific commit
                                match self.storage.get_commit(hash) {
                                    Ok(Some(commit)) => vec![commit],
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
                                match self.storage.get_all_commits() {
                                    Ok(commits) => commits,
                                    Err(e) => {
                                        eprintln!("Error getting all commits: {}", e);
                                        vec![]
                                    }
                                }
                            };
                            
                            let response = CommitResponse { commits };
                            
                            println!("Sending {} commits to peer {}", response.commits.len(), peer);
                            
                            if let Err(e) = self.swarm.behaviour_mut().commit_response.send_response(
                                channel,
                                response
                            ) {
                                eprintln!("Failed to send commit response: {:?}", e);
                            }
                        }
                        libp2p::request_response::Message::Response { 
                            request_id, 
                            response,
                            ..
                        } => {
                            println!("DEBUG: Received commit response for request {:?}: {} commits", 
                                request_id, response.commits.len());
                        }
                    }
                }
                SwarmEvent::Behaviour(FAIEvent::RequestResponse(
                    libp2p::request_response::Event::OutboundFailure { 
                        request_id, 
                        peer: failure_peer, 
                        error 
                    }
                )) => {
                    println!("DEBUG: Outbound request failed: request_id={:?}, peer={:?}, error={:?}", request_id, failure_peer, error);
                }
                SwarmEvent::Behaviour(FAIEvent::RequestResponse(
                    libp2p::request_response::Event::InboundFailure { 
                        request_id, 
                        peer: failure_peer, 
                        error 
                    }
                )) => {
                    println!("DEBUG: Inbound request failed: request_id={:?}, peer={:?}, error={:?}", request_id, failure_peer, error);
                }
                SwarmEvent::Behaviour(FAIEvent::CommitResponse(
                    libp2p::request_response::Event::OutboundFailure { 
                        request_id, 
                        peer: failure_peer, 
                        error 
                    }
                )) => {
                    println!("DEBUG: Commit request failed: request_id={:?}, peer={:?}, error={:?}", request_id, failure_peer, error);
                }
                SwarmEvent::Behaviour(FAIEvent::CommitResponse(
                    libp2p::request_response::Event::InboundFailure { 
                        request_id, 
                        peer: failure_peer, 
                        error 
                    }
                )) => {
                    println!("DEBUG: Commit inbound request failed: request_id={:?}, peer={:?}, error={:?}", request_id, failure_peer, error);
                }
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Listening on {}", address);
                }
                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    println!("DEBUG: Connection established to {}", peer_id);
                }
                SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                    println!("DEBUG: Connection closed to {} (reason: {:?})", peer_id, cause);
                }
                _ => {}
            }
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

    /// Request a chunk of data from a peer
    /// 
    /// # Arguments
    /// * `peer` - The peer to request from
    /// * `hash` - The hash of the data to request
    /// 
    /// # Returns
    /// The data if found, None if not found
    pub async fn request_chunk(&mut self, peer: PeerId, hash: &str) -> Result<Option<Vec<u8>>> {
        println!("DEBUG: request_chunk called with peer={}, hash={}", peer, hash);
        
        // Check if we have an active connection to this peer
        let connected_peers = self.swarm.connected_peers().collect::<Vec<_>>();
        println!("DEBUG: Currently connected to {} peers: {:?}", connected_peers.len(), connected_peers);
        
        if !connected_peers.iter().any(|p| **p == peer) {
            println!("DEBUG: Peer {} is not connected, attempting to dial", peer);
            // Try to find addresses for this peer
            if let Some(peer_info) = self.discovered_peers.get(&peer) {
                println!("DEBUG: Found {} addresses for peer {}", peer_info.addresses.len(), peer);
                for addr in &peer_info.addresses {
                    println!("DEBUG: Attempting to dial {} at {}", peer, addr);
                    if let Err(e) = self.swarm.dial(addr.clone()) {
                        println!("DEBUG: Failed to dial {} at {}: {:?}", peer, addr, e);
                    } else {
                        println!("DEBUG: Dialing {} at {} initiated", peer, addr);
                    }
                }
            } else {
                println!("DEBUG: No addresses found for peer {}", peer);
            }
            
            // Wait a bit for connection to establish
            for _ in 0..50 {
                let current_peers = self.swarm.connected_peers().collect::<Vec<_>>();
                if current_peers.iter().any(|p| **p == peer) {
                    println!("DEBUG: Successfully connected to peer {}", peer);
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
        
        // Ensure we're connected before sending request
        if !self.swarm.is_connected(&peer) {
            println!("DEBUG: Not connected to peer {}, attempting to reconnect", peer);
            
            // Try to re-establish connection
            if let Some(peer_info) = self.discovered_peers.get(&peer) {
                println!("DEBUG: Attempting to reconnect to peer {} at {} addresses", peer, peer_info.addresses.len());
                for addr in &peer_info.addresses {
                    if let Err(e) = self.swarm.dial(addr.clone()) {
                        println!("DEBUG: Failed to dial {} at {}: {:?}", peer, addr, e);
                    } else {
                        println!("DEBUG: Re-dialing {} at {} initiated", peer, addr);
                    }
                }
                
                // Wait for connection to establish
                for i in 0..100 { // 10 seconds
                    let current_peers = self.swarm.connected_peers().collect::<Vec<_>>();
                    if current_peers.iter().any(|p| **p == peer) {
                        println!("DEBUG: Successfully reconnected to peer {}", peer);
                        break;
                    }
                    
                    if i % 10 == 0 {
                        println!("DEBUG: Waiting for reconnection... {}/10 seconds", i / 10);
                    }
                    
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
            }
            
            // Check if we're connected now
            if !self.swarm.is_connected(&peer) {
                println!("DEBUG: Still not connected to peer {}, cannot send request", peer);
                return Ok(None);
            }
        }
        
        let request_id = self.swarm.behaviour_mut().request_response.send_request(
            &peer,
            ChunkRequest { hash: hash.to_string() },
        );
        
        println!("DEBUG: Sent chunk request {} to peer {}, request_id={:?}", hash, peer, request_id);
        println!("DEBUG: Starting to wait for response...");
        
        // Wait for response
        use futures::StreamExt;
        while let Some(event) = self.swarm.next().await {
            match event {
                SwarmEvent::Behaviour(FAIEvent::RequestResponse(
                    libp2p::request_response::Event::Message { 
                        peer: _response_peer, 
                        message 
                    }
                )) => {
                    match message {
                        libp2p::request_response::Message::Response { 
                            request_id: response_id, 
                            response 
                        } if response_id == request_id => {
                            let data_len = response.data.as_ref().map(|d| d.len()).unwrap_or(0);
                            println!("DEBUG: Received matching response for request {:?}: hash={}, data_len={}, has_data={}", 
                                response_id, response.hash, data_len, response.data.is_some());
                            if let Some(ref data) = response.data {
                                println!("DEBUG: Successfully received {} bytes: {:?}", data.len(), &data[..data.len().min(20)]);
                            } else {
                                println!("DEBUG: Response data is None - server couldn't find the chunk");
                            }
                            return Ok(response.data);
                        }
                        libp2p::request_response::Message::Response { 
                            request_id: response_id, 
                            response 
                        } => {
                            println!("DEBUG: Received non-matching response for request {:?}: hash={}", response_id, response.hash);
                        }
                        _ => {}
                    }
                }
                SwarmEvent::Behaviour(FAIEvent::RequestResponse(
                    libp2p::request_response::Event::OutboundFailure { 
                        request_id: response_id, 
                        peer: _, 
                        error 
                    }
                )) if response_id == request_id => {
                    println!("Request failed for hash: {} (error: {:?})", hash, error);
                    return Ok(None);
                }
                SwarmEvent::Behaviour(FAIEvent::Mdns(mdns::Event::Discovered(list))) => {
                    for (peer_id, addr) in list {
                        println!("Discovered peer {} at {}", peer_id, addr);
                        self.swarm.dial(addr)?;
                    }
                }
                _ => {}
            }
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
    pub async fn request_commits(&mut self, peer: PeerId, commit_hash: Option<String>) -> Result<Vec<crate::storage::CommitInfo>> {
        println!("DEBUG: request_commits called with peer={}, commit_hash={:?}", peer, commit_hash);
        
        // Check if we have an active connection to this peer
        let connected_peers = self.swarm.connected_peers().collect::<Vec<_>>();
        println!("DEBUG: Currently connected to {} peers: {:?}", connected_peers.len(), connected_peers);
        
        if !connected_peers.iter().any(|p| **p == peer) {
            println!("DEBUG: Peer {} is not connected, attempting to dial", peer);
            // Try to find addresses for this peer
            if let Some(peer_info) = self.discovered_peers.get(&peer) {
                println!("DEBUG: Found {} addresses for peer {}", peer_info.addresses.len(), peer);
                for addr in &peer_info.addresses {
                    println!("DEBUG: Attempting to dial {} at {}", peer, addr);
                    if let Err(e) = self.swarm.dial(addr.clone()) {
                        println!("DEBUG: Failed to dial {} at {}: {:?}", peer, addr, e);
                    } else {
                        println!("DEBUG: Dialing {} at {} initiated", peer, addr);
                    }
                }
            } else {
                println!("DEBUG: No addresses found for peer {}", peer);
            }
            
            // Wait a bit for connection to establish
            for _ in 0..50 {
                let current_peers = self.swarm.connected_peers().collect::<Vec<_>>();
                if current_peers.iter().any(|p| **p == peer) {
                    println!("DEBUG: Successfully connected to peer {}", peer);
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
        
        // Ensure we're connected before sending request
        if !self.swarm.is_connected(&peer) {
            println!("DEBUG: Not connected to peer {}, cannot send commit request", peer);
            return Ok(vec![]);
        }
        
        let request_id = self.swarm.behaviour_mut().commit_response.send_request(
            &peer,
            CommitRequest { commit_hash },
        );
        
        println!("DEBUG: Sent commit request to peer {}, request_id={:?}", peer, request_id);
        println!("DEBUG: Awaiting response from server...");
        
        // Wait for response with timeout
        use futures::StreamExt;
        let timeout_duration = std::time::Duration::from_secs(30);
        let start_time = std::time::Instant::now();
        
        while start_time.elapsed() < timeout_duration {
            tokio::select! {
                event = self.swarm.next() => {
                    if let Some(event) = event {
                        match event {
                            SwarmEvent::Behaviour(FAIEvent::CommitResponse(
                                libp2p::request_response::Event::Message { 
                                    peer: _response_peer, 
                                    message 
                                }
                            )) => {
                                match message {
                                    libp2p::request_response::Message::Response { 
                                        request_id: response_id, 
                                        response 
                                    } => {
                                        println!("DEBUG: Received commit response for request {:?}: {} commits", 
                                            response_id, response.commits.len());
                                        for (i, commit) in response.commits.iter().enumerate() {
                                            println!("DEBUG: Commit {}: {} - {}", i, &commit.hash[..8], commit.message);
                                        }
                                        return Ok(response.commits);
                                    }
                                    libp2p::request_response::Message::Request { 
                                        request, 
                                        channel, 
                                        request_id,
                                        ..
                                    } => {
                                        println!("DEBUG: Received unexpected commit request from peer {} (request_id: {:?})", _response_peer, request_id);
                                        // This shouldn't happen in client mode, but handle it gracefully
                                        let response = CommitResponse { commits: vec![] };
                                        if let Err(e) = self.swarm.behaviour_mut().commit_response.send_response(channel, response) {
                                            println!("DEBUG: Failed to send response to unexpected request: {:?}", e);
                                        }
                                    }
                                }
                            }
                            SwarmEvent::Behaviour(FAIEvent::CommitResponse(
                                libp2p::request_response::Event::OutboundFailure { 
                                    request_id: response_id, 
                                    peer: _, 
                                    error 
                                }
                            )) if response_id == request_id => {
                                println!("DEBUG: Commit request failed (error: {:?})", error);
                                return Ok(vec![]);
                            }
                            SwarmEvent::Behaviour(FAIEvent::CommitResponse(
                                libp2p::request_response::Event::InboundFailure { 
                                    request_id: response_id, 
                                    peer: _, 
                                    error 
                                }
                            )) if response_id == request_id => {
                                println!("DEBUG: Commit request inbound failure (error: {:?})", error);
                                return Ok(vec![]);
                            }
                            SwarmEvent::Behaviour(FAIEvent::Mdns(mdns::Event::Discovered(list))) => {
                                for (peer_id, addr) in list {
                                    println!("DEBUG: Discovered additional peer {} at {}", peer_id, addr);
                                    let _ = self.swarm.dial(addr);
                                }
                            }
                            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                                println!("DEBUG: Connection established to {}", peer_id);
                            }
                            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                                println!("DEBUG: Connection closed to {}", peer_id);
                            }
                            _ => {
                                println!("DEBUG: Received other event type, continuing to wait...");
                            }
                        }
                    } else {
                        println!("DEBUG: Event stream ended unexpectedly");
                        break;
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                    // Continue loop
                }
            }
        }
        
        println!("DEBUG: Commit request timed out after {} seconds", timeout_duration.as_secs());
        Ok(vec![])
    }
}
