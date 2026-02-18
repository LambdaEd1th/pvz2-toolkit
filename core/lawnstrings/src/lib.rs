use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub mod process;

// --- JSON Structure ---

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

// --- Parsing Logic (Text -> Map) ---

pub fn parse_lawn_strings(text: &str) -> Result<LawnStringsRoot> {
    let mut root = LawnStringsRoot::default();
    let map = &mut root.objects[0].objdata.loc_string_values;

    let mut current_key: Option<String> = None;
    let mut current_value = String::new();

    for line in text.lines() {
        let trimmed = line.trim();
        // Check for key format: [KEY_ID]
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            // Save previous
            if let Some(key) = current_key.take() {
                map.insert(key, current_value.trim().to_string());
                current_value.clear();
            }

            // New key
            let key_content = &trimmed[1..trimmed.len() - 1];
            current_key = Some(key_content.to_string());
        } else {
            // Append to value (preserve newlines if multiline?)
            // The original parser handles newlines. `lines()` strips them.
            // We should append with newline if not empty?
            // Actually, usually multiline strings in this format might be just concatenated?
            // Re-reading common.dart:
            // "textList = text.split('\n')"
            // It appends `textList[k]` + `\r\n`.
            if !current_value.is_empty() {
                current_value.push('\n');
            }
            current_value.push_str(line); // Keep original indent? dart uses `textList[k]`
        }
    }

    // Save last
    if let Some(key) = current_key {
        map.insert(key, current_value.trim().to_string());
    }

    Ok(root)
}

// --- Writing Logic (Map -> Text) ---

pub fn write_lawn_strings(root: &LawnStringsRoot) -> Result<String> {
    let map = &root.objects[0].objdata.loc_string_values;
    let mut output = String::new();

    for (key, value) in map {
        output.push_str(&format!("[{}]\n", key));
        output.push_str(value);
        output.push('\n');
        output.push('\n'); // Double newline separator commonly used
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_write_roundtrip() {
        let input = r#"[KEY_1]
Value 1

[KEY_2]
Value 2 with
Multiple Lines
"#;
        let parsed = parse_lawn_strings(input).unwrap();
        let map = &parsed.objects[0].objdata.loc_string_values;

        assert_eq!(map.get("KEY_1").unwrap(), "Value 1");
        assert!(map.get("KEY_2").unwrap().contains("Multiple Lines"));

        let output = write_lawn_strings(&parsed).unwrap();
        // Verify output contains keys
        assert!(output.contains("[KEY_1]"));
        assert!(output.contains("[KEY_2]"));
    }
}
