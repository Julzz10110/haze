//! Error types for HAZE blockchain

use thiserror::Error;

#[derive(Error, Debug)]
pub enum HazeError {
    #[error("Consensus error: {0}")]
    Consensus(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("State error: {0}")]
    State(String),

    #[error("VM error: {0}")]
    VM(String),

    #[error("Asset error: {0}")]
    Asset(String),

    #[error("Asset size exceeded: {0} bytes exceeds limit of {1} bytes")]
    AssetSizeExceeded(usize, usize),

    #[error("Invalid metadata format: {0}")]
    InvalidMetadataFormat(String),

    #[error("Invalid density transition: cannot transition from {0} to {1}")]
    InvalidDensityTransition(String, String),

    #[error("Access denied: {0}")]
    AccessDenied(String),

    #[error("Crypto error: {0}")]
    Crypto(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),

    #[error("Invalid block: {0}")]
    InvalidBlock(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, HazeError>;