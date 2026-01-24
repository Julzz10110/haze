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
    extract::{Path, State, ws::WebSocketUpgrade},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use axum::extract::ws::Message;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use crate::config::Config;
use crate::consensus::ConsensusEngine;
use crate::state::StateManager;
use crate::types::{Transaction, AssetAction, Hash, AssetPermission, PermissionLevel, Address};
use crate::state::AssetState;
pub use crate::ws_events::WsEvent;

// Use std::result::Result for API handlers to avoid conflict with crate::error::Result
type ApiResult<T> = std::result::Result<T, StatusCode>;

/// WebSocket subscription request
#[derive(Debug, Deserialize)]
pub struct WsSubscribeRequest {
    pub subscribe: Vec<WsSubscription>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WsSubscription {
    #[serde(rename = "type")]
    pub sub_type: String,
    pub asset_id: Option<String>,
    pub owner: Option<String>,
    pub game_id: Option<String>,
}

/// API state shared across handlers
#[derive(Clone)]
pub struct ApiState {
    pub consensus: Arc<ConsensusEngine>,
    pub state: Arc<StateManager>,
    pub config: Config,
    pub ws_tx: broadcast::Sender<WsEvent>,
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
#[derive(Debug, Serialize, Clone)]
pub struct BlockchainInfo {
    pub current_height: u64,
    pub total_supply: u64,
    pub current_wave: u64,
    pub state_root: String, // Hex-encoded state root hash
    pub last_finalized_height: u64,
    pub last_finalized_wave: u64,
}

/// Create API router
pub fn create_router(state: ApiState) -> Router {
    let enable_cors = state.config.api.enable_cors;
    
    let router = Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/blockchain/info", get(get_blockchain_info))
        .route("/api/v1/metrics/basic", get(get_basic_metrics))
        .route("/api/v1/transactions", post(send_transaction))
        .route("/api/v1/transactions/:hash", get(get_transaction))
        .route("/api/v1/blocks/:hash", get(get_block_by_hash))
        .route("/api/v1/blocks/height/:height", get(get_block_by_height))
        .route("/api/v1/accounts/:address", get(get_account))
        .route("/api/v1/accounts/:address/balance", get(get_balance))
        .route("/api/v1/assets/:asset_id", get(get_asset))
        .route("/api/v1/assets/:asset_id/history", get(get_asset_history))
        .route("/api/v1/assets/:asset_id/versions", get(get_asset_versions))
        .route("/api/v1/assets/:asset_id/versions/:version", get(get_asset_version))
        .route("/api/v1/assets/:asset_id/snapshot", post(create_asset_snapshot))
        .route("/api/v1/assets", post(create_asset))
        .route("/api/v1/assets/search", get(search_assets))
        .route("/api/v1/assets/:asset_id/condense", post(condense_asset))
        .route("/api/v1/assets/:asset_id/evaporate", post(evaporate_asset))
        .route("/api/v1/assets/:asset_id/merge", post(merge_assets))
        .route("/api/v1/assets/:asset_id/split", post(split_asset))
        .route("/api/v1/assets/estimate-gas", post(estimate_asset_gas))
        .route("/api/v1/assets/:asset_id/permissions", get(get_asset_permissions))
        .route("/api/v1/assets/:asset_id/permissions", post(set_asset_permissions))
        .route("/api/v1/assets/:asset_id/export", get(export_asset))
        .route("/api/v1/assets/import", post(import_asset))
        .route("/api/v1/economy/pools", get(get_liquidity_pools))
        .route("/api/v1/economy/pools", post(create_liquidity_pool))
        .route("/api/v1/economy/pools/:pool_id", get(get_liquidity_pool))
        .route("/api/v1/ws", get(ws_handler))
        .route("/api/v1/sync/start", post(start_sync))
        .route("/api/v1/sync/status", get(get_sync_status))
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
    
    // Compute state root
    let state_root = api_state.state.compute_state_root();
    
    // Get finalized checkpoint info
    let last_finalized_height = api_state.consensus.get_last_finalized_height();
    let last_finalized_wave = api_state.consensus.get_last_finalized_wave();
    
    let info = BlockchainInfo {
        current_height: height,
        total_supply,
        current_wave,
        state_root: hex::encode(state_root),
        last_finalized_height,
        last_finalized_wave,
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
    
    match api_state.consensus.add_transaction(tx.clone()) {
        Ok(()) => {
            // Broadcast transaction to network (async, don't wait)
            // For now, transactions will be broadcast when blocks are created
            // In future, we can add direct transaction broadcasting here
            tracing::debug!("Transaction added to pool, will be broadcast with next block");
            
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
    
    // Check in executed blocks
    // Iterate through blocks to find the transaction
    // Note: In production, this should use an index for better performance
    for entry in api_state.state.blocks().iter() {
        let block = entry.value();
        for tx in &block.transactions {
            if tx.hash() == hash {
                let response = TransactionResponse {
                    hash: hex::encode(hash),
                    status: "executed".to_string(),
                };
                return Ok(Json(ApiResponse::success(response)));
            }
        }
    }
    
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
        // Convert blob_refs to hex strings for JSON
        let blob_refs_json: std::collections::HashMap<String, String> = asset_state.blob_refs.iter()
            .map(|(k, v)| (k.clone(), hex::encode(v)))
            .collect();
        
        let permissions_json: Vec<serde_json::Value> = asset_state.permissions.iter().map(|p| {
            serde_json::json!({
                "grantee": hex::encode(p.grantee),
                "level": format!("{:?}", p.level),
                "game_id": p.game_id,
                "expires_at": p.expires_at,
            })
        }).collect();
        let asset_json = serde_json::json!({
            "asset_id": hex::encode(asset_id),
            "owner": hex::encode(asset_state.owner),
            "density": format!("{:?}", asset_state.data.density),
            "metadata": asset_state.data.metadata,
            "attributes": asset_state.data.attributes,
            "game_id": asset_state.data.game_id,
            "created_at": asset_state.created_at,
            "updated_at": asset_state.updated_at,
            "blob_refs": blob_refs_json,
            "history_count": asset_state.history.len(),
            "permissions": permissions_json,
            "public_read": asset_state.public_read,
        });
        Ok(Json(ApiResponse::success(asset_json)))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Get asset history query parameters
#[derive(Debug, Deserialize)]
pub struct AssetHistoryQuery {
    pub limit: Option<usize>,
}

/// Create asset snapshot
async fn create_asset_snapshot(
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
    
    match api_state.state.create_asset_snapshot(&asset_id) {
        Ok(version) => {
            let response = serde_json::json!({
                "asset_id": hex::encode(asset_id),
                "version": version,
                "status": "created",
            });
            Ok(Json(ApiResponse::success(response)))
        }
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Get asset versions
async fn get_asset_versions(
    State(api_state): State<ApiState>,
    Path(asset_id_str): Path<String>,
) -> ApiResult<Json<ApiResponse<Vec<serde_json::Value>>>> {
    let asset_id_bytes = hex::decode(&asset_id_str)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    if asset_id_bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let mut asset_id = [0u8; 32];
    asset_id.copy_from_slice(&asset_id_bytes);
    
    if let Some(versions) = api_state.state.get_asset_versions(&asset_id) {
        let versions_json: Vec<serde_json::Value> = versions.iter()
            .map(|v| {
                let blob_refs_json: std::collections::HashMap<String, String> = v.blob_refs.iter()
                    .map(|(k, h)| (k.clone(), hex::encode(h)))
                    .collect();
                
                serde_json::json!({
                    "version": v.version,
                    "timestamp": v.timestamp,
                    "density": format!("{:?}", v.data.density),
                    "metadata": v.data.metadata,
                    "attributes": v.data.attributes,
                    "game_id": v.data.game_id,
                    "blob_refs": blob_refs_json,
                })
            })
            .collect();
        Ok(Json(ApiResponse::success(versions_json)))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Get asset version by version number
async fn get_asset_version(
    State(api_state): State<ApiState>,
    Path((asset_id_str, version_str)): Path<(String, String)>,
) -> ApiResult<Json<ApiResponse<serde_json::Value>>> {
    let asset_id_bytes = hex::decode(&asset_id_str)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    if asset_id_bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let mut asset_id = [0u8; 32];
    asset_id.copy_from_slice(&asset_id_bytes);
    
    let version = version_str.parse::<u64>()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    if let Some(asset_version) = api_state.state.get_asset_version(&asset_id, version) {
        let blob_refs_json: std::collections::HashMap<String, String> = asset_version.blob_refs.iter()
            .map(|(k, h)| (k.clone(), hex::encode(h)))
            .collect();
        
        let version_json = serde_json::json!({
            "asset_id": hex::encode(asset_id),
            "version": asset_version.version,
            "timestamp": asset_version.timestamp,
            "density": format!("{:?}", asset_version.data.density),
            "metadata": asset_version.data.metadata,
            "attributes": asset_version.data.attributes,
            "game_id": asset_version.data.game_id,
            "blob_refs": blob_refs_json,
        });
        Ok(Json(ApiResponse::success(version_json)))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Get asset history
async fn get_asset_history(
    State(api_state): State<ApiState>,
    Path(asset_id_str): Path<String>,
    axum::extract::Query(query): axum::extract::Query<AssetHistoryQuery>,
) -> ApiResult<Json<ApiResponse<Vec<serde_json::Value>>>> {
    let asset_id_bytes = hex::decode(&asset_id_str)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    if asset_id_bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let mut asset_id = [0u8; 32];
    asset_id.copy_from_slice(&asset_id_bytes);
    
    let limit = query.limit.unwrap_or(0); // 0 = all
    
    if let Some(history) = api_state.state.get_asset_history(&asset_id, limit) {
        let history_json: Vec<serde_json::Value> = history.iter()
            .map(|entry| {
                serde_json::json!({
                    "timestamp": entry.timestamp,
                    "action": format!("{:?}", entry.action),
                    "changes": entry.changes,
                })
            })
            .collect();
        Ok(Json(ApiResponse::success(history_json)))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Create asset
///
/// Expects a **signed** `Transaction::MistbornAsset { action: Create, ... }`.
/// The server does not sign transactions on behalf of clients.
async fn create_asset(
    State(api_state): State<ApiState>,
    Json(request): Json<SendTransactionRequest>,
) -> ApiResult<Json<ApiResponse<TransactionResponse>>> {
    let tx = request.transaction;

    // Must be a signed Create tx
    let (action, asset_id, signature) = match &tx {
        Transaction::MistbornAsset { action, asset_id, signature, .. } => (action, asset_id, signature),
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    if !matches!(action, AssetAction::Create) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if signature.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    if api_state.state.get_asset(asset_id).is_some() {
        return Err(StatusCode::CONFLICT);
    }

    let tx_hash = tx.hash();
    match api_state.consensus.add_transaction(tx) {
        Ok(()) => Ok(Json(ApiResponse::success(TransactionResponse {
            hash: hex::encode(tx_hash),
            status: "pending".to_string(),
        }))),
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

/// Search assets query parameters
#[derive(Debug, Deserialize)]
pub struct SearchAssetsQuery {
    pub owner: Option<String>,
    pub game_id: Option<String>,
    pub density: Option<String>,
    pub q: Option<String>, // Full-text search query
    pub sort_by: Option<String>, // created_at, updated_at, rarity
    pub sort_order: Option<String>, // asc, desc
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Condense asset (increase density)
///
/// Expects a **signed** `Transaction::MistbornAsset { action: Condense, asset_id: <path>, ... }`.
async fn condense_asset(
    State(api_state): State<ApiState>,
    Path(asset_id_str): Path<String>,
    Json(request): Json<SendTransactionRequest>,
) -> ApiResult<Json<ApiResponse<TransactionResponse>>> {
    let path_asset_id_bytes = hex::decode(&asset_id_str)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    if path_asset_id_bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let mut path_asset_id = [0u8; 32];
    path_asset_id.copy_from_slice(&path_asset_id_bytes);

    let tx = request.transaction;
    let (action, asset_id, signature) = match &tx {
        Transaction::MistbornAsset { action, asset_id, signature, .. } => (action, asset_id, signature),
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    if !matches!(action, AssetAction::Condense) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if *asset_id != path_asset_id {
        return Err(StatusCode::BAD_REQUEST);
    }
    if signature.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    if api_state.state.get_asset(&path_asset_id).is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    let tx_hash = tx.hash();
    match api_state.consensus.add_transaction(tx) {
        Ok(()) => Ok(Json(ApiResponse::success(TransactionResponse {
            hash: hex::encode(tx_hash),
            status: "pending".to_string(),
        }))),
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

/// Evaporate asset (decrease density)
///
/// Expects a **signed** `Transaction::MistbornAsset { action: Evaporate, asset_id: <path>, ... }`.
async fn evaporate_asset(
    State(api_state): State<ApiState>,
    Path(asset_id_str): Path<String>,
    Json(request): Json<SendTransactionRequest>,
) -> ApiResult<Json<ApiResponse<TransactionResponse>>> {
    let path_asset_id_bytes = hex::decode(&asset_id_str)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    if path_asset_id_bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let mut path_asset_id = [0u8; 32];
    path_asset_id.copy_from_slice(&path_asset_id_bytes);

    let tx = request.transaction;
    let (action, asset_id, signature) = match &tx {
        Transaction::MistbornAsset { action, asset_id, signature, .. } => (action, asset_id, signature),
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    if !matches!(action, AssetAction::Evaporate) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if *asset_id != path_asset_id {
        return Err(StatusCode::BAD_REQUEST);
    }
    if signature.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    if api_state.state.get_asset(&path_asset_id).is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    let tx_hash = tx.hash();
    match api_state.consensus.add_transaction(tx) {
        Ok(()) => Ok(Json(ApiResponse::success(TransactionResponse {
            hash: hex::encode(tx_hash),
            status: "pending".to_string(),
        }))),
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

/// Merge two assets
///
/// Expects a **signed** `Transaction::MistbornAsset { action: Merge, asset_id: <path>, data: { metadata: { "_other_asset_id": "<hex>" } }, ... }`.
async fn merge_assets(
    State(api_state): State<ApiState>,
    Path(asset_id_str): Path<String>,
    Json(request): Json<SendTransactionRequest>,
) -> ApiResult<Json<ApiResponse<TransactionResponse>>> {
    let path_asset_id_bytes = hex::decode(&asset_id_str)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    if path_asset_id_bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let mut path_asset_id = [0u8; 32];
    path_asset_id.copy_from_slice(&path_asset_id_bytes);
    
    let tx = request.transaction;
    let (action, asset_id, signature, data) = match &tx {
        Transaction::MistbornAsset { action, asset_id, signature, data } => (action, asset_id, signature, data),
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    
    if !matches!(action, AssetAction::Merge) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if *asset_id != path_asset_id {
        return Err(StatusCode::BAD_REQUEST);
    }
    if signature.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    // Verify _other_asset_id is present in metadata
    let other_asset_id_str = data.metadata.get("_other_asset_id")
        .ok_or_else(|| StatusCode::BAD_REQUEST)?;
    
    let other_asset_id_bytes = hex::decode(other_asset_id_str)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    if other_asset_id_bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let mut other_asset_id = [0u8; 32];
    other_asset_id.copy_from_slice(&other_asset_id_bytes);
    
    // Verify both assets exist
    if api_state.state.get_asset(&path_asset_id).is_none() {
        return Err(StatusCode::NOT_FOUND);
    }
    if api_state.state.get_asset(&other_asset_id).is_none() {
        return Err(StatusCode::NOT_FOUND);
    }
    
    let tx_hash = tx.hash();
    match api_state.consensus.add_transaction(tx) {
        Ok(()) => Ok(Json(ApiResponse::success(TransactionResponse {
            hash: hex::encode(tx_hash),
            status: "pending".to_string(),
        }))),
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

/// Split asset into components
///
/// Expects a **signed** `Transaction::MistbornAsset { action: Split, asset_id: <path>, data: { metadata: { "_components": "component1,component2,..." } }, ... }`.
async fn split_asset(
    State(api_state): State<ApiState>,
    Path(asset_id_str): Path<String>,
    Json(request): Json<SendTransactionRequest>,
) -> ApiResult<Json<ApiResponse<TransactionResponse>>> {
    let path_asset_id_bytes = hex::decode(&asset_id_str)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    if path_asset_id_bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let mut path_asset_id = [0u8; 32];
    path_asset_id.copy_from_slice(&path_asset_id_bytes);
    
    let tx = request.transaction;
    let (action, asset_id, signature, data) = match &tx {
        Transaction::MistbornAsset { action, asset_id, signature, data } => (action, asset_id, signature, data),
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    
    if !matches!(action, AssetAction::Split) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if *asset_id != path_asset_id {
        return Err(StatusCode::BAD_REQUEST);
    }
    if signature.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    // Verify _components is present in metadata
    let components_str = data.metadata.get("_components")
        .ok_or_else(|| StatusCode::BAD_REQUEST)?;
    
    let components: Vec<&str> = components_str.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if components.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    if api_state.state.get_asset(&path_asset_id).is_none() {
        return Err(StatusCode::NOT_FOUND);
    }
    
    let tx_hash = tx.hash();
    match api_state.consensus.add_transaction(tx) {
        Ok(()) => Ok(Json(ApiResponse::success(TransactionResponse {
            hash: hex::encode(tx_hash),
            status: "pending".to_string(),
        }))),
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

/// Estimate gas cost for asset operation
#[derive(Debug, Deserialize, Serialize)]
pub struct EstimateGasRequest {
    pub transaction: Transaction,
}

/// Gas estimate response
#[derive(Debug, Serialize)]
pub struct GasEstimateResponse {
    pub gas_cost: u64,
    pub gas_fee: u64,
    pub gas_price: u64,
}

async fn estimate_asset_gas(
    State(api_state): State<ApiState>,
    Json(request): Json<EstimateGasRequest>,
) -> ApiResult<Json<ApiResponse<GasEstimateResponse>>> {
    let tx = request.transaction;
    
    // Extract asset operation data
    let (action, data) = match &tx {
        Transaction::MistbornAsset { action, data, .. } => (action, data),
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    
    // Calculate gas cost
    let gas_cost = crate::assets::calculate_asset_operation_gas(
        &api_state.config,
        action,
        data,
        Some(&data.metadata),
    );
    
    // Calculate gas fee (gas_cost * gas_price)
    let gas_fee = gas_cost * api_state.config.vm.gas_price;
    
    Ok(Json(ApiResponse::success(GasEstimateResponse {
        gas_cost,
        gas_fee,
        gas_price: api_state.config.vm.gas_price,
    })))
}

/// Get asset permissions
async fn get_asset_permissions(
    State(api_state): State<ApiState>,
    Path(asset_id_str): Path<String>,
) -> ApiResult<Json<ApiResponse<serde_json::Value>>> {
    let asset_id_bytes = hex::decode(&asset_id_str).map_err(|_| StatusCode::BAD_REQUEST)?;
    if asset_id_bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut asset_id = [0u8; 32];
    asset_id.copy_from_slice(&asset_id_bytes);

    if let Some(asset_state) = api_state.state.get_asset(&asset_id) {
        let permissions_json: Vec<serde_json::Value> = asset_state.permissions.iter().map(|p| {
            serde_json::json!({
                "grantee": hex::encode(p.grantee),
                "level": format!("{:?}", p.level),
                "game_id": p.game_id,
                "expires_at": p.expires_at,
            })
        }).collect();
        let response = serde_json::json!({
            "asset_id": hex::encode(asset_id),
            "owner": hex::encode(asset_state.owner),
            "permissions": permissions_json,
            "public_read": asset_state.public_read,
        });
        Ok(Json(ApiResponse::success(response)))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Permission DTO for API (grantee as hex string)
#[derive(Debug, Deserialize)]
pub struct PermissionDto {
    pub grantee: String,
    pub level: String,
    pub game_id: Option<String>,
    pub expires_at: Option<i64>,
}

/// Set asset permissions request
#[derive(Debug, Deserialize)]
pub struct SetPermissionsRequest {
    pub permissions: Vec<PermissionDto>,
    pub public_read: bool,
    /// Owner address (hex string)
    pub owner: String,
    pub signature: Vec<u8>,
}

/// Set asset permissions (owner only)
async fn set_asset_permissions(
    State(api_state): State<ApiState>,
    Path(asset_id_str): Path<String>,
    Json(req): Json<SetPermissionsRequest>,
) -> ApiResult<Json<ApiResponse<TransactionResponse>>> {
    let asset_id_bytes = hex::decode(&asset_id_str).map_err(|_| StatusCode::BAD_REQUEST)?;
    if asset_id_bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut asset_id = [0u8; 32];
    asset_id.copy_from_slice(&asset_id_bytes);

    let owner_bytes = hex::decode(&req.owner).map_err(|_| StatusCode::BAD_REQUEST)?;
    if owner_bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut owner = [0u8; 32];
    owner.copy_from_slice(&owner_bytes);

    if api_state.state.get_asset(&asset_id).is_none() {
        return Err(StatusCode::NOT_FOUND);
    }
    if req.signature.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut permissions = Vec::with_capacity(req.permissions.len());
    for p in req.permissions {
        let grantee_bytes = hex::decode(&p.grantee).map_err(|_| StatusCode::BAD_REQUEST)?;
        if grantee_bytes.len() != 32 {
            return Err(StatusCode::BAD_REQUEST);
        }
        let mut grantee: Address = [0u8; 32];
        grantee.copy_from_slice(&grantee_bytes);
        let level = match p.level.as_str() {
            "GameContract" => PermissionLevel::GameContract,
            "PublicRead" => PermissionLevel::PublicRead,
            _ => return Err(StatusCode::BAD_REQUEST),
        };
        permissions.push(AssetPermission {
            grantee,
            level,
            game_id: p.game_id,
            expires_at: p.expires_at,
        });
    }

    let tx = Transaction::SetAssetPermissions {
        asset_id,
        permissions,
        public_read: req.public_read,
        owner,
        signature: req.signature,
    };
    let tx_hash = tx.hash();
    match api_state.consensus.add_transaction(tx) {
        Ok(()) => Ok(Json(ApiResponse::success(TransactionResponse {
            hash: hex::encode(tx_hash),
            status: "pending".to_string(),
        }))),
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

/// Export asset as JSON (full state: metadata, attributes, blob_refs, history, versions, permissions)
async fn export_asset(
    State(api_state): State<ApiState>,
    Path(asset_id_str): Path<String>,
) -> ApiResult<Json<ApiResponse<serde_json::Value>>> {
    let asset_id_bytes = hex::decode(&asset_id_str).map_err(|_| StatusCode::BAD_REQUEST)?;
    if asset_id_bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut asset_id = [0u8; 32];
    asset_id.copy_from_slice(&asset_id_bytes);

    let asset_state = api_state.state.get_asset(&asset_id).ok_or(StatusCode::NOT_FOUND)?;

    let blob_refs_json: std::collections::HashMap<String, String> = asset_state
        .blob_refs
        .iter()
        .map(|(k, v)| (k.clone(), hex::encode(v)))
        .collect();
    let permissions_json: Vec<serde_json::Value> = asset_state
        .permissions
        .iter()
        .map(|p| {
            serde_json::json!({
                "grantee": hex::encode(p.grantee),
                "level": format!("{:?}", p.level),
                "game_id": p.game_id,
                "expires_at": p.expires_at,
            })
        })
        .collect();
    let history_json: Vec<serde_json::Value> = asset_state
        .history
        .iter()
        .map(|e| {
            serde_json::json!({
                "timestamp": e.timestamp,
                "action": format!("{:?}", e.action),
                "changes": e.changes,
            })
        })
        .collect();
    let versions_json: Vec<serde_json::Value> = asset_state
        .versions
        .iter()
        .map(|v| {
            serde_json::json!({
                "version": v.version,
                "timestamp": v.timestamp,
            })
        })
        .collect();

    let export_json = serde_json::json!({
        "asset_id": hex::encode(asset_id),
        "owner": hex::encode(asset_state.owner),
        "density": format!("{:?}", asset_state.data.density),
        "metadata": asset_state.data.metadata,
        "attributes": asset_state.data.attributes,
        "game_id": asset_state.data.game_id,
        "created_at": asset_state.created_at,
        "updated_at": asset_state.updated_at,
        "blob_refs": blob_refs_json,
        "history": history_json,
        "versions": versions_json,
        "current_version": asset_state.current_version,
        "permissions": permissions_json,
        "public_read": asset_state.public_read,
    });
    Ok(Json(ApiResponse::success(export_json)))
}

/// Import asset request (export-like JSON + signature for Create tx)
#[derive(Debug, Deserialize)]
pub struct ImportAssetRequest {
    pub asset_id: String,
    pub owner: String,
    pub density: String,
    pub metadata: std::collections::HashMap<String, String>,
    pub attributes: Vec<crate::types::Attribute>,
    pub game_id: Option<String>,
    #[serde(default)]
    pub blob_refs: std::collections::HashMap<String, String>,
    /// Signature hex string
    pub signature: String,
}

/// Import asset from JSON (creates asset via Create transaction)
async fn import_asset(
    State(api_state): State<ApiState>,
    Json(req): Json<ImportAssetRequest>,
) -> ApiResult<Json<ApiResponse<TransactionResponse>>> {
    let asset_id_bytes = hex::decode(&req.asset_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    if asset_id_bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut asset_id = [0u8; 32];
    asset_id.copy_from_slice(&asset_id_bytes);

    let owner_bytes = hex::decode(&req.owner).map_err(|_| StatusCode::BAD_REQUEST)?;
    if owner_bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut owner = [0u8; 32];
    owner.copy_from_slice(&owner_bytes);

    if api_state.state.get_asset(&asset_id).is_some() {
        return Err(StatusCode::CONFLICT);
    }
    let signature = hex::decode(&req.signature).map_err(|_| StatusCode::BAD_REQUEST)?;
    if signature.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let density = match req.density.as_str() {
        "Ethereal" => crate::types::DensityLevel::Ethereal,
        "Light" => crate::types::DensityLevel::Light,
        "Dense" => crate::types::DensityLevel::Dense,
        "Core" => crate::types::DensityLevel::Core,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    let mut metadata = req.metadata;
    if !req.blob_refs.is_empty() {
        let blob_refs_json = serde_json::to_string(&req.blob_refs).map_err(|_| StatusCode::BAD_REQUEST)?;
        metadata.insert("_blob_refs".to_string(), blob_refs_json);
    }

    let data = crate::types::AssetData {
        density,
        metadata,
        attributes: req.attributes,
        game_id: req.game_id,
        owner,
    };

    let tx = Transaction::MistbornAsset {
        action: AssetAction::Create,
        asset_id,
        data,
        signature,
    };
    let tx_hash = tx.hash();
    match api_state.consensus.add_transaction(tx) {
        Ok(()) => Ok(Json(ApiResponse::success(TransactionResponse {
            hash: hex::encode(tx_hash),
            status: "pending".to_string(),
        }))),
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

/// Search assets
async fn search_assets(
    State(api_state): State<ApiState>,
    axum::extract::Query(query): axum::extract::Query<SearchAssetsQuery>,
) -> ApiResult<Json<ApiResponse<Vec<serde_json::Value>>>> {
    let limit = query.limit.unwrap_or(100).min(1000);
    let offset = query.offset.unwrap_or(0);
    let mut candidate_ids: Vec<Hash> = Vec::new();
    
    // Use indexes for efficient filtering
    if let Some(ref owner_filter) = query.owner {
        if let Ok(owner_bytes) = hex::decode(owner_filter) {
            if owner_bytes.len() == 32 {
                let mut owner = [0u8; 32];
                owner.copy_from_slice(&owner_bytes);
                candidate_ids = api_state.state.search_assets_by_owner(&owner);
            }
        }
    } else if let Some(ref game_id_filter) = query.game_id {
        candidate_ids = api_state.state.search_assets_by_game_id(game_id_filter);
    } else if let Some(ref density_filter) = query.density {
        // Parse density level
        let density = match density_filter.as_str() {
            "Ethereal" => crate::types::DensityLevel::Ethereal,
            "Light" => crate::types::DensityLevel::Light,
            "Dense" => crate::types::DensityLevel::Dense,
            "Core" => crate::types::DensityLevel::Core,
            _ => return Err(StatusCode::BAD_REQUEST),
        };
        candidate_ids = api_state.state.search_assets_by_density(density);
    } else {
        // No specific filter, use all assets
        candidate_ids = api_state.state.assets().iter().map(|e| *e.key()).collect();
    }
    
    // Apply full-text search if provided
    if let Some(ref search_query) = query.q {
        if !search_query.is_empty() {
            let text_search_results = api_state.state.search_assets_by_metadata(search_query);
            // Intersect with candidate_ids
            let text_search_set: std::collections::HashSet<Hash> = text_search_results.into_iter().collect();
            candidate_ids.retain(|id| text_search_set.contains(id));
        }
    }
    
    // Build results
    let mut results: Vec<(Hash, AssetState)> = candidate_ids.iter()
        .filter_map(|id| {
            api_state.state.get_asset(id).map(|state| (*id, state))
        })
        .collect();
    
    // Sort results
    let sort_by = query.sort_by.as_deref().unwrap_or("created_at");
    let sort_order = query.sort_order.as_deref().unwrap_or("desc");
    let ascending = sort_order == "asc";
    
    match sort_by {
        "created_at" => {
            results.sort_by(|a, b| {
                if ascending {
                    a.1.created_at.cmp(&b.1.created_at)
                } else {
                    b.1.created_at.cmp(&a.1.created_at)
                }
            });
        }
        "updated_at" => {
            results.sort_by(|a, b| {
                if ascending {
                    a.1.updated_at.cmp(&b.1.updated_at)
                } else {
                    b.1.updated_at.cmp(&a.1.updated_at)
                }
            });
        }
        "rarity" => {
            results.sort_by(|a, b| {
                let rarity_a = a.1.data.attributes.iter()
                    .find(|attr| attr.name == "rarity")
                    .and_then(|attr| attr.rarity)
                    .unwrap_or(0.0);
                let rarity_b = b.1.data.attributes.iter()
                    .find(|attr| attr.name == "rarity")
                    .and_then(|attr| attr.rarity)
                    .unwrap_or(0.0);
                if ascending {
                    rarity_a.partial_cmp(&rarity_b).unwrap_or(std::cmp::Ordering::Equal)
                } else {
                    rarity_b.partial_cmp(&rarity_a).unwrap_or(std::cmp::Ordering::Equal)
                }
            });
        }
        _ => {
            // Default: sort by created_at desc
            results.sort_by(|a, b| b.1.created_at.cmp(&a.1.created_at));
        }
    }
    
    // Apply pagination
    let paginated_results: Vec<serde_json::Value> = results
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|(asset_id, asset_state)| {
            let blob_refs_json: std::collections::HashMap<String, String> = asset_state.blob_refs.iter()
                .map(|(k, v)| (k.clone(), hex::encode(v)))
                .collect();
            
            serde_json::json!({
                "asset_id": hex::encode(asset_id),
                "owner": hex::encode(asset_state.owner),
                "density": format!("{:?}", asset_state.data.density),
                "metadata": asset_state.data.metadata,
                "attributes": asset_state.data.attributes,
                "game_id": asset_state.data.game_id,
                "created_at": asset_state.created_at,
                "updated_at": asset_state.updated_at,
                "blob_refs": blob_refs_json,
                "history_count": asset_state.history.len(),
            })
        })
        .collect();
    
    Ok(Json(ApiResponse::success(paginated_results)))
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

/// WebSocket handler
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(api_state): State<ApiState>,
) -> axum::response::Response {
    ws.on_upgrade(|socket| handle_socket(socket, api_state))
}

/// Handle WebSocket connection
async fn handle_socket(socket: axum::extract::ws::WebSocket, state: ApiState) {
    use futures_util::{SinkExt, StreamExt};
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.ws_tx.subscribe();
    let subscriptions = Arc::new(tokio::sync::Mutex::new(Vec::<WsSubscription>::new()));

    // Clone Arc for send task
    let subscriptions_send = Arc::clone(&subscriptions);
    // Spawn task to send events to client
    let mut send_task = tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            // Check if event matches any subscription
            let subs = subscriptions_send.lock().await;
            let should_send = subs.is_empty() || subs.iter().any(|sub| {
                match (&sub.sub_type[..], &event) {
                    ("asset_created", WsEvent::AssetCreated { asset_id, owner, .. }) => {
                        sub.asset_id.as_ref().map(|id| id == asset_id).unwrap_or(true) &&
                        sub.owner.as_ref().map(|o| o == owner).unwrap_or(true)
                    }
                    ("asset_updated", WsEvent::AssetUpdated { asset_id, owner, .. }) => {
                        sub.asset_id.as_ref().map(|id| id == asset_id).unwrap_or(true) &&
                        sub.owner.as_ref().map(|o| o == owner).unwrap_or(true)
                    }
                    ("asset_condensed", WsEvent::AssetCondensed { asset_id, .. }) => {
                        sub.asset_id.as_ref().map(|id| id == asset_id).unwrap_or(true)
                    }
                    ("asset_evaporated", WsEvent::AssetEvaporated { asset_id, .. }) => {
                        sub.asset_id.as_ref().map(|id| id == asset_id).unwrap_or(true)
                    }
                    ("asset_merged", WsEvent::AssetMerged { asset_id, .. }) => {
                        sub.asset_id.as_ref().map(|id| id == asset_id).unwrap_or(true)
                    }
                    ("asset_split", WsEvent::AssetSplit { asset_id, .. }) => {
                        sub.asset_id.as_ref().map(|id| id == asset_id).unwrap_or(true)
                    }
                    ("asset_permission_changed", WsEvent::AssetPermissionChanged { asset_id, owner, .. }) => {
                        sub.asset_id.as_ref().map(|id| id == asset_id).unwrap_or(true) &&
                        sub.owner.as_ref().map(|o| o == owner).unwrap_or(true)
                    }
                    ("asset_attribute_updated", WsEvent::AssetAttributeUpdated { asset_id, owner, .. }) => {
                        sub.asset_id.as_ref().map(|id| id == asset_id).unwrap_or(true) &&
                        sub.owner.as_ref().map(|o| o == owner).unwrap_or(true)
                    }
                    ("asset_version_created", WsEvent::AssetVersionCreated { asset_id, owner, .. }) => {
                        sub.asset_id.as_ref().map(|id| id == asset_id).unwrap_or(true) &&
                        sub.owner.as_ref().map(|o| o == owner).unwrap_or(true)
                    }
                    _ => false,
                }
            });
            drop(subs); // Release lock before potential await

            if should_send {
                if let Ok(json) = serde_json::to_string(&event) {
                    if sender.send(Message::Text(json)).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    // Clone Arc for receive task
    let subscriptions_recv = Arc::clone(&subscriptions);
    // Spawn task to receive messages from client
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                if let Ok(request) = serde_json::from_str::<WsSubscribeRequest>(&text) {
                    let mut subs = subscriptions_recv.lock().await;
                    *subs = request.subscribe;
                }
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}

/// Broadcast asset event to all WebSocket clients
pub fn broadcast_asset_event(tx: &broadcast::Sender<WsEvent>, event: WsEvent) {
    let _ = tx.send(event);
}

/// Sync status response
#[derive(Debug, Serialize)]
pub struct SyncStatus {
    pub current_height: u64,
    pub last_finalized_height: u64,
    pub last_finalized_wave: u64,
    pub syncing: bool,
}

/// Basic metrics response
#[derive(Debug, Serialize)]
pub struct BasicMetrics {
    pub current_height: u64,
    pub last_finalized_height: u64,
    pub last_finalized_wave: u64,
    pub tx_pool_size: usize,
    pub connected_peers: usize, // MVP: always 0, network not accessible from API
    pub block_time_avg_ms: Option<u64>, // Average block time in ms (if available)
}

/// Start sync with peers
async fn start_sync(
    State(_api_state): State<ApiState>,
) -> ApiResult<Json<ApiResponse<&'static str>>> {
    // For MVP: sync is automatic when blocks are received
    // This endpoint is a placeholder for future manual sync control
    tracing::info!("Sync start requested (automatic sync is enabled)");
    Ok(Json(ApiResponse::success("Sync is automatic")))
}

/// Get sync status
async fn get_sync_status(
    State(api_state): State<ApiState>,
) -> ApiResult<Json<ApiResponse<SyncStatus>>> {
    let current_height = api_state.state.current_height();
    let last_finalized_height = api_state.consensus.get_last_finalized_height();
    let last_finalized_wave = api_state.consensus.get_last_finalized_wave();
    
    let status = SyncStatus {
        current_height,
        last_finalized_height,
        last_finalized_wave,
        syncing: false, // MVP: always false, sync is automatic
    };
    
    Ok(Json(ApiResponse::success(status)))
}

/// Get basic metrics for observability
async fn get_basic_metrics(
    State(api_state): State<ApiState>,
) -> ApiResult<Json<ApiResponse<BasicMetrics>>> {
    let current_height = api_state.state.current_height();
    let last_finalized_height = api_state.consensus.get_last_finalized_height();
    let last_finalized_wave = api_state.consensus.get_last_finalized_wave();
    let tx_pool_size = api_state.consensus.tx_pool_size();
    
    // MVP: peer count not accessible from API state (network is separate)
    // In future, we could add a channel or shared state to expose this
    let connected_peers = 0;
    
    // Calculate average block time from recent blocks (last 10 blocks)
    let block_time_avg_ms = if current_height > 0 {
        let mut timestamps = Vec::new();
        let start_height = current_height.saturating_sub(10);
        for h in start_height..=current_height {
            if let Some(block) = api_state.state.get_block_by_height(h) {
                timestamps.push(block.header.timestamp);
            }
        }
        if timestamps.len() >= 2 {
            let total_time = timestamps.last().unwrap() - timestamps.first().unwrap();
            let block_count = timestamps.len() - 1;
            if block_count > 0 {
                Some((total_time as u64 * 1000) / block_count as u64) // Convert to ms
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };
    
    let metrics = BasicMetrics {
        current_height,
        last_finalized_height,
        last_finalized_wave,
        tx_pool_size,
        connected_peers,
        block_time_avg_ms,
    };
    
    Ok(Json(ApiResponse::success(metrics)))
}

/// Start API server
pub async fn start_api_server(state: ApiState) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let app = create_router(state.clone());
    
    let listener = tokio::net::TcpListener::bind(&state.config.api.listen_addr).await?;
    tracing::info!("API server listening on http://{}", state.config.api.listen_addr);
    tracing::info!("Health check: http://{}/health", state.config.api.listen_addr);
    tracing::info!("API docs: http://{}/api/v1/blockchain/info", state.config.api.listen_addr);
    tracing::info!("WebSocket: ws://{}/api/v1/ws", state.config.api.listen_addr);
    
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
        
        let (ws_tx, _) = tokio::sync::broadcast::channel(100);
        ApiState {
            consensus,
            state,
            config,
            ws_tx,
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
