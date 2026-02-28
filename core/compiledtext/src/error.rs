use thiserror::Error;

pub type Result<T> = std::result::Result<T, CompiledTextError>;

#[derive(Error, Debug)]
pub enum CompiledTextError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid PopCap Zlib magic: expected 0xDEADFED4")]
    InvalidZlibMagic,
    #[error("Rijndael cipher error: {0}")]
    Cipher(String),
    #[error("Base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),
}
