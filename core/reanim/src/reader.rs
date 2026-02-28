use crate::error::ReanimError;
use crate::types::{Reanim, ReanimTrack, ReanimTransform};
use byteorder::{LE, ReadBytesExt};
use flate2::read::ZlibDecoder;
use std::io::{Cursor, Read};

/// Helper to read a length-prefixed string (length is i32 LE).
fn read_string_by_int32<R: Read>(reader: &mut R) -> Result<String, ReanimError> {
    let len = reader.read_i32::<LE>()?;
    if len <= 0 {
        return Ok(String::new());
    }
    let mut buf = vec![0u8; len as usize];
    reader.read_exact(&mut buf)?;
    String::from_utf8(buf).map_err(|_| ReanimError::StringDecodeError)
}

/// Helper to read an optionally empty null-terminated or exact length string if we wanted,
/// but C# `readString(len)` uses the exact length.
fn read_exact_string<R: Read>(reader: &mut R, len: usize) -> Result<String, ReanimError> {
    if len == 0 {
        return Ok(String::new());
    }
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;
    // the C# code might include a null terminator in the length, let's strip it if it exists
    let mut end = len;
    while end > 0 && buf[end - 1] == 0 {
        end -= 1;
    }
    String::from_utf8(buf[..end].to_vec()).map_err(|_| ReanimError::StringDecodeError)
}

pub fn decode_pc(data: &[u8]) -> Result<Reanim, ReanimError> {
    let mut cursor = Cursor::new(data);
    let first_int = cursor.read_i32::<LE>()?;

    let mut uncompressed_data = Vec::new();
    let mut reader: Box<dyn Read> = if first_int == -559022380 {
        // PopCapZlib magic 0xDEADBEE4 (-559022380 in signed i32)
        // Skip next 4 bytes (size)
        cursor.read_i32::<LE>()?;
        let mut decoder = ZlibDecoder::new(cursor);
        decoder.read_to_end(&mut uncompressed_data)?;
        Box::new(Cursor::new(&uncompressed_data))
    } else {
        cursor.set_position(0);
        Box::new(cursor)
    };

    // SenFile.readOffset = 8;
    // We already read 8 bytes or not, but wait, the C# code does:
    // SenFile.readOffset = 8;
    // That means it *skips* the first 8 bytes of the UNCOMPRESSED data (or raw data if not compressed).
    let mut unused = [0u8; 8];
    reader.read_exact(&mut unused)?;

    let mut reanim = Reanim::default();
    let tracks_number = reader.read_i32::<LE>()?;
    reanim.fps = reader.read_f32::<LE>()?;

    // SenFile.readOffset += 4;
    let mut unused_4 = [0u8; 4];
    reader.read_exact(&mut unused_4)?;

    if reader.read_i32::<LE>()? != 0x0C {
        return Err(ReanimError::InvalidMagic(0x0C, 0));
    }

    for _ in 0..tracks_number {
        // SenFile.readOffset += 8;
        let mut unused_8 = [0u8; 8];
        reader.read_exact(&mut unused_8)?;
        let transform_count = reader.read_i32::<LE>()?;

        reanim.tracks.push(ReanimTrack {
            name: String::new(), // Set later
            transforms: vec![ReanimTransform::default(); transform_count as usize],
        });
    }

    for i in 0..tracks_number as usize {
        let name_len = reader.read_i32::<LE>()?;
        reanim.tracks[i].name = read_exact_string(&mut reader, name_len as usize)?;

        if reader.read_i32::<LE>()? != 0x2C {
            return Err(ReanimError::InvalidTrack);
        }

        let times = reanim.tracks[i].transforms.len();
        for k in 0..times {
            let mut ts = ReanimTransform::default();
            let mut read_opt = || -> Result<Option<f32>, ReanimError> {
                let v = reader.read_f32::<LE>()?;
                if v != -10000.0 { Ok(Some(v)) } else { Ok(None) }
            };

            ts.x = read_opt()?;
            ts.y = read_opt()?;
            ts.kx = read_opt()?;
            ts.ky = read_opt()?;
            ts.sx = read_opt()?;
            ts.sy = read_opt()?;
            ts.f = read_opt()?;
            ts.a = read_opt()?;

            // SenFile.readOffset += 12;
            let mut unused_12 = [0u8; 12];
            reader.read_exact(&mut unused_12)?;

            reanim.tracks[i].transforms[k] = ts;
        }

        for k in 0..times {
            let ts = &mut reanim.tracks[i].transforms[k];

            let i_str = read_string_by_int32(&mut reader)?;
            if !i_str.is_empty() {
                ts.i = Some(i_str);
            }

            let font = read_string_by_int32(&mut reader)?;
            if !font.is_empty() {
                ts.font = Some(font);
            }

            let text = read_string_by_int32(&mut reader)?;
            if !text.is_empty() {
                ts.text = Some(text);
            }
        }
    }

    Ok(reanim)
}

