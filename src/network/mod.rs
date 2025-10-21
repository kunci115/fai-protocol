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
    time::SystemTime,
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

/// Simple network behavior for peer discovery and chunk requests
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "FAIEvent")]
pub struct FAIBehaviour {
    /// mDNS for local peer discovery
    mdns: mdns::tokio::Behaviour,
    /// Request-response protocol for chunk requests
    request_response: libp2p::request_response::cbor::Behaviour<ChunkRequest, ChunkResponse>,
}

/// Network events
#[derive(Debug)]
pub enum FAIEvent {
    Mdns(mdns::Event),
    RequestResponse(libp2p::request_response::Event<ChunkRequest, ChunkResponse>),
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

        // Create behavior with mDNS and chunk request/response
        let behaviour = FAIBehaviour {
            mdns: mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)?,
            request_response: libp2p::request_response::cbor::Behaviour::new(
                [(libp2p::StreamProtocol::new("/fai/chunk/1.0.0"), ProtocolSupport::Full)],
                libp2p::request_response::Config::default(),
            ),
        };

        // Create swarm using the new builder pattern with TCP transport
        let swarm = SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default(),
                libp2p::noise::Config::new,
                yamux::Config::default,
            )?
            .with_behaviour(|_| behaviour)?
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
                    if let libp2p::request_response::Message::Request { 
                        request, 
                        channel, 
                        .. 
                    } = message {
                    println!("Received chunk request from {} for hash: {}", peer, request.hash);
                    
                    // Try to retrieve the data from storage
                    let data = match self.storage.retrieve(&request.hash) {
                        Ok(data) => {
                            println!("Found chunk {} ({} bytes)", request.hash, data.len());
                            Some(data)
                        }
                        Err(_) => {
                            println!("Chunk {} not found", request.hash);
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
                }
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Listening on {}", address);
                }
                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    println!("Connected to {}", peer_id);
                }
                SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                    println!("Disconnected from {} (reason: {:?})", peer_id, cause);
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
                            println!("Received chunk response for hash: {}", response.hash);
                            return Ok(response.data);
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
}
