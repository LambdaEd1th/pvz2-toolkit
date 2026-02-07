use byteorder::{ReadBytesExt, BE, LE};
use std::io::{Read, Seek, SeekFrom};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BnkError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid magic: expected BKHD")]
    InvalidMagic,
    #[error("Parse error: {0}")]
    ParseError(String),
}

type Result<T> = std::result::Result<T, BnkError>;

#[derive(Debug, Clone, Default)]
pub struct BnkHeader {
    pub version: u32,
    pub id: u32,
    pub language: u32, // often 0 or not present in older versions in same way? vgmstream reads it.
                       // vgmstream bkhd.c:
                       // 0x08: version
                       // 0x0C: id
                       // 0x10: language (sometimes)
}

#[derive(Debug, Clone)]
pub struct BnkFileEntry {
    pub id: u32,
    pub offset: u32, // Absolute offset in file (or relative to DATA start, converted to absolute)
    pub size: u32,
}

pub struct Bnk {
    pub header: BnkHeader,
    pub entries: Vec<BnkFileEntry>,
    pub data_start_offset: u32, // Where DATA body starts
}

impl Bnk {
    pub fn new<R: Read + Seek>(mut reader: R) -> Result<Self> {
        let mut bnk = Bnk {
            header: BnkHeader::default(),
            entries: Vec::new(),
            data_start_offset: 0,
        };
        parse_bnk(&mut reader, &mut bnk)?;
        Ok(bnk)
    }
}

fn parse_bnk<R: Read + Seek>(reader: &mut R, bnk: &mut Bnk) -> Result<()> {
    // Basic checks
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic)?;
    if &magic != b"BKHD" {
        // vgmstream checks for AKBK too, but let's stick to BKHD for now
        return Err(BnkError::InvalidMagic);
    }

    let bkhd_length = reader.read_u32::<LE>()?;
    let start_pos = reader.stream_position()?;

    // Version at 0x08 relative to start (0x00 relative to body)
    let version = reader.read_u32::<LE>()?;
    let id = reader.read_u32::<LE>()?;

    // vgmstream: if version == 0 || version == 1, read again at +0x10?
    // Let's assume standard behavior first or follow vgmstream logic closely.
    bnk.header.version = version;
    bnk.header.id = id;

    // Skip remaining BKHD
    reader.seek(SeekFrom::Start(start_pos + bkhd_length as u64))?;

    // Find other chunks
    // vgmstream version <= 26 logic vs newer
    if bnk.header.version <= 26 {
        // Old format: Index is inside DATA chunk
        // We need to find DATA chunk
        match find_chunk(reader, b"DATA")? {
            Some((offset, size)) => {
                // Index logic for old version
                parse_old_index(reader, offset, size, bnk)?;
            }
            None => {
                return Err(BnkError::ParseError(
                    "DATA chunk not found (v <= 26)".to_string(),
                ))
            }
        }
    } else {
        // Newer format: DIDX chunk has index, DATA has media
        let didx_chunk = find_chunk(reader, b"DIDX")?;
        let data_chunk = find_chunk(reader, b"DATA")?;

        if let (Some((didx_off, didx_size)), Some((data_off, _data_size))) =
            (didx_chunk, data_chunk)
        {
            // Parse DIDX
            bnk.data_start_offset = (data_off + 8) as u32; // Body start
            parse_didx(reader, didx_off, didx_size, bnk.data_start_offset, bnk)?;
        } else {
            // Some banks might not have DIDX if they are event-only, but for audio extraction we need it
            // If missing DATA/DIDX, just return empty?
        }
    }

    Ok(())
}

fn find_chunk<R: Read + Seek>(reader: &mut R, target_id: &[u8; 4]) -> Result<Option<(u64, u32)>> {
    // Simple linear scan from current position
    // Usually chunks follow BKHD
    // Note: This naive scan presumes we are at start of a chunk sequence.
    // In `parse_bnk`, we seek to end of BKHD, so we are at start of first chunk.

    let end_pos = reader.seek(SeekFrom::End(0))?;
    // We need to jump back to where we started scanning.
    // `parse_bnk` seeks to `start_pos + bkhd_length`.
    // But we need to save that position?
    // Let's actually scan from `reader.stream_position()`.

    // We should probably save start position passed in or assume valid stream state.
    // Let's assume reader is at valid position.

    let mut current_pos = reader.stream_position()?;

    while current_pos < end_pos {
        reader.seek(SeekFrom::Start(current_pos))?;

        let mut id = [0u8; 4];
        if reader.read_exact(&mut id).is_err() {
            break;
        }

        let size = reader.read_u32::<LE>()?;

        if &id == target_id {
            // Found
            return Ok(Some((current_pos, size)));
        }

        // Next chunk
        current_pos += 8 + size as u64;
    }

    // Reset position? Or just return None.
    Ok(None)
}

fn parse_didx<R: Read + Seek>(
    reader: &mut R,
    chunk_offset: u64,
    chunk_size: u32,
    data_body_offset: u32,
    bnk: &mut Bnk,
) -> Result<()> {
    reader.seek(SeekFrom::Start(chunk_offset + 8))?;

    // each entry is 12 bytes
    let count = chunk_size / 12;

    for _ in 0..count {
        let id = reader.read_u32::<LE>()?;
        let offset = reader.read_u32::<LE>()?;
        let size = reader.read_u32::<LE>()?;

        bnk.entries.push(BnkFileEntry {
            id,
            offset: data_body_offset + offset,
            size,
        });
    }

    Ok(())
}

fn parse_old_index<R: Read + Seek>(
    reader: &mut R,
    chunk_offset: u64,
    chunk_size: u32,
    bnk: &mut Bnk,
) -> Result<()> {
    // Old index format inside DATA
    // offset points to chunk header. Body is at +8.

    // vgmstream logic for old version:
    // 0x00: entries count
    // ...
    // 0x18: data start (relative to chunk body?)

    reader.seek(SeekFrom::Start(chunk_offset + 8))?;
    let entries_count = reader.read_u32::<LE>()?;

    reader.seek(SeekFrom::Start(chunk_offset + 8 + 0x18))?;
    let data_start = reader.read_u32::<LE>()?;

    let table_offset = chunk_offset + 8 + 0x20;

    for i in 0..entries_count {
        let entry_pos = table_offset + (i as u64 * 0x18); // 0x18 per entry?
        reader.seek(SeekFrom::Start(entry_pos))?;

        // 0x08: ID
        // 0x10: offset (relative to data_start?)
        // 0x14: size

        reader.seek(SeekFrom::Current(8))?;
        let id = reader.read_u32::<LE>()?;
        let rel_offset = reader.read_u32::<LE>()?;
        let size = reader.read_u32::<LE>()?;

        let abs_offset = (chunk_offset + 8 + data_start as u64) as u32 + rel_offset;

        bnk.entries.push(BnkFileEntry {
            id,
            offset: abs_offset,
            size,
        });
    }

    Ok(())
}
