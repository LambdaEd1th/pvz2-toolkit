use thiserror::Error;

pub type Result<T> = std::result::Result<T, Cfw2Error>;

#[derive(Error, Debug)]
pub enum Cfw2Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
