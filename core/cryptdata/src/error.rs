use thiserror::Error;

pub type Result<T> = std::result::Result<T, CryptDataError>;

#[derive(Error, Debug)]
pub enum CryptDataError {
    #[error("Invalid CRYPT_RES magic")]
    InvalidMagic,
    #[error("Data too short to contain size field")]
    TooShort,
    #[error("Decryption key not provided")]
    MissingKey,
}
