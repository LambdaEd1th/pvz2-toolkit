use byteorder::{LE, ReadBytesExt};
use std::io::{Read, Seek, SeekFrom};

use crate::error::Result;
use crate::types::*;

/// Read a length-prefixed UTF-8 string (i32 LE length + bytes).
fn read_string_by_i32<R: Read>(r: &mut R) -> Result<String> {
    let len = r.read_i32::<LE>()? as usize;
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    Ok(String::from_utf8_lossy(&buf).to_string())
}

/// Read a bool as a single byte (0 = false, non-0 = true).
fn read_bool<R: Read>(r: &mut R) -> Result<bool> {
    Ok(r.read_u8()? != 0)
}

pub fn decode<R: Read + Seek>(reader: &mut R) -> Result<CharacterFontWidget2> {
    // Skip the first 16 bytes (header / reserved).
    reader.seek(SeekFrom::Start(16))?;

    let mut cfw2 = CharacterFontWidget2::default();
    cfw2.ascent = reader.read_i32::<LE>()?;
    cfw2.ascent_padding = reader.read_i32::<LE>()?;
    cfw2.height = reader.read_i32::<LE>()?;
    cfw2.line_spacing_offset = reader.read_i32::<LE>()?;
    cfw2.initialized = read_bool(reader)?;
    cfw2.default_point_size = reader.read_i32::<LE>()?;

    // Character items
    let char_count = reader.read_u32::<LE>()? as usize;
    cfw2.character = Vec::with_capacity(char_count);
    for _ in 0..char_count {
        cfw2.character.push(CharacterItem {
            index: reader.read_u16::<LE>()?,
            value: reader.read_u16::<LE>()?,
        });
    }

    // Layers
    let layer_count = reader.read_u32::<LE>()? as usize;
    cfw2.layer = Vec::with_capacity(layer_count);
    for _ in 0..layer_count {
        let name = read_string_by_i32(reader)?;

        let tag_req_count = reader.read_u32::<LE>()? as usize;
        let mut tag_require = Vec::with_capacity(tag_req_count);
        for _ in 0..tag_req_count {
            tag_require.push(read_string_by_i32(reader)?);
        }

        let tag_exc_count = reader.read_u32::<LE>()? as usize;
        let mut tag_exclude = Vec::with_capacity(tag_exc_count);
        for _ in 0..tag_exc_count {
            tag_exclude.push(read_string_by_i32(reader)?);
        }

        let kerning_count = reader.read_u32::<LE>()? as usize;
        let mut kerning = Vec::with_capacity(kerning_count);
        for _ in 0..kerning_count {
            kerning.push(FontKerning {
                offset: reader.read_u16::<LE>()?,
                index: reader.read_u16::<LE>()?,
            });
        }

        let char_count = reader.read_u32::<LE>()? as usize;
        let mut characters = Vec::with_capacity(char_count);
        for _ in 0..char_count {
            characters.push(FontCharacter {
                index: reader.read_u16::<LE>()?,
                image_rect_x: reader.read_i32::<LE>()?,
                image_rect_y: reader.read_i32::<LE>()?,
                image_rect_width: reader.read_i32::<LE>()?,
                image_rect_height: reader.read_i32::<LE>()?,
                image_offset_x: reader.read_i32::<LE>()?,
                image_offset_y: reader.read_i32::<LE>()?,
                kerning_count: reader.read_u16::<LE>()?,
                kerning_first: reader.read_u16::<LE>()?,
                width: reader.read_i32::<LE>()?,
                order: reader.read_i32::<LE>()?,
            });
        }

        cfw2.layer.push(FontLayer {
            name,
            tag_require,
            tag_exclude,
            kerning,
            character: characters,
            multiply_red: reader.read_i32::<LE>()?,
            multiply_green: reader.read_i32::<LE>()?,
            multiply_blue: reader.read_i32::<LE>()?,
            multiply_alpha: reader.read_i32::<LE>()?,
            add_red: reader.read_i32::<LE>()?,
            add_green: reader.read_i32::<LE>()?,
            add_blue: reader.read_i32::<LE>()?,
            add_alpha: reader.read_i32::<LE>()?,
            image_file: read_string_by_i32(reader)?,
            draw_mode: reader.read_i32::<LE>()?,
            offset_x: reader.read_i32::<LE>()?,
            offset_y: reader.read_i32::<LE>()?,
            spacing: reader.read_i32::<LE>()?,
            minimum_point_size: reader.read_i32::<LE>()?,
            maximum_point_size: reader.read_i32::<LE>()?,
            point_size: reader.read_i32::<LE>()?,
            ascent: reader.read_i32::<LE>()?,
            ascent_padding: reader.read_i32::<LE>()?,
            height: reader.read_i32::<LE>()?,
            default_height: reader.read_i32::<LE>()?,
            line_spacing_offset: reader.read_i32::<LE>()?,
            base_order: reader.read_i32::<LE>()?,
        });
    }

    cfw2.source_file = read_string_by_i32(reader)?;
    cfw2.error_header = read_string_by_i32(reader)?;
    cfw2.point_size = reader.read_i32::<LE>()?;

    let tag_count = reader.read_u32::<LE>()? as usize;
    cfw2.tag = Vec::with_capacity(tag_count);
    for _ in 0..tag_count {
        cfw2.tag.push(read_string_by_i32(reader)?);
    }

    cfw2.scale = reader.read_f64::<LE>()?;
    cfw2.force_scaled_image_white = read_bool(reader)?;
    cfw2.activate_all_layer = read_bool(reader)?;

    Ok(cfw2)
}
