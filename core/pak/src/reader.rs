use crate::{PakError, PakInfo, PakRecord, Result};
use byteorder::{LE, ReadBytesExt};
use flate2::read::ZlibDecoder;
use std::io::{Read, Seek, SeekFrom};

/// The standard PAK magic number written by C# as `writeInt32LE(-1161803072)`.
/// In LE bytes: `C0 4A C0 BA`.
const PAK_MAGIC: u32 = (-1161803072i32) as u32; // 0xBAC04AC0

/// The XOR'd magic that appears in PC-encrypted PAK files.
/// `PAK_MAGIC ^ 0xF7F7F7F7` => bytes `37 BD 37 4D`, read as LE u32 => `0x4D37BD37`.
const PC_MAGIC: u32 = PAK_MAGIC ^ 0xF7F7F7F7;

/// ZIP magic for TV platform PAK files.
const TV_MAGIC: u32 = 0x04034B50;

pub fn unpack<R: Read + Seek>(mut reader: R) -> Result<(PakInfo, Vec<PakRecord>)> {
    // Read the entire file into memory first.
    reader.seek(SeekFrom::Start(0))?;
    let mut raw = Vec::new();
    reader.read_to_end(&mut raw)?;

    if raw.len() < 4 {
        return Err(PakError::InvalidMagic(PAK_MAGIC, 0));
    }

    // Peek at the first 4 bytes (LE u32) to determine the platform.
    let peek_magic = u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]);

    let mut pak_platform: String;
    let is_tv;

    if peek_magic == PC_MAGIC {
        pak_platform = "PC".to_string();
        is_tv = false;
        // XOR the entire buffer to decrypt.
        for b in raw.iter_mut() {
            *b ^= 0xF7;
        }
    } else if peek_magic == PAK_MAGIC {
        pak_platform = "Xbox360".to_string();
        is_tv = false;
    } else if peek_magic == TV_MAGIC {
        pak_platform = "TV".to_string();
        is_tv = true;
    } else {
        return Err(PakError::InvalidMagic(PAK_MAGIC, peek_magic));
    }

    if is_tv {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "TV PAK (ZIP) unpacking is not supported natively",
        )
        .into());
    }

    // Now `raw` is the decrypted data starting with PAK_MAGIC.
    let mut cursor = std::io::Cursor::new(raw);

    let pak_magic = cursor.read_u32::<LE>()?;
    if pak_magic != PAK_MAGIC {
        return Err(PakError::InvalidMagic(PAK_MAGIC, pak_magic));
    }

    let pak_version = cursor.read_u32::<LE>()?;
    if pak_version != 0x0 {
        return Err(PakError::InvalidVersion(0x0, pak_version));
    }

    // --- Parse file info directory ---

    struct FileEntry {
        name: String,
        size: u32,
        zlib_size: Option<u32>,
    }

    let mut entries = Vec::new();
    let mut zlib_compress: Option<bool> = None;

    loop {
        let marker = cursor.read_u8()?;
        if marker == 0x80 {
            break;
        }

        let name_len = cursor.read_u8()? as usize;
        let mut name_bytes = vec![0u8; name_len];
        cursor.read_exact(&mut name_bytes)?;
        let name = String::from_utf8_lossy(&name_bytes).to_string();

        let size = cursor.read_u32::<LE>()?;

        // Determine compression on the first entry using the C# lookahead:
        // From the current position (after `size`), peek 12 bytes ahead.
        // If compressed: next 12 bytes = zlib_size(4) + file_time(8), then the marker byte.
        // If uncompressed: next 8 bytes = file_time(8), then marker, but 12 bytes overshoots by 4.
        // So at +12, a compressed file hits the next marker (0x0 or 0x80), uncompressed doesn't.
        if zlib_compress.is_none() {
            let saved = cursor.position();
            if cursor.seek(SeekFrom::Current(12)).is_ok() {
                if let Ok(bp) = cursor.read_u8() {
                    zlib_compress = Some(bp == 0x0 || bp == 0x80);
                } else {
                    zlib_compress = Some(false);
                }
            } else {
                zlib_compress = Some(false);
            }
            cursor.seek(SeekFrom::Start(saved))?;
        }

        let is_compressed = zlib_compress.unwrap_or(false);
        let zlib_size = if is_compressed {
            Some(cursor.read_u32::<LE>()?)
        } else {
            None
        };

        let _file_time = cursor.read_u64::<LE>()?;

        entries.push(FileEntry {
            name,
            size,
            zlib_size,
        });
    }

    // --- Detect path separator ---
    let mut windows_path_separate = true;
    for entry in &entries {
        if entry.name.contains('/') {
            windows_path_separate = false;
            break;
        }
        if entry.name.contains('\\') {
            break;
        }
    }

    let is_zlib = zlib_compress.unwrap_or(false);

    // --- Read file payloads ---
    let mut records = Vec::new();

    for entry in entries {
        if pak_platform != "PC" {
            // Skip alignment padding (u16 length + padding bytes).
            let jmp = cursor.read_u16::<LE>()? as i64;
            cursor.seek(SeekFrom::Current(jmp))?;
            if jmp > 8 {
                pak_platform = "Xbox360".to_string();
            }
        }

        let read_size = entry.zlib_size.unwrap_or(entry.size) as usize;
        let mut file_data = vec![0u8; read_size];
        cursor.read_exact(&mut file_data)?;

        if is_zlib && entry.zlib_size.is_some() {
            let mut decoder = ZlibDecoder::new(&file_data[..]);
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed)?;
            file_data = decompressed;
        }

        records.push(PakRecord {
            path: entry.name,
            data: file_data,
        });
    }

    Ok((
        PakInfo {
            pak_platform,
            pak_use_windows_path_separate: windows_path_separate,
            pak_use_zlib_compress: is_zlib,
        },
        records,
    ))
}
