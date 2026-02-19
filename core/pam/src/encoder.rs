use crate::types::*;
use anyhow::Result;
use byteorder::{LE, WriteBytesExt};
use std::io::Write;

pub fn encode_pam<W: Write>(pam: &PamInfo, writer: &mut W) -> Result<()> {
    writer.write_u32::<LE>(PAM_MAGIC)?;
    writer.write_i32::<LE>(pam.version)?;
    writer.write_u8(pam.frame_rate as u8)?;

    for p in &pam.position {
        writer.write_u16::<LE>((p * 20.0) as u16)?;
    }

    for s in &pam.size {
        writer.write_u16::<LE>((s * 20.0) as u16)?;
    }

    writer.write_u16::<LE>(pam.image.len() as u16)?;
    for img in &pam.image {
        write_image_info(img, writer, pam.version)?;
    }

    writer.write_u16::<LE>(pam.sprite.len() as u16)?;
    for sprite in &pam.sprite {
        write_sprite_info(sprite, writer, pam.version)?;
    }

    if pam.version > 3 {
        // Check if main_sprite is arguably "empty" or default?
        // decode_pam logic:
        // if version <= 3 { has_main = true } else { has_main = read_bool }
        // We should write true if we have a main sprite.
        // The struct always has main_sprite (default if not present).
        // Let's assume we always write it if version > 3 for consistency with "repacking",
        // unless we can detect it's invalid.
        // For now, always write true.
        writer.write_u8(1)?; // has_main_sprite = true
        write_sprite_info(&pam.main_sprite, writer, pam.version)?;
    } else {
        // Version <= 3 always has main sprite implicit
        write_sprite_info(&pam.main_sprite, writer, pam.version)?;
    }

    Ok(())
}

fn write_string_by_u16<W: Write>(s: &str, writer: &mut W) -> Result<()> {
    let bytes = s.as_bytes();
    writer.write_u16::<LE>(bytes.len() as u16)?;
    writer.write_all(bytes)?;
    Ok(())
}

fn write_image_info<W: Write>(img: &ImageInfo, writer: &mut W, version: i32) -> Result<()> {
    write_string_by_u16(&img.name, writer)?;

    if version >= 4 {
        for s in &img.size {
            writer.write_u16::<LE>(*s as u16)?;
        }
    }

    if version == 1 {
        // transform: [cos, sin, -sin, cos, tx, ty] roughly
        // decode:
        // num = read_u16 / 1000.0
        // t[0]=cos(num), t[2]=-sin(num), t[1]=sin(num), t[3]=cos(num)
        // t[4]=read_i16/20, t[5]...

        // We need to reverse cos/sin to finding 'num'.
        // num = acos(t[0]) ?
        // let num = t[0].acos();
        // write (num * 1000.0) as u16
        // This is lossy and specific to version 1.
        // Attempting approximation.
        let val = if !img.transform.is_empty() {
            img.transform[0].acos() * 1000.0
        } else {
            0.0
        };
        writer.write_u16::<LE>(val as u16)?;

        let tx = if img.transform.len() > 4 {
            img.transform[4]
        } else {
            0.0
        };
        let ty = if img.transform.len() > 5 {
            img.transform[5]
        } else {
            0.0
        };
        writer.write_i16::<LE>((tx * 20.0) as i16)?;
        writer.write_i16::<LE>((ty * 20.0) as i16)?;
    } else {
        // t[0] = read_i32 / 1310720.0
        let t = &img.transform;
        let c = 1310720.0;
        let get = |i| {
            if i < t.len() {
                t[i]
            } else {
                if i % 4 == 0 || i % 4 == 3 { 1.0 } else { 0.0 }
            }
        };

        writer.write_i32::<LE>((get(0) * c) as i32)?; // a
        writer.write_i32::<LE>((get(2) * c) as i32)?; // c (stored as second int in decode)
        writer.write_i32::<LE>((get(1) * c) as i32)?; // b
        writer.write_i32::<LE>((get(3) * c) as i32)?; // d

        // tx, ty
        let tx = if t.len() > 4 { t[4] } else { 0.0 };
        let ty = if t.len() > 5 { t[5] } else { 0.0 };
        writer.write_i16::<LE>((tx * 20.0) as i16)?;
        writer.write_i16::<LE>((ty * 20.0) as i16)?;
    }

    Ok(())
}

