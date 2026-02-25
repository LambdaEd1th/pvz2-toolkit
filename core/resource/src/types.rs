use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubgroupDefinition {
    pub composite: bool,
    pub subgroups: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManifestDefinition {
    #[serde(flatten)]
    pub groups: std::collections::BTreeMap<String, SubgroupDefinition>,
}

// Subgroups json content mapped directly to RSB's DescriptionSubGroup struct
pub use rsb::types::DescriptionGroup;
pub use rsb::types::DescriptionResources;
pub use rsb::types::DescriptionSubGroup;
pub use rsb::types::PropertiesPtxInfo;
pub use rsb::types::ResourcesDescription;

// PopCap resources.json layout (RSG Packets manifest)

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PopCapResourceManifest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_version: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot_count: Option<i32>,
    pub groups: Vec<PopCapResourceGroup>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum PopCapResourceGroup {
    Composite(ResourceComposite),
    Resources(ResourceBlock),
    Other(serde_json::Value), // Catch-all for extra undocumented groups
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResourceComposite {
    pub id: String,
    #[serde(rename = "type")]
    pub res_type: String, // "composite"
    pub subgroups: Vec<ResourceSubgroupRef>,

    // Additional generic data capture
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResourceSubgroupRef {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub res: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResourceBlock {
    pub id: String,
    pub resources: Vec<serde_json::Value>, // Resources can heavily vary so keep loose Map

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,

    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
}

// content.json layouts for split metadata

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContentJson {
    #[serde(flatten)]
    pub groups: std::collections::BTreeMap<String, ContentGroupDef>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContentGroupDef {
    pub is_composite: bool,
    pub subgroups: std::collections::BTreeMap<String, ContentSubgroupDef>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContentSubgroupDef {
    #[serde(rename = "type")]
    pub res_type: Option<String>,
}
