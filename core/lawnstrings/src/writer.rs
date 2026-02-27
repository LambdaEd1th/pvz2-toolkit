use crate::Result;
use crate::types::LawnStringsRoot;

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
