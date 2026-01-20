//! Network layer for HAZE using libp2p
//! 
//! Features:
//! - Haze Mesh topology
//! - Priority channels
//! - Node types (core, edge, light, mobile)

use std::sync::Arc;
use std::collections::HashSet;
use std::time::Duration;
use tokio::sync::mpsc;
use futures::StreamExt;
use libp2p::{
    identity,
    swarm::{Swarm, SwarmEvent, NetworkBehaviour},
    SwarmBuilder,
    PeerId, Multiaddr,
    noise,
    yamux,
    tcp,
};
use libp2p_request_response::{
    Behaviour as RequestResponse, Config as RequestResponseConfig, Codec as RequestResponseCodec, 
    ProtocolSupport,
};
use crate::config::Config;
use crate::consensus::ConsensusEngine;
use crate::error::{HazeError, Result as HazeResult};
use crate::types::{Block, Transaction};
use hex;

/// Network event
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    BlockReceived(Block),
    TransactionReceived(Transaction),
    PeerConnected(String),
    PeerDisconnected(String),
}

/// Protocol name for blocks
const BLOCKS_PROTOCOL_NAME: &[u8] = b"/haze/blocks/1.0.0";
/// Protocol name for transactions
const TRANSACTIONS_PROTOCOL_NAME: &[u8] = b"/haze/transactions/1.0.0";

/// Request types for request-response protocol
#[derive(Debug, Clone)]
pub enum HazeRequest {
    Block(Block),
    Transaction(Transaction),
}

/// Response types for request-response protocol
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum HazeResponse {
    BlockAck,
    TransactionAck,
    Error(String),
}

/// Codec for blocks and transactions using bincode
/// 
/// Implements RequestResponseCodec for serialization/deserialization
/// using bincode format with length-prefixed encoding
#[derive(Clone, Default)]
pub struct HazeCodec {
    protocol: Vec<u8>,
}

impl HazeCodec {
    fn new(protocol: Vec<u8>) -> Self {
        Self { protocol }
    }
    
    fn protocol_string(&self) -> String {
        String::from_utf8_lossy(&self.protocol).to_string()
    }
}

#[async_trait::async_trait]
impl RequestResponseCodec for HazeCodec {
    type Protocol = String;
    type Request = HazeRequest;
    type Response = HazeResponse;

    async fn read_request<T>(&mut self, _: &Self::Protocol, io: &mut T) -> std::io::Result<Self::Request>
    where
        T: futures::AsyncRead + Unpin + Send,
    {
        use futures::AsyncReadExt;
        
        // Read length prefix
        let mut length_bytes = [0u8; 4];
        io.read_exact(&mut length_bytes).await?;
        let length = u32::from_be_bytes(length_bytes) as usize;
        
        // Read payload
        let mut buffer = vec![0u8; length];
        io.read_exact(&mut buffer).await?;
        
        // Deserialize based on protocol
        let protocol_str = String::from_utf8_lossy(&self.protocol);
        if protocol_str.as_ref() == String::from_utf8_lossy(BLOCKS_PROTOCOL_NAME).as_ref() {
            let block: Block = bincode::deserialize(&buffer)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            Ok(HazeRequest::Block(block))
        } else if protocol_str.as_ref() == String::from_utf8_lossy(TRANSACTIONS_PROTOCOL_NAME).as_ref() {
            let tx: Transaction = bincode::deserialize(&buffer)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            Ok(HazeRequest::Transaction(tx))
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unknown protocol",
            ))
        }
    }

    async fn read_response<T>(&mut self, _: &Self::Protocol, io: &mut T) -> std::io::Result<Self::Response>
    where
        T: futures::AsyncRead + Unpin + Send,
    {
        use futures::AsyncReadExt;
        
        // Read length prefix
        let mut length_bytes = [0u8; 4];
        io.read_exact(&mut length_bytes).await?;
        let length = u32::from_be_bytes(length_bytes) as usize;
        
        // Read payload
        let mut buffer = vec![0u8; length];
        io.read_exact(&mut buffer).await?;
        
        // Deserialize response
        bincode::deserialize(&buffer)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    async fn write_request<T>(&mut self, _: &Self::Protocol, io: &mut T, request: Self::Request) -> std::io::Result<()>
    where
        T: futures::AsyncWrite + Unpin + Send,
    {
        use futures::AsyncWriteExt;
        
        let data = match request {
            HazeRequest::Block(block) => bincode::serialize(&block)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
            HazeRequest::Transaction(tx) => bincode::serialize(&tx)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
        };
        
        // Write length prefix
        let length = data.len() as u32;
        io.write_all(&length.to_be_bytes()).await?;
        
        // Write payload
        io.write_all(&data).await?;
        io.flush().await?;
        
        Ok(())
    }

    async fn write_response<T>(&mut self, _: &Self::Protocol, io: &mut T, response: Self::Response) -> std::io::Result<()>
    where
        T: futures::AsyncWrite + Unpin + Send,
    {
        use futures::AsyncWriteExt;
        
        let data = bincode::serialize(&response)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        
        // Write length prefix
        let length = data.len() as u32;
        io.write_all(&length.to_be_bytes()).await?;
        
        // Write payload
        io.write_all(&data).await?;
        io.flush().await?;
        
        Ok(())
    }
}


