//! REST API server for HAZE blockchain
//! 
//! Provides HTTP endpoints for:
//! - Transactions (send, get status)
//! - Blocks (get by hash, height)
//! - Accounts (balance, nonce, state)
//! - Mistborn NFT operations
//! - Fog Economy operations
//! - WebSocket for real-time updates

use std::sync::Arc;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use crate::config::Config;
use crate::consensus::ConsensusEngine;
use crate::state::StateManager;
use crate::types::{Transaction, AssetAction, AssetData, DensityLevel, Attribute};
use crate::types::sha256;

// Use std::result::Result for API handlers to avoid conflict with crate::error::Result
type ApiResult<T> = std::result::Result<T, StatusCode>;

/// API state shared across handlers
#[derive(Clone)]
pub struct ApiState {
    pub consensus: Arc<ConsensusEngine>,
    pub state: Arc<StateManager>,
    pub config: Config,
}

/// API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    
    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

/// Transaction request
#[derive(Debug, Deserialize)]
pub struct SendTransactionRequest {
    pub transaction: Transaction,
}

/// Transaction response
#[derive(Debug, Serialize)]
pub struct TransactionResponse {
    pub hash: String,
    pub status: String,
}

/// Account info response
#[derive(Debug, Serialize)]
pub struct AccountInfo {
    pub address: String,
    pub balance: u64,
    pub nonce: u64,
    pub staked: u64,
}

/// Block info response
#[derive(Debug, Serialize)]
pub struct BlockInfo {
    pub hash: String,
    pub parent_hash: String,
    pub height: u64,
    pub timestamp: i64,
    pub validator: String,
    pub transaction_count: usize,
    pub wave_number: u64,
}

/// Blockchain info response
#[derive(Debug, Serialize)]
pub struct BlockchainInfo {
    pub current_height: u64,
    pub total_supply: u64,
    pub current_wave: u64,
}

/// Create API router
pub fn create_router(state: ApiState) -> Router {
    let enable_cors = state.config.api.enable_cors;
    
    let router = Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/blockchain/info", get(get_blockchain_info))
        .route("/api/v1/transactions", post(send_transaction))
        .route("/api/v1/transactions/:hash", get(get_transaction))
        .route("/api/v1/blocks/:hash", get(get_block_by_hash))
        .route("/api/v1/blocks/height/:height", get(get_block_by_height))
        .route("/api/v1/accounts/:address", get(get_account))
        .route("/api/v1/accounts/:address/balance", get(get_balance))
        .route("/api/v1/assets/:asset_id", get(get_asset))
        .route("/api/v1/assets", post(create_asset))
        .route("/api/v1/assets/search", get(search_assets))
        .route("/api/v1/assets/:asset_id/condense", post(condense_asset))
        .route("/api/v1/assets/:asset_id/evaporate", post(evaporate_asset))
        .route("/api/v1/assets/:asset_id/merge", post(merge_assets))
        .route("/api/v1/assets/:asset_id/split", post(split_asset))
        .route("/api/v1/economy/pools", get(get_liquidity_pools))
        .route("/api/v1/economy/pools", post(create_liquidity_pool))
        .route("/api/v1/economy/pools/:pool_id", get(get_liquidity_pool))
        .with_state(state);
    
    // Add CORS if enabled
    if enable_cors {
        router.layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any)
        )
    } else {
        router
    }
}

/// Health check endpoint
async fn health_check() -> Json<ApiResponse<&'static str>> {
    Json(ApiResponse::success("OK"))
}

/// Get blockchain info
async fn get_blockchain_info(
    State(api_state): State<ApiState>,
) -> ApiResult<Json<ApiResponse<BlockchainInfo>>> {
    let height = api_state.state.current_height();
    let total_supply = api_state.state.tokenomics().total_supply();
    
    // Get current wave from consensus
    let current_wave = api_state.consensus.get_current_wave();
    
    let info = BlockchainInfo {
        current_height: height,
        total_supply,
        current_wave,
    };
    
    Ok(Json(ApiResponse::success(info)))
}

