use crate::{PakInfo, PakRecord, Result};
use byteorder::{LE, WriteBytesExt};
use flate2::Compression;
use flate2::write::ZlibEncoder;
use std::io::Write;

/// The standard PAK magic, same constant as in reader.
const PAK_MAGIC: u32 = (-1161803072i32) as u32;

pub fn pack<W: Write>(writer: &mut W, info: &PakInfo, files: &[PakRecord]) -> Result<()> {
    if info.pak_platform == "TV" {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "TV PAK packing is not supported",
        )
        .into());
    }

    let is_compress = info.pak_use_zlib_compress;

    // --- Build directory header ---
    let mut buf = Vec::new();
    buf.write_u32::<LE>(PAK_MAGIC)?;
    buf.write_u32::<LE>(0x0)?; // version

    // We need to write the directory first (with placeholder sizes),
    // then write the payloads, then rewrite the directory with real sizes.
    // C# does exactly this: Write() -> payloads -> seek(0) -> Write() again.
    // We'll collect sizes first, then build the whole thing.

    struct EntryMeta {
        name: String,
        original_size: u32,
        compressed_data: Vec<u8>,
        zlib_size: u32,
    }

    let mut metas = Vec::new();

    for record in files {
        let name = if info.pak_use_windows_path_separate {
            record.path.replace('/', "\\")
        } else {
            record.path.replace('\\', "/")
        };

        let original_size = record.data.len() as u32;
        let (compressed_data, zlib_size) = if is_compress {
            let mut out = Vec::new();
            let mut encoder = ZlibEncoder::new(&mut out, Compression::best());
            encoder.write_all(&record.data)?;
            encoder.finish()?;
            let zs = out.len() as u32;
            (out, zs)
        } else {
            (record.data.clone(), 0u32)
        };

        metas.push(EntryMeta {
            name,
            original_size,
            compressed_data,
            zlib_size,
        });
    }

    // Write file info entries
    for meta in &metas {
        buf.write_u8(0x0)?; // entry marker
        buf.write_u8(meta.name.len() as u8)?;
        buf.write_all(meta.name.as_bytes())?;
        buf.write_u32::<LE>(meta.original_size)?;
        if is_compress {
            buf.write_u32::<LE>(meta.zlib_size)?;
        }
        buf.write_u64::<LE>(129146222018596744)?; // fixed file_time
    }

    // End-of-directory marker
    buf.write_u8(0x80)?;

    // Write file payloads
    for meta in &metas {
        if info.pak_platform != "PC" {
            if info.pak_platform != "Xbox360" && meta.name.to_lowercase().ends_with(".ptx") {
                fill_0x1000(&mut buf)?;
            } else {
                fill_alignment(&mut buf)?;
            }
        }

        let data = if is_compress {
            &meta.compressed_data
        } else {
            &meta.compressed_data // same as original when not compressing
        };
        buf.write_all(data)?;
    }

    // For PC, XOR the entire buffer (the magic inside becomes the PC magic).
    if info.pak_platform == "PC" {
        for b in buf.iter_mut() {
            *b ^= 0xF7;
        }
    }

    writer.write_all(&buf)?;
    Ok(())
}

fn fill_0x1000(buf: &mut Vec<u8>) -> Result<()> {
    let length = buf.len() & (0x1000 - 1);
    if length == 0 {
        buf.write_u16::<LE>((0x1000 - 2) as u16)?;
        buf.write_all(&vec![0u8; 0x1000 - 2])?;
    } else if length > 0x1000 - 2 {
        let w = (0x2000 - 2 - length) as u16;
        buf.write_u16::<LE>(w)?;
        buf.write_all(&vec![0u8; w as usize])?;
    } else {
        let w = (0x1000 - 2 - length) as u16;
        buf.write_u16::<LE>(w)?;
        buf.write_all(&vec![0u8; w as usize])?;
    }
    Ok(())
}

fn fill_alignment(buf: &mut Vec<u8>) -> Result<()> {
    let length = buf.len() & 0b111;
    if length == 0 {
        buf.write_u16::<LE>(0x06)?;
        buf.write_all(&vec![0u8; 6])?;
    } else if length > 5 {
        let w = (14 - length) as u16;
        buf.write_u16::<LE>(w)?;
        buf.write_all(&vec![0u8; w as usize])?;
    } else {
        let w = (6 - length) as u16;
        buf.write_u16::<LE>(w)?;
        buf.write_all(&vec![0u8; w as usize])?;
    }
    Ok(())
}
