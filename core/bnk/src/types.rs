use serde::{Deserialize, Serialize};

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
