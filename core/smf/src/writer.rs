use crate::SMF_MAGIC;
use crate::error::SmfError;
use byteorder::{LE, WriteBytesExt};
use flate2::Compression;
use flate2::write::ZlibEncoder;
use std::io::Write;

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
