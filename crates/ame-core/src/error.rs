use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("storage open failed: {0}")]
    StorageOpen(#[from] sled::Error),
    #[error("storage serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("secure store error: {0}")]
    Secure(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;
