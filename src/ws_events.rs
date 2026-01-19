//! WebSocket event types for real-time notifications
//!
//! This module contains event types that are broadcast to WebSocket clients
//! when asset operations occur in the blockchain.

use serde::Serialize;

/// WebSocket event types
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum WsEvent {
    #[serde(rename = "asset_created")]
    AssetCreated {
        asset_id: String,
        owner: String,
        density: String,
    },
    #[serde(rename = "asset_updated")]
    AssetUpdated {
        asset_id: String,
        owner: String,
    },
    #[serde(rename = "asset_condensed")]
    AssetCondensed {
        asset_id: String,
        new_density: String,
    },
    #[serde(rename = "asset_evaporated")]
    AssetEvaporated {
        asset_id: String,
        new_density: String,
    },
    #[serde(rename = "asset_merged")]
    AssetMerged {
        asset_id: String,
        merged_asset_id: String,
    },
    #[serde(rename = "asset_split")]
    AssetSplit {
        asset_id: String,
        created_assets: Vec<String>,
    },
    #[serde(rename = "error")]
    Error { message: String },
}
