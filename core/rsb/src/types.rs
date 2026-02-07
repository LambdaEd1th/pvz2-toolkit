use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RsbHeader {
    pub magic: [u8; 4], // "1bsr"
    pub version: u32,
    pub file_offset: u32,
    pub file_list_length: u32,
    pub file_list_begin_offset: u32,
    pub rsg_list_length: u32,
    pub rsg_list_begin_offset: u32,
    pub rsg_number: u32,
    pub rsg_info_begin_offset: u32,
    pub rsg_info_each_length: u32, // 204
    pub composite_number: u32,
    pub composite_info_begin_offset: u32,
    pub composite_info_each_length: u32, // 1156
    pub composite_list_length: u32,
    pub composite_list_begin_offset: u32,
    pub autopool_number: u32,
    pub autopool_info_begin_offset: u32,
    pub autopool_info_each_length: u32, // 152
    pub ptx_number: u32,
    pub ptx_info_begin_offset: u32,
    pub ptx_info_each_length: u32,
    pub part1_begin_offset: u32,
    pub part2_begin_offset: u32,
    pub part3_begin_offset: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileListInfo {
    pub name_path: String,
    pub pool_index: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RsgInfo {
    pub name: String,
    pub rsg_offset: u32,
    pub rsg_length: u32,
    pub pool_index: i32,
    pub ptx_number: u32,
    pub ptx_before_number: u32,
    pub packet_head_info: Option<Vec<u8>>, // 32 bytes usually
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeInfo {
    pub name: String,
    pub is_composite: bool,
    pub packet_number: u32,
    pub packet_info: Vec<CompositePacketInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositePacketInfo {
    pub packet_index: i32,
    pub category: [String; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoPoolInfo {
    pub name: String,
    pub part0_size: u32,
    pub part1_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RsbPtxInfo {
    pub ptx_index: i32,
    pub width: i32,
    pub height: i32,
    pub pitch: i32,
    pub format: i32,
    pub alpha_size: Option<i32>,
    pub alpha_format: Option<i32>,
}

// Structs for description.json serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesDescription {
    pub groups: HashMap<String, DescriptionGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescriptionGroup {
    pub composite: bool,
    pub subgroups: HashMap<String, DescriptionSubGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescriptionSubGroup {
    pub res: String,
    pub language: String,
    pub resources: HashMap<String, DescriptionResources>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescriptionResources {
    #[serde(rename = "type")]
    pub res_type: i32,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ptx_info: Option<PropertiesPtxInfo>,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertiesPtxInfo {
    pub imagetype: String,
    pub aflags: String,
    pub x: String,
    pub y: String,
    pub ax: String,
    pub ay: String,
    pub aw: String,
    pub ah: String,
    pub rows: String,
    pub cols: String,
    pub parent: String,
}
