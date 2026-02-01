//! E2E API integration tests

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use bytes::Bytes;
use haze::api::{create_router, ApiState, EstimateGasRequest};
use haze::config::Config;
use haze::consensus::ConsensusEngine;
use haze::state::StateManager;
use haze::types::{AssetAction, AssetData, DensityLevel, Transaction};
use tower::util::ServiceExt;

static INTEGRATION_TEST_DB_COUNTER: AtomicU64 = AtomicU64::new(0);

fn create_test_api_state() -> ApiState {
    let id = INTEGRATION_TEST_DB_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut config = Config::default();
    config.storage.db_path = PathBuf::from(format!("./haze_db_test_integration_{}", id));
    config.api.enable_cors = false;

    let state = Arc::new(StateManager::new(&config).unwrap());
    let consensus = Arc::new(ConsensusEngine::new(config.clone(), state.clone()).unwrap());
    let (ws_tx, _) = tokio::sync::broadcast::channel(100);

    ApiState {
        consensus,
        state,
        config,
        ws_tx,
        connected_peers: Arc::new(AtomicUsize::new(0)),
    }
}

#[tokio::test]
async fn e2e_health() {
    let api_state = create_test_api_state();
    let app = create_router(api_state);

    let req = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn e2e_blockchain_info() {
    let api_state = create_test_api_state();
    let app = create_router(api_state);

    let req = Request::builder()
        .uri("/api/v1/blockchain/info")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn e2e_get_asset_not_found() {
    let api_state = create_test_api_state();
    let app = create_router(api_state);

    let fake_id = "0".repeat(64);
    let req = Request::builder()
        .uri(format!("/api/v1/assets/{}", fake_id))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn e2e_estimate_gas_create() {
    let api_state = create_test_api_state();
    let app = create_router(api_state);

    let mut meta = std::collections::HashMap::new();
    meta.insert("name".to_string(), "test".to_string());
    let owner = [1u8; 32];
    let tx = Transaction::MistbornAsset {
        from: owner,
        action: AssetAction::Create,
        asset_id: [0u8; 32],
        data: AssetData {
            density: DensityLevel::Ethereal,
            metadata: meta,
            attributes: vec![],
            game_id: None,
            owner,
        },
        fee: 0,
        nonce: 0,
        chain_id: None,
        valid_until_height: None,
        signature: vec![0; 64],
    };

    let body = serde_json::to_vec(&EstimateGasRequest { transaction: tx }).unwrap();
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/assets/estimate-gas")
        .header("content-type", "application/json")
        .body(Body::from(Bytes::from(body)))
        .unwrap();
    let response = app.oneshot(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
