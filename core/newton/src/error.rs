use thiserror::Error;

#[derive(Error, Debug)]
pub enum NewtonError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid magic: expected {0}, found {1}")]
    InvalidMagic(String, String),
    #[error("Invalid version: {0}")]
    InvalidVersion(u32),
    #[error("Invalid compression flag: {0}")]
    InvalidCompression(u32),
    #[error("Utif8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("Json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Xml error: {0}")]
    Xml(#[from] serde_xml_rs::Error),
    #[error("Other: {0}")]
    Other(String),
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
}

pub type Result<T> = std::result::Result<T, NewtonError>;
