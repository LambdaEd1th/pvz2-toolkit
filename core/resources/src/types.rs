use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Path definition which can be either a string or an array of strings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum PathDef {
    Array(Vec<String>),
    String(String),
}

/// A dimension property format used in composite atlases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dimension {
    pub width: u32,
    pub height: u32,
}

/// The official layout structure when bundled (resources.xml equivalent structure)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceGroup {
    pub version: Option<u32>,
    pub content_version: Option<u32>,
    pub slot_count: u32,
    pub groups: Vec<ShellSubgroupData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellSubgroupData {
    pub id: String,
    pub r#type: String, // "composite" or "simple"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub res: Option<String>,
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
    pub r#type: String, // E.g., "Image", "Sound", "File"
    pub slot: u32,
    pub id: String,
    pub path: PathDef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atlas: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cols: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ax: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ay: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aw: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ah: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "forceOriginalVectorSymbolSize"
    )]
    pub force_original_vector_symbol_size: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub srcpath: Option<PathDef>,
}

// ---------------------------------------------------------
// Flattened dictionary layout (res.json equivalent structure)
// ---------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResInfo {
    pub expand_path: String, // "string" or "array"
    pub groups: HashMap<String, GroupDictionary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupDictionary {
    pub is_composite: bool,
    pub subgroup: HashMap<String, MSubgroupData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MSubgroupData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>, // if "null", uses type inside packet
    pub packet: serde_json::Value, // Can be common wrapper or atlas wrapper
}

// Below are internal packet structures for ResInfo serialization

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasWrapper {
    pub r#type: String,
    pub path: PathDef,
    pub dimension: Dimension,
    pub data: HashMap<String, SpriteData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpriteData {
    pub r#type: String,
    pub path: PathDef,
    pub r#default: DefaultProperty,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultProperty {
    pub ax: u32,
    pub ay: u32,
    pub aw: u32,
    pub ah: u32,
    pub x: i32,
    pub y: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cols: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonWrapper {
    pub r#type: String,
    pub data: HashMap<String, CommonDataWrapper>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonDataWrapper {
    pub r#type: String,
    pub path: PathDef,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "forceOriginalVectorSymbolSize"
    )]
    pub force_original_vector_symbol_size: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub srcpath: Option<PathDef>,
}
