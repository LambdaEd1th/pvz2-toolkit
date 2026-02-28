use byteorder::{LE, WriteBytesExt};
use std::io::Write;

use crate::error::Result;
use crate::types::*;

/// Write a length-prefixed UTF-8 string (i32 LE length + bytes).
fn write_string_by_i32<W: Write>(w: &mut W, s: &str) -> Result<()> {
    w.write_i32::<LE>(s.len() as i32)?;
    w.write_all(s.as_bytes())?;
    Ok(())
}

/// Write a bool as a single byte.
fn write_bool<W: Write>(w: &mut W, v: bool) -> Result<()> {
    w.write_u8(if v { 1 } else { 0 })?;
    Ok(())
}

pub fn encode<W: Write>(writer: &mut W, cfw2: &CharacterFontWidget2) -> Result<()> {
    // 16 bytes of header (zeroed / reserved).
    writer.write_all(&[0u8; 16])?;

    writer.write_i32::<LE>(cfw2.ascent)?;
    writer.write_i32::<LE>(cfw2.ascent_padding)?;
    writer.write_i32::<LE>(cfw2.height)?;
    writer.write_i32::<LE>(cfw2.line_spacing_offset)?;
    write_bool(writer, cfw2.initialized)?;
    writer.write_i32::<LE>(cfw2.default_point_size)?;

    // Character items
    writer.write_u32::<LE>(cfw2.character.len() as u32)?;
    for ch in &cfw2.character {
        writer.write_u16::<LE>(ch.index)?;
        writer.write_u16::<LE>(ch.value)?;
    }

    // Layers
    writer.write_u32::<LE>(cfw2.layer.len() as u32)?;
    for layer in &cfw2.layer {
        write_string_by_i32(writer, &layer.name)?;

        writer.write_u32::<LE>(layer.tag_require.len() as u32)?;
        for t in &layer.tag_require {
            write_string_by_i32(writer, t)?;
        }

        writer.write_u32::<LE>(layer.tag_exclude.len() as u32)?;
        for t in &layer.tag_exclude {
            write_string_by_i32(writer, t)?;
        }

        writer.write_u32::<LE>(layer.kerning.len() as u32)?;
        for k in &layer.kerning {
            writer.write_u16::<LE>(k.offset)?;
            writer.write_u16::<LE>(k.index)?;
        }

        writer.write_u32::<LE>(layer.character.len() as u32)?;
        for ch in &layer.character {
            writer.write_u16::<LE>(ch.index)?;
            writer.write_i32::<LE>(ch.image_rect_x)?;
            writer.write_i32::<LE>(ch.image_rect_y)?;
            writer.write_i32::<LE>(ch.image_rect_width)?;
            writer.write_i32::<LE>(ch.image_rect_height)?;
            writer.write_i32::<LE>(ch.image_offset_x)?;
            writer.write_i32::<LE>(ch.image_offset_y)?;
            writer.write_u16::<LE>(ch.kerning_count)?;
            writer.write_u16::<LE>(ch.kerning_first)?;
            writer.write_i32::<LE>(ch.width)?;
            writer.write_i32::<LE>(ch.order)?;
        }

        writer.write_i32::<LE>(layer.multiply_red)?;
        writer.write_i32::<LE>(layer.multiply_green)?;
        writer.write_i32::<LE>(layer.multiply_blue)?;
        writer.write_i32::<LE>(layer.multiply_alpha)?;
        writer.write_i32::<LE>(layer.add_red)?;
        writer.write_i32::<LE>(layer.add_green)?;
        writer.write_i32::<LE>(layer.add_blue)?;
        writer.write_i32::<LE>(layer.add_alpha)?;
        write_string_by_i32(writer, &layer.image_file)?;
        writer.write_i32::<LE>(layer.draw_mode)?;
        writer.write_i32::<LE>(layer.offset_x)?;
        writer.write_i32::<LE>(layer.offset_y)?;
        writer.write_i32::<LE>(layer.spacing)?;
        writer.write_i32::<LE>(layer.minimum_point_size)?;
        writer.write_i32::<LE>(layer.maximum_point_size)?;
        writer.write_i32::<LE>(layer.point_size)?;
        writer.write_i32::<LE>(layer.ascent)?;
        writer.write_i32::<LE>(layer.ascent_padding)?;
        writer.write_i32::<LE>(layer.height)?;
        writer.write_i32::<LE>(layer.default_height)?;
        writer.write_i32::<LE>(layer.line_spacing_offset)?;
        writer.write_i32::<LE>(layer.base_order)?;
    }

    write_string_by_i32(writer, &cfw2.source_file)?;
    write_string_by_i32(writer, &cfw2.error_header)?;
    writer.write_i32::<LE>(cfw2.point_size)?;

    writer.write_u32::<LE>(cfw2.tag.len() as u32)?;
    for t in &cfw2.tag {
        write_string_by_i32(writer, t)?;
    }

    writer.write_f64::<LE>(cfw2.scale)?;
    write_bool(writer, cfw2.force_scaled_image_white)?;
    write_bool(writer, cfw2.activate_all_layer)?;

    Ok(())
}
