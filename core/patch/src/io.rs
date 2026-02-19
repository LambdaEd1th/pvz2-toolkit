use crate::error::RsbPatchError;
use crate::types::{RsbPatchHeader, RsbPatchPacketInfo};
use byteorder::{ReadBytesExt, LE};
use std::io::{Read, Seek, SeekFrom};

pub struct RsbPatchReader<R> {
    reader: R,
}

impl<R: Read + Seek> RsbPatchReader<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    pub fn read_header(&mut self) -> Result<RsbPatchHeader, RsbPatchError> {
        let mut magic = [0u8; 4];
        self.reader.read_exact(&mut magic)?;
        if &magic != b"PBSR" {
            return Err(RsbPatchError::InvalidFile(
                "Invalid Magic: Not a PBSR file".to_string(),
            ));
        }

        let version1 = self.reader.read_i32::<LE>()?;
        let version2 = self.reader.read_i32::<LE>()?;

        if version1 != 1 || version2 != 2 {
            return Err(RsbPatchError::InvalidFile(format!(
                "Unsupported version: {}.{}",
                version1, version2
            )));
        }

        let rsb_after_size = self.reader.read_i32::<LE>()?;
        // Padding 8 bytes
        self.reader.read_exact(&mut [0u8; 8])?;

        let mut md5_before = [0u8; 16];
        self.reader.read_exact(&mut md5_before)?;

        // seek to 0x14
        self.reader.seek(SeekFrom::Start(0x14))?;
        let rsb_head_size = self.reader.read_i32::<LE>()?;

        // seek to 0x18
        self.reader.seek(SeekFrom::Start(0x18))?;
        let mut md5_before_reverify = [0u8; 16];
        self.reader.read_exact(&mut md5_before_reverify)?;

        // 0x28
        let rsg_number = self.reader.read_i32::<LE>()?;

        // 0x2C
        let rsb_need_patch_flag = self.reader.read_i32::<LE>()?;
        let rsb_need_patch = rsb_need_patch_flag == 1;

        Ok(RsbPatchHeader {
            rsb_after_size,
            rsb_head_size,
            md5_before, // Use the first read or verify? They should be same.
            rsg_number,
            rsb_need_patch,
        })
    }

    pub fn extract_header_diff(
        &mut self,
        header: &RsbPatchHeader,
    ) -> Result<Option<Vec<u8>>, RsbPatchError> {
        if !header.rsb_need_patch || header.rsb_head_size <= 0 {
            return Ok(None);
        }

        self.reader.seek(SeekFrom::Start(0x30))?;
        let mut buf = vec![0u8; header.rsb_head_size as usize];
        self.reader.read_exact(&mut buf)?;
        Ok(Some(buf))
    }

    pub fn next_packet_info(&mut self) -> Result<Option<(RsbPatchPacketInfo, u64)>, RsbPatchError> {
        let start_pos = self.reader.stream_position()?;

        // Check if EOF
        let mut buf_check = [0u8; 1];
        if self.reader.read(&mut buf_check)? == 0 {
            return Ok(None);
        }
        self.reader.seek(SeekFrom::Start(start_pos))?; // Rewind check

        let mut header_buf = [0u8; 152];
        if self.reader.read_exact(&mut header_buf).is_err() {
            return Ok(None); // EOF or partial
        }

        let packet_patch_size = i32::from_le_bytes(header_buf[4..8].try_into().unwrap());

        // Name at 0x08, max 128 bytes.
        let name_slice = &header_buf[8..136];
        // Find null terminator or take all
        let name_len = name_slice.iter().position(|&c| c == 0).unwrap_or(128);
        let packet_name = String::from_utf8_lossy(&name_slice[..name_len]).to_string();

        let md5_packet = header_buf[136..152].try_into().unwrap();

        let info = RsbPatchPacketInfo {
            packet_patch_size,
            packet_name,
            md5_packet,
        };

        // Return info and current position (which is start of diff data)
        Ok(Some((info, start_pos + 152)))
    }

    pub fn extract_packet_diff(&mut self, size: i32) -> Result<Vec<u8>, RsbPatchError> {
        let mut buf = vec![0u8; size as usize];
        self.reader.read_exact(&mut buf)?;
        Ok(buf)
    }
}
