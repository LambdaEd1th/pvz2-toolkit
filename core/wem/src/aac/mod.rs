pub mod decoder;
pub mod encoder;

pub use decoder::{decode_aac_to_wav, extract_wem_aac};
pub use encoder::{M4aToWem, probe_m4a_metadata};
