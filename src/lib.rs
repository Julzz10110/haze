//! HAZE (High-performance Asset Zone Engine)
//! 
//! Specialized Layer 1 blockchain for GameFi
//! Concept of "digital fog" - distributed, fluid, omnipresent environment

pub mod consensus;
pub mod vm;
pub mod assets;
pub mod network;
pub mod types;
pub mod state;
pub mod crypto;
pub mod config;
pub mod error;
pub mod tokenomics;
pub mod economy;
pub mod api;

// Re-export commonly used types
pub use types::{Block, Transaction, Address, Hash, AssetAction, AssetData, DensityLevel, sha256, hash_to_hex, hex_to_hash};
pub use crypto::KeyPair;
pub use tokenomics::{Tokenomics, StakeRecord, ValidatorInfo};
pub use economy::{FogEconomy, EconomicZone, VortexMarket, LiquidityPool, MarketConditions};
pub use assets::MistbornAsset;
pub use error::{HazeError, Result};
