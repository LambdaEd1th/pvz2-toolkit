use thiserror::Error;

#[derive(Error, Debug)]
pub enum BnkError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid magic: expected BKHD")]
    InvalidMagic,
    #[error("Parse error: {0}")]
    ParseError(String),
}

pub type Result<T> = std::result::Result<T, BnkError>;
