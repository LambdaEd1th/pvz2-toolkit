use byteorder::{ReadBytesExt, LE};
use serde::{Deserialize, Serialize};
use std::io::{Read, Seek, SeekFrom, Write};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PopfxError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid magic: expected xfcp")]
    InvalidMagic,
    #[error("Invalid version: expected {expected}, got {got}")]
    InvalidVersion { expected: u32, got: u32 },
    #[error("Invalid block size for block {block}: expected {expected}, got {got}")]
    InvalidBlockSize {
        block: usize,
        expected: u32,
        got: u32,
    },
    #[error("String encoding error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

type Result<T> = std::result::Result<T, PopfxError>;

#[derive(Serialize, Deserialize, Debug)]
pub struct PopcapRenderEffectObject {
    pub block_1: Vec<Block1>,
    pub block_2: Vec<Block2>,
    pub block_3: Vec<Block3>,
    pub block_4: Vec<Block4>,
    pub block_5: Vec<Block5>,
    pub block_6: Vec<Block6>,
    pub block_7: Vec<Block7>,
    pub block_8: Vec<Block8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Block1 {
    pub unknown_1: u32,
    pub unknown_2: u32,
    pub unknown_3: u32,
    pub unknown_4: u32,
    pub unknown_5: u32,
    pub unknown_6: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Block2 {
    pub unknown_1: u32,
    pub unknown_2: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Block3 {
    pub unknown_2: u32,
    pub string: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Block4 {
    pub unknown_1: u32,
    pub unknown_2: u32,
    pub unknown_3: u32,
    pub unknown_4: u32,
    pub unknown_5: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Block5 {
    pub unknown_1: u32,
    pub unknown_2: u32,
    pub unknown_3: u32,
    pub unknown_4: u32,
    pub unknown_5: u32,
    pub unknown_6: u32,
    pub unknown_7: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Block6 {
    pub unknown_1: u32,
    pub unknown_2: u32,
    pub unknown_3: u32,
    pub unknown_4: u32,
    pub unknown_5: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Block7 {
    pub unknown_1: u32,
    pub unknown_2: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Block8 {
    pub unknown_1: u32,
    pub unknown_2: u32,
    pub unknown_3: u32,
    pub unknown_4: u32,
    pub unknown_5: u32,
}

const BLOCK_SIZES: [u32; 8] = [0x18, 0x08, 0x0C, 0x14, 0x1C, 0x14, 0x08, 0x14];

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
    // Needed to read strings later
    // The structure is: Length, Unknown2, StringOffset (absolute or relative to string section?)
    // C# code:
    // StringSection = new SenBuffer(... stringSectionOffset)
    // for i... block3[i].string = StringSection.readStringByEmpty(readUInt32LE())
    // Wait, the block 3 struct in file has 3 u32s?
    // C# says loop: readUInt32LE(); block3[i] = { unknown_2 = read(); string = ...readStringByEmpty(read()) }
    // So 1st uint: length? (C# ignores it).
    // 2nd uint: unknown_2
    // 3rd uint: offset in string section?
    // Let's verify C#:
    // POPFXReader.readUInt32LE(); // Reads first uint (Length?)
    // block3[i] = new Block3 { unknown_2 = POPFXReader.readUInt32LE(), @string = StringSection.readStringByEmpty(POPFXReader.readUInt32LE()) }

    // So yes: [Unused/Len], [Unknown2], [StringOffset]

    // We need to read the string section first potentially, or just seek.
    // The string section starts at `string_section_offset`.
    // The offset in Block3 is relative to `string_section_offset`.

    // Let's read the whole string section into a buffer?
    // Calculate string section size?
    // C# reads: `POPFXReader.getBytes((int)(POPFXReader.length - POPFXHeadInfo.StringSectionOffset), POPFXHeadInfo.StringSectionOffset)`
    // So it reads from `StringSectionOffset` to the end of file.

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

pub fn encode_popfx<W: Write + Seek>(
    _obj: &PopcapRenderEffectObject,
    _writer: &mut W,
) -> Result<()> {
    // Placeholder. User only asked for parsing, but I can implement structure if needed.
    // For now, let's focus on decode.
    Ok(())
}
