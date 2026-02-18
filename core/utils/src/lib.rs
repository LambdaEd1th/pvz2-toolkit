use std::io::{self, Read, Seek, SeekFrom, Write};
pub mod file_list;

/// Extension trait for reading binary data similar to common patterns in the codebase
pub trait BinReadExt: Read {
    /// Read a null-terminated string (reads until 0x00)
    fn read_null_term_string(&mut self) -> io::Result<String> {
        let mut bytes = Vec::new();
        loop {
            let mut buf = [0u8; 1];
            self.read_exact(&mut buf)?;
            if buf[0] == 0 {
                break;
            }
            bytes.push(buf[0]);
        }
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    /// Read a fixed-length string and trim nulls
    fn read_fixed_string(&mut self, len: usize) -> io::Result<String> {
        let mut buf = vec![0u8; len];
        self.read_exact(&mut buf)?;
        Ok(String::from_utf8_lossy(&buf).trim_matches('\0').to_string())
    }
}

/// Extension trait for writing binary data
pub trait BinWriteExt: Write {
    /// Write a string followed by a null terminator
    fn write_null_term_string(&mut self, s: &str) -> io::Result<()> {
        self.write_all(s.as_bytes())?;
        self.write_all(&[0u8])?;
        Ok(())
    }

    /// Write a string padded with nulls to a fixed length
    fn write_fixed_string(&mut self, s: &str, len: usize) -> io::Result<()> {
        let bytes = s.as_bytes();
        if bytes.len() > len {
            // Truncate if too long? Or error?
            // For now, write truncated to fit
            self.write_all(&bytes[..len])?;
        } else {
            self.write_all(bytes)?;
            let padding = len - bytes.len();
            let zeros = vec![0u8; padding];
            self.write_all(&zeros)?;
        }
        Ok(())
    }

    /// Write padding bytes to align to the given boundary
    fn align(&mut self, pos: u64, alignment: u64) -> io::Result<usize> {
        let remainder = pos % alignment;
        if remainder == 0 {
            return Ok(0);
        }
        let padding = alignment - remainder;
        let zeros = vec![0u8; padding as usize];
        self.write_all(&zeros)?;
        Ok(padding as usize)
    }
}

// Implement for all types that implement Read/Write
impl<R: Read + ?Sized> BinReadExt for R {}
impl<W: Write + ?Sized> BinWriteExt for W {}

/// Helper to read a string from a specific offset and return to original position
pub fn read_string_at<R: Read + Seek>(reader: &mut R, offset: u64) -> io::Result<String> {
    let current = reader.stream_position()?;
    reader.seek(SeekFrom::Start(offset))?;
    let s = reader.read_null_term_string()?;
    reader.seek(SeekFrom::Start(current))?;
    Ok(s)
}
