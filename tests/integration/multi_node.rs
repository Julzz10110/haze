//! Multi-node synchronization tests

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use haze::config::Config;
use haze::consensus::ConsensusEngine;
use haze::crypto::KeyPair;
use haze::state::StateManager;
use haze::types::{AssetAction, AssetData, DensityLevel, Transaction};
use hex;

static MULTI_NODE_TEST_DB_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Sign MistbornAsset transaction (must mirror consensus::get_transaction_data_for_signing)
fn sign_mistborn_asset_tx(
    keypair: &KeyPair,
    action: &AssetAction,
    asset_id: &haze::types::Hash,
    data: &AssetData,
) -> Vec<u8> {
    let mut serialized = Vec::new();
    serialized.extend_from_slice(b"MistbornAsset");
    // from (signer) — в тестах это всегда владелец
    serialized.extend_from_slice(&data.owner);
    // action as u8
    serialized.push(match action {
        AssetAction::Create => 0,
        AssetAction::Update => 1,
        AssetAction::Condense => 2,
        AssetAction::Evaporate => 3,
        AssetAction::Merge => 4,
        AssetAction::Split => 5,
    });
    serialized.extend_from_slice(asset_id);
    serialized.extend_from_slice(&data.owner);
    serialized.push(match data.density {
        DensityLevel::Ethereal => 0,
        DensityLevel::Light => 1,
        DensityLevel::Dense => 2,
        DensityLevel::Core => 3,
    });
    
    // For Merge: include other_asset_id in signature
    if matches!(action, AssetAction::Merge) {
        if let Some(other_asset_id_str) = data.metadata.get("_other_asset_id") {
            if let Ok(other_asset_id_bytes) = hex::decode(other_asset_id_str) {
                if other_asset_id_bytes.len() == 32 {
                    serialized.extend_from_slice(&other_asset_id_bytes);
                }
            }
        }
    }
    
    // For Split: include components in signature
    if matches!(action, AssetAction::Split) {
        if let Some(components_str) = data.metadata.get("_components") {
            serialized.extend_from_slice(components_str.as_bytes());
        }
    }
    
    // fee и nonce — в тестах всегда 0
    serialized.extend_from_slice(&0u64.to_le_bytes());
    serialized.extend_from_slice(&0u64.to_le_bytes());
    
    keypair.sign(&serialized)
}

fn create_test_node(_id: u64) -> (Arc<StateManager>, Arc<ConsensusEngine>, KeyPair) {
    let db_id = MULTI_NODE_TEST_DB_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut config = Config::default();
    config.storage.db_path = PathBuf::from(format!("./haze_db_test_multi_node_{}", db_id));
    
    let state = Arc::new(StateManager::new(&config).unwrap());
    let consensus = Arc::new(ConsensusEngine::new(config, state.clone()).unwrap());
    let keypair = KeyPair::generate();
    
    (state, consensus, keypair)
}

#[tokio::test]
async fn test_multi_node_asset_sync() {
    // Create two nodes
    let (state1, consensus1, keypair1) = create_test_node(1);
    let (state2, consensus2, _keypair2) = create_test_node(2);
    
    let owner = keypair1.address();
    
    // Create account on node 1 with balance for gas
    state1.create_test_account(owner, 100_000, 0);
    
    // Create asset transaction on node 1
    let asset_id = haze::types::sha256(b"sync_test_asset");
    let mut meta = std::collections::HashMap::new();
    meta.insert("name".to_string(), "Synced Asset".to_string());
    
    let data = AssetData {
        density: DensityLevel::Ethereal,
        metadata: meta,
        attributes: vec![],
        game_id: Some("test_game".to_string()),
        owner,
    };
    let signature = sign_mistborn_asset_tx(&keypair1, &AssetAction::Create, &asset_id, &data);
    
    let tx = Transaction::MistbornAsset {
        from: owner,
        action: AssetAction::Create,
        asset_id,
        data,
        fee: 0,
        nonce: 0,
        signature,
    };
    
    // Add transaction to node 1's pool
    consensus1.add_transaction(tx.clone()).unwrap();
    
    // Create block on node 1
    let block = consensus1.create_block(owner).unwrap();
    
    // Process block on node 1
    consensus1.process_block(&block).unwrap();
    
    // Verify asset exists on node 1
    assert!(state1.get_asset(&asset_id).is_some());
    
    // Sync: process block on node 2
    // First, create account on node 2 (needed for gas fee processing)
    state2.create_test_account(owner, 100_000, 0);
    
    // Process block on node 2
    consensus2.process_block(&block).unwrap();
    
    // Verify asset synced to node 2
    let asset2 = state2.get_asset(&asset_id);
    assert!(asset2.is_some());
    let asset2_state = asset2.unwrap();
    assert_eq!(asset2_state.owner, owner);
    assert_eq!(asset2_state.data.metadata.get("name"), Some(&"Synced Asset".to_string()));
    assert_eq!(asset2_state.data.game_id, Some("test_game".to_string()));
}

#[tokio::test]
async fn test_multi_node_block_chain_sync() {
    // Create two nodes
    let (state1, consensus1, keypair1) = create_test_node(1);
    let (state2, consensus2, _keypair2) = create_test_node(2);
    
    let owner = keypair1.address();
    state1.create_test_account(owner, 100_000, 0);
    state2.create_test_account(owner, 100_000, 0);
    
    // Create 3 assets on node 1
    let mut blocks = Vec::new();
    for i in 0..3 {
        let asset_id = haze::types::sha256(&format!("chain_asset_{}", i).as_bytes());
        let data = AssetData {
            density: DensityLevel::Ethereal,
            metadata: std::collections::HashMap::new(),
            attributes: vec![],
            game_id: None,
            owner,
        };
        let signature = sign_mistborn_asset_tx(&keypair1, &AssetAction::Create, &asset_id, &data);
        
        let tx = Transaction::MistbornAsset {
            from: owner,
            action: AssetAction::Create,
            asset_id,
            data,
            fee: 0,
            nonce: 0,
            signature,
        };
        
        consensus1.add_transaction(tx).unwrap();
        let block = consensus1.create_block(owner).unwrap();
        // Process block on node 1
        consensus1.process_block(&block).unwrap();
        blocks.push(block);
    }
    
    // Sync all blocks to node 2
    for block in &blocks {
        consensus2.process_block(block).unwrap();
    }
    
    // Verify all assets synced
    assert_eq!(state1.current_height(), 3);
    assert_eq!(state2.current_height(), 3);
    
    for i in 0..3 {
        let asset_id = haze::types::sha256(&format!("chain_asset_{}", i).as_bytes());
        assert!(state1.get_asset(&asset_id).is_some());
        assert!(state2.get_asset(&asset_id).is_some());
    }
}
