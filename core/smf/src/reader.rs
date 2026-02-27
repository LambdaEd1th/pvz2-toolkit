use crate::SMF_MAGIC;
use crate::error::SmfError;
use byteorder::{LE, ReadBytesExt};
use flate2::read::ZlibDecoder;
use std::io::{Read, Seek};

pub fn decode<R: Read + Seek>(mut reader: R, use_64bit: bool) -> Result<Vec<u8>, SmfError> {
    let magic = reader.read_u32::<LE>()?;
    if magic != SMF_MAGIC {
        return Err(SmfError::InvalidMagic(magic));
    }

    if use_64bit {
        let _pad1 = reader.read_u32::<LE>()?;
        let _size = reader.read_u32::<LE>()?;
        let _pad2 = reader.read_u32::<LE>()?;
    } else {
        let _size = reader.read_u32::<LE>()?;
    }

    let mut decoder = ZlibDecoder::new(reader);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;

    Ok(decompressed)
}
