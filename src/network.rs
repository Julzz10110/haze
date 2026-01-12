//! Network layer for HAZE using libp2p
//! 
//! Features:
//! - Haze Mesh topology
//! - Priority channels
//! - Node types (core, edge, light, mobile)

use std::sync::Arc;
use std::collections::HashSet;
use tokio::sync::mpsc;
use libp2p::{
    identity,
    swarm::{Swarm, SwarmEvent, NetworkBehaviour},
    PeerId, Multiaddr,
};
use crate::config::Config;
use crate::consensus::ConsensusEngine;
use crate::error::{HazeError, Result as HazeResult};
use crate::types::{Block, Transaction};

/// Network event
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    BlockReceived(Block),
    TransactionReceived(Transaction),
    PeerConnected(String),
    PeerDisconnected(String),
}

/// Haze network behaviour combining multiple protocols
#[derive(NetworkBehaviour)]
pub struct HazeBehaviour {
    pub ping: libp2p::ping::Behaviour,
    // Future: Add request-response for blocks/transactions
    // pub blocks: request_response::Behaviour<...>,
    // pub transactions: request_response::Behaviour<...>,
}

impl HazeBehaviour {
    fn new() -> Self {
        Self {
            ping: libp2p::ping::Behaviour::new(
                libp2p::ping::Config::new(),
            ),
        }
    }
}

/// Network manager with full libp2p integration
/// 
/// Note: Currently returns error on initialization due to libp2p 0.53 transport API issues.
/// Will be fully implemented once transport API is fixed.
pub struct Network {
    #[allow(dead_code)] // Will be used when transport is fixed
    swarm: Option<Swarm<HazeBehaviour>>,
    event_sender: mpsc::UnboundedSender<NetworkEvent>,
    #[allow(dead_code)] // Will be used in event loop
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

        // Create transport: TCP + Noise + Yamux
        // TODO: Fix libp2p 0.53 transport API - tcp::tokio requires tokio feature
        // For now, return error to allow compilation
        // Future implementation will use proper transport setup
        return Err(HazeError::Network(
            "libp2p transport initialization not yet fully implemented. Transport API needs to be updated for libp2p 0.53".to_string()
        ));
    }

    /// Start network event loop
    pub async fn run(&mut self) -> HazeResult<()> {
        tracing::info!("Network event loop started");
        
        // Event loop - will be properly implemented when transport is fixed
        loop {
            tokio::select! {
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
    /// 
    /// Will be implemented when transport is fixed
    #[allow(dead_code)]
    async fn handle_swarm_event(&mut self, _event: SwarmEvent<libp2p::ping::Event>) -> HazeResult<()> {
        // Will be implemented when transport is fixed
        Ok(())
    }

    /// Handle internal events
    async fn handle_internal_event(&mut self, _event: NetworkEvent) -> HazeResult<()> {
        // Handle internal events if needed
        Ok(())
    }

    /// Broadcast block to all connected peers
    pub fn broadcast_block(&mut self, block: &Block) -> HazeResult<()> {
        // Serialize block
        let block_data = bincode::serialize(block)
            .map_err(|e| HazeError::Serialization(format!("Failed to serialize block: {e}")))?;
        
        // Broadcast to all connected peers
        // Note: In a full implementation, this would use request-response protocol
        // For now, we'll use the event channel and log the broadcast
        let _ = self.event_sender.send(NetworkEvent::BlockReceived(block.clone()));
        
        tracing::debug!(
            "Block broadcast: height = {}, size = {} bytes, peers = {}",
            block.header.height,
            block_data.len(),
            self.connected_peers.len()
        );
        
        // Future: Use request-response to actually send to peers
        // for peer_id in &self.connected_peers {
        //     self.swarm.behaviour_mut().blocks.send_request(peer_id, block_data.clone());
        // }
        
        Ok(())
    }

    /// Broadcast transaction to all connected peers
    pub fn broadcast_transaction(&mut self, tx: &Transaction) -> HazeResult<()> {
        // Serialize transaction
        let tx_data = bincode::serialize(tx)
            .map_err(|e| HazeError::Serialization(format!("Failed to serialize transaction: {e}")))?;
        
        // Broadcast to all connected peers
        let _ = self.event_sender.send(NetworkEvent::TransactionReceived(tx.clone()));
        
        tracing::debug!(
            "Transaction broadcast: size = {} bytes, peers = {}",
            tx_data.len(),
            self.connected_peers.len()
        );
        
        // Future: Use request-response to actually send to peers
        
        Ok(())
    }

    /// Connect to a peer
    /// 
    /// Will be implemented when transport is fixed
    #[allow(dead_code)]
    pub fn dial(&mut self, _addr: Multiaddr) -> HazeResult<()> {
        // Will be implemented when transport is fixed
        Err(HazeError::Network("Network not initialized - transport API needs to be fixed".to_string()))
    }

    /// Get connected peers count
    pub fn connected_peers_count(&self) -> usize {
        self.connected_peers.len()
    }
}

// Network cannot be cloned - use Arc<Mutex<Network>> if needed
