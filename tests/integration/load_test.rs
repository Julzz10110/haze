//! Load tests: performance and scalability

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use haze::config::Config;
use haze::consensus::ConsensusEngine;
use haze::crypto::KeyPair;
use haze::state::StateManager;
use haze::types::{AssetAction, AssetData, DensityLevel, Transaction};
use hex;

static LOAD_TEST_DB_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Sign MistbornAsset transaction (matches consensus signing format)
fn sign_mistborn_asset_tx(
    keypair: &KeyPair,
    action: &AssetAction,
    asset_id: &haze::types::Hash,
    data: &AssetData,
) -> Vec<u8> {
    let mut serialized = Vec::new();
    serialized.extend_from_slice(b"MistbornAsset");
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
    
    keypair.sign(&serialized)
}

fn create_test_node() -> (Arc<StateManager>, Arc<ConsensusEngine>, KeyPair) {
    let db_id = LOAD_TEST_DB_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut config = Config::default();
    config.storage.db_path = PathBuf::from(format!("./haze_db_test_load_{}", db_id));
    
    let state = Arc::new(StateManager::new(&config).unwrap());
    let consensus = Arc::new(ConsensusEngine::new(config, state.clone()).unwrap());
    let keypair = KeyPair::generate();
    
    (state, consensus, keypair)
}

#[tokio::test]
async fn test_load_create_many_assets() {
    let (state, consensus, keypair) = create_test_node();
    let owner = keypair.address();
    state.create_test_account(owner, 10_000_000, 0); // Large balance for many assets
    
    const ASSET_COUNT: usize = 100;
    let start = Instant::now();
    
    // Create many assets
    for i in 0..ASSET_COUNT {
        let asset_id = haze::types::sha256(&format!("load_asset_{}", i).as_bytes());
        let mut meta = std::collections::HashMap::new();
        meta.insert("index".to_string(), i.to_string());
        meta.insert("name".to_string(), format!("Load Asset {}", i));
        
        let data = AssetData {
            density: DensityLevel::Ethereal,
            metadata: meta,
            attributes: vec![],
            game_id: None,
            owner,
        };
        let signature = sign_mistborn_asset_tx(&keypair, &AssetAction::Create, &asset_id, &data);
        
        let tx = Transaction::MistbornAsset {
            action: AssetAction::Create,
            asset_id,
            data,
            signature,
        };
        
        consensus.add_transaction(tx).unwrap();
    }
    
    // Create blocks to process all transactions
    let mut blocks_created = 0;
    while consensus.tx_pool_size() > 0 {
        let block = consensus.create_block(owner).unwrap();
        consensus.process_block(&block).unwrap();
        blocks_created += 1;
    }
    
    let elapsed = start.elapsed();
    
    // Verify all assets created
    assert_eq!(state.current_height(), blocks_created);
    
    // Count assets by owner
    let owner_assets = state.search_assets_by_owner(&owner);
    assert_eq!(owner_assets.len(), ASSET_COUNT);
    
    // Verify quota usage
    let quota = state.get_quota_usage(&owner);
    assert_eq!(quota.assets_count, ASSET_COUNT as u64);
    
    println!("Created {} assets in {} blocks, took {:?}", ASSET_COUNT, blocks_created, elapsed);
    println!("Assets per second: {:.2}", ASSET_COUNT as f64 / elapsed.as_secs_f64());
}

#[tokio::test]
async fn test_load_batch_operations() {
    let (state, consensus, keypair) = create_test_node();
    let owner = keypair.address();
    state.create_test_account(owner, 10_000_000, 0);
    
    const BATCH_SIZE: usize = 50;
    let start = Instant::now();
    
    // Create batch of assets
    for i in 0..BATCH_SIZE {
        let asset_id = haze::types::sha256(&format!("batch_asset_{}", i).as_bytes());
        let data = AssetData {
            density: DensityLevel::Ethereal,
            metadata: std::collections::HashMap::new(),
            attributes: vec![],
            game_id: None,
            owner,
        };
        let signature = sign_mistborn_asset_tx(&keypair, &AssetAction::Create, &asset_id, &data);
        
        let tx = Transaction::MistbornAsset {
            action: AssetAction::Create,
            asset_id,
            data,
            signature,
        };
        
        consensus.add_transaction(tx).unwrap();
    }
    
    // Process in single block
    let block = consensus.create_block(owner).unwrap();
    consensus.process_block(&block).unwrap();
    
    let elapsed = start.elapsed();
    
    // Verify
    let owner_assets = state.search_assets_by_owner(&owner);
    assert_eq!(owner_assets.len(), BATCH_SIZE);
    
    println!("Batch created {} assets in single block, took {:?}", BATCH_SIZE, elapsed);
}

#[tokio::test]
async fn test_load_search_performance() {
    let (state, consensus, keypair) = create_test_node();
    let owner = keypair.address();
    state.create_test_account(owner, 10_000_000, 0);
    
    const ASSET_COUNT: usize = 200;
    
    // Create assets
    for i in 0..ASSET_COUNT {
        let asset_id = haze::types::sha256(&format!("search_asset_{}", i).as_bytes());
        let mut meta = std::collections::HashMap::new();
        meta.insert("index".to_string(), i.to_string());
        
        let data = AssetData {
            density: DensityLevel::Ethereal,
            metadata: meta,
            attributes: vec![],
            game_id: Some("load_test_game".to_string()),
            owner,
        };
        let signature = sign_mistborn_asset_tx(&keypair, &AssetAction::Create, &asset_id, &data);
        
        let tx = Transaction::MistbornAsset {
            action: AssetAction::Create,
            asset_id,
            data,
            signature,
        };
        
        consensus.add_transaction(tx).unwrap();
    }
    
    // Process all
    while consensus.tx_pool_size() > 0 {
        let block = consensus.create_block(owner).unwrap();
        consensus.process_block(&block).unwrap();
    }
    
    // Test search performance
    let start = Instant::now();
    let by_owner = state.search_assets_by_owner(&owner);
    let elapsed_owner = start.elapsed();
    
    let start = Instant::now();
    let by_game = state.search_assets_by_game_id("load_test_game");
    let elapsed_game = start.elapsed();
    
    assert_eq!(by_owner.len(), ASSET_COUNT);
    assert_eq!(by_game.len(), ASSET_COUNT);
    
    println!("Search by owner: {} assets in {:?}", by_owner.len(), elapsed_owner);
    println!("Search by game_id: {} assets in {:?}", by_game.len(), elapsed_game);
}
