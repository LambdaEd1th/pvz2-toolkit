use crate::codec::{read_string, read_track_nodes};
use crate::error::ParticlesError;
use crate::types::{Particles, ParticlesEmitter, ParticlesField};
use byteorder::{LE, ReadBytesExt};
use flate2::read::ZlibDecoder;
use std::io::{Cursor, Read};

/// Alias for backward compatibility
fn read_string_by_int32<R: Read>(reader: &mut R) -> Result<String, ParticlesError> {
    read_string(reader)
}

pub fn decode_pc(data: &[u8]) -> Result<Particles, ParticlesError> {
    let mut cursor = Cursor::new(data);
    let first_int = cursor.read_i32::<LE>()?;

    let mut uncompressed_data = Vec::new();
    let mut reader: Box<dyn Read> = if first_int == -559022380 {
        // PopCapZlib
        cursor.read_i32::<LE>()?;
        let mut decoder = ZlibDecoder::new(cursor);
        decoder.read_to_end(&mut uncompressed_data)?;
        Box::new(Cursor::new(&uncompressed_data))
    } else {
        cursor.set_position(0);
        Box::new(cursor)
    };

    let mut unused_8 = [0u8; 8];
    reader.read_exact(&mut unused_8)?;

    let count = reader.read_i32::<LE>()?;
    let mut particles = Particles {
        emitters: vec![ParticlesEmitter::default(); count as usize],
    };

    if reader.read_i32::<LE>()? != 0x164 {
        return Err(ParticlesError::UnsupportedFormat(0x164, 0));
    }

    for i in 0..count as usize {
        let emitter = &mut particles.emitters[i];
        let mut unused_4 = [0u8; 4];
        reader.read_exact(&mut unused_4)?;

        // ImageCol, ImageRow, ImageFrames, Animated
        let val = reader.read_i32::<LE>()?;
        if val != 0 {
            emitter.image_col = Some(val);
        }
        let val = reader.read_i32::<LE>()?;
        if val != 0 {
            emitter.image_row = Some(val);
        }
        let val = reader.read_i32::<LE>()?;
        if val != 1 {
            emitter.image_frames = Some(val);
        }
        let val = reader.read_i32::<LE>()?;
        if val != 0 {
            emitter.animated = Some(val);
        }

        emitter.particle_flags = reader.read_i32::<LE>()?;
        let val = reader.read_i32::<LE>()?;
        if val != 1 {
            emitter.emitter_type = Some(val);
        }

        let mut unused_188 = [0u8; 188];
        reader.read_exact(&mut unused_188)?;

        let val = reader.read_i32::<LE>()?;
        if val != 0 {
            emitter.field = Some(vec![ParticlesField::default(); val as usize]);
        }
        reader.read_exact(&mut unused_4)?;

        let val = reader.read_i32::<LE>()?;
        if val != 0 {
            emitter.system_field = Some(vec![ParticlesField::default(); val as usize]);
        }
        let mut unused_128 = [0u8; 128];
        reader.read_exact(&mut unused_128)?;
    }

    for i in 0..count as usize {
        let emitter = &mut particles.emitters[i];

        let img = read_string_by_int32(&mut reader)?;
        if !img.is_empty() {
            emitter.image = Some(img);
        }

        let name = read_string_by_int32(&mut reader)?;
        if !name.is_empty() {
            emitter.name = Some(name);
        }

        emitter.system_duration = read_track_nodes(&mut reader)?;
        let on_dur = read_string_by_int32(&mut reader)?;
        if !on_dur.is_empty() {
            emitter.on_duration = Some(on_dur);
        }

        emitter.cross_fade_duration = read_track_nodes(&mut reader)?;
        emitter.spawn_rate = read_track_nodes(&mut reader)?;
        emitter.spawn_min_active = read_track_nodes(&mut reader)?;
        emitter.spawn_max_active = read_track_nodes(&mut reader)?;
        emitter.spawn_max_launched = read_track_nodes(&mut reader)?;
        emitter.emitter_radius = read_track_nodes(&mut reader)?;
        emitter.emitter_offset_x = read_track_nodes(&mut reader)?;
        emitter.emitter_offset_y = read_track_nodes(&mut reader)?;
        emitter.emitter_box_x = read_track_nodes(&mut reader)?;
        emitter.emitter_box_y = read_track_nodes(&mut reader)?;
        emitter.emitter_path = read_track_nodes(&mut reader)?;
        emitter.emitter_skew_x = read_track_nodes(&mut reader)?;
        emitter.emitter_skew_y = read_track_nodes(&mut reader)?;
        emitter.particle_duration = read_track_nodes(&mut reader)?;
        emitter.system_red = read_track_nodes(&mut reader)?;
        emitter.system_green = read_track_nodes(&mut reader)?;
        emitter.system_blue = read_track_nodes(&mut reader)?;
        emitter.system_alpha = read_track_nodes(&mut reader)?;
        emitter.system_brightness = read_track_nodes(&mut reader)?;
        emitter.launch_speed = read_track_nodes(&mut reader)?;
        emitter.launch_angle = read_track_nodes(&mut reader)?;

        // Read fields
        if reader.read_i32::<LE>()? != 0x14 {
            return Err(ParticlesError::UnsupportedFormat(0x14, 0));
        }
        if let Some(fields) = &mut emitter.field {
            for k in 0..fields.len() {
                let typ = reader.read_i32::<LE>()?;
                if typ != 0 {
                    fields[k].field_type = Some(typ);
                }
                let mut unused_16 = [0u8; 16];
                reader.read_exact(&mut unused_16)?;
            }
            for k in 0..fields.len() {
                fields[k].x = read_track_nodes(&mut reader)?;
                fields[k].y = read_track_nodes(&mut reader)?;
            }
        }

        if reader.read_i32::<LE>()? != 0x14 {
            return Err(ParticlesError::UnsupportedFormat(0x14, 0));
        }
        if let Some(sys_fields) = &mut emitter.system_field {
            for k in 0..sys_fields.len() {
                let typ = reader.read_i32::<LE>()?;
                if typ != 0 {
                    sys_fields[k].field_type = Some(typ);
                }
                let mut unused_16 = [0u8; 16];
                reader.read_exact(&mut unused_16)?;
            }
            for k in 0..sys_fields.len() {
                sys_fields[k].x = read_track_nodes(&mut reader)?;
                sys_fields[k].y = read_track_nodes(&mut reader)?;
            }
        }

        emitter.particle_red = read_track_nodes(&mut reader)?;
        emitter.particle_green = read_track_nodes(&mut reader)?;
        emitter.particle_blue = read_track_nodes(&mut reader)?;
        emitter.particle_alpha = read_track_nodes(&mut reader)?;
        emitter.particle_brightness = read_track_nodes(&mut reader)?;
        emitter.particle_spin_angle = read_track_nodes(&mut reader)?;
        emitter.particle_spin_speed = read_track_nodes(&mut reader)?;
        emitter.particle_scale = read_track_nodes(&mut reader)?;
        emitter.particle_stretch = read_track_nodes(&mut reader)?;
        emitter.collision_reflect = read_track_nodes(&mut reader)?;
        emitter.collision_spin = read_track_nodes(&mut reader)?;
        emitter.clip_top = read_track_nodes(&mut reader)?;
        emitter.clip_bottom = read_track_nodes(&mut reader)?;
        emitter.clip_left = read_track_nodes(&mut reader)?;
        emitter.clip_right = read_track_nodes(&mut reader)?;
        emitter.animation_rate = read_track_nodes(&mut reader)?;
    }

    Ok(particles)
}

