pub mod decoder;
pub mod encoder;

pub use decoder::{AdpcmParams, process_adpcm};
pub use encoder::WavToAdpcm;
