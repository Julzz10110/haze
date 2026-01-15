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

use anyhow::Result;
use tracing::{info, error};

use std::sync::Arc;
use crate::config::Config;
use crate::network::Network;
use crate::consensus::ConsensusEngine;
use crate::state::StateManager;
use crate::api::{ApiState, start_api_server};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("HAZE Blockchain starting...");
    info!("Where games breathe blockchain");

    // Load configuration
    let config = Config::load()?;
    info!("Configuration loaded: {:?}", config);

    // Initialize state manager (includes tokenomics and economy)
    let state_manager = Arc::new(StateManager::new(&config)?);
    info!("State manager initialized");
    info!("Tokenomics: Total supply: {} HAZE", state_manager.tokenomics().total_supply());
    info!("Economy: Fog Economics initialized");

    // Initialize consensus engine
    let consensus = Arc::new(ConsensusEngine::new(config.clone(), state_manager.clone())?);
    info!("Consensus engine initialized");

    // Initialize network
    let mut network = Network::new(config.clone(), consensus.clone()).await?;
    info!("Network layer initialized");

    // Initialize API server
    let api_state = ApiState {
        consensus: consensus.clone(),
        state: state_manager.clone(),
        config: config.clone(),
    };
    info!("API server state initialized");

    // Start the node
    info!("HAZE node is running...");
    info!("Press Ctrl+C to shutdown");
    
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
    
    network_handle.abort();
    api_handle.abort();

    Ok(())
}