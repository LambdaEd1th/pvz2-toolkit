use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReanimError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Invalid Reanim platform variant")]
    InvalidVariant,

    #[error("Invalid track signature")]
    InvalidTrack,

    #[error("Invalid magic number: expected {0}, got {1}")]
    InvalidMagic(u32, u32),

    #[error("String decode error")]
    StringDecodeError,
}
