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
    if !config.network.bootstrap_nodes.is_empty() {
        info!("  Bootstrap nodes: {} node(s)", config.network.bootstrap_nodes.len());
        for (i, addr) in config.network.bootstrap_nodes.iter().enumerate() {
            info!("    {}: {}", i + 1, addr);
        }
    }

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
                let block_start_time = std::time::Instant::now();
                tracing::info!("Creating block with {} transactions from pool", tx_pool_size);
                
                // Create block
                match consensus_for_blocks.create_block(validator_addr) {
                    Ok(block) => {
                        let block_creation_time = block_start_time.elapsed();
                        let block_hash = hex::encode(block.header.hash);
                        let height = block.header.height;
                        let tx_count = block.transactions.len();
                        
                        tracing::info!("Block created: height={}, hash={}, txs={}, creation_time={}ms", 
                            height, 
                            &block_hash[..16],
                            tx_count,
                            block_creation_time.as_millis());
                        
                        // Process block (add to DAG and apply to state)
                        let process_start = std::time::Instant::now();
                        if let Err(e) = consensus_for_blocks.process_block(&block) {
                            error!("Failed to process block: {}", e);
                        } else {
                            let process_time = process_start.elapsed();
                            let total_time = block_start_time.elapsed();
                            tracing::info!("Block processed: height={}, process_time={}ms, total_time={}ms", 
                                height, process_time.as_millis(), total_time.as_millis());
                        }
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
    
    // Start periodic metrics logging task
    let consensus_for_metrics = consensus.clone();
    let state_for_metrics = state_manager.clone();
    let metrics_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30)); // Log metrics every 30 seconds
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        
        loop {
            interval.tick().await;
            
            let height = state_for_metrics.current_height();
            let finalized_height = consensus_for_metrics.get_last_finalized_height();
            let finalized_wave = consensus_for_metrics.get_last_finalized_wave();
            let tx_pool_size = consensus_for_metrics.tx_pool_size();
            
            // Calculate tx/sec from recent blocks (last 10 blocks)
            let tx_per_sec = if height > 0 {
                let mut total_txs = 0u64;
                let mut block_count = 0u64;
                let start_height = height.saturating_sub(10);
                for h in start_height..=height {
                    if let Some(block) = state_for_metrics.get_block_by_height(h) {
                        total_txs += block.transactions.len() as u64;
                        block_count += 1;
                    }
                }
                if block_count > 0 {
                    // Estimate: assume ~5 second block time for MVP
                    let estimated_seconds = block_count * 5;
                    if estimated_seconds > 0 {
                        (total_txs * 1000) / estimated_seconds / 1000 // tx/sec (rough estimate)
                    } else {
                        0
                    }
                } else {
                    0
                }
            } else {
                0
            };
            
            tracing::info!(
                "Metrics: height={}, finalized_height={}, finalized_wave={}, tx_pool={}, tx_per_sec_est={}",
                height, finalized_height, finalized_wave, tx_pool_size, tx_per_sec
            );
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
    metrics_handle.abort();
    network_handle.abort();
    api_handle.abort();

    Ok(())
}