/// Haze network behaviour combining multiple protocols
#[derive(NetworkBehaviour)]
pub struct HazeBehaviour {
    pub ping: libp2p::ping::Behaviour,
    pub blocks: RequestResponse<HazeCodec>,
    pub transactions: RequestResponse<HazeCodec>,
}

impl HazeBehaviour {
    fn new() -> Self {
        // Configure blocks protocol
        let blocks_config = RequestResponseConfig::default();
        let blocks_protocol = ProtocolSupport::Full;
        let blocks_protocol_name = String::from_utf8_lossy(BLOCKS_PROTOCOL_NAME).to_string();
        let blocks: RequestResponse<HazeCodec> = RequestResponse::new(
            [(blocks_protocol_name.clone(), blocks_protocol)],
            blocks_config,
        );

        // Configure transactions protocol
        let transactions_config = RequestResponseConfig::default();
        let transactions_protocol = ProtocolSupport::Full;
        let transactions_protocol_name = String::from_utf8_lossy(TRANSACTIONS_PROTOCOL_NAME).to_string();
        let transactions: RequestResponse<HazeCodec> = RequestResponse::new(
            [(transactions_protocol_name.clone(), transactions_protocol)],
            transactions_config,
        );

        Self {
            ping: libp2p::ping::Behaviour::new(
                libp2p::ping::Config::new()
                    .with_interval(Duration::from_secs(30))
                    .with_timeout(Duration::from_secs(10)),
            ),
            blocks,
            transactions,
        }
    }
}

/// Network manager with full libp2p integration
pub struct Network {
    swarm: Swarm<HazeBehaviour>,
    event_sender: mpsc::UnboundedSender<NetworkEvent>,
    event_receiver: mpsc::UnboundedReceiver<NetworkEvent>,
    config: Config,
    consensus: Arc<ConsensusEngine>,
    connected_peers: HashSet<PeerId>,
}

impl Network {
    pub async fn new(
        config: Config,
        consensus: Arc<ConsensusEngine>,
    ) -> HazeResult<Self> {
        tracing::info!("Initializing network layer...");
        tracing::info!("Listen address: {}", config.network.listen_addr);

        // Create event channel
        let (event_sender, event_receiver) = mpsc::unbounded_channel::<NetworkEvent>();

        // Generate local key pair
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        tracing::info!("Local peer ID: {}", local_peer_id);

        // Create behaviour
        let behaviour = HazeBehaviour::new();

        // Create swarm with SwarmBuilder for libp2p 0.53
        // First specify provider (tokio), then transport (tcp)
        let swarm = SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )
            .map_err(|e| HazeError::Network(format!("Failed to create TCP transport: {}", e)))?
            .with_behaviour(|_| behaviour)
            .map_err(|e| HazeError::Network(format!("Failed to set behaviour: {:?}", e)))?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        // Parse listen address
        let listen_addr: Multiaddr = config.network.listen_addr.parse()
            .map_err(|e| HazeError::Network(format!("Invalid listen address: {}", e)))?;

