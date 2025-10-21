//! Network layer for FAI Protocol
//! 
//! Handles peer-to-peer networking for decentralized model sharing.

use anyhow::Result;
use libp2p::{
    Swarm, SwarmBuilder,
    identity::Keypair,
    mdns,
    tcp,
    noise,
    yamux,
    Multiaddr,
    PeerId,
    swarm::{NetworkBehaviour, SwarmEvent},
    request_response::{self, ProtocolSupport, RequestResponse, RequestResponseConfig, Behaviour},
};
use libp2p::swarm::TransportExt;
use std::{
    collections::HashMap,
    time::SystemTime,
};

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

/// Custom network behavior combining mDNS discovery and request-response
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "FAIComposedEvent")]
pub struct FAIBehaviour {
    /// mDNS for local peer discovery
    mdns: mdns::tokio::Behaviour,
    /// Request-response protocol for model sharing
    request_response: RequestResponse<FAIProtocolCodec>,
}

/// Event type for composed behavior
#[derive(Debug)]
pub enum FAIComposedEvent {
    Mdns(mdns::Event),
    RequestResponse(request_response::Event<FAIProtocolRequest, FAIProtocolResponse>),
}

impl From<mdns::Event> for FAIComposedEvent {
    fn from(event: mdns::Event) -> Self {
        FAIComposedEvent::Mdns(event)
    }
}

impl From<request_response::Event<FAIProtocolRequest, FAIProtocolResponse>> for FAIComposedEvent {
    fn from(event: request_response::Event<FAIProtocolRequest, FAIProtocolResponse>) -> Self {
        FAIComposedEvent::RequestResponse(event)
    }
}

/// Protocol codec for FAI Protocol requests/responses using JSON
#[derive(Debug, Clone, Default)]
pub struct FAIProtocolCodec;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FAIProtocolRequest {
    /// Request type (e.g., "get_model", "list_models")
    pub request_type: String,
    /// Request data (e.g., model hash)
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FAIProtocolResponse {
    /// Response data
    pub data: Vec<u8>,
    /// Success status
    pub success: bool,
}

#[async_trait::async_trait]
impl request_response::Codec for FAIProtocolCodec {
    type Protocol = &'static str;
    type Request = FAIProtocolRequest;
    type Response = FAIProtocolResponse;

    async fn read_request<T>(&mut self, protocol: &Self::Protocol, io: &mut T) -> std::io::Result<Self::Request>
    where
        T: futures::AsyncRead + Unpin + Send
    {
        // Use a simple approach: read length prefix, then read JSON
        use futures::AsyncReadExt;
        
        // Read length prefix (4 bytes)
        let mut len_bytes = [0u8; 4];
        io.read_exact(&mut len_bytes).await?;
        let len = u32::from_be_bytes(len_bytes) as usize;
        
        // Read JSON data
        let mut buffer = vec![0u8; len];
        io.read_exact(&mut buffer).await?;
        
        // Deserialize JSON
        serde_json::from_slice(&buffer)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    async fn read_response<T>(&mut self, protocol: &Self::Protocol, io: &mut T) -> std::io::Result<Self::Response>
    where
        T: futures::AsyncRead + Unpin + Send
    {
        // Use same approach as read_request
        use futures::AsyncReadExt;
        
        let mut len_bytes = [0u8; 4];
        io.read_exact(&mut len_bytes).await?;
        let len = u32::from_be_bytes(len_bytes) as usize;
        
        let mut buffer = vec![0u8; len];
        io.read_exact(&mut buffer).await?;
        
        serde_json::from_slice(&buffer)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    async fn write_request<T>(&mut self, protocol: &Self::Protocol, io: &mut T, req: Self::Request) -> std::io::Result<()>
    where
        T: futures::AsyncWrite + Unpin + Send
    {
        use futures::AsyncWriteExt;
        
        // Serialize to JSON
        let json_data = serde_json::to_vec(&req)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        
        // Write length prefix
        let len = json_data.len() as u32;
        io.write_all(&len.to_be_bytes()).await?;
        
        // Write JSON data
        io.write_all(&json_data).await?;
        io.flush().await?;
        
        Ok(())
    }

    async fn write_response<T>(&mut self, protocol: &Self::Protocol, io: &mut T, res: Self::Response) -> std::io::Result<()>
    where
        T: futures::AsyncWrite + Unpin + Send
    {
        use futures::AsyncWriteExt;
        
        let json_data = serde_json::to_vec(&res)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        
        let len = json_data.len() as u32;
        io.write_all(&len.to_be_bytes()).await?;
        io.write_all(&json_data).await?;
        io.flush().await?;
        
        Ok(())
    }
}

/// Network manager for FAI Protocol
pub struct NetworkManager {
    /// libp2p swarm for network operations
    swarm: Swarm<FAIBehaviour>,
    /// Map of discovered peers
    discovered_peers: HashMap<PeerId, PeerInfo>,
}

impl NetworkManager {
    /// Create a new network manager
    /// 
    /// # Returns
    /// A new NetworkManager instance with configured libp2p stack
    pub fn new() -> Result<Self> {
        // Generate or load identity
        let local_key = Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        // Create behavior
        let behaviour = FAIBehaviour {
            mdns: mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)?,
            request_response: RequestResponse::new(
                [(b"/fai-protocol/1.0.0", ProtocolSupport::Full)],
                RequestResponseConfig::default(),
            ),
        };

        // Create swarm using the new builder pattern
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
                SwarmEvent::Behaviour(FAIComposedEvent::Mdns(mdns::Event::Discovered(list))) => {
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
                SwarmEvent::Behaviour(FAIComposedEvent::Mdns(mdns::Event::Expired(list))) => {
                    for (peer_id, _addr) in list {
                        println!("Peer {} expired", peer_id);
                        self.discovered_peers.remove(&peer_id);
                    }
                }
                SwarmEvent::Behaviour(FAIComposedEvent::RequestResponse(event)) => {
                    println!("Request-Response event: {:?}", event);
                    // TODO: Handle request-response events
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
}
