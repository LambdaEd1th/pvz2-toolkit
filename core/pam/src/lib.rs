use anyhow::{Result, bail};
use byteorder::{LE, ReadBytesExt};
use serde::{Deserialize, Serialize};
use std::io::{Read, Seek};

pub mod html5;

pub const PAM_MAGIC: u32 = 0xBAF01954;

#[derive(Debug, Serialize, Deserialize)]
pub struct PamInfo {
    pub version: i32,
    pub frame_rate: i32,
    pub position: [f64; 2],
    pub size: [f64; 2],
    pub image: Vec<ImageInfo>,
    pub sprite: Vec<SpriteInfo>,
    pub main_sprite: SpriteInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageInfo {
    pub name: String,
    pub size: [i32; 2],
    pub transform: Vec<f64>, // Using Vec because length varies
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SpriteInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub frame_rate: f64,
    pub work_area: [i32; 2],
    pub frame: Vec<FrameInfo>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct FrameInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub stop: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub command: Vec<[String; 2]>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub remove: Vec<RemovesInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub append: Vec<AddsInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub change: Vec<MovesInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RemovesInfo {
    pub index: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddsInfo {
    pub index: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub resource: i32,
    pub sprite: bool,
    pub additive: bool,
    pub preload_frame: i32,
    pub time_scale: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MovesInfo {
    pub index: i32,
    pub transform: Vec<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<[f64; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_rectangle: Option<[i32; 4]>,
    pub sprite_frame_number: i32,
}

pub fn decode_pam<R: Read + Seek>(reader: &mut R) -> Result<PamInfo> {
    let magic = reader.read_u32::<LE>()?;
    if magic != PAM_MAGIC {
        bail!("Invalid PAM magic: {:#X}", magic);
    }

    let version = reader.read_i32::<LE>()?;
    if !(1..=6).contains(&version) {
        bail!("PAM version out of range: {}", version);
    }

    let frame_rate = reader.read_u8()? as i32;

    let mut position = [0.0; 2];
    for p in &mut position {
        *p = reader.read_u16::<LE>()? as f64 / 20.0;
    }

    let mut size = [0.0; 2];
    for s in &mut size {
        *s = reader.read_u16::<LE>()? as f64 / 20.0;
    }

    let images_count = reader.read_u16::<LE>()? as usize;
    let mut image = Vec::with_capacity(images_count);
    for _ in 0..images_count {
        image.push(read_image_info(reader, version)?);
    }

    let sprites_count = reader.read_u16::<LE>()? as usize;
    let mut sprite = Vec::with_capacity(sprites_count);
    for _ in 0..sprites_count {
        let mut s = read_sprite_info(reader, version)?;
        if version < 4 {
            s.frame_rate = frame_rate as f64;
        }
        sprite.push(s);
    }

    let mut main_sprite = SpriteInfo::default();
    let has_main_sprite = if version <= 3 {
        true
    } else {
        read_bool(reader)?
    };

    if has_main_sprite {
        main_sprite = read_sprite_info(reader, version)?;
        if version < 4 {
            main_sprite.frame_rate = frame_rate as f64;
        }
    }

    Ok(PamInfo {
        version,
        frame_rate,
        position,
        size,
        image,
        sprite,
        main_sprite,
    })
}

fn read_string_by_u16<R: Read>(reader: &mut R) -> Result<String> {
    let len = reader.read_u16::<LE>()? as usize;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;
    Ok(String::from_utf8_lossy(&buf).to_string())
}

fn read_bool<R: Read>(reader: &mut R) -> Result<bool> {
    Ok(reader.read_u8()? != 0)
}

fn read_image_info<R: Read>(reader: &mut R, version: i32) -> Result<ImageInfo> {
    let name = read_string_by_u16(reader)?;
    let mut size = [-1; 2];

    if version >= 4 {
        for s in &mut size {
            *s = reader.read_u16::<LE>()? as i32;
        }
    }

    let transform: Vec<f64>;
    if version == 1 {
        let num = reader.read_u16::<LE>()? as f64 / 1000.0;
        let mut t = vec![0.0; 6];
        t[0] = num.cos();
        t[2] = -num.sin();
        t[1] = num.sin();
        t[3] = num.cos();
        t[4] = reader.read_i16::<LE>()? as f64 / 20.0;
        t[5] = reader.read_i16::<LE>()? as f64 / 20.0;
        transform = t;
    } else {
        let mut t = vec![0.0; 6];
        t[0] = reader.read_i32::<LE>()? as f64 / 1310720.0;
        t[2] = reader.read_i32::<LE>()? as f64 / 1310720.0;
        t[1] = reader.read_i32::<LE>()? as f64 / 1310720.0;
        t[3] = reader.read_i32::<LE>()? as f64 / 1310720.0;
        t[4] = reader.read_i16::<LE>()? as f64 / 20.0;
        t[5] = reader.read_i16::<LE>()? as f64 / 20.0;
        transform = t;
    }

    Ok(ImageInfo {
        name,
        size,
        transform,
    })
}

fn read_sprite_info<R: Read>(reader: &mut R, version: i32) -> Result<SpriteInfo> {
    let mut name = None;
    let mut description = None;
    let mut frame_rate = -1.0;

    if version >= 4 {
        name = Some(read_string_by_u16(reader)?);
        if version >= 6 {
            description = Some(read_string_by_u16(reader)?);
        }
        frame_rate = reader.read_i32::<LE>()? as f64 / 65536.0;
    }

    let frames_count = reader.read_u16::<LE>()? as usize;
    let mut work_area = [0, frames_count as i32];

    if version >= 5 {
        work_area[0] = reader.read_u16::<LE>()? as i32;
        work_area[1] = reader.read_u16::<LE>()? as i32; // Overwrites the default length if present? C# code says: `sprite.work_area[1] = framesCount;` afterwards.
    // Actually C# code reads it, but then immediately overwrites `work_area[1] = framesCount`.
    // We will replicate the read to advance the cursor, but respect the overwrite logic.
    } else {
        work_area[0] = 0;
        work_area[1] = (frames_count as i32).saturating_sub(1);
    }
    work_area[1] = frames_count as i32; // Enforce C# logic overwrite

    let mut frame = Vec::with_capacity(frames_count);
    for _ in 0..frames_count {
        frame.push(read_frame_info(reader, version)?);
    }

    Ok(SpriteInfo {
        name,
        description,
        frame_rate,
        work_area,
        frame,
    })
}

fn read_frame_info<R: Read>(reader: &mut R, version: i32) -> Result<FrameInfo> {
    let flags_byte = reader.read_u8()?;
    let flags = FrameFlags::from_bits_truncate(flags_byte);

    let mut remove = Vec::new();
    if flags.contains(FrameFlags::REMOVES) {
        let mut count = reader.read_u8()? as usize;
        if count == 255 {
            count = reader.read_u16::<LE>()? as usize;
        }
        for _ in 0..count {
            remove.push(read_removes_info(reader)?);
        }
    }

    let mut append = Vec::new();
    if flags.contains(FrameFlags::ADDS) {
        let mut count = reader.read_u8()? as usize;
        if count == 255 {
            count = reader.read_u16::<LE>()? as usize;
        }
        for _ in 0..count {
            append.push(read_adds_info(reader, version)?);
        }
    }

    let mut change = Vec::new();
    if flags.contains(FrameFlags::MOVES) {
        let mut count = reader.read_u8()? as usize;
        if count == 255 {
            count = reader.read_u16::<LE>()? as usize;
        }
        for _ in 0..count {
            change.push(read_moves_info(reader, version)?);
        }
    }

    let mut label = None;
    if flags.contains(FrameFlags::FRAME_NAME) {
        label = Some(read_string_by_u16(reader)?);
    }

    let stop = flags.contains(FrameFlags::STOP);

    let mut command = Vec::new();
    if flags.contains(FrameFlags::COMMANDS) {
        let count = reader.read_u8()? as usize;
        for _ in 0..count {
            let s1 = read_string_by_u16(reader)?;
            let s2 = read_string_by_u16(reader)?;
            command.push([s1, s2]);
        }
    }

    Ok(FrameInfo {
        label,
        stop,
        command,
        remove,
        append,
        change,
    })
}

bitflags::bitflags! {
    struct FrameFlags: u8 {
        const REMOVES = 1;
        const ADDS = 2;
        const MOVES = 4;
        const FRAME_NAME = 8;
        const STOP = 16;
        const COMMANDS = 32;
    }
}

fn read_removes_info<R: Read>(reader: &mut R) -> Result<RemovesInfo> {
    let mut index = reader.read_u16::<LE>()? as i32;
    if index >= 2047 {
        index = reader.read_i32::<LE>()?;
    }
    Ok(RemovesInfo { index })
}

fn read_adds_info<R: Read>(reader: &mut R, version: i32) -> Result<AddsInfo> {
    let num = reader.read_u16::<LE>()?;
    let mut index = (num & 2047) as i32;
    if index == 2047 {
        index = reader.read_i32::<LE>()?;
    }

    let sprite = (num & 32768) != 0;
    let additive = (num & 16384) != 0;

    let mut resource = reader.read_u8()? as i32;
    if version >= 6 && resource == 255 {
        resource = reader.read_u16::<LE>()? as i32;
    }

    let preload_frame = if (num & 8192) != 0 {
        reader.read_u16::<LE>()? as i32
    } else {
        0
    };

    let name = if (num & 4096) != 0 {
        Some(read_string_by_u16(reader)?)
    } else {
        None
    };

    let time_scale = if (num & 2048) != 0 {
        reader.read_i32::<LE>()? as f32 / 65536.0
    } else {
        1.0
    };

    Ok(AddsInfo {
        index,
        name,
        resource,
        sprite,
        additive,
        preload_frame,
        time_scale,
    })
}

bitflags::bitflags! {
    struct MoveFlags: u16 {
        const SRC_RECT = 32768;
        const ROTATE = 16384;
        const COLOR = 8192;
        const MATRIX = 4096;
        const LONG_COORDS = 2048;
        const ANIM_FRAME_NUM = 1024;
    }
}

fn read_moves_info<R: Read>(reader: &mut R, _version: i32) -> Result<MovesInfo> {
    let num7 = reader.read_u16::<LE>()?;
    let mut index = (num7 & 1023) as i32;
    if index == 1023 {
        index = reader.read_i32::<LE>()?;
    }

    let flags = MoveFlags::from_bits_truncate(num7);
    let mut transform = Vec::new();

    if flags.contains(MoveFlags::MATRIX) {
        transform.resize(6, 0.0);
        transform[0] = reader.read_i32::<LE>()? as f64 / 65536.0;
        transform[2] = reader.read_i32::<LE>()? as f64 / 65536.0;
        transform[1] = reader.read_i32::<LE>()? as f64 / 65536.0;
        transform[3] = reader.read_i32::<LE>()? as f64 / 65536.0;
    } else if flags.contains(MoveFlags::ROTATE) {
        transform.resize(3, 0.0);
        transform[0] = reader.read_i16::<LE>()? as f64 / 1000.0;
    } else {
        transform.resize(2, 0.0);
    }

    let val1;
    let val2;

    if flags.contains(MoveFlags::LONG_COORDS) {
        val1 = reader.read_i32::<LE>()? as f64 / 20.0;
        val2 = reader.read_i32::<LE>()? as f64 / 20.0;
    } else {
        val1 = reader.read_i16::<LE>()? as f64 / 20.0;
        val2 = reader.read_i16::<LE>()? as f64 / 20.0;
    }

    // Place coords at the end of the transform array (mimicking C# logic `transform[^2]`)
    let len = transform.len();
    transform[len - 2] = val1;
    transform[len - 1] = val2;

    let mut source_rectangle = None;
    if flags.contains(MoveFlags::SRC_RECT) {
        let mut sr = [0; 4];
        for v in &mut sr {
            *v = reader.read_i16::<LE>()? as i32 / 20;
        }
        source_rectangle = Some(sr);
    }

    let mut color = None;
    if flags.contains(MoveFlags::COLOR) {
        let mut c = [0.0; 4];
        for v in &mut c {
            *v = reader.read_u8()? as f64 / 255.0;
        }
        color = Some(c);
    }

    let sprite_frame_number = if flags.contains(MoveFlags::ANIM_FRAME_NUM) {
        reader.read_u16::<LE>()? as i32
    } else {
        0
    };

    Ok(MovesInfo {
        index,
        transform,
        color,
        source_rectangle,
        sprite_frame_number,
    })
}
