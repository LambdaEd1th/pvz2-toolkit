use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom, Write};

use crate::error::{PopfxError, Result};
use crate::types::{
    BLOCK_SIZES, Block1, Block2, Block3, Block4, Block5, Block6, Block7, Block8,
    PopcapRenderEffectObject,
};

pub fn decode_popfx<R: Read + Seek>(reader: &mut R) -> Result<PopcapRenderEffectObject> {
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic)?;
    if &magic != b"xfcp" {
        return Err(PopfxError::InvalidMagic);
    }

    let version = reader.read_u32::<LE>()?;
    if version != 1 {
        return Err(PopfxError::InvalidVersion {
            expected: 1,
            got: version,
        });
    }

    let mut counts = [0u32; 8];
    let mut offsets = [0u32; 8];

    // Read header info
    // Block 1
    counts[0] = reader.read_u32::<LE>()?;
    offsets[0] = reader.read_u32::<LE>()?;
    let size = reader.read_u32::<LE>()?;
    if size != BLOCK_SIZES[0] {
        return Err(PopfxError::InvalidBlockSize {
            block: 1,
            expected: BLOCK_SIZES[0],
            got: size,
        });
    }

    // Block 2
    counts[1] = reader.read_u32::<LE>()?;
    offsets[1] = reader.read_u32::<LE>()?;
    let size = reader.read_u32::<LE>()?;
    if size != BLOCK_SIZES[1] {
        return Err(PopfxError::InvalidBlockSize {
            block: 2,
            expected: BLOCK_SIZES[1],
            got: size,
        });
    }

    // Block 3
    counts[2] = reader.read_u32::<LE>()?;
    offsets[2] = reader.read_u32::<LE>()?;
    let size = reader.read_u32::<LE>()?;
    if size != BLOCK_SIZES[2] {
        return Err(PopfxError::InvalidBlockSize {
            block: 3,
            expected: BLOCK_SIZES[2],
            got: size,
        });
    }

    // Block 4
    counts[3] = reader.read_u32::<LE>()?;
    offsets[3] = reader.read_u32::<LE>()?;
    let size = reader.read_u32::<LE>()?;
    if size != BLOCK_SIZES[3] {
        return Err(PopfxError::InvalidBlockSize {
            block: 4,
            expected: BLOCK_SIZES[3],
            got: size,
        });
    }

    // Block 5
    counts[4] = reader.read_u32::<LE>()?;
    offsets[4] = reader.read_u32::<LE>()?;
    let size = reader.read_u32::<LE>()?;
    if size != BLOCK_SIZES[4] {
        return Err(PopfxError::InvalidBlockSize {
            block: 5,
            expected: BLOCK_SIZES[4],
            got: size,
        });
    }

    // Block 6
    counts[5] = reader.read_u32::<LE>()?;
    offsets[5] = reader.read_u32::<LE>()?;
    let size = reader.read_u32::<LE>()?;
    if size != BLOCK_SIZES[5] {
        return Err(PopfxError::InvalidBlockSize {
            block: 6,
            expected: BLOCK_SIZES[5],
            got: size,
        });
    }

    // Block 7
    counts[6] = reader.read_u32::<LE>()?;
    offsets[6] = reader.read_u32::<LE>()?;
    let size = reader.read_u32::<LE>()?;
    if size != BLOCK_SIZES[6] {
        return Err(PopfxError::InvalidBlockSize {
            block: 7,
            expected: BLOCK_SIZES[6],
            got: size,
        });
    }

    // Block 8
    counts[7] = reader.read_u32::<LE>()?;
    offsets[7] = reader.read_u32::<LE>()?;
    let size = reader.read_u32::<LE>()?;
    if size != BLOCK_SIZES[7] {
        return Err(PopfxError::InvalidBlockSize {
            block: 8,
            expected: BLOCK_SIZES[7],
            got: size,
        });
    }

    let string_section_offset = reader.read_u32::<LE>()?;

    // Helper to jump and read

    // Block 1
    let mut block_1 = Vec::with_capacity(counts[0] as usize);
    if counts[0] > 0 {
        reader.seek(SeekFrom::Start(offsets[0] as u64))?;
        for _ in 0..counts[0] {
            block_1.push(Block1 {
                unknown_1: reader.read_u32::<LE>()?,
                unknown_2: reader.read_u32::<LE>()?,
                unknown_3: reader.read_u32::<LE>()?,
                unknown_4: reader.read_u32::<LE>()?,
                unknown_5: reader.read_u32::<LE>()?,
                unknown_6: reader.read_u32::<LE>()?,
            });
        }
    }

    // Block 2
    let mut block_2 = Vec::with_capacity(counts[1] as usize);
    if counts[1] > 0 {
        reader.seek(SeekFrom::Start(offsets[1] as u64))?;
        for _ in 0..counts[1] {
            block_2.push(Block2 {
                unknown_1: reader.read_u32::<LE>()?,
                unknown_2: reader.read_u32::<LE>()?,
            });
        }
    }

    // Block 3
    let mut block_3 = Vec::with_capacity(counts[2] as usize);

    let file_len = reader.seek(SeekFrom::End(0))?;
    let string_section_len = file_len - string_section_offset as u64;
    reader.seek(SeekFrom::Start(string_section_offset as u64))?;
    let mut string_section = vec![0u8; string_section_len as usize];
    reader.read_exact(&mut string_section)?;

    if counts[2] > 0 {
        reader.seek(SeekFrom::Start(offsets[2] as u64))?;
        for _ in 0..counts[2] {
            let _len = reader.read_u32::<LE>()?;
            let unknown_2 = reader.read_u32::<LE>()?;
            let str_offset = reader.read_u32::<LE>()?;

            // Read string from string_section
            let s = read_string_from_buffer(&string_section, str_offset as usize)?;
            block_3.push(Block3 {
                unknown_2,
                string: s,
            });
        }
    }

    // Block 4
    let mut block_4 = Vec::with_capacity(counts[3] as usize);
    if counts[3] > 0 {
        reader.seek(SeekFrom::Start(offsets[3] as u64))?;
        for _ in 0..counts[3] {
            block_4.push(Block4 {
                unknown_1: reader.read_u32::<LE>()?,
                unknown_2: reader.read_u32::<LE>()?,
                unknown_3: reader.read_u32::<LE>()?,
                unknown_4: reader.read_u32::<LE>()?,
                unknown_5: reader.read_u32::<LE>()?,
            });
        }
    }

    // Block 5
    let mut block_5 = Vec::with_capacity(counts[4] as usize);
    if counts[4] > 0 {
        reader.seek(SeekFrom::Start(offsets[4] as u64))?;
        for _ in 0..counts[4] {
            block_5.push(Block5 {
                unknown_1: reader.read_u32::<LE>()?,
                unknown_2: reader.read_u32::<LE>()?,
                unknown_3: reader.read_u32::<LE>()?,
                unknown_4: reader.read_u32::<LE>()?,
                unknown_5: reader.read_u32::<LE>()?,
                unknown_6: reader.read_u32::<LE>()?,
                unknown_7: reader.read_u32::<LE>()?,
            });
        }
    }

    // Block 6
    let mut block_6 = Vec::with_capacity(counts[5] as usize);
    if counts[5] > 0 {
        reader.seek(SeekFrom::Start(offsets[5] as u64))?;
        for _ in 0..counts[5] {
            block_6.push(Block6 {
                unknown_1: reader.read_u32::<LE>()?,
                unknown_2: reader.read_u32::<LE>()?,
                unknown_3: reader.read_u32::<LE>()?,
                unknown_4: reader.read_u32::<LE>()?,
                unknown_5: reader.read_u32::<LE>()?,
            });
        }
    }

    // Block 7
    let mut block_7 = Vec::with_capacity(counts[6] as usize);
    if counts[6] > 0 {
        reader.seek(SeekFrom::Start(offsets[6] as u64))?;
        for _ in 0..counts[6] {
            block_7.push(Block7 {
                unknown_1: reader.read_u32::<LE>()?,
                unknown_2: reader.read_u32::<LE>()?,
            });
        }
    }

    // Block 8
    let mut block_8 = Vec::with_capacity(counts[7] as usize);
    if counts[7] > 0 {
        reader.seek(SeekFrom::Start(offsets[7] as u64))?;
        for _ in 0..counts[7] {
            block_8.push(Block8 {
                unknown_1: reader.read_u32::<LE>()?,
                unknown_2: reader.read_u32::<LE>()?,
                unknown_3: reader.read_u32::<LE>()?,
                unknown_4: reader.read_u32::<LE>()?,
                unknown_5: reader.read_u32::<LE>()?,
            });
        }
    }

    Ok(PopcapRenderEffectObject {
        block_1,
        block_2,
        block_3,
        block_4,
        block_5,
        block_6,
        block_7,
        block_8,
    })
}

