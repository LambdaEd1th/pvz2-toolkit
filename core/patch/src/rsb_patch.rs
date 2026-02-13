use byteorder::{ReadBytesExt, LE};
use std::io::{Read, Seek, SeekFrom};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RsbPatchError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid RSBPatch file: {0}")]
    InvalidFile(String),
}

pub struct RsbPatchHeader {
    pub rsb_after_size: i32,
    pub rsb_head_size: i32,
    pub md5_before: [u8; 16],
    pub rsg_number: i32,
    pub rsb_need_patch: bool,
}

pub struct RsbPatchPacketInfo {
    pub packet_patch_size: i32,
    pub packet_name: String,
    pub md5_packet: [u8; 16],
}

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

        // Offset 0x28 -> mapped to existing write logic which seems to put size/offset here?
        // Wait, Dart:
        // senWriter.writeInt32LE(1);
        // senWriter.writeInt32LE(2);
        // senWriter.writeInt32LE(senAfter.length); // 0xC
        // senWriter.writeNull(8); // 0x10, 0x14
        // senWriter.writeBytes(Uint8List.fromList(md5DigestOld.bytes)); // 0x18
        // senWriter.writeInt32LE(senAfter.readInt32LE(0x28)); // 0x28 ? No, readInt32LE(0x28) from senAfter might be fileOffset of header?
        // In Dart readRSBPatchHead:
        // "rsbHeadSize": senFile.readInt32LE(0x14),
        // Wait, writeNull(8) at 0x10 means 0x10-0x17 are null.
        // But readRSBPatchHead reads 0x14 for rsbHeadSize?
        // Let's re-read Dart code carefully.

        // Dart write:
        // 0x00: PBSR
        // 0x04: 1
        // 0x08: 2
        // 0x0C: senAfter.length
        // 0x10: 0 (4 bytes)
        // 0x14: 0 (4 bytes) -> Later overwritten: senWriter.writeInt32LE(rsbHeadDiff.length, 0x14);
        // 0x18: md5DigestOld (16 bytes)
        // 0x28: senAfter.readInt32LE(0x28) (This matches RSB Header logic, likely RSG number or offset)
        // 0x2C: 0 (4 bytes) -> Later overwritten: senWriter.writeInt32LE(1, 0x2C); -> rsbNeedPatch

        // Dart read:
        // "rsbAfterSize": senFile.readInt32LE(0xC),
        // "rsbHeadSize": senFile.readInt32LE(0x14),
        // "md5Before": senFile.readBytes(16), (Implicitly at 0x18?) No `readBytes` usually advances offset unless specified.
        // `readRSBPatchHead` uses absolute offsets for some fields?
        // senFile.readInt32LE(0xC) -> Absolute 0xC
        // senFile.readInt32LE(0x14) -> Absolute 0x14
        // senFile.readBytes(16) -> If stream based, this depends on cursor.
        // senFile.readInt32LE() -> Stream based.

        // Let's assume sequential read after 0x28 (end of MD5 is 0x18+16 = 0x28).
        // 0x28: rsgNumber
        // 0x2C: rsbNeedPatch

        // Re-aligning my read logic to sequential:
        // 0x00: PBSR
        // 0x04: 1
        // 0x08: 2
        // 0x0C: rsb_after_size
        // 0x10: Padding (4 bytes)
        // 0x14: rsb_head_size (diff size)
        // 0x18: md5_before (16 bytes)
        // 0x28: rsg_number
        // 0x2C: rsb_need_patch (1 = true, 0 = false)

        // Let's implement seeking to be safe or just read sequentially since structure is fixed.

        // Back to 0x10:
        // Dart write: senWriter.writeNull(8); -> Writes 8 zero bytes.
        // Offsets: 0x10, 0x11 ... 0x17.
        // 0x14 is inside this null block.
        // But then: senWriter.writeInt32LE(rsbHeadDiff.length, 0x14); -> Overwrites 0x14-0x17.
        // So 0x10-0x13 is 0. 0x14-0x17 is rsbHeadSize.

        // So sequential read:
        // 0x10: Skip 4 bytes.
        // 0x14: Read rsb_head_size.

        // Resume implementation:

        // Already read up to 0x10 (rsb_after_size at 0xC + read).
        // Actually I read 0x8 padding (writeNull(8)).
        // I need to seek back or re-read?
        // Let's just read strictly.

        // seek to 0x14
        self.reader.seek(SeekFrom::Start(0x14))?;
        let rsb_head_size = self.reader.read_i32::<LE>()?;

        // seek to 0x18
        self.reader.seek(SeekFrom::Start(0x18))?;
        let mut md5_before = [0u8; 16];
        self.reader.read_exact(&mut md5_before)?;

        // 0x28
        let rsg_number = self.reader.read_i32::<LE>()?;

        // 0x2C
        let rsb_need_patch_flag = self.reader.read_i32::<LE>()?;
        let rsb_need_patch = rsb_need_patch_flag == 1;

        Ok(RsbPatchHeader {
            rsb_after_size,
            rsb_head_size,
            md5_before,
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

        // Header diff is presumably after the header block?
        // Dart:
        // senWriter.writeBytes(rsbHeadDiff);
        // Written right after 0x2C + 4 = 0x30?
        // Wait, 0x2C is written at 0x2C (4 bytes). End is 0x30.
        // Yes.

        self.reader.seek(SeekFrom::Start(0x30))?;
        let mut buf = vec![0u8; header.rsb_head_size as usize];
        self.reader.read_exact(&mut buf)?;
        Ok(Some(buf))
    }

    pub fn next_packet_info(&mut self) -> Result<Option<(RsbPatchPacketInfo, u64)>, RsbPatchError> {
        // Read packet info
        // Dart readSubGroupInfo:
        // startOffset = current
        // packetPatchSize = readInt32LE(startOffset + 4)
        // packetName = readStringByEmpty() (at startOffset + ?)
        // md5Packet = readBytes(16, startOffset + 136)

        // Dart write loop:
        // senWriter.writeNull(8); (0x00 - 0x07)
        // senWriter.writeString(packetAfterName); (0x08...)
        // senWriter.writeNull((0x80 - packetAfterName.length)); -> Padding to 0x80 (128 bytes) for name?
        // Wait: writeString writes len + chars? Or just chars?
        // SenBuffer.writeString usually writes 4-byte len then string.
        // But readStringByEmpty implies null-terminated or fixed size?
        // Let's check `readStringByEmpty` in Sen if possible, or assume standard.
        // Actually: `senWriter.writeString(packetAfterName)`
        // `senWriter.writeNull((0x80 - packetAfterName.length).toInt());`
        // This suggests a fixed buffer of 128 (0x80) bytes for the name is RESERVED, but `writeString` behavior matters.
        // If `writeString` writes length, then the math `0x80 - length` is aiming for 128 bytes TOTAL for the string field.
        // Let's assume `writeString` writes the string bytes (utf8) + null terminator? Or just bytes?
        // In typical binary formats, fixed size strings are common.

        // Also: `md5Packet` written after name padding.
        // `final digestAfter = md5.convert(packetAfter);`
        // `senWriter.writeBytes(Uint8List.fromList(digestAfter.bytes));`

        // And then:
        // `senWriter.writeInt32LE(subGroupDiff.length, pos - 148);`
        // pos is where? `pos = senWriter.writeOffset` (before writing diff).
        // 148 bytes back?
        // Packet Header Size:
        // 8 (Null) + String + Padding + 16 (MD5) = ?
        // If String + Padding is 128 bytes.
        // 8 + 128 + 16 = 152 bytes?
        // 148?

        // Let's trace `pos - 148`.
        // The write structure:
        // 1. writeNull(8) [0..8]
        // 2. writeString(name) [8..?]
        // 3. writeNull(128 - name.len) [?..136] (assuming writeString is just bytes)
        // 4. writeBytes(MD5) [136..152] (16 bytes)
        // Total so far: 152 bytes.

        // `pos` is current offset (152).
        // `pos - 148` = 4.
        // `writeInt32LE(len, 4)` -> Writes to offset 4 of the packet header.
        // This matches `readInt32LE(startOffset + 4)` for `packetPatchSize`.

        // So structure:
        // 0x00: Padding (4 bytes)
        // 0x04: Packet Patch Size (4 bytes)
        // 0x08: Packet Name (128 bytes fixed field, presumably null-terminated or just padded)
        // 0x88 (136): MD5 (16 bytes)
        // 0x98 (152): End of Header, Start of Diff Data.

        // Total header size = 152 bytes.

        // Let's implement this.

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
