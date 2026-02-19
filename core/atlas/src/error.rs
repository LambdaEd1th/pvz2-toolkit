use thiserror::Error;

#[derive(Error, Debug)]
pub enum AtlasError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("Atlas error: {0}")]
    Generic(String),
}

pub type Result<T> = std::result::Result<T, AtlasError>;
