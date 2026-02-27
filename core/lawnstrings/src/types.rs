use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct LawnStringsRoot {
    pub objects: Vec<ObjectMap>,
    pub version: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ObjectMap {
    pub aliases: Vec<String>,
    pub objclass: String,
    pub objdata: ObjdataMap,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ObjdataMap {
    #[serde(rename = "LocStringValues")]
    pub loc_string_values: BTreeMap<String, String>, // Use BTreeMap to preserve sorted order if desired, or HashMap
}

impl Default for LawnStringsRoot {
    fn default() -> Self {
        Self {
            version: 1,
            objects: vec![ObjectMap {
                aliases: vec!["LawnStringsData".to_string()],
                objclass: "LawnStringsData".to_string(),
                objdata: ObjdataMap {
                    loc_string_values: BTreeMap::new(),
                },
            }],
        }
    }
}