pub fn decode_phone32(data: &[u8]) -> Result<Particles, ParticlesError> {
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

    let mut unused_8 = [0u8; 8];
    reader.read_exact(&mut unused_8)?;

    let count = reader.read_i32::<LE>()?;
    let mut particles = Particles {
        emitters: vec![ParticlesEmitter::default(); count as usize],
    };

    if reader.read_i32::<LE>()? != 0x164 {
        return Err(ParticlesError::UnsupportedFormat(0x164, 0));
    }

    for i in 0..count as usize {
        let emitter = &mut particles.emitters[i];
        let mut unused_4 = [0u8; 4];
        reader.read_exact(&mut unused_4)?;

        let val = reader.read_i32::<LE>()?;
        if val != 0 {
            emitter.image_col = Some(val);
        }
        let val = reader.read_i32::<LE>()?;
        if val != 0 {
            emitter.image_row = Some(val);
        }
        let val = reader.read_i32::<LE>()?;
        if val != 1 {
            emitter.image_frames = Some(val);
        }
        let val = reader.read_i32::<LE>()?;
        if val != 0 {
            emitter.animated = Some(val);
        }

        emitter.particle_flags = reader.read_i32::<LE>()?;
        let val = reader.read_i32::<LE>()?;
        if val != 1 {
            emitter.emitter_type = Some(val);
        }

        let mut unused_188 = [0u8; 188];
        reader.read_exact(&mut unused_188)?;

        let val = reader.read_i32::<LE>()?;
        if val != 0 {
            emitter.field = Some(vec![ParticlesField::default(); val as usize]);
        }
        reader.read_exact(&mut unused_4)?;
        let val = reader.read_i32::<LE>()?;
        if val != 0 {
            emitter.system_field = Some(vec![ParticlesField::default(); val as usize]);
        }
        let mut unused_128 = [0u8; 128];
        reader.read_exact(&mut unused_128)?;
    }

    for i in 0..count as usize {
        let emitter = &mut particles.emitters[i];

        let img_int = reader.read_i32::<LE>()?;
        if img_int != -1 {
            emitter.image = Some(img_int.to_string());
        }

        let name = read_string_by_int32(&mut reader)?;
        if !name.is_empty() {
            emitter.name = Some(name);
        }

        emitter.system_duration = read_track_nodes(&mut reader)?;
        let on_dur = read_string_by_int32(&mut reader)?;
        if !on_dur.is_empty() {
            emitter.on_duration = Some(on_dur);
        }

        emitter.cross_fade_duration = read_track_nodes(&mut reader)?;
        emitter.spawn_rate = read_track_nodes(&mut reader)?;
        emitter.spawn_min_active = read_track_nodes(&mut reader)?;
        emitter.spawn_max_active = read_track_nodes(&mut reader)?;
        emitter.spawn_max_launched = read_track_nodes(&mut reader)?;
        emitter.emitter_radius = read_track_nodes(&mut reader)?;
        emitter.emitter_offset_x = read_track_nodes(&mut reader)?;
        emitter.emitter_offset_y = read_track_nodes(&mut reader)?;
        emitter.emitter_box_x = read_track_nodes(&mut reader)?;
        emitter.emitter_box_y = read_track_nodes(&mut reader)?;
        emitter.emitter_path = read_track_nodes(&mut reader)?;
        emitter.emitter_skew_x = read_track_nodes(&mut reader)?;
        emitter.emitter_skew_y = read_track_nodes(&mut reader)?;
        emitter.particle_duration = read_track_nodes(&mut reader)?;
        emitter.system_red = read_track_nodes(&mut reader)?;
        emitter.system_green = read_track_nodes(&mut reader)?;
        emitter.system_blue = read_track_nodes(&mut reader)?;
        emitter.system_alpha = read_track_nodes(&mut reader)?;
        emitter.system_brightness = read_track_nodes(&mut reader)?;
        emitter.launch_speed = read_track_nodes(&mut reader)?;
        emitter.launch_angle = read_track_nodes(&mut reader)?;

        // Fields
        if reader.read_i32::<LE>()? != 0x14 {
            return Err(ParticlesError::UnsupportedFormat(0x14, 0));
        }
        if let Some(fields) = &mut emitter.field {
            for k in 0..fields.len() {
                let typ = reader.read_i32::<LE>()?;
                if typ != 0 {
                    fields[k].field_type = Some(typ);
                }
                let mut unused_16 = [0u8; 16];
                reader.read_exact(&mut unused_16)?;
            }
            for k in 0..fields.len() {
                fields[k].x = read_track_nodes(&mut reader)?;
                fields[k].y = read_track_nodes(&mut reader)?;
            }
        }

        if reader.read_i32::<LE>()? != 0x14 {
            return Err(ParticlesError::UnsupportedFormat(0x14, 0));
        }
        if let Some(sys_fields) = &mut emitter.system_field {
            for k in 0..sys_fields.len() {
                let typ = reader.read_i32::<LE>()?;
                if typ != 0 {
                    sys_fields[k].field_type = Some(typ);
                }
                let mut unused_16 = [0u8; 16];
                reader.read_exact(&mut unused_16)?;
            }
            for k in 0..sys_fields.len() {
                sys_fields[k].x = read_track_nodes(&mut reader)?;
                sys_fields[k].y = read_track_nodes(&mut reader)?;
            }
        }

        emitter.particle_red = read_track_nodes(&mut reader)?;
        emitter.particle_green = read_track_nodes(&mut reader)?;
        emitter.particle_blue = read_track_nodes(&mut reader)?;
        emitter.particle_alpha = read_track_nodes(&mut reader)?;
        emitter.particle_brightness = read_track_nodes(&mut reader)?;
        emitter.particle_spin_angle = read_track_nodes(&mut reader)?;
        emitter.particle_spin_speed = read_track_nodes(&mut reader)?;
        emitter.particle_scale = read_track_nodes(&mut reader)?;
        emitter.particle_stretch = read_track_nodes(&mut reader)?;
        emitter.collision_reflect = read_track_nodes(&mut reader)?;
        emitter.collision_spin = read_track_nodes(&mut reader)?;
        emitter.clip_top = read_track_nodes(&mut reader)?;
        emitter.clip_bottom = read_track_nodes(&mut reader)?;
        emitter.clip_left = read_track_nodes(&mut reader)?;
        emitter.clip_right = read_track_nodes(&mut reader)?;
        emitter.animation_rate = read_track_nodes(&mut reader)?;
    }

    Ok(particles)
}

