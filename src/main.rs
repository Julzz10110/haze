//! HAZE (High-performance Asset Zone Engine)
//! 
//! Specialized Layer 1 blockchain for GameFi
//! Concept of "digital fog" - distributed, fluid, omnipresent environment

mod consensus;
mod vm;
mod assets;
mod network;
mod types;
mod state;
mod crypto;
mod config;
mod error;
mod tokenomics;
mod economy;
mod api;
mod ws_events;

use anyhow::Result;
use tracing::{info, error};
use hex;

use std::sync::Arc;
use std::time::Duration;
use crate::config::Config;
use crate::network::Network;
use crate::consensus::ConsensusEngine;
use crate::state::StateManager;
use crate::api::start_api_server;
use crate::crypto::KeyPair;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("═══════════════════════════════════════════════════════════");
    info!("  HAZE Blockchain - High-performance Asset Zone Engine");
    info!("═══════════════════════════════════════════════════════════");

    // Load configuration
    let config = Config::load()?;
    info!("✓ Configuration loaded from: haze_config.json");
    info!("  Node ID: {}", config.node_id);
    info!("  Database: {:?}", config.storage.db_path);
    info!("  Network listen: {}", config.network.listen_addr);
    info!("  API listen: {}", config.api.listen_addr);

    // Initialize state manager (includes tokenomics and economy)
    let state_manager = Arc::new(StateManager::new(&config)?);
    info!("✓ State manager initialized");
    info!("  Tokenomics: Total supply: {} HAZE", state_manager.tokenomics().total_supply());
    info!("  Economy: Fog Economics initialized");
    info!("  Current height: {}", state_manager.current_height());

    // Initialize consensus engine
    let consensus = Arc::new(ConsensusEngine::new(config.clone(), state_manager.clone())?);
    info!("✓ Consensus engine initialized");
    info!("  Current wave: {}", consensus.get_current_wave());
    info!("  Max transactions per block: {}", config.consensus.max_transactions_per_block);

    // Generate validator keypair for block creation (MVP: single node validator)
    let validator_keypair = KeyPair::generate();
    let validator_address = validator_keypair.address();
    info!("✓ Validator keypair generated");
    info!("  Validator address: {}", hex::encode(validator_address));

    // Initialize network
    let mut network = Network::new(config.clone(), consensus.clone()).await?;
    info!("✓ Network layer initialized");
    info!("  Listening on: {}", config.network.listen_addr);
    info!("  Connected peers: {}", network.connected_peers_count());

    // Initialize WebSocket broadcast channel
    let (ws_tx, _) = tokio::sync::broadcast::channel::<crate::ws_events::WsEvent>(100);
    
    // Set WebSocket broadcaster in state manager
    state_manager.set_ws_tx(ws_tx.clone());
    info!("✓ WebSocket event broadcaster initialized");
    
    // Initialize API server
    let api_state = crate::api::ApiState {
        consensus: consensus.clone(),
        state: state_manager.clone(),
        config: config.clone(),
        ws_tx: ws_tx.clone(),
    };
    info!("✓ API server state initialized");

    // Start the node
    info!("═══════════════════════════════════════════════════════════");
    info!("  HAZE node is running!");
    info!("  API: http://{}/health", config.api.listen_addr);
    info!("  WebSocket: ws://{}/api/v1/ws", config.api.listen_addr);
    info!("  Press Ctrl+C to shutdown");
    info!("═══════════════════════════════════════════════════════════");
    
    // Clone consensus and validator address for block production task
    let consensus_for_blocks = consensus.clone();
    let validator_addr = validator_address;
    
    // Start block production task (MVP: create blocks periodically)
    let block_production_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5)); // Create block every 5 seconds
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        
        loop {
            interval.tick().await;
            
            // Check if there are transactions in the pool
            let tx_pool_size = consensus_for_blocks.tx_pool_size();
            
            if tx_pool_size > 0 {
                tracing::info!("Creating block with {} transactions from pool", tx_pool_size);
                
                // Create block
                match consensus_for_blocks.create_block(validator_addr) {
                    Ok(block) => {
                        let block_hash = hex::encode(block.header.hash);
                        tracing::info!("Block created: height={}, hash={}, txs={}", 
                            block.header.height, 
                            &block_hash[..16],
                            block.transactions.len());
                        
                        // Process block (add to DAG and apply to state)
                        if let Err(e) = consensus_for_blocks.process_block(&block) {
                            error!("Failed to process block: {}", e);
                        }
                        // Note: Block broadcasting will be handled by network layer when it receives blocks
                    }
                    Err(e) => {
                        error!("Failed to create block: {}", e);
                    }
                }
            } else {
                tracing::debug!("No transactions in pool, skipping block creation");
            }
        }
    });
    
    // Start network in background
    let network_handle = tokio::spawn(async move {
        if let Err(e) = network.run().await {
            error!("Network error: {}", e);
        }
    });
    
    // Start API server in background
    let api_handle = tokio::spawn(async move {
        if let Err(e) = start_api_server(api_state).await {
            error!("API server error: {}", e);
        }
    });
    
    // Keep the node running
    tokio::signal::ctrl_c().await?;
    info!("Shutting down HAZE node...");
    
    block_production_handle.abort();
    network_handle.abort();
    api_handle.abort();

    Ok(())
}