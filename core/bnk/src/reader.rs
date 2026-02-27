use crate::error::BnkError;
use crate::types::*;
use byteorder::{LE, ReadBytesExt};
use std::io::{Read, Seek, SeekFrom};
use utils::BinReadExt;

type Result<T> = std::result::Result<T, BnkError>;

// Parsing functions

pub(crate) fn parse_bnk<R: Read + Seek>(reader: &mut R, bnk: &mut Bnk) -> Result<()> {
    // 1. Header (BKHD)
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic)?;
    if &magic != b"BKHD" {
        return Err(BnkError::InvalidMagic);
    }

    let bkhd_length = reader.read_u32::<LE>()?;
    let start_pos = reader.stream_position()?;

    // Version, ID, Language
    let version = reader.read_u32::<LE>()?;
    let id = reader.read_u32::<LE>()?;
    let language = reader.read_u32::<LE>()?;

    // Remaining header data as hex
    let current = reader.stream_position()?;
    let expand_len = (start_pos + bkhd_length as u64) - current;
    let mut expand_bytes = vec![0u8; expand_len as usize];
    reader.read_exact(&mut expand_bytes)?;
    let head_expand = bytes_to_hex_space(&expand_bytes);

    bnk.header = BankHeader {
        version,
        id,
        language,
        head_expand,
    };

    reader.seek(SeekFrom::Start(start_pos + bkhd_length as u64))?;

    // 2. Parse Other Chunks
    let file_end = reader.seek(SeekFrom::End(0))?;
    reader.seek(SeekFrom::Start(start_pos + bkhd_length as u64))?;

    while reader.stream_position()? < file_end {
        let mut chunk_id = [0u8; 4];
        if reader.read_exact(&mut chunk_id).is_err() {
            break;
        }
        let chunk_size = reader.read_u32::<LE>()?;
        let chunk_start = reader.stream_position()?;

        match &chunk_id {
            b"DIDX" => parse_didx(reader, chunk_size, bnk)?,
            b"DATA" => {
                bnk.data_chunk_offset = Some(chunk_start);
                // Just skip DATA body
            }
            b"INIT" => parse_init(reader, chunk_size, bnk)?,
            b"STMG" => parse_stmg(reader, chunk_size, bnk)?,
            b"ENVS" => parse_envs(reader, chunk_size, bnk)?,
            b"HIRC" => parse_hirc(reader, chunk_size, bnk)?,
            b"STID" => parse_stid(reader, chunk_size, bnk)?,
            b"PLAT" => parse_plat(reader, chunk_size, bnk)?,
            _ => {
                // Unknown chunk, skip
            }
        }

        // Seek to next chunk
        reader.seek(SeekFrom::Start(chunk_start + chunk_size as u64))?;
    }

    Ok(())
}

fn parse_didx<R: Read + Seek>(reader: &mut R, size: u32, bnk: &mut Bnk) -> Result<()> {
    // 12 bytes per entry: ID(4), Offset(4), Size(4)
    let count = size / 12;
    for _ in 0..count {
        let id = reader.read_u32::<LE>()?;
        let offset = reader.read_u32::<LE>()?;
        let size = reader.read_u32::<LE>()?;
        bnk.data_index.push(DidxEntry { id, offset, size });
        bnk.embedded_media.push(id);
    }
    Ok(())
}

fn parse_init<R: Read + Seek>(reader: &mut R, _size: u32, bnk: &mut Bnk) -> Result<()> {
    let count = reader.read_u32::<LE>()?;
    let mut list = Vec::new();
    for _ in 0..count {
        let id = reader.read_u32::<LE>()?;
        let name = reader.read_null_term_string()?;
        list.push(InitEntry { id, name });
    }
    bnk.initialization = Some(list);
    Ok(())
}

fn parse_stmg<R: Read + Seek>(reader: &mut R, _size: u32, bnk: &mut Bnk) -> Result<()> {
    let mut vol_threshold = [0u8; 4];
    reader.read_exact(&mut vol_threshold)?;

    let mut max_voice = [0u8; 2];
    reader.read_exact(&mut max_voice)?;

    let mut unknown_type_1 = 0;
    if bnk.header.version >= 140 {
        unknown_type_1 = reader.read_u16::<LE>()?;
    }

    // Stage Groups
    let stage_count = reader.read_u32::<LE>()?;
    let mut stages = Vec::new();
    for _ in 0..stage_count {
        let id = reader.read_u32::<LE>()?;
        let mut def_trans = [0u8; 4];
        reader.read_exact(&mut def_trans)?;

        let custom_count = reader.read_u32::<LE>()?;
        let mut custom = Vec::new();
        for _ in 0..custom_count {
            let mut buf = [0u8; 12];
            reader.read_exact(&mut buf)?;
            custom.push(bytes_to_hex_space(&buf));
        }
        stages.push(StageGroup {
            id,
            data: StageGroupData {
                default_transition_time: bytes_to_hex_space(&def_trans),
                custom_transition: custom,
            },
        });
    }

    // Switch Groups
    let switch_count = reader.read_u32::<LE>()?;
    let mut switches = Vec::new();
    for _ in 0..switch_count {
        let id = reader.read_u32::<LE>()?;
        let param = reader.read_u32::<LE>()?;
        let mut cat = 0;
        if bnk.header.version >= 112 {
            cat = reader.read_u8()?;
        }

        let point_count = reader.read_u32::<LE>()?;
        let mut points = Vec::new();
        for _ in 0..point_count {
            let mut buf = [0u8; 12];
            reader.read_exact(&mut buf)?;
            points.push(bytes_to_hex_space(&buf));
        }
        switches.push(SwitchGroup {
            id,
            data: SwitchGroupData {
                parameter: param,
                parameter_category: cat,
                point: points,
            },
        });
    }

    // Game Parameters
    let param_count = reader.read_u32::<LE>()?;
    let mut params = Vec::new();
    for _ in 0..param_count {
        let id = reader.read_u32::<LE>()?;
        let data_size = if bnk.header.version >= 112 { 17 } else { 4 };
        let mut buf = vec![0u8; data_size];
        reader.read_exact(&mut buf)?;
        params.push(GameParameter {
            id,
            data: bytes_to_hex_space(&buf),
        });
    }

    let mut unknown_type_2 = 0;
    if bnk.header.version >= 140 {
        unknown_type_2 = reader.read_u32::<LE>()?;
    }

    bnk.game_sync = Some(GameSync {
        volume_threshold: bytes_to_hex_space(&vol_threshold),
        max_voice_instances: bytes_to_hex_space(&max_voice),
        unknown_type_1,
        stage_group: stages,
        switch_group: switches,
        game_parameter: params,
        unknown_type_2,
    });

    Ok(())
}

