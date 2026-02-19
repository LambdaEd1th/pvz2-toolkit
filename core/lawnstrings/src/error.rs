use thiserror::Error;

#[derive(Error, Debug)]
pub enum LawnStringsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Parse error: {0}")]
    Parse(String),
}

pub type Result<T> = std::result::Result<T, LawnStringsError>;
