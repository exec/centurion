//! Error handling for Centurion server

use thiserror::Error;

/// Main error type for Centurion server operations
#[derive(Error, Debug)]
pub enum CenturionError {
    /// Generic error
    #[error("Server error: {0}")]
    Generic(String),
    
    /// Network/connection error
    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),
    
    /// Database error
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),
    
    /// Protocol error
    #[error("Protocol error: {0}")]
    Protocol(#[from] legion_protocol::IronError),
}