pub fn decode_phone64(data: &[u8]) -> Result<Particles, ParticlesError> {
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

    let mut unused_12 = [0u8; 12];
    reader.read_exact(&mut unused_12)?;

    let count = reader.read_i32::<LE>()?;
    let mut particles = Particles {
        emitters: vec![ParticlesEmitter::default(); count as usize],
    };

    let mut unused_4 = [0u8; 4];
    reader.read_exact(&mut unused_4)?;

    if reader.read_i32::<LE>()? != 0x2B0 {
        return Err(ParticlesError::UnsupportedFormat(0x2B0, 0));
    }

    for i in 0..count as usize {
        let emitter = &mut particles.emitters[i];
        let mut unused_8 = [0u8; 8];
        reader.read_exact(&mut unused_8)?;

        let val = reader.read_i32::<LE>()?;
        if val != 0 {
            emitter.image_col = Some(val);
        }
        let val = reader.read_i32::<LE>()?;
        if val != 0 {
            emitter.image_row = Some(val);
        }
        let val = reader.read_i32::<LE>()?;
        if val != 1 {
            emitter.image_frames = Some(val);
        }
        let val = reader.read_i32::<LE>()?;
        if val != 0 {
            emitter.animated = Some(val);
        }

        emitter.particle_flags = reader.read_i32::<LE>()?;
        let val = reader.read_i32::<LE>()?;
        if val != 1 {
            emitter.emitter_type = Some(val);
        }

        let mut unused_376 = [0u8; 376];
        reader.read_exact(&mut unused_376)?;

        let val = reader.read_i32::<LE>()?;
        if val != 0 {
            emitter.field = Some(vec![ParticlesField::default(); val as usize]);
        }
        let mut unused_12 = [0u8; 12];
        reader.read_exact(&mut unused_12)?;
        let val = reader.read_i32::<LE>()?;
        if val != 0 {
            emitter.system_field = Some(vec![ParticlesField::default(); val as usize]);
        }
        let mut unused_260 = [0u8; 260];
        reader.read_exact(&mut unused_260)?;
    }

    for i in 0..count as usize {
        let emitter = &mut particles.emitters[i];

        let img_int = reader.read_i32::<LE>()?;
        if img_int != -1 {
            emitter.image = Some(img_int.to_string());
        }

        let name = read_string_by_int32(&mut reader)?;
        if !name.is_empty() {
            emitter.name = Some(name);
        }

        emitter.system_duration = read_track_nodes(&mut reader)?;
        let on_dur = read_string_by_int32(&mut reader)?;
        if !on_dur.is_empty() {
            emitter.on_duration = Some(on_dur);
        }

        emitter.cross_fade_duration = read_track_nodes(&mut reader)?;
        emitter.spawn_rate = read_track_nodes(&mut reader)?;
        emitter.spawn_min_active = read_track_nodes(&mut reader)?;
        emitter.spawn_max_active = read_track_nodes(&mut reader)?;
        emitter.spawn_max_launched = read_track_nodes(&mut reader)?;
        emitter.emitter_radius = read_track_nodes(&mut reader)?;
        emitter.emitter_offset_x = read_track_nodes(&mut reader)?;
        emitter.emitter_offset_y = read_track_nodes(&mut reader)?;
        emitter.emitter_box_x = read_track_nodes(&mut reader)?;
        emitter.emitter_box_y = read_track_nodes(&mut reader)?;
        emitter.emitter_path = read_track_nodes(&mut reader)?;
        emitter.emitter_skew_x = read_track_nodes(&mut reader)?;
        emitter.emitter_skew_y = read_track_nodes(&mut reader)?;
        emitter.particle_duration = read_track_nodes(&mut reader)?;
        emitter.system_red = read_track_nodes(&mut reader)?;
        emitter.system_green = read_track_nodes(&mut reader)?;
        emitter.system_blue = read_track_nodes(&mut reader)?;
        emitter.system_alpha = read_track_nodes(&mut reader)?;
        emitter.system_brightness = read_track_nodes(&mut reader)?;
        emitter.launch_speed = read_track_nodes(&mut reader)?;
        emitter.launch_angle = read_track_nodes(&mut reader)?;

        // Fields
        if reader.read_i32::<LE>()? != 0x18 {
            return Err(ParticlesError::UnsupportedFormat(0x18, 0));
        }
        if let Some(fields) = &mut emitter.field {
            for k in 0..fields.len() {
                let typ = reader.read_i32::<LE>()?;
                if typ != 0 {
                    fields[k].field_type = Some(typ);
                }
                let mut unused_20 = [0u8; 20];
                reader.read_exact(&mut unused_20)?;
            }
            for k in 0..fields.len() {
                fields[k].x = read_track_nodes(&mut reader)?;
                fields[k].y = read_track_nodes(&mut reader)?;
            }
        }

        if reader.read_i32::<LE>()? != 0x18 {
            return Err(ParticlesError::UnsupportedFormat(0x18, 0));
        }
        if let Some(sys_fields) = &mut emitter.system_field {
            for k in 0..sys_fields.len() {
                let typ = reader.read_i32::<LE>()?;
                if typ != 0 {
                    sys_fields[k].field_type = Some(typ);
                }
                let mut unused_20 = [0u8; 20];
                reader.read_exact(&mut unused_20)?;
            }
            for k in 0..sys_fields.len() {
                sys_fields[k].x = read_track_nodes(&mut reader)?;
                sys_fields[k].y = read_track_nodes(&mut reader)?;
            }
        }

        emitter.particle_red = read_track_nodes(&mut reader)?;
        emitter.particle_green = read_track_nodes(&mut reader)?;
        emitter.particle_blue = read_track_nodes(&mut reader)?;
        emitter.particle_alpha = read_track_nodes(&mut reader)?;
        emitter.particle_brightness = read_track_nodes(&mut reader)?;
        emitter.particle_spin_angle = read_track_nodes(&mut reader)?;
        emitter.particle_spin_speed = read_track_nodes(&mut reader)?;
        emitter.particle_scale = read_track_nodes(&mut reader)?;
        emitter.particle_stretch = read_track_nodes(&mut reader)?;
        emitter.collision_reflect = read_track_nodes(&mut reader)?;
        emitter.collision_spin = read_track_nodes(&mut reader)?;
        emitter.clip_top = read_track_nodes(&mut reader)?;
        emitter.clip_bottom = read_track_nodes(&mut reader)?;
        emitter.clip_left = read_track_nodes(&mut reader)?;
        emitter.clip_right = read_track_nodes(&mut reader)?;
        emitter.animation_rate = read_track_nodes(&mut reader)?;
    }

    Ok(particles)
}

pub fn decode(data: &[u8]) -> Result<Particles, ParticlesError> {
    if let Ok(p) = decode_pc(data) {
        return Ok(p);
    }
    if let Ok(p) = decode_phone32(data) {
        return Ok(p);
    }
    if let Ok(p) = decode_phone64(data) {
        return Ok(p);
    }
    Err(ParticlesError::InvalidVariant)
}
