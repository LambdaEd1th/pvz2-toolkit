use crate::Result;
use crate::types::LawnStringsRoot;

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
            // Append to value
            if !current_value.is_empty() {
                current_value.push('\n');
            }
            current_value.push_str(line);
        }
    }

    // Save last
    if let Some(key) = current_key {
        map.insert(key, current_value.trim().to_string());
    }

    Ok(root)
}
