//! wem library for converting Wwise files.

pub mod aac;
pub mod adpcm;
pub mod bit_stream;
pub mod error;
pub mod pcm;
pub mod process;
pub mod vorbis;
pub mod wav;

// Re-export specific types for convenience
pub use error::WemError;
pub use vorbis::codebook::CodebookLibrary;
pub use vorbis::decoder::{ConversionOptions, ForcePacketFormat, WwiseRiffVorbis};
pub use vorbis::encoder::OggToWem;

pub use aac::{M4aToWem, probe_m4a_metadata};
pub use pcm::WavToWem;
