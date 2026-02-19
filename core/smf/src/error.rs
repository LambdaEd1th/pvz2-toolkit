use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SmfError {
    #[error("IO Error: {0}")]
    Io(#[from] io::Error),
    #[error("Invalid Magic: expected 0xDEADFED4, got {0:#010x}")]
    InvalidMagic(u32),
    #[error("SMF Error: {0}")]
    Generic(String),
}

pub type Result<T> = std::result::Result<T, SmfError>;
