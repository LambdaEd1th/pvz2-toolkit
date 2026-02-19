pub mod codec;
pub mod error;
pub mod process;
pub mod types;

pub use codec::{decode_popfx, encode_popfx};
pub use error::{PopfxError, Result};
pub use types::PopcapRenderEffectObject;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Block1, Block2, Block3};
    use std::io::Cursor;

    #[test]
    fn test_popfx_round_trip() {
        // Create a sample object
        let original = PopcapRenderEffectObject {
            block_1: vec![Block1 {
                unknown_1: 1,
                unknown_2: 2,
                unknown_3: 3,
                unknown_4: 4,
                unknown_5: 5,
                unknown_6: 6,
            }],
            block_2: vec![Block2 {
                unknown_1: 7,
                unknown_2: 8,
            }],
            block_3: vec![
                Block3 {
                    unknown_2: 9,
                    string: "test_string_1".to_string(),
                },
                Block3 {
                    unknown_2: 10,
                    string: "test_string_2".to_string(),
                },
                Block3 {
                    unknown_2: 11,
                    string: "test_string_1".to_string(), // Duplicate to test dedup
                },
            ],
            block_4: vec![],
            block_5: vec![],
            block_6: vec![],
            block_7: vec![],
            block_8: vec![],
        };

        // Encode
        let mut buffer = Cursor::new(Vec::new());
        encode_popfx(&original, &mut buffer).expect("Encoding failed");

        // Decode
        buffer.set_position(0);
        let decoded = decode_popfx(&mut buffer).expect("Decoding failed");

        // Verify
        assert_eq!(decoded.block_1.len(), original.block_1.len());
        assert_eq!(decoded.block_1[0].unknown_1, original.block_1[0].unknown_1);

        assert_eq!(decoded.block_3.len(), original.block_3.len());
        assert_eq!(decoded.block_3[0].string, "test_string_1");
        assert_eq!(decoded.block_3[1].string, "test_string_2");
        assert_eq!(decoded.block_3[2].string, "test_string_1");
    }
}