/// Send transaction
async fn send_transaction(
    State(api_state): State<ApiState>,
    Json(request): Json<SendTransactionRequest>,
) -> ApiResult<Json<ApiResponse<TransactionResponse>>> {
    let tx = request.transaction;
    let tx_hash = tx.hash();
    
    match api_state.consensus.add_transaction(tx) {
        Ok(()) => {
            let response = TransactionResponse {
                hash: hex::encode(tx_hash),
                status: "pending".to_string(),
            };
            Ok(Json(ApiResponse::success(response)))
        }
        Err(_e) => {
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

/// Get transaction by hash
async fn get_transaction(
    State(api_state): State<ApiState>,
    Path(hash_str): Path<String>,
) -> ApiResult<Json<ApiResponse<TransactionResponse>>> {
    let hash_bytes = hex::decode(&hash_str)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    if hash_bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&hash_bytes);
    
    // Try to get from transaction pool
    if api_state.consensus.get_transaction(&hash).is_some() {
        let response = TransactionResponse {
            hash: hex::encode(hash),
            status: "pending".to_string(),
        };
        return Ok(Json(ApiResponse::success(response)));
    }
    
    // TODO: Also check in blocks (executed transactions)
    Err(StatusCode::NOT_FOUND)
}

/// Get block by hash
async fn get_block_by_hash(
    State(api_state): State<ApiState>,
    Path(hash_str): Path<String>,
) -> ApiResult<Json<ApiResponse<BlockInfo>>> {
    let hash_bytes = hex::decode(&hash_str)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    if hash_bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&hash_bytes);
    
    if let Some(block) = api_state.state.get_block(&hash) {
        let info = BlockInfo {
            hash: hex::encode(block.header.hash),
            parent_hash: hex::encode(block.header.parent_hash),
            height: block.header.height,
            timestamp: block.header.timestamp,
            validator: hex::encode(block.header.validator),
            transaction_count: block.transactions.len(),
            wave_number: block.header.wave_number,
        };
        Ok(Json(ApiResponse::success(info)))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Get block by height
async fn get_block_by_height(
    State(api_state): State<ApiState>,
    Path(height): Path<u64>,
) -> ApiResult<Json<ApiResponse<BlockInfo>>> {
    if let Some(block) = api_state.state.get_block_by_height(height) {
        let info = BlockInfo {
            hash: hex::encode(block.header.hash),
            parent_hash: hex::encode(block.header.parent_hash),
            height: block.header.height,
            timestamp: block.header.timestamp,
            validator: hex::encode(block.header.validator),
            transaction_count: block.transactions.len(),
            wave_number: block.header.wave_number,
        };
        Ok(Json(ApiResponse::success(info)))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Get account info
async fn get_account(
    State(api_state): State<ApiState>,
    Path(address_str): Path<String>,
) -> ApiResult<Json<ApiResponse<AccountInfo>>> {
    let address_bytes = hex::decode(&address_str)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    if address_bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let mut address = [0u8; 32];
    address.copy_from_slice(&address_bytes);
    
    if let Some(account) = api_state.state.get_account(&address) {
        let info = AccountInfo {
            address: hex::encode(address),
            balance: account.balance,
            nonce: account.nonce,
            staked: account.staked,
        };
        Ok(Json(ApiResponse::success(info)))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Get account balance
async fn get_balance(
    State(api_state): State<ApiState>,
    Path(address_str): Path<String>,
) -> ApiResult<Json<ApiResponse<u64>>> {
    let address_bytes = hex::decode(&address_str)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    if address_bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let mut address = [0u8; 32];
    address.copy_from_slice(&address_bytes);
    
    if let Some(account) = api_state.state.get_account(&address) {
        Ok(Json(ApiResponse::success(account.balance)))
    } else {
        Ok(Json(ApiResponse::success(0))) // New account has 0 balance
    }
}

/// Get asset info
async fn get_asset(
    State(api_state): State<ApiState>,
    Path(asset_id_str): Path<String>,
) -> ApiResult<Json<ApiResponse<serde_json::Value>>> {
    let asset_id_bytes = hex::decode(&asset_id_str)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    if asset_id_bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let mut asset_id = [0u8; 32];
    asset_id.copy_from_slice(&asset_id_bytes);
    
    if let Some(asset_state) = api_state.state.get_asset(&asset_id) {
        let asset_json = serde_json::json!({
            "asset_id": hex::encode(asset_id),
            "owner": hex::encode(asset_state.owner),
            "density": format!("{:?}", asset_state.data.density),
            "metadata": asset_state.data.metadata,
            "attributes": asset_state.data.attributes,
            "game_id": asset_state.data.game_id,
            "created_at": asset_state.created_at,
            "updated_at": asset_state.updated_at,
        });
        Ok(Json(ApiResponse::success(asset_json)))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Create asset request
#[derive(Debug, Deserialize)]
pub struct CreateAssetRequest {
    pub owner: String,
    pub density: String,
    pub metadata: std::collections::HashMap<String, String>,
    pub attributes: Option<Vec<AttributeRequest>>,
    pub game_id: Option<String>,
    pub asset_id_seed: Option<String>, // Optional seed for deterministic asset ID
}

#[derive(Debug, Deserialize)]
pub struct AttributeRequest {
    pub name: String,
    pub value: String,
    pub rarity: Option<f64>,
}

/// Create asset
async fn create_asset(
    State(api_state): State<ApiState>,
    Json(request): Json<CreateAssetRequest>,
) -> ApiResult<Json<ApiResponse<serde_json::Value>>> {
    // Parse owner address
    let address_bytes = hex::decode(&request.owner)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    if address_bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let mut owner = [0u8; 32];
    owner.copy_from_slice(&address_bytes);
    
    // Parse density level
    let density = match request.density.as_str() {
        "Ethereal" => DensityLevel::Ethereal,
        "Light" => DensityLevel::Light,
        "Dense" => DensityLevel::Dense,
        "Core" => DensityLevel::Core,
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    
    // Generate asset ID
    let asset_id = if let Some(seed) = &request.asset_id_seed {
        sha256(seed.as_bytes())
    } else {
        // Generate from owner + metadata + timestamp
        let mut seed_data = Vec::new();
        seed_data.extend_from_slice(&owner);
        seed_data.extend_from_slice(&serde_json::to_vec(&request.metadata).unwrap_or_default());
        seed_data.extend_from_slice(&chrono::Utc::now().timestamp().to_le_bytes());
        sha256(&seed_data)
    };
    
    // Check if asset already exists
    if api_state.state.get_asset(&asset_id).is_some() {
        return Err(StatusCode::CONFLICT);
    }
    
    // Convert attributes
    let attributes: Vec<Attribute> = request.attributes
        .unwrap_or_default()
        .into_iter()
        .map(|a| Attribute {
            name: a.name,
            value: a.value,
            rarity: a.rarity,
        })
        .collect();
    
    // Create asset data
    let asset_data = AssetData {
        density,
        metadata: request.metadata,
        attributes,
        game_id: request.game_id,
        owner,
    };
    
    // Create transaction
    let transaction = Transaction::MistbornAsset {
        action: AssetAction::Create,
        asset_id,
        data: asset_data,
        signature: vec![], // Will be signed by client
    };
    
    // Add transaction to pool
    match api_state.consensus.add_transaction(transaction) {
        Ok(()) => {
            let response = serde_json::json!({
                "asset_id": hex::encode(asset_id),
                "status": "pending",
                "message": "Asset creation transaction submitted"
            });
            Ok(Json(ApiResponse::success(response)))
        }
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

/// Condense asset request
#[derive(Debug, Deserialize)]
pub struct CondenseAssetRequest {
    pub owner: String,
    pub new_density: String,
    pub additional_metadata: Option<std::collections::HashMap<String, String>>,
    pub additional_attributes: Option<Vec<AttributeRequest>>,
}

/// Evaporate asset request
#[derive(Debug, Deserialize)]
pub struct EvaporateAssetRequest {
    pub owner: String,
    pub new_density: String,
}

/// Merge assets request
#[derive(Debug, Deserialize)]
pub struct MergeAssetsRequest {
    pub owner: String,
    pub other_asset_id: String,
}

/// Split asset request
#[derive(Debug, Deserialize)]
pub struct SplitAssetRequest {
    pub owner: String,
    pub components: Vec<String>,
}

/// Search assets query parameters
#[derive(Debug, Deserialize)]
pub struct SearchAssetsQuery {
    pub owner: Option<String>,
    pub game_id: Option<String>,
    pub density: Option<String>,
    pub limit: Option<usize>,
}

/// Condense asset (increase density)
async fn condense_asset(
    State(api_state): State<ApiState>,
    Path(asset_id_str): Path<String>,
    Json(request): Json<CondenseAssetRequest>,
) -> ApiResult<Json<ApiResponse<serde_json::Value>>> {
    let asset_id_bytes = hex::decode(&asset_id_str)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    if asset_id_bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let mut asset_id = [0u8; 32];
    asset_id.copy_from_slice(&asset_id_bytes);
    
    // Get current asset
    let asset_state = api_state.state.get_asset(&asset_id)
        .ok_or(StatusCode::NOT_FOUND)?;
    
    // Verify ownership
    let owner_bytes = hex::decode(&request.owner)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    if owner_bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut owner = [0u8; 32];
    owner.copy_from_slice(&owner_bytes);
    
    if asset_state.owner != owner {
        return Err(StatusCode::FORBIDDEN);
    }
    
    // Parse new density
    let new_density = match request.new_density.as_str() {
        "Ethereal" => DensityLevel::Ethereal,
        "Light" => DensityLevel::Light,
        "Dense" => DensityLevel::Dense,
        "Core" => DensityLevel::Core,
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    
    // Check density increase
    if new_density as u8 <= asset_state.data.density as u8 {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    // Prepare asset data
    let mut metadata = asset_state.data.metadata.clone();
    if let Some(additional) = request.additional_metadata {
        metadata.extend(additional);
    }
    
    let mut attributes = asset_state.data.attributes.clone();
    if let Some(additional) = request.additional_attributes {
        attributes.extend(additional.into_iter().map(|a| Attribute {
            name: a.name,
            value: a.value,
            rarity: a.rarity,
        }));
    }
    
    let asset_data = AssetData {
        density: new_density,
        metadata,
        attributes,
        game_id: asset_state.data.game_id.clone(),
        owner,
    };
    
    let transaction = Transaction::MistbornAsset {
        action: AssetAction::Condense,
        asset_id,
        data: asset_data,
        signature: vec![],
    };
    
    match api_state.consensus.add_transaction(transaction) {
        Ok(()) => {
            let response = serde_json::json!({
                "asset_id": hex::encode(asset_id),
                "status": "pending",
                "message": "Condense transaction submitted"
            });
            Ok(Json(ApiResponse::success(response)))
        }
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

/// Evaporate asset (decrease density)
async fn evaporate_asset(
    State(api_state): State<ApiState>,
    Path(asset_id_str): Path<String>,
    Json(request): Json<EvaporateAssetRequest>,
) -> ApiResult<Json<ApiResponse<serde_json::Value>>> {
    let asset_id_bytes = hex::decode(&asset_id_str)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    if asset_id_bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let mut asset_id = [0u8; 32];
    asset_id.copy_from_slice(&asset_id_bytes);
    
    let asset_state = api_state.state.get_asset(&asset_id)
        .ok_or(StatusCode::NOT_FOUND)?;
    
    let owner_bytes = hex::decode(&request.owner)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    if owner_bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut owner = [0u8; 32];
    owner.copy_from_slice(&owner_bytes);
    
    if asset_state.owner != owner {
        return Err(StatusCode::FORBIDDEN);
    }
    
    let new_density = match request.new_density.as_str() {
        "Ethereal" => DensityLevel::Ethereal,
        "Light" => DensityLevel::Light,
        "Dense" => DensityLevel::Dense,
        "Core" => DensityLevel::Core,
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    
    if new_density as u8 >= asset_state.data.density as u8 {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let asset_data = AssetData {
        density: new_density,
        metadata: asset_state.data.metadata.clone(),
        attributes: asset_state.data.attributes.clone(),
        game_id: asset_state.data.game_id.clone(),
        owner,
    };
    
    let transaction = Transaction::MistbornAsset {
        action: AssetAction::Evaporate,
        asset_id,
        data: asset_data,
        signature: vec![],
    };
    
    match api_state.consensus.add_transaction(transaction) {
        Ok(()) => {
            let response = serde_json::json!({
                "asset_id": hex::encode(asset_id),
                "status": "pending",
                "message": "Evaporate transaction submitted"
            });
            Ok(Json(ApiResponse::success(response)))
        }
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

/// Merge two assets
async fn merge_assets(
    State(api_state): State<ApiState>,
    Path(asset_id_str): Path<String>,
    Json(request): Json<MergeAssetsRequest>,
) -> ApiResult<Json<ApiResponse<serde_json::Value>>> {
    let asset_id_bytes = hex::decode(&asset_id_str)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let other_asset_id_bytes = hex::decode(&request.other_asset_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    if asset_id_bytes.len() != 32 || other_asset_id_bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let mut asset_id = [0u8; 32];
    asset_id.copy_from_slice(&asset_id_bytes);
    let mut other_asset_id = [0u8; 32];
    other_asset_id.copy_from_slice(&other_asset_id_bytes);
    
    let asset_state = api_state.state.get_asset(&asset_id)
        .ok_or(StatusCode::NOT_FOUND)?;
    let other_asset_state = api_state.state.get_asset(&other_asset_id)
        .ok_or(StatusCode::NOT_FOUND)?;
    
    let owner_bytes = hex::decode(&request.owner)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    if owner_bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut owner = [0u8; 32];
    owner.copy_from_slice(&owner_bytes);
    
    if asset_state.owner != owner || other_asset_state.owner != owner {
        return Err(StatusCode::FORBIDDEN);
    }
    
    // Merge metadata and attributes
    let mut metadata = asset_state.data.metadata.clone();
    for (key, value) in &other_asset_state.data.metadata {
        if !metadata.contains_key(key) {
            metadata.insert(key.clone(), value.clone());
        }
    }
    
    let mut attributes = asset_state.data.attributes.clone();
    attributes.extend(other_asset_state.data.attributes.clone());
    
    // Use higher density
    let density = if other_asset_state.data.density as u8 > asset_state.data.density as u8 {
        other_asset_state.data.density
    } else {
        asset_state.data.density
    };
    
    let asset_data = AssetData {
        density,
        metadata,
        attributes,
        game_id: asset_state.data.game_id.clone(),
        owner,
    };
    
    let transaction = Transaction::MistbornAsset {
        action: AssetAction::Merge,
        asset_id,
        data: asset_data,
        signature: vec![],
    };
    
    match api_state.consensus.add_transaction(transaction) {
        Ok(()) => {
            let response = serde_json::json!({
                "asset_id": hex::encode(asset_id),
                "merged_asset_id": hex::encode(other_asset_id),
                "status": "pending",
                "message": "Merge transaction submitted"
            });
            Ok(Json(ApiResponse::success(response)))
        }
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

/// Split asset into components
async fn split_asset(
    State(api_state): State<ApiState>,
    Path(asset_id_str): Path<String>,
    Json(request): Json<SplitAssetRequest>,
) -> ApiResult<Json<ApiResponse<serde_json::Value>>> {
    let asset_id_bytes = hex::decode(&asset_id_str)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    if asset_id_bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let mut asset_id = [0u8; 32];
    asset_id.copy_from_slice(&asset_id_bytes);
    
    let asset_state = api_state.state.get_asset(&asset_id)
        .ok_or(StatusCode::NOT_FOUND)?;
    
    let owner_bytes = hex::decode(&request.owner)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    if owner_bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut owner = [0u8; 32];
    owner.copy_from_slice(&owner_bytes);
    
    if asset_state.owner != owner {
        return Err(StatusCode::FORBIDDEN);
    }
    
    // Create split transactions for each component
    let mut created_assets = Vec::new();
    for component_name in &request.components {
        let component_asset_id = sha256(&[
            &asset_id,
            component_name.as_bytes(),
        ].concat());
        
        let mut component_metadata = std::collections::HashMap::new();
        if let Some(value) = asset_state.data.metadata.get(component_name) {
            component_metadata.insert(component_name.clone(), value.clone());
        }
        
        let asset_data = AssetData {
            density: DensityLevel::Ethereal,
            metadata: component_metadata,
            attributes: vec![],
            game_id: asset_state.data.game_id.clone(),
            owner,
        };
        
        let transaction = Transaction::MistbornAsset {
            action: AssetAction::Split,
            asset_id: component_asset_id,
            data: asset_data,
            signature: vec![],
        };
        
        if api_state.consensus.add_transaction(transaction).is_ok() {
            created_assets.push(hex::encode(component_asset_id));
        }
    }
    
    let response = serde_json::json!({
        "original_asset_id": hex::encode(asset_id),
        "created_assets": created_assets,
        "status": "pending",
        "message": "Split transactions submitted"
    });
    Ok(Json(ApiResponse::success(response)))
}

/// Search assets
async fn search_assets(
    State(api_state): State<ApiState>,
    axum::extract::Query(query): axum::extract::Query<SearchAssetsQuery>,
) -> ApiResult<Json<ApiResponse<Vec<serde_json::Value>>>> {
    let limit = query.limit.unwrap_or(100).min(1000);
    let mut results = Vec::new();
    
    for entry in api_state.state.assets().iter() {
        let asset_state = entry.value();
        
        // Filter by owner
        if let Some(ref owner_filter) = query.owner {
            let owner_bytes = hex::decode(owner_filter).ok();
            if let Some(owner_bytes) = owner_bytes {
                if owner_bytes.len() == 32 {
                    let mut owner = [0u8; 32];
                    owner.copy_from_slice(&owner_bytes);
                    if asset_state.owner != owner {
                        continue;
                    }
                }
            }
        }
        
        // Filter by game_id
        if let Some(ref game_id_filter) = query.game_id {
            if asset_state.data.game_id.as_ref() != Some(game_id_filter) {
                continue;
            }
        }
        
        // Filter by density
        if let Some(ref density_filter) = query.density {
            let density_str = format!("{:?}", asset_state.data.density);
            if density_str != *density_filter {
                continue;
            }
        }
        
        let asset_json = serde_json::json!({
            "asset_id": hex::encode(*entry.key()),
            "owner": hex::encode(asset_state.owner),
            "density": format!("{:?}", asset_state.data.density),
            "metadata": asset_state.data.metadata,
            "attributes": asset_state.data.attributes,
            "game_id": asset_state.data.game_id,
            "created_at": asset_state.created_at,
            "updated_at": asset_state.updated_at,
        });
        
        results.push(asset_json);
        
        if results.len() >= limit {
            break;
        }
    }
    
    Ok(Json(ApiResponse::success(results)))
}

/// Create liquidity pool request
#[derive(Debug, Deserialize)]
pub struct CreatePoolRequest {
    pub asset1: String,
    pub asset2: String,
    pub reserve1: u64,
    pub reserve2: u64,
    pub fee_rate: u64,
}

/// Get liquidity pools
async fn get_liquidity_pools(
    State(api_state): State<ApiState>,
) -> ApiResult<Json<ApiResponse<Vec<serde_json::Value>>>> {
    let economy = api_state.state.economy();
    let pools: Vec<serde_json::Value> = economy.liquidity_pools()
        .iter()
        .map(|entry| {
            let pool = entry.value();
            serde_json::json!({
                "pool_id": pool.pool_id,
                "asset1": pool.asset1,
                "asset2": pool.asset2,
                "reserve1": pool.reserve1,
                "reserve2": pool.reserve2,
                "fee_rate": pool.fee_rate,
                "total_liquidity": pool.total_liquidity,
            })
        })
        .collect();
    
    Ok(Json(ApiResponse::success(pools)))
}

/// Create liquidity pool
async fn create_liquidity_pool(
    State(api_state): State<ApiState>,
    Json(request): Json<CreatePoolRequest>,
) -> ApiResult<Json<ApiResponse<serde_json::Value>>> {
    let economy = api_state.state.economy();
    
    match economy.create_liquidity_pool(
        request.asset1,
        request.asset2,
        request.reserve1,
        request.reserve2,
        request.fee_rate,
    ) {
        Ok(pool_id) => {
            let response = serde_json::json!({
                "pool_id": pool_id,
                "status": "created",
            });
            Ok(Json(ApiResponse::success(response)))
        }
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

/// Get liquidity pool by ID
async fn get_liquidity_pool(
    State(api_state): State<ApiState>,
    Path(pool_id): Path<String>,
) -> ApiResult<Json<ApiResponse<serde_json::Value>>> {
    let economy = api_state.state.economy();
    
    if let Some(pool) = economy.get_liquidity_pool(&pool_id) {
        let pool_json = serde_json::json!({
            "pool_id": pool.pool_id,
            "asset1": pool.asset1,
            "asset2": pool.asset2,
            "reserve1": pool.reserve1,
            "reserve2": pool.reserve2,
            "fee_rate": pool.fee_rate,
            "total_liquidity": pool.total_liquidity,
        });
        Ok(Json(ApiResponse::success(pool_json)))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Start API server
pub async fn start_api_server(state: ApiState) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let app = create_router(state.clone());
    
    let listener = tokio::net::TcpListener::bind(&state.config.api.listen_addr).await?;
    tracing::info!("API server listening on http://{}", state.config.api.listen_addr);
    tracing::info!("Health check: http://{}/health", state.config.api.listen_addr);
    tracing::info!("API docs: http://{}/api/v1/blockchain/info", state.config.api.listen_addr);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::state::StateManager;
    use crate::consensus::ConsensusEngine;
    use std::sync::Arc;
    
    fn create_test_api_state() -> ApiState {
        let mut config = Config::default();
        // Use unique database path for tests
        config.storage.db_path = std::path::PathBuf::from("./haze_db_test_api");
        let state = Arc::new(StateManager::new(&config).unwrap());
        let consensus = Arc::new(ConsensusEngine::new(config.clone(), state.clone()).unwrap());
        
        ApiState {
            consensus,
            state,
            config,
        }
    }
    
    #[test]
    fn test_router_creation() {
        let state = create_test_api_state();
        let _app = create_router(state);
        // Test that router can be created without errors
    }
    
    #[test]
    fn test_api_response_success() {
        let response = ApiResponse::success("test");
        assert!(response.success);
        assert_eq!(response.data, Some("test"));
        assert_eq!(response.error, None);
    }
    
    #[test]
    fn test_api_response_error() {
        let response = ApiResponse::<()>::error("test error".to_string());
        assert!(!response.success);
        assert_eq!(response.data, None);
        assert_eq!(response.error, Some("test error".to_string()));
    }
}
