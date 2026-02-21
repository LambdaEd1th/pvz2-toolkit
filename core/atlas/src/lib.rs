pub mod error;
pub mod process;
pub mod types;

pub use error::{AtlasError, Result};
pub use process::{merge_atlas, split_atlas};
