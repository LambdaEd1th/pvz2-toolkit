use std::io;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, PakError>;

#[derive(Error, Debug)]
pub enum PakError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("Invalid PAK magic number: expected {0}, got {1}")]
    InvalidMagic(u32, u32),
    #[error("Invalid PAK version: expected {0}, got {1}")]
    InvalidVersion(u32, u32),
    #[error("Platform must be PC, Xbox_360, or TV")]
    InvalidPlatform,
}