        // Create network instance
        let mut network = Self {
            swarm,
            event_sender,
            event_receiver,
            config,
            consensus,
            connected_peers: HashSet::new(),
        };

        // Start listening
        network.swarm.listen_on(listen_addr)
            .map_err(|e| HazeError::Network(format!("Failed to start listening: {}", e)))?;

        tracing::info!("Network layer initialized successfully");
        Ok(network)
    }

    /// Start network event loop
    pub async fn run(&mut self) -> HazeResult<()> {
        tracing::info!("Network event loop started");
        
        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => {
                    self.handle_swarm_event(event).await?;
                }
                event = self.event_receiver.recv() => {
                    if let Some(event) = event {
                        self.handle_internal_event(event).await?;
                    } else {
                        // Channel closed
                        break;
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    tracing::info!("Shutdown signal received");
                    break;
                }
            }
        }
        
        tracing::info!("Network event loop stopped");
        Ok(())
    }

    /// Handle swarm events
    async fn handle_swarm_event(&mut self, event: SwarmEvent<HazeBehaviourEvent>) -> HazeResult<()> {
        match event {
            SwarmEvent::Behaviour(event) => {
                self.handle_behaviour_event(event).await?;
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                tracing::info!("Listening on {}", address);
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                tracing::info!("Connected to peer: {}", peer_id);
                self.connected_peers.insert(peer_id);
                let _ = self.event_sender.send(NetworkEvent::PeerConnected(peer_id.to_string()));
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                tracing::info!("Disconnected from peer: {}", peer_id);
                self.connected_peers.remove(&peer_id);
                let _ = self.event_sender.send(NetworkEvent::PeerDisconnected(peer_id.to_string()));
            }
            SwarmEvent::IncomingConnection { .. } => {
                // Accept incoming connections
            }
            SwarmEvent::IncomingConnectionError { error, .. } => {
                tracing::warn!("Incoming connection error: {}", error);
            }
            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                tracing::warn!("Outgoing connection error to {}: {}", peer_id.map(|p| p.to_string()).unwrap_or_else(|| "unknown".to_string()), error);
            }
            SwarmEvent::Dialing { peer_id, .. } => {
                tracing::debug!("Dialing peer: {}", peer_id.map(|p| p.to_string()).unwrap_or_else(|| "unknown".to_string()));
            }
            SwarmEvent::ListenerClosed { addresses, reason, .. } => {
                tracing::warn!("Listener closed on {:?}: {:?}", addresses, reason);
            }
            SwarmEvent::ListenerError { error, .. } => {
                tracing::warn!("Listener error: {}", error);
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle behaviour events (ping, request-response)
    async fn handle_behaviour_event(&mut self, event: HazeBehaviourEvent) -> HazeResult<()> {
        match event {
            HazeBehaviourEvent::Ping(ping_event) => {
                // Handle ping events if needed
                tracing::debug!("Ping event: {:?}", ping_event);
            }
            HazeBehaviourEvent::Blocks(libp2p::request_response::Event::Message { message, .. }) => {
                match message {
                    libp2p::request_response::Message::Request { request, channel, .. } => {
                        match request {
                            HazeRequest::Block(block) => {
                                tracing::debug!("Received block request: height = {}", block.header.height);
                                // Forward to consensus engine
                                if let Err(e) = self.consensus.process_block(&block) {
                                    tracing::warn!("Failed to process block: {}", e);
                                }
                                // Send acknowledgment
                                let _ = self.swarm.behaviour_mut().blocks.send_response(
                                    channel,
                                    HazeResponse::BlockAck,
                                );
                                let _ = self.event_sender.send(NetworkEvent::BlockReceived(block));
                            }
                            HazeRequest::Transaction(tx) => {
                                tracing::debug!("Received transaction request");
                                // Forward to consensus engine
                                match self.consensus.add_transaction(tx.clone()) {
                                    Ok(()) => {
                                        tracing::info!("Transaction added to pool: {}", hex::encode(tx.hash()));
                                        // Send acknowledgment
                                        let _ = self.swarm.behaviour_mut().transactions.send_response(
                                            channel,
                                            HazeResponse::TransactionAck,
                                        );
                                        let _ = self.event_sender.send(NetworkEvent::TransactionReceived(tx));
                                    }
                                    Err(e) => {
                                        tracing::warn!("Failed to add transaction: {}", e);
                                        // Send error response
                                        let _ = self.swarm.behaviour_mut().transactions.send_response(
                                            channel,
                                            HazeResponse::Error(format!("Failed to add transaction: {}", e)),
                                        );
                                    }
                                }
                            }
                        }
                    }
                    libp2p::request_response::Message::Response { response, .. } => {
                        match response {
                            HazeResponse::BlockAck => {
                                tracing::debug!("Received block acknowledgment");
                            }
                            HazeResponse::TransactionAck => {
                                tracing::debug!("Received transaction acknowledgment");
                            }
                            HazeResponse::Error(msg) => {
                                tracing::warn!("Received error response: {}", msg);
                            }
                        }
                    }
                }
            }
            HazeBehaviourEvent::Transactions(libp2p::request_response::Event::Message { message, .. }) => {
                match message {
                    libp2p::request_response::Message::Request { request, channel, .. } => {
                        match request {
                            HazeRequest::Block(block) => {
                                tracing::debug!("Received block request via transactions protocol");
                                match self.consensus.process_block(&block) {
                                    Ok(()) => {
                                        tracing::info!("Block processed: height={}", block.header.height);
                                        let _ = self.swarm.behaviour_mut().transactions.send_response(
                                            channel,
                                            HazeResponse::BlockAck,
                                        );
                                        let _ = self.event_sender.send(NetworkEvent::BlockReceived(block));
                                    }
                                    Err(e) => {
                                        tracing::warn!("Failed to process block: {}", e);
                                        let _ = self.swarm.behaviour_mut().transactions.send_response(
                                            channel,
                                            HazeResponse::Error(format!("Failed to process block: {}", e)),
                                        );
                                    }
                                }
                            }
                            HazeRequest::Transaction(tx) => {
                                tracing::debug!("Received transaction request");
                                match self.consensus.add_transaction(tx.clone()) {
                                    Ok(()) => {
                                        tracing::info!("Transaction added to pool: {}", hex::encode(tx.hash()));
                                        let _ = self.swarm.behaviour_mut().transactions.send_response(
                                            channel,
                                            HazeResponse::TransactionAck,
                                        );
                                        let _ = self.event_sender.send(NetworkEvent::TransactionReceived(tx));
                                    }
                                    Err(e) => {
                                        tracing::warn!("Failed to add transaction: {}", e);
                                        let _ = self.swarm.behaviour_mut().transactions.send_response(
                                            channel,
                                            HazeResponse::Error(format!("Failed to add transaction: {}", e)),
                                        );
                                    }
                                }
                            }
                        }
                    }
                    libp2p::request_response::Message::Response { response, .. } => {
                        match response {
                            HazeResponse::BlockAck => {
                                tracing::debug!("Received block acknowledgment");
                            }
                            HazeResponse::TransactionAck => {
                                tracing::debug!("Received transaction acknowledgment");
                            }
                            HazeResponse::Error(msg) => {
                                tracing::warn!("Received error response: {}", msg);
                            }
                        }
                    }
                }
            }
            HazeBehaviourEvent::Blocks(libp2p::request_response::Event::OutboundFailure { request_id, error, .. }) => {
                tracing::warn!("Blocks outbound failure (request {}): {:?}", request_id, error);
            }
            HazeBehaviourEvent::Transactions(libp2p::request_response::Event::OutboundFailure { request_id, error, .. }) => {
                tracing::warn!("Transactions outbound failure (request {}): {:?}", request_id, error);
            }
            HazeBehaviourEvent::Blocks(libp2p::request_response::Event::InboundFailure { error, .. }) => {
                tracing::warn!("Blocks inbound failure: {:?}", error);
            }
            HazeBehaviourEvent::Transactions(libp2p::request_response::Event::InboundFailure { error, .. }) => {
                tracing::warn!("Transactions inbound failure: {:?}", error);
            }
            HazeBehaviourEvent::Blocks(libp2p::request_response::Event::ResponseSent { .. }) => {
                // Response sent successfully
            }
            HazeBehaviourEvent::Transactions(libp2p::request_response::Event::ResponseSent { .. }) => {
                // Response sent successfully
            }
        }
        Ok(())
    }

    /// Handle internal events
    async fn handle_internal_event(&mut self, event: NetworkEvent) -> HazeResult<()> {
        match event {
            NetworkEvent::BlockReceived(block) => {
                tracing::debug!("Internal block event: height = {}", block.header.height);
            }
            NetworkEvent::TransactionReceived(_tx) => {
                tracing::debug!("Internal transaction event");
            }
            NetworkEvent::PeerConnected(peer_id) => {
                tracing::info!("Peer connected: {}", peer_id);
            }
            NetworkEvent::PeerDisconnected(peer_id) => {
                tracing::info!("Peer disconnected: {}", peer_id);
            }
        }
        Ok(())
    }

    /// Broadcast block to all connected peers
    pub fn broadcast_block(&mut self, block: &Block) -> HazeResult<()> {
        // Serialize block
        let block_data = bincode::serialize(block)
            .map_err(|e| HazeError::Serialization(format!("Failed to serialize block: {e}")))?;
        
        tracing::debug!(
            "Broadcasting block: height = {}, size = {} bytes, peers = {}",
            block.header.height,
            block_data.len(),
            self.connected_peers.len()
        );
        
        // Send to all connected peers using request-response protocol
        let request = HazeRequest::Block(block.clone());
        for peer_id in &self.connected_peers {
            let _request_id = self.swarm.behaviour_mut().blocks.send_request(peer_id, request.clone());
            tracing::debug!("Sent block request to {}: request_id = {:?}", peer_id, _request_id);
        }
        
        Ok(())
    }

    /// Broadcast transaction to all connected peers
    pub fn broadcast_transaction(&mut self, tx: &Transaction) -> HazeResult<()> {
        // Serialize transaction
        let tx_data = bincode::serialize(tx)
            .map_err(|e| HazeError::Serialization(format!("Failed to serialize transaction: {e}")))?;
        
        tracing::debug!(
            "Broadcasting transaction: size = {} bytes, peers = {}",
            tx_data.len(),
            self.connected_peers.len()
        );
        
        // Send to all connected peers using request-response protocol
        let request = HazeRequest::Transaction(tx.clone());
        for peer_id in &self.connected_peers {
            let _request_id = self.swarm.behaviour_mut().transactions.send_request(peer_id, request.clone());
            tracing::debug!("Sent transaction request to {}: request_id = {:?}", peer_id, _request_id);
        }
        
        Ok(())
    }

    /// Connect to a peer
    pub fn dial(&mut self, addr: Multiaddr) -> HazeResult<()> {
        self.swarm.dial(addr)
            .map_err(|e| HazeError::Network(format!("Failed to dial peer: {}", e)))?;
        Ok(())
    }

    /// Get connected peers count
    pub fn connected_peers_count(&self) -> usize {
        self.connected_peers.len()
    }

    /// Get swarm reference for advanced operations
    pub fn swarm_mut(&mut self) -> &mut Swarm<HazeBehaviour> {
        &mut self.swarm
    }
}

// Network cannot be cloned - use Arc<Mutex<Network>> if needed
