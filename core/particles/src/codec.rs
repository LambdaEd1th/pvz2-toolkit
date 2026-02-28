use crate::error::ParticlesError;
use crate::types::ParticlesTrackNode;
use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use std::io::{Read, Write};

/// Magic number for PopCap compiled binary formats (Zlib wrapper)
pub const POPCAP_COMPILED_MAGIC: u32 = 0xDEADFED4;

/// Common trait for PopCap compiled binary formats.
///
/// All PopCap PvZ1 compiled formats (Particles, Trail, Reanim, etc.)
/// share the same outer Zlib wrapper (`0xDEADFED4` + uncompressed_size + zlib_data).
/// This trait provides a unified decode/encode interface.
pub trait PopCapCodec: Sized {
    /// Decode from compiled binary data (auto-detects variant if applicable)
    fn decode(data: &[u8]) -> Result<Self, ParticlesError>;

    /// Encode to compiled binary data (uses default variant)
    fn encode(&self) -> Result<Vec<u8>, ParticlesError>;
}

// ─── Shared Zlib Wrapper Helpers ─────────────────────────────────────────────

/// Decompress a PopCap compiled binary (0xDEADFED4 + size + Zlib)
pub fn popcap_decompress(data: &[u8]) -> Result<Vec<u8>, ParticlesError> {
    if data.len() < 8 {
        return Err(ParticlesError::InvalidVariant);
    }
    let mut cursor = std::io::Cursor::new(data);
    let magic = cursor.read_u32::<LE>()?;
    if magic != POPCAP_COMPILED_MAGIC {
        return Err(ParticlesError::InvalidVariant);
    }
    let _uncompressed_size = cursor.read_u32::<LE>()?;

    let mut decompressed = Vec::new();
    let mut decoder = ZlibDecoder::new(&data[8..]);
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}

/// Compress data and wrap with PopCap compiled header (0xDEADFED4 + size + Zlib)
pub fn popcap_compress(data: &[u8]) -> Result<Vec<u8>, ParticlesError> {
    let mut compressed = Vec::new();
    let mut encoder = ZlibEncoder::new(&mut compressed, Compression::default());
    encoder.write_all(data)?;
    encoder.finish()?;

    let mut output = Vec::new();
    output.write_u32::<LE>(POPCAP_COMPILED_MAGIC)?;
    output.write_u32::<LE>(data.len() as u32)?;
    output.write_all(&compressed)?;
    Ok(output)
}

// ─── Shared String I/O ───────────────────────────────────────────────────────

/// Read a length-prefixed string (i32 length + bytes)
pub fn read_string<R: Read>(reader: &mut R) -> Result<String, ParticlesError> {
    let len = reader.read_i32::<LE>()?;
    if len <= 0 {
        return Ok(String::new());
    }
    let mut buf = vec![0u8; len as usize];
    reader.read_exact(&mut buf)?;
    // Remove null terminator if present
    let mut end = buf.len();
    while end > 0 && buf[end - 1] == 0 {
        end -= 1;
    }
    String::from_utf8(buf[..end].to_vec()).map_err(|_| ParticlesError::StringDecodeError)
}

/// Read a length-prefixed string, returning None for empty strings
pub fn read_string_opt<R: Read>(reader: &mut R) -> Result<Option<String>, ParticlesError> {
    let s = read_string(reader)?;
    Ok(if s.is_empty() { None } else { Some(s) })
}

/// Write a length-prefixed string
pub fn write_string<W: Write>(writer: &mut W, s: &str) -> Result<(), ParticlesError> {
    writer.write_i32::<LE>(s.len() as i32)?;
    if !s.is_empty() {
        writer.write_all(s.as_bytes())?;
    }
    Ok(())
}

// ─── Shared TrackNode I/O ────────────────────────────────────────────────────

/// Read a track node array (i32 count + nodes)
pub fn read_track_nodes<R: Read>(
    reader: &mut R,
) -> Result<Option<Vec<ParticlesTrackNode>>, ParticlesError> {
    let count = reader.read_i32::<LE>()?;
    if count == 0 {
        return Ok(None);
    }
    let mut nodes = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let time = reader.read_f32::<LE>()?;

        let val = reader.read_f32::<LE>()?;
        let low_value = if val != 0.0 { Some(val) } else { None };

        let val = reader.read_f32::<LE>()?;
        let high_value = if val != 0.0 { Some(val) } else { None };

        let val = reader.read_i32::<LE>()?;
        let curve_type = if val != 1 { Some(val) } else { None };

        let val = reader.read_i32::<LE>()?;
        let distribution = if val != 1 { Some(val) } else { None };

        nodes.push(ParticlesTrackNode {
            time,
            low_value,
            high_value,
            curve_type,
            distribution,
        });
    }
    Ok(Some(nodes))
}

/// Write a track node array (i32 count + nodes)
pub fn write_track_nodes<W: Write>(
    writer: &mut W,
    nodes: &Option<Vec<ParticlesTrackNode>>,
) -> Result<(), ParticlesError> {
    if let Some(nodes) = nodes {
        writer.write_i32::<LE>(nodes.len() as i32)?;
        for node in nodes {
            writer.write_f32::<LE>(node.time)?;
            writer.write_f32::<LE>(node.low_value.unwrap_or(0.0))?;
            writer.write_f32::<LE>(node.high_value.unwrap_or(0.0))?;
            writer.write_i32::<LE>(node.curve_type.unwrap_or(1))?;
            writer.write_i32::<LE>(node.distribution.unwrap_or(1))?;
        }
    } else {
        writer.write_i32::<LE>(0)?;
    }
    Ok(())
}
