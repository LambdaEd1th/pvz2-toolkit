pub mod binary; // Keep binary for BinaryBlob
// mod constants; // Moved to types
pub mod crypto;
pub mod de;
pub mod error;
pub mod process;
// mod rtid; // Moved to types
pub mod ser;
pub mod types;
// mod value; // Moved to types
pub mod varint;

pub use binary::BinaryBlob; // Also re-exported from types usage?
pub use error::{Error, Result};
pub use types::{Rtid, RtidIdentifier, RtonIdentifier, RtonValue};
pub use varint::VarInt;

pub use de::{from_bytes, from_reader};
pub use process::{rton_decode, rton_decrypt_file, rton_encode, rton_encrypt_file};
pub use ser::{to_bytes, to_writer};

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_rton_round_trip() {
        // Create a sample RtonValue
        let original = RtonValue::Object(vec![
            ("key1".to_string(), RtonValue::String("value1".to_string())),
            ("key2".to_string(), RtonValue::Int32(123)),
            (
                "key3".to_string(),
                RtonValue::Array(vec![RtonValue::Bool(true), RtonValue::Bool(false)]),
            ),
        ]);

        // Serialize to bytes (using default key/writer logic)
        let mut buffer = Vec::new();
        to_writer(&mut buffer, &original, None).expect("Serialization failed");

        // Deserialize from bytes
        let mut cursor = Cursor::new(buffer);
        let decoded: RtonValue = from_reader(&mut cursor, None).expect("Deserialization failed");

        // Verify equality
        assert_eq!(original, decoded);
    }
}
