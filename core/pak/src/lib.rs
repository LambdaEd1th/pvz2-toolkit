pub mod error;
pub mod reader;
pub mod types;
pub mod writer;

pub use error::{PakError, Result};
pub use reader::*;
pub use types::*;
pub use writer::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_pak_roundtrip_pc_uncompressed() {
        let original_info = PakInfo {
            pak_platform: "PC".to_string(),
            pak_use_windows_path_separate: false,
            pak_use_zlib_compress: false,
        };

        let original_records = vec![
            PakRecord {
                path: "test/file1.txt".to_string(),
                data: b"Hello".to_vec(),
            },
            PakRecord {
                path: "test/file2.bin".to_string(),
                data: vec![0x00, 0xFF, 0x12, 0x34],
            },
        ];

        let mut out_buf = Vec::new();
        pack(&mut out_buf, &original_info, &original_records).expect("Packing failed");

        let mut cursor = Cursor::new(out_buf);
        let (decoded_info, decoded_records) = unpack(&mut cursor).expect("Unpacking failed");

        assert_eq!(decoded_info.pak_platform, original_info.pak_platform);
        assert_eq!(decoded_info.pak_use_windows_path_separate, false); // always detected based on slashes
        assert_eq!(
            decoded_info.pak_use_zlib_compress,
            original_info.pak_use_zlib_compress
        );

        assert_eq!(decoded_records.len(), original_records.len());
        assert_eq!(decoded_records[0].path, original_records[0].path);
        assert_eq!(decoded_records[0].data, original_records[0].data);
    }

    #[test]
    fn test_pak_roundtrip_xbox_compressed() {
        let original_info = PakInfo {
            pak_platform: "Xbox360".to_string(),
            pak_use_windows_path_separate: true,
            pak_use_zlib_compress: true,
        };

        let original_records = vec![PakRecord {
            path: "test\\file1.txt".to_string(),
            data: b"Compress me nicely! Compress me nicely! Compress me nicely!".to_vec(),
        }];

        let mut out_buf = Vec::new();
        pack(&mut out_buf, &original_info, &original_records).expect("Packing failed");

        let mut cursor = Cursor::new(out_buf);
        let (decoded_info, decoded_records) = unpack(&mut cursor).expect("Unpacking failed");

        assert_eq!(decoded_info.pak_platform, original_info.pak_platform);
        assert_eq!(decoded_info.pak_use_windows_path_separate, true);
        assert_eq!(
            decoded_info.pak_use_zlib_compress,
            original_info.pak_use_zlib_compress
        );

        assert_eq!(decoded_records[0].path, original_records[0].path);
        assert_eq!(decoded_records[0].data, original_records[0].data);
    }
}
