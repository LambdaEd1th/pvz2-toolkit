pub mod error;
pub mod process;

use crate::error::SmfError;
use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use std::io::{Read, Seek, Write};

pub const SMF_MAGIC: u32 = 0xDEADFED4;

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

pub fn encode<W: Write>(writer: &mut W, data: &[u8], use_64bit: bool) -> Result<(), SmfError> {
    writer.write_u32::<LE>(SMF_MAGIC)?;

    if use_64bit {
        writer.write_u32::<LE>(0)?;
        writer.write_u32::<LE>(data.len() as u32)?;
        writer.write_u32::<LE>(0)?;
    } else {
        writer.write_u32::<LE>(data.len() as u32)?;
    }

    let mut encoder = ZlibEncoder::new(writer, Compression::best());
    encoder.write_all(data)?;
    encoder.finish()?;

    Ok(())
}
