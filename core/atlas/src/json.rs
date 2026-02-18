use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct OfficialAtlas {
    pub id: String,
    // Provide default values for optional fields if they are missing in some files
    #[serde(default)]
    pub parent: String,
    #[serde(default)]
    pub res: String,
    #[serde(rename = "type", default)]
    pub type_: String,
    pub resources: Vec<Resource>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Resource {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    // path can be a string or array of strings
    pub path: Option<PathOrPaths>,
    pub width: Option<u32>,
    pub height: Option<u32>,

    // Atlas coordinates
    pub ax: Option<u32>,
    pub ay: Option<u32>,
    pub aw: Option<u32>,
    pub ah: Option<u32>,

    // Offsets
    pub x: Option<i32>,
    pub y: Option<i32>,

    // Animation frames
    pub cols: Option<u32>,
    pub rows: Option<u32>,

    pub atlas: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PathOrPaths {
    Single(String),
    Multiple(Vec<String>),
}
