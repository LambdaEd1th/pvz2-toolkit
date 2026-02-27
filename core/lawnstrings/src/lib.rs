pub mod error;
pub mod reader;
pub mod types;
pub mod writer;

pub use error::{LawnStringsError, Result};
pub use reader::*;
pub use types::*;
pub use writer::*;

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
