use thiserror::Error;

#[derive(Error, Debug)]
pub enum ResourcesError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON format error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Group `{0}` has missing nested members")]
    MissingGroupMembers(String),
}

pub type Result<T> = std::result::Result<T, ResourcesError>;