fn write_sprite_info<W: Write>(sprite: &SpriteInfo, writer: &mut W, version: i32) -> Result<()> {
    if version >= 4 {
        write_string_by_u16(sprite.name.as_deref().unwrap_or(""), writer)?;
        if version >= 6 {
            write_string_by_u16(sprite.description.as_deref().unwrap_or(""), writer)?;
        }
        writer.write_i32::<LE>((sprite.frame_rate * 65536.0) as i32)?;
    }

    // frames_count
    writer.write_u16::<LE>(sprite.frame.len() as u16)?;

    if version >= 5 {
        writer.write_u16::<LE>(sprite.work_area[0] as u16)?;
        writer.write_u16::<LE>(sprite.work_area[1] as u16)?;
    } else {
        // Implicit work_area logic in decode, nothing to write.
    }

    for frame in &sprite.frame {
        write_frame_info(frame, writer, version)?;
    }

    Ok(())
}

fn write_frame_info<W: Write>(frame: &FrameInfo, writer: &mut W, version: i32) -> Result<()> {
    let mut flags = FrameFlags::empty();
    if !frame.remove.is_empty() {
        flags |= FrameFlags::REMOVES;
    }
    if !frame.append.is_empty() {
        flags |= FrameFlags::ADDS;
    }
    if !frame.change.is_empty() {
        flags |= FrameFlags::MOVES;
    }
    if frame.label.is_some() {
        flags |= FrameFlags::FRAME_NAME;
    }
    if frame.stop {
        flags |= FrameFlags::STOP;
    }
    if !frame.command.is_empty() {
        flags |= FrameFlags::COMMANDS;
    }

    writer.write_u8(flags.bits())?;

    if flags.contains(FrameFlags::REMOVES) {
        let count = frame.remove.len();
        if count >= 255 {
            writer.write_u8(255)?;
            writer.write_u16::<LE>(count as u16)?;
        } else {
            writer.write_u8(count as u8)?;
        }
        for rem in &frame.remove {
            write_removes_info(rem, writer)?;
        }
    }

    if flags.contains(FrameFlags::ADDS) {
        let count = frame.append.len();
        if count >= 255 {
            writer.write_u8(255)?;
            writer.write_u16::<LE>(count as u16)?;
        } else {
            writer.write_u8(count as u8)?;
        }
        for add in &frame.append {
            write_adds_info(add, writer, version)?;
        }
    }

    if flags.contains(FrameFlags::MOVES) {
        let count = frame.change.len();
        if count >= 255 {
            writer.write_u8(255)?;
            writer.write_u16::<LE>(count as u16)?;
        } else {
            writer.write_u8(count as u8)?;
        }
        for change in &frame.change {
            write_moves_info(change, writer, version)?;
        }
    }

    if let Some(label) = &frame.label {
        write_string_by_u16(label, writer)?;
    }

    if flags.contains(FrameFlags::COMMANDS) {
        writer.write_u8(frame.command.len() as u8)?;
        for cmd in &frame.command {
            write_string_by_u16(&cmd[0], writer)?;
            write_string_by_u16(&cmd[1], writer)?;
        }
    }

    Ok(())
}

fn write_removes_info<W: Write>(info: &RemovesInfo, writer: &mut W) -> Result<()> {
    if info.index >= 2047 {
        // Technically read logic: read_u16, if >= 2047 then read_i32.
        // We need to encode such that decode reads it back.
        // But 2047 barely fits in u16.
        // Logic: val = read_u16; if val >= 2047 { val = read_i32 }
        // So to write large index, we write 2047 (u16) then the actual index (i32).
        writer.write_u16::<LE>(2047)?;
        writer.write_i32::<LE>(info.index)?;
    } else {
        writer.write_u16::<LE>(info.index as u16)?;
    }
    Ok(())
}

fn write_adds_info<W: Write>(info: &AddsInfo, writer: &mut W, version: i32) -> Result<()> {
    // num encoding
    // num & 2047 = index
    // num & 32768 = sprite (bool)
    // num & 16384 = additive (bool)
    // num & 8192 = has preload_frame
    // num & 4096 = has name
    // num & 2048 = has time_scale

    let mut num = 0u16;
    let large_index = info.index >= 2047;

    if large_index {
        num |= 2047;
    } else {
        num |= info.index as u16;
    }

    if info.sprite {
        num |= 32768;
    }
    if info.additive {
        num |= 16384;
    }
    if info.preload_frame > 0 {
        num |= 8192;
    } // Logic check: read only reads if flag set
    // What if preload_frame is 0? decode: if flag set read, else 0.
    // So if 0, we don't set flag.

    if info.name.is_some() {
        num |= 4096;
    }
    if (info.time_scale - 1.0).abs() > 0.0001 {
        num |= 2048;
    }

    writer.write_u16::<LE>(num)?;

    if large_index {
        writer.write_i32::<LE>(info.index)?;
    }

    // Resource
    if version >= 6 && info.resource >= 255 {
        writer.write_u8(255)?;
        writer.write_u16::<LE>(info.resource as u16)?;
    } else {
        writer.write_u8(info.resource as u8)?;
    }

    if (num & 8192) != 0 {
        writer.write_u16::<LE>(info.preload_frame as u16)?;
    }

    if let Some(name) = &info.name {
        write_string_by_u16(name, writer)?;
    }

    if (num & 2048) != 0 {
        writer.write_i32::<LE>((info.time_scale * 65536.0) as i32)?;
    }

    Ok(())
}

