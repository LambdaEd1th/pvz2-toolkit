pub mod error;
pub mod reader;
pub mod writer;

pub use error::{Result, SmfError};
pub use reader::*;
pub use writer::*;

pub const SMF_MAGIC: u32 = 0xDEADFED4;