pub fn decode_phone32(data: &[u8]) -> Result<Reanim, ReanimError> {
    let mut cursor = Cursor::new(data);
    let first_int = cursor.read_i32::<LE>()?;

    let mut uncompressed_data = Vec::new();
    let mut reader: Box<dyn Read> = if first_int == -559022380 {
        cursor.read_i32::<LE>()?;
        let mut decoder = ZlibDecoder::new(cursor);
        decoder.read_to_end(&mut uncompressed_data)?;
        Box::new(Cursor::new(&uncompressed_data))
    } else {
        cursor.set_position(0);
        Box::new(cursor)
    };

    let mut unused = [0u8; 8];
    reader.read_exact(&mut unused)?;

    let mut reanim = Reanim::default();
    let tracks_number = reader.read_i32::<LE>()?;
    reanim.fps = reader.read_f32::<LE>()?;

    let mut unused_4 = [0u8; 4];
    reader.read_exact(&mut unused_4)?;

    if reader.read_i32::<LE>()? != 0x10 {
        return Err(ReanimError::InvalidMagic(0x10, 0));
    }

    for _ in 0..tracks_number {
        let mut unused_12 = [0u8; 12];
        reader.read_exact(&mut unused_12)?;
        let transform_count = reader.read_i32::<LE>()?;

        reanim.tracks.push(ReanimTrack {
            name: String::new(),
            transforms: vec![ReanimTransform::default(); transform_count as usize],
        });
    }

    for i in 0..tracks_number as usize {
        let name_len = reader.read_i32::<LE>()?;
        reanim.tracks[i].name = read_exact_string(&mut reader, name_len as usize)?;

        if reader.read_i32::<LE>()? != 0x2C {
            return Err(ReanimError::InvalidTrack);
        }

        let times = reanim.tracks[i].transforms.len();
        for k in 0..times {
            let mut ts = ReanimTransform::default();
            let mut read_opt = || -> Result<Option<f32>, ReanimError> {
                let v = reader.read_f32::<LE>()?;
                if v != -10000.0 { Ok(Some(v)) } else { Ok(None) }
            };

            ts.x = read_opt()?;
            ts.y = read_opt()?;
            ts.kx = read_opt()?;
            ts.ky = read_opt()?;
            ts.sx = read_opt()?;
            ts.sy = read_opt()?;
            ts.f = read_opt()?;
            ts.a = read_opt()?;

            let mut unused_12 = [0u8; 12];
            reader.read_exact(&mut unused_12)?;
            reanim.tracks[i].transforms[k] = ts;
        }

        for k in 0..times {
            let ts = &mut reanim.tracks[i].transforms[k];

            let i_int = reader.read_i32::<LE>()?;
            if i_int != -1 {
                ts.i = Some(i_int.to_string());
            }

            let font = read_string_by_int32(&mut reader)?;
            if !font.is_empty() {
                ts.font = Some(font);
            }

            let text = read_string_by_int32(&mut reader)?;
            if !text.is_empty() {
                ts.text = Some(text);
            }
        }
    }

    Ok(reanim)
}