fn read_string_from_buffer(buf: &[u8], offset: usize) -> Result<String> {
    if offset >= buf.len() {
        return Ok(String::new()); // Or error?
    }
    let mut end = offset;
    while end < buf.len() && buf[end] != 0 {
        end += 1;
    }
    let s = String::from_utf8(buf[offset..end].to_vec())?;
    Ok(s)
}

pub fn encode_popfx<W: Write + Seek>(obj: &PopcapRenderEffectObject, writer: &mut W) -> Result<()> {
    // 1. Gather Strings from Block 3
    let mut string_map = HashMap::new();
    let mut string_section = Vec::new();

    for b3 in &obj.block_3 {
        if !string_map.contains_key(&b3.string) {
            let offset = string_section.len() as u32;
            string_section.extend_from_slice(b3.string.as_bytes());
            string_section.push(0); // Null terminator
            string_map.insert(b3.string.clone(), offset);
        }
    }

    // 2. Calculate Offsets and Counts
    // Header size: 4 (Magic) + 4 (Version) + 8 * (4+4+4) (Block Table) + 4 (StringSectionOffset)
    // = 8 + 96 + 4 = 108 bytes.

    let header_size = 108u32;
    let mut current_offset = header_size;

    let mut counts = [0u32; 8];
    let mut offsets = [0u32; 8];
    let sizes = BLOCK_SIZES; // Copy

    // Block 1
    counts[0] = obj.block_1.len() as u32;
    if counts[0] > 0 {
        offsets[0] = current_offset;
        current_offset += counts[0] * sizes[0];
    }

    // Block 2
    counts[1] = obj.block_2.len() as u32;
    if counts[1] > 0 {
        offsets[1] = current_offset;
        current_offset += counts[1] * sizes[1];
    }

    // Block 3
    counts[2] = obj.block_3.len() as u32;
    if counts[2] > 0 {
        offsets[2] = current_offset;
        current_offset += counts[2] * sizes[2];
    }

    // Block 4
    counts[3] = obj.block_4.len() as u32;
    if counts[3] > 0 {
        offsets[3] = current_offset;
        current_offset += counts[3] * sizes[3];
    }

    // Block 5
    counts[4] = obj.block_5.len() as u32;
    if counts[4] > 0 {
        offsets[4] = current_offset;
        current_offset += counts[4] * sizes[4];
    }

    // Block 6
    counts[5] = obj.block_6.len() as u32;
    if counts[5] > 0 {
        offsets[5] = current_offset;
        current_offset += counts[5] * sizes[5];
    }

    // Block 7
    counts[6] = obj.block_7.len() as u32;
    if counts[6] > 0 {
        offsets[6] = current_offset;
        current_offset += counts[6] * sizes[6];
    }

    // Block 8
    counts[7] = obj.block_8.len() as u32;
    if counts[7] > 0 {
        offsets[7] = current_offset;
        current_offset += counts[7] * sizes[7];
    }

    let string_section_offset = current_offset;

    // 3. Write Header
    writer.write_all(b"xfcp")?;
    writer.write_u32::<LE>(1)?; // Version

    for i in 0..8 {
        writer.write_u32::<LE>(counts[i])?;
        writer.write_u32::<LE>(offsets[i])?;
        writer.write_u32::<LE>(sizes[i])?;
    }

    writer.write_u32::<LE>(string_section_offset)?;

    // 4. Write Blocks

    // Block 1
    if counts[0] > 0 {
        writer.seek(SeekFrom::Start(offsets[0] as u64))?;
        for b in &obj.block_1 {
            writer.write_u32::<LE>(b.unknown_1)?;
            writer.write_u32::<LE>(b.unknown_2)?;
            writer.write_u32::<LE>(b.unknown_3)?;
            writer.write_u32::<LE>(b.unknown_4)?;
            writer.write_u32::<LE>(b.unknown_5)?;
            writer.write_u32::<LE>(b.unknown_6)?;
        }
    }

    // Block 2
    if counts[1] > 0 {
        writer.seek(SeekFrom::Start(offsets[1] as u64))?;
        for b in &obj.block_2 {
            writer.write_u32::<LE>(b.unknown_1)?;
            writer.write_u32::<LE>(b.unknown_2)?;
        }
    }

    // Block 3
    if counts[2] > 0 {
        writer.seek(SeekFrom::Start(offsets[2] as u64))?;
        for b in &obj.block_3 {
            writer.write_u32::<LE>(b.string.len() as u32)?;
            writer.write_u32::<LE>(b.unknown_2)?;
            let offset = string_map.get(&b.string).unwrap_or(&0);
            writer.write_u32::<LE>(*offset)?;
        }
    }

    // Block 4
    if counts[3] > 0 {
        writer.seek(SeekFrom::Start(offsets[3] as u64))?;
        for b in &obj.block_4 {
            writer.write_u32::<LE>(b.unknown_1)?;
            writer.write_u32::<LE>(b.unknown_2)?;
            writer.write_u32::<LE>(b.unknown_3)?;
            writer.write_u32::<LE>(b.unknown_4)?;
            writer.write_u32::<LE>(b.unknown_5)?;
        }
    }

    // Block 5
    if counts[4] > 0 {
        writer.seek(SeekFrom::Start(offsets[4] as u64))?;
        for b in &obj.block_5 {
            writer.write_u32::<LE>(b.unknown_1)?;
            writer.write_u32::<LE>(b.unknown_2)?;
            writer.write_u32::<LE>(b.unknown_3)?;
            writer.write_u32::<LE>(b.unknown_4)?;
            writer.write_u32::<LE>(b.unknown_5)?;
            writer.write_u32::<LE>(b.unknown_6)?;
            writer.write_u32::<LE>(b.unknown_7)?;
        }
    }

    // Block 6
    if counts[5] > 0 {
        writer.seek(SeekFrom::Start(offsets[5] as u64))?;
        for b in &obj.block_6 {
            writer.write_u32::<LE>(b.unknown_1)?;
            writer.write_u32::<LE>(b.unknown_2)?;
            writer.write_u32::<LE>(b.unknown_3)?;
            writer.write_u32::<LE>(b.unknown_4)?;
            writer.write_u32::<LE>(b.unknown_5)?;
        }
    }

    // Block 7
    if counts[6] > 0 {
        writer.seek(SeekFrom::Start(offsets[6] as u64))?;
        for b in &obj.block_7 {
            writer.write_u32::<LE>(b.unknown_1)?;
            writer.write_u32::<LE>(b.unknown_2)?;
        }
    }

    // Block 8
    if counts[7] > 0 {
        writer.seek(SeekFrom::Start(offsets[7] as u64))?;
        for b in &obj.block_8 {
            writer.write_u32::<LE>(b.unknown_1)?;
            writer.write_u32::<LE>(b.unknown_2)?;
            writer.write_u32::<LE>(b.unknown_3)?;
            writer.write_u32::<LE>(b.unknown_4)?;
            writer.write_u32::<LE>(b.unknown_5)?;
        }
    }

    // 5. Write String Section
    writer.seek(SeekFrom::Start(string_section_offset as u64))?;
    writer.write_all(&string_section)?;

    Ok(())
}
