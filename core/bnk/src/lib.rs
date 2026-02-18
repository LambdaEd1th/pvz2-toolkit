use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use serde::{Deserialize, Serialize};
use std::io::{Read, Seek, SeekFrom, Write};
use thiserror::Error;
use utils::{BinReadExt, BinWriteExt};
pub mod process;

#[derive(Error, Debug)]
pub enum BnkError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid magic: expected BKHD")]
    InvalidMagic,
    #[error("Parse error: {0}")]
    ParseError(String),
}

type Result<T> = std::result::Result<T, BnkError>;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct BankHeader {
    pub version: u32,
    pub id: u32,
    pub language: u32,
    pub head_expand: String, // Hex string
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InitEntry {
    pub id: u32,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameSync {
    pub volume_threshold: String,    // Hex (4 bytes)
    pub max_voice_instances: String, // Hex (2 bytes)
    pub unknown_type_1: u16,
    pub stage_group: Vec<StageGroup>,
    pub switch_group: Vec<SwitchGroup>,
    pub game_parameter: Vec<GameParameter>,
    pub unknown_type_2: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StageGroup {
    pub id: u32,
    pub data: StageGroupData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StageGroupData {
    pub default_transition_time: String, // Hex (4 bytes)
    pub custom_transition: Vec<String>,  // Hex (12 bytes each)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SwitchGroup {
    pub id: u32,
    pub data: SwitchGroupData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SwitchGroupData {
    pub parameter: u32,
    pub parameter_category: u8,
    pub point: Vec<String>, // Hex (12 bytes each)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameParameter {
    pub id: u32,
    pub data: String, // Hex (Variable size)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Environments {
    pub obstruction: EnvironmentItem,
    pub occlusion: EnvironmentItem,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnvironmentItem {
    pub volume: EnvironmentVolume,
    pub low_pass_filter: EnvironmentFilter,
    pub high_pass_filter: Option<EnvironmentFilter>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnvironmentVolume {
    pub volume_value: String,      // Hex (2 bytes)
    pub volume_point: Vec<String>, // Hex (12 bytes)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnvironmentFilter {
    #[serde(rename = "low_pass_filter_vaule", alias = "high_pass_filter_vaule")]
    pub value: String, // Hex (2 bytes)
    #[serde(rename = "low_pass_filter_point", alias = "high_pass_filter_point")]
    pub point: Vec<String>, // Hex (12 bytes)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HircObject {
    #[serde(rename = "type")]
    pub obj_type: u8,
    pub id: u32,
    pub data: String, // Hex
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Reference {
    #[serde(rename = "data")]
    pub entries: Vec<ReferenceEntry>, // Rename to match Sen "data"
    pub unknown_type: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReferenceEntry {
    pub id: u32,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlatformSetting {
    pub platform: String,
}

#[derive(Debug, Clone)]
pub struct DidxEntry {
    pub id: u32,
    pub offset: u32,
    pub size: u32,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Bnk {
    #[serde(rename = "bank_header")]
    pub header: BankHeader,

    #[serde(rename = "embedded_media", skip_serializing_if = "Vec::is_empty")]
    pub embedded_media: Vec<u32>, // IDs only, derived from DIDX

    #[serde(rename = "initialization", skip_serializing_if = "Option::is_none")]
    pub initialization: Option<Vec<InitEntry>>,

    #[serde(
        rename = "game_synchronization",
        skip_serializing_if = "Option::is_none"
    )]
    pub game_sync: Option<GameSync>,

    #[serde(rename = "environments", skip_serializing_if = "Option::is_none")]
    pub environments: Option<Environments>,

    #[serde(rename = "hierarchy", skip_serializing_if = "Vec::is_empty", default)]
    pub hierarchy: Vec<HircObject>,

    #[serde(rename = "reference", skip_serializing_if = "Option::is_none", default)]
    pub reference: Option<Reference>,

    #[serde(
        rename = "platform_setting",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub platform: Option<PlatformSetting>,

    // Internal data not serialized directly to JSON structure
    #[serde(skip)]
    pub data_index: Vec<DidxEntry>,
    #[serde(skip)]
    pub data_chunk_offset: Option<u64>,
}

// ... (Previous code)

impl Bnk {
    pub fn new<R: Read + Seek>(mut reader: R) -> Result<Self> {
        let mut bnk = Bnk::default();
        parse_bnk(&mut reader, &mut bnk)?;
        Ok(bnk)
    }

    pub fn write<W: Write + Seek>(&self, writer: &mut W) -> Result<()> {
        // 1. Header (BKHD)
        let mut bkhd_data = Vec::new();
        self.header.write(&mut bkhd_data)?;

        writer.write_all(b"BKHD")?;
        writer.write_u32::<LE>(bkhd_data.len() as u32)?;
        writer.write_all(&bkhd_data)?;

        // 2. DIDX (Data Index)
        if !self.data_index.is_empty() {
            let mut didx_data = Vec::new();
            for entry in &self.data_index {
                entry.write(&mut didx_data)?;
            }
            writer.write_all(b"DIDX")?;
            writer.write_u32::<LE>(didx_data.len() as u32)?;
            writer.write_all(&didx_data)?;
        }

        // 3. INIT (Initialization)
        if let Some(init) = &self.initialization {
            let mut init_data = Vec::new();
            init_data.write_u32::<LE>(init.len() as u32)?;
            for entry in init {
                entry.write(&mut init_data)?;
            }
            writer.write_all(b"INIT")?;
            writer.write_u32::<LE>(init_data.len() as u32)?;
            writer.write_all(&init_data)?;
        }

        // 4. STMG (Game Sync/State Mgmt)
        if let Some(stmg) = &self.game_sync {
            let mut stmg_data = Vec::new();
            stmg.write(&mut stmg_data, self.header.version)?;
            writer.write_all(b"STMG")?;
            writer.write_u32::<LE>(stmg_data.len() as u32)?;
            writer.write_all(&stmg_data)?;
        }

        // 5. ENVS (Environments)
        if let Some(envs) = &self.environments {
            let mut envs_data = Vec::new();
            envs.write(&mut envs_data, self.header.version)?;
            writer.write_all(b"ENVS")?;
            writer.write_u32::<LE>(envs_data.len() as u32)?;
            writer.write_all(&envs_data)?;
        }

        // 6. HIRC (Hierarchy)
        if !self.hierarchy.is_empty() {
            let mut hirc_data = Vec::new();
            hirc_data.write_u32::<LE>(self.hierarchy.len() as u32)?;
            for obj in &self.hierarchy {
                obj.write(&mut hirc_data)?;
            }
            writer.write_all(b"HIRC")?;
            writer.write_u32::<LE>(hirc_data.len() as u32)?;
            writer.write_all(&hirc_data)?;
        }

        // 7. STID (String Mappings)
        if let Some(ref_data) = &self.reference {
            let mut stid_data = Vec::new();
            stid_data.write_u32::<LE>(ref_data.unknown_type)?;
            stid_data.write_u32::<LE>(ref_data.entries.len() as u32)?;
            for entry in &ref_data.entries {
                entry.write(&mut stid_data)?;
            }
            writer.write_all(b"STID")?;
            writer.write_u32::<LE>(stid_data.len() as u32)?;
            writer.write_all(&stid_data)?;
        }

        // 8. PLAT (Platform)
        if let Some(plat) = &self.platform {
            let mut plat_data = Vec::new();
            plat.write(&mut plat_data)?;
            writer.write_all(b"PLAT")?;
            writer.write_u32::<LE>(plat_data.len() as u32)?;
            writer.write_all(&plat_data)?;
        }

        // DATA chunk is NOT written here. It must be appended manually by the caller
        // because Bnk struct does not hold the raw audio data.

        Ok(())
    }
}

// Implement Write for sub-structs

impl BankHeader {
    fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u32::<LE>(self.version)?;
        writer.write_u32::<LE>(self.id)?;
        writer.write_u32::<LE>(self.language)?;
        let expand_bytes = hex_space_to_bytes(&self.head_expand)?;
        writer.write_all(&expand_bytes)?;
        Ok(())
    }
}

impl DidxEntry {
    fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u32::<LE>(self.id)?;
        writer.write_u32::<LE>(self.offset)?;
        writer.write_u32::<LE>(self.size)?;
        Ok(())
    }
}

impl InitEntry {
    fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u32::<LE>(self.id)?;
        writer.write_null_term_string(&self.name)?;
        Ok(())
    }
}

impl GameSync {
    fn write<W: Write>(&self, writer: &mut W, version: u32) -> Result<()> {
        let vol_bytes = hex_space_to_bytes(&self.volume_threshold)?;
        writer.write_all(&vol_bytes)?;

        let voice_bytes = hex_space_to_bytes(&self.max_voice_instances)?;
        writer.write_all(&voice_bytes)?;

        if version >= 140 {
            writer.write_u16::<LE>(self.unknown_type_1)?;
        }

        writer.write_u32::<LE>(self.stage_group.len() as u32)?;
        for s in &self.stage_group {
            s.write(writer)?;
        }

        writer.write_u32::<LE>(self.switch_group.len() as u32)?;
        for s in &self.switch_group {
            s.write(writer, version)?;
        }

        writer.write_u32::<LE>(self.game_parameter.len() as u32)?;
        for p in &self.game_parameter {
            p.write(writer, version)?;
        }

        if version >= 140 {
            writer.write_u32::<LE>(self.unknown_type_2)?;
        }

        Ok(())
    }
}

impl StageGroup {
    fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u32::<LE>(self.id)?;
        let def_trans = hex_space_to_bytes(&self.data.default_transition_time)?;
        writer.write_all(&def_trans)?;

        writer.write_u32::<LE>(self.data.custom_transition.len() as u32)?;
        for c in &self.data.custom_transition {
            let b = hex_space_to_bytes(c)?;
            writer.write_all(&b)?;
        }
        Ok(())
    }
}

impl SwitchGroup {
    fn write<W: Write>(&self, writer: &mut W, version: u32) -> Result<()> {
        writer.write_u32::<LE>(self.id)?;
        writer.write_u32::<LE>(self.data.parameter)?;

        if version >= 112 {
            writer.write_u8(self.data.parameter_category)?;
        }

        writer.write_u32::<LE>(self.data.point.len() as u32)?;
        for p in &self.data.point {
            let b = hex_space_to_bytes(p)?;
            writer.write_all(&b)?;
        }
        Ok(())
    }
}

impl GameParameter {
    fn write<W: Write>(&self, writer: &mut W, _version: u32) -> Result<()> {
        writer.write_u32::<LE>(self.id)?;
        // Size is implicitly handled by the hex string length,
        // but parsing logic hardcoded 17 or 4 based on version.
        // We trust the hex string in JSON to have correct length.
        let b = hex_space_to_bytes(&self.data)?;
        writer.write_all(&b)?;
        Ok(())
    }
}

impl Environments {
    fn write<W: Write>(&self, writer: &mut W, version: u32) -> Result<()> {
        self.obstruction.write(writer, version)?;
        self.occlusion.write(writer, version)?;
        Ok(())
    }
}

impl EnvironmentItem {
    fn write<W: Write>(&self, writer: &mut W, version: u32) -> Result<()> {
        // Volume
        let vol_val = hex_space_to_bytes(&self.volume.volume_value)?;
        writer.write_all(&vol_val)?;
        writer.write_u16::<LE>(self.volume.volume_point.len() as u16)?;
        for p in &self.volume.volume_point {
            let b = hex_space_to_bytes(p)?;
            writer.write_all(&b)?;
        }

        // Low Pass
        let lp_val = hex_space_to_bytes(&self.low_pass_filter.value)?;
        writer.write_all(&lp_val)?;
        writer.write_u16::<LE>(self.low_pass_filter.point.len() as u16)?;
        for p in &self.low_pass_filter.point {
            let b = hex_space_to_bytes(p)?;
            writer.write_all(&b)?;
        }

        // High Pass
        if version >= 112 {
            if let Some(hp) = &self.high_pass_filter {
                let hp_val = hex_space_to_bytes(&hp.value)?;
                writer.write_all(&hp_val)?;
                writer.write_u16::<LE>(hp.point.len() as u16)?;
                for p in &hp.point {
                    let b = hex_space_to_bytes(p)?;
                    writer.write_all(&b)?;
                }
            } else {
                // Should not happen if version >= 112, but if data is missing, write zeros?
                // Or error out?
                // Current JSON structure has Option. If None, we probably shouldn't be here or write defaults.
                // Let's assume valid data for now.
                writer.write_all(&[0u8; 2])?;
                writer.write_u16::<LE>(0)?;
            }
        }
        Ok(())
    }
}

impl HircObject {
    fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u8(self.obj_type)?;
        let data_bytes = hex_space_to_bytes(&self.data)?;
        let length = data_bytes.len() as u32 + 4; // +4 for ID
        writer.write_u32::<LE>(length)?;
        writer.write_u32::<LE>(self.id)?;
        writer.write_all(&data_bytes)?;
        Ok(())
    }
}

impl ReferenceEntry {
    fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u32::<LE>(self.id)?;
        writer.write_u8(self.name.len() as u8)?;
        writer.write_all(self.name.as_bytes())?;
        Ok(())
    }
}

impl PlatformSetting {
    fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_null_term_string(&self.platform)?;
        Ok(())
    }
}

// Parsing functions (parse_bnk, etc.) remain below...

// Helper Utils

fn hex_space_to_bytes(hex_string: &str) -> Result<Vec<u8>> {
    let clean_string = hex_string.replace(' ', "");
    hex::decode(&clean_string).map_err(|e| BnkError::ParseError(format!("Invalid hex: {}", e)))
}

fn parse_bnk<R: Read + Seek>(reader: &mut R, bnk: &mut Bnk) -> Result<()> {
    // ... existing parse_bnk implementation ...
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