pub fn decode_phone64(data: &[u8]) -> Result<Reanim, ReanimError> {
    let mut cursor = Cursor::new(data);
    let first_int = cursor.read_i32::<LE>()?;

    let mut uncompressed_data = Vec::new();
    let mut reader: Box<dyn Read> = if first_int == -559022380 {
        // rawFile.readOffset += 8;
        let mut unused_4 = [0u8; 4];
        cursor.read_exact(&mut unused_4)?; // skipping the +4 we already got from first_int, to equal 8 total
        // let size = rawFile.readInt32LE();
        cursor.read_i32::<LE>()?;
        // rawFile.readOffset += 4;
        cursor.read_exact(&mut unused_4)?;

        let mut decoder = ZlibDecoder::new(cursor);
        decoder.read_to_end(&mut uncompressed_data)?;
        Box::new(Cursor::new(&uncompressed_data))
    } else {
        cursor.set_position(0);
        Box::new(cursor)
    };

    // SenFile.readOffset = 12;
    let mut unused_12 = [0u8; 12];
    reader.read_exact(&mut unused_12)?;

    let mut reanim = Reanim::default();
    let tracks_number = reader.read_i32::<LE>()?;
    reanim.fps = reader.read_f32::<LE>()?;

    let mut unused_8 = [0u8; 8];
    reader.read_exact(&mut unused_8)?;

    if reader.read_i32::<LE>()? != 0x20 {
        return Err(ReanimError::InvalidMagic(0x20, 0));
    }

    for _ in 0..tracks_number {
        let mut unused_24 = [0u8; 24];
        reader.read_exact(&mut unused_24)?;
        let size = reader.read_i32::<LE>()?;
        let mut unused_4 = [0u8; 4];
        reader.read_exact(&mut unused_4)?;

        reanim.tracks.push(ReanimTrack {
            name: String::new(),
            transforms: vec![ReanimTransform::default(); size as usize],
        });
    }

    for i in 0..tracks_number as usize {
        let name_len = reader.read_i32::<LE>()?;
        reanim.tracks[i].name = read_exact_string(&mut reader, name_len as usize)?;

        if reader.read_i32::<LE>()? != 0x38 {
            return Err(ReanimError::InvalidTrack);
        }

        let times = reanim.tracks[i].transforms.len();
        for k in 0..times {
            let mut ts = ReanimTransform::default();
            let mut read_opt = || -> Result<Option<f32>, ReanimError> {
                let v = reader.read_f32::<LE>()?;
                if v != -10000.0 { Ok(Some(v)) } else { Ok(None) }
            };

            ts.x = read_opt()?;
            ts.y = read_opt()?;
            ts.kx = read_opt()?;
            ts.ky = read_opt()?;
            ts.sx = read_opt()?;
            ts.sy = read_opt()?;
            ts.f = read_opt()?;
            ts.a = read_opt()?;

            let mut unused_24 = [0u8; 24];
            reader.read_exact(&mut unused_24)?;
            reanim.tracks[i].transforms[k] = ts;
        }

        for k in 0..times {
            let ts = &mut reanim.tracks[i].transforms[k];

            let i_int = reader.read_i32::<LE>()?;
            if i_int != -1 {
                ts.i = Some(i_int.to_string());
            }

            let font = read_string_by_int32(&mut reader)?;
            if !font.is_empty() {
                ts.font = Some(font);
            }

            let text = read_string_by_int32(&mut reader)?;
            if !text.is_empty() {
                ts.text = Some(text);
            }
        }
    }

    Ok(reanim)
}

pub fn decode(data: &[u8]) -> Result<crate::types::Reanim, ReanimError> {
    // Try each in sequence like the C# code does
    if let Ok(r) = decode_pc(data) {
        return Ok(r);
    }
    if let Ok(r) = decode_phone32(data) {
        return Ok(r);
    }
    if let Ok(r) = decode_phone64(data) {
        return Ok(r);
    }
    Err(ReanimError::InvalidVariant)
}
