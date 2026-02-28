use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PakPlatform {
    PC,
    Xbox360,
    TV,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PakInfo {
    pub pak_platform: String,
    pub pak_use_windows_path_separate: bool,
    pub pak_use_zlib_compress: bool,
}

#[derive(Debug, Clone)]
pub struct PakRecord {
    pub path: String,
    pub data: Vec<u8>,
}
