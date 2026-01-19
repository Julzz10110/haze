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
use futures_util::{SinkExt, StreamExt};
use crate::config::Config;
use crate::consensus::ConsensusEngine;
use crate::state::StateManager;
use crate::types::{Transaction, AssetAction};
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
        .route("/api/v1/ws", get(ws_handler))
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
    pub limit: Option<usize>,
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
