use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParticlesError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Invalid Particles platform variant")]
    InvalidVariant,

    #[error("Unsupported Particles format: expected {0}, got {1}")]
    UnsupportedFormat(u32, u32),

    #[error("String decode error")]
    StringDecodeError,
}
