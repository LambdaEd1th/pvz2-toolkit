use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RsbPatchError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Invalid RSBPatch file: {0}")]
    InvalidFile(String),
}

#[derive(Error, Debug)]
pub enum PatchError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("VCDiff error: {0}")]
    VCDiff(String),
    #[error("RSB Patch error: {0}")]
    RsbPatch(#[from] RsbPatchError),
}

pub type Result<T> = std::result::Result<T, PatchError>;