fn parse_envs<R: Read + Seek>(reader: &mut R, _size: u32, bnk: &mut Bnk) -> Result<()> {
    let obstruction = parse_env_item(reader, bnk.header.version)?;
    let occlusion = parse_env_item(reader, bnk.header.version)?;

    bnk.environments = Some(Environments {
        obstruction,
        occlusion,
    });
    Ok(())
}

fn parse_env_item<R: Read + Seek>(reader: &mut R, version: u32) -> Result<EnvironmentItem> {
    // Volume
    let mut vol_val = [0u8; 2];
    reader.read_exact(&mut vol_val)?;
    let vol_count = reader.read_u16::<LE>()?;
    let mut vol_points = Vec::new();
    for _ in 0..vol_count {
        let mut buf = [0u8; 12];
        reader.read_exact(&mut buf)?;
        vol_points.push(bytes_to_hex_space(&buf));
    }

    // Low Pass
    let mut lp_val = [0u8; 2];
    reader.read_exact(&mut lp_val)?;
    let lp_count = reader.read_u16::<LE>()?;
    let mut lp_points = Vec::new();
    for _ in 0..lp_count {
        let mut buf = [0u8; 12];
        reader.read_exact(&mut buf)?;
        lp_points.push(bytes_to_hex_space(&buf));
    }

    // High Pass (v >= 112)
    let mut hp_filter = None;
    if version >= 112 {
        let mut hp_val = [0u8; 2];
        reader.read_exact(&mut hp_val)?;
        let hp_count = reader.read_u16::<LE>()?;
        let mut hp_points = Vec::new();
        for _ in 0..hp_count {
            let mut buf = [0u8; 12];
            reader.read_exact(&mut buf)?;
            hp_points.push(bytes_to_hex_space(&buf));
        }
        hp_filter = Some(EnvironmentFilter {
            value: bytes_to_hex_space(&hp_val),
            point: hp_points,
        });
    }

    Ok(EnvironmentItem {
        volume: EnvironmentVolume {
            volume_value: bytes_to_hex_space(&vol_val),
            volume_point: vol_points,
        },
        low_pass_filter: EnvironmentFilter {
            value: bytes_to_hex_space(&lp_val),
            point: lp_points,
        },
        high_pass_filter: hp_filter,
    })
}

fn parse_hirc<R: Read + Seek>(reader: &mut R, _size: u32, bnk: &mut Bnk) -> Result<()> {
    let count = reader.read_u32::<LE>()?;
    for _ in 0..count {
        let obj_type = reader.read_u8()?;
        let length = reader.read_u32::<LE>()?;
        let id = reader.read_u32::<LE>()?;

        let data_len = length - 4; // id is part of 'length' in Sen logic
        let mut data = vec![0u8; data_len as usize];
        reader.read_exact(&mut data)?;

        bnk.hierarchy.push(HircObject {
            obj_type,
            id,
            data: bytes_to_hex_space(&data),
        });
    }
    Ok(())
}

fn parse_stid<R: Read + Seek>(reader: &mut R, _size: u32, bnk: &mut Bnk) -> Result<()> {
    let unknown_type = reader.read_u32::<LE>()?;
    let count = reader.read_u32::<LE>()?;
    let mut entries = Vec::new();
    for _ in 0..count {
        let id = reader.read_u32::<LE>()?;
        let name_len = reader.read_u8()?;
        let mut name_bytes = vec![0u8; name_len as usize];
        reader.read_exact(&mut name_bytes)?;
        let name = String::from_utf8_lossy(&name_bytes).to_string();
        entries.push(ReferenceEntry { id, name });
    }
    bnk.reference = Some(Reference {
        entries,
        unknown_type,
    });
    Ok(())
}

fn parse_plat<R: Read + Seek>(reader: &mut R, _size: u32, bnk: &mut Bnk) -> Result<()> {
    let platform = reader.read_null_term_string()?;
    bnk.platform = Some(PlatformSetting { platform });
    Ok(())
}

// Utils
fn bytes_to_hex_space(bytes: &[u8]) -> String {
    let hex_string = hex::encode_upper(bytes);
    // Insert space every 2 chars
    let mut result = String::with_capacity(hex_string.len() + hex_string.len() / 2);
    for (i, c) in hex_string.char_indices() {
        if i > 0 && i % 2 == 0 {
            result.push(' ');
        }
        result.push(c);
    }
    result
}

// API for instantiating Bnk struct

impl Bnk {
    pub fn new<R: Read + Seek>(mut reader: R) -> Result<Self> {
        let mut bnk = Bnk::default();
        parse_bnk(&mut reader, &mut bnk)?;
        Ok(bnk)
    }
}
