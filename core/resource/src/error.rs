use thiserror::Error;

pub type Result<T> = std::result::Result<T, ResourceError>;

#[derive(Debug, Error)]
pub enum ResourceError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization/deserialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Missing directory or file: {0}")]
    MissingPath(String),

    #[error("Invalid RSB Description structure: {0}")]
    InvalidStructure(String),
}
