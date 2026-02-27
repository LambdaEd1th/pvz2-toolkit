use crate::error::{NewtonError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MResourceGroup {
    pub slot_count: u32,
    pub groups: Vec<ShellSubgroupData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellSubgroupData {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub res: Option<String>,
    #[serde(rename = "type")]
    pub group_type: String, // "composite" or "simple"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subgroups: Option<Vec<SubgroupWrapper>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<Vec<MSubgroupWrapper>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubgroupWrapper {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub res: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MSubgroupWrapper {
    #[serde(rename = "type")]
    pub res_type: String,
    pub slot: u32,
    pub id: String,
    pub path: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ax: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ay: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aw: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ah: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cols: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atlas: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub srcpath: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_original_vector_symbol_size: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResourceType {
    Image = 1,
    PopAnim = 2,
    SoundBank = 3,
    File = 4,
    PrimeFont = 5,
    RenderEffect = 6,
    DecodedSoundBank = 7,
}

impl ResourceType {
    pub fn from_u8(v: u8) -> Result<Self> {
        match v {
            1 => Ok(ResourceType::Image),
            2 => Ok(ResourceType::PopAnim),
            3 => Ok(ResourceType::SoundBank),
            4 => Ok(ResourceType::File),
            5 => Ok(ResourceType::PrimeFont),
            6 => Ok(ResourceType::RenderEffect),
            7 => Ok(ResourceType::DecodedSoundBank),
            _ => Err(NewtonError::DeserializationError(format!(
                "Unknown resource type: {}",
                v
            ))),
        }
    }

    pub fn to_u8(self) -> u8 {
        self as u8
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "Image" => Ok(ResourceType::Image),
            "PopAnim" => Ok(ResourceType::PopAnim),
            "SoundBank" => Ok(ResourceType::SoundBank),
            "File" => Ok(ResourceType::File),
            "PrimeFont" => Ok(ResourceType::PrimeFont),
            "RenderEffect" => Ok(ResourceType::RenderEffect),
            "DecodedSoundBank" => Ok(ResourceType::DecodedSoundBank),
            _ => Err(NewtonError::DeserializationError(format!(
                "Unknown resource type string: {}",
                s
            ))),
        }
    }
}
