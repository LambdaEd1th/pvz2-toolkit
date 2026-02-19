use thiserror::Error;

#[derive(Error, Debug)]
pub enum PopfxError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid magic: expected xfcp")]
    InvalidMagic,
    #[error("Invalid version: expected {expected}, got {got}")]
    InvalidVersion { expected: u32, got: u32 },
    #[error("Invalid block size for block {block}: expected {expected}, got {got}")]
    InvalidBlockSize {
        block: usize,
        expected: u32,
        got: u32,
    },
    #[error("String encoding error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("JSON Error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, PopfxError>;