fn write_moves_info<W: Write>(info: &MovesInfo, writer: &mut W, _version: i32) -> Result<()> {
    // num7 encoding
    // num & 1023 = index
    // Flags

    let mut num = 0u16;
    let large_index = info.index >= 1023;
    if large_index {
        num |= 1023;
    } else {
        num |= info.index as u16;
    }

    // Detect flags based on data presence
    // Decoding: if MATRIX read 4 i32s, else if ROTATE read 1 i16, else 0.
    // Then read coords.

    // We need to compare transform against identity/defaults to be minimal?
    // Or just respect what's in the vector.
    // If we have 6 elements, use MATRIX.
    // If 3 elements (idx 0 only used), use ROTATE?
    // But standard MovesInfo struct stores everything in a Vec.

    // Logic:
    // If transform.len() >= 6 (a,b,c,d,tx,ty) -> Matrix (tx,ty separate)
    // Actually decode puts tx,ty at end of vec.
    // MATRIX reads indices 0,2,1,3.
    // ROTATE reads index 0.
    // Then tx,ty are read (LONG_COORDS check).

    let mut flags = MoveFlags::empty();

    if info.transform.len() >= 6 {
        flags |= MoveFlags::MATRIX;
    } else if info.transform.len() >= 3 && info.transform[0].abs() > 0.00001 {
        // If it was rotated. Note: transform[0] is rotation in radians/1000?
        // No, in decode: t[0] = read_i16 / 1000.0.
        flags |= MoveFlags::ROTATE;
    }

    // Coords
    // We always write coords? decode always reads val1, val2.
    // Check if they need LONG_COORDS
    let tx = if info.transform.len() >= 2 {
        info.transform[info.transform.len() - 2]
    } else {
        0.0
    };
    let ty = if info.transform.len() >= 1 {
        info.transform[info.transform.len() - 1]
    } else {
        0.0
    };

    let tx_val = (tx * 20.0) as i32;
    let ty_val = (ty * 20.0) as i32;

    let needs_long = tx_val.abs() > 32767 || ty_val.abs() > 32767;
    if needs_long {
        flags |= MoveFlags::LONG_COORDS;
    }

    if info.source_rectangle.is_some() {
        flags |= MoveFlags::SRC_RECT;
    }
    if info.color.is_some() {
        flags |= MoveFlags::COLOR;
    }
    if info.sprite_frame_number > 0 {
        flags |= MoveFlags::ANIM_FRAME_NUM;
    }

    // Write num
    num |= flags.bits();
    writer.write_u16::<LE>(num)?;

    if large_index {
        writer.write_i32::<LE>(info.index)?;
    }

    // Write Transform components
    if flags.contains(MoveFlags::MATRIX) {
        // 0,2,1,3
        let t = &info.transform;
        let c = 65536.0;
        writer.write_i32::<LE>((t[0] * c) as i32)?;
        writer.write_i32::<LE>((t[2] * c) as i32)?;
        writer.write_i32::<LE>((t[1] * c) as i32)?;
        writer.write_i32::<LE>((t[3] * c) as i32)?;
    } else if flags.contains(MoveFlags::ROTATE) {
        let t = &info.transform;
        // t[0]
        writer.write_i16::<LE>((t[0] * 1000.0) as i16)?;
    }

    // Write Coords
    if flags.contains(MoveFlags::LONG_COORDS) {
        writer.write_i32::<LE>(tx_val)?;
        writer.write_i32::<LE>(ty_val)?;
    } else {
        writer.write_i16::<LE>(tx_val as i16)?;
        writer.write_i16::<LE>(ty_val as i16)?;
    }

    if let Some(rect) = info.source_rectangle {
        for v in rect {
            writer.write_i16::<LE>((v * 20) as i16)?;
        }
    }

    if let Some(color) = info.color {
        for v in color {
            writer.write_u8((v * 255.0) as u8)?;
        }
    }

    if flags.contains(MoveFlags::ANIM_FRAME_NUM) {
        writer.write_u16::<LE>(info.sprite_frame_number as u16)?;
    }

    Ok(())
}
