use crate::error::{WemError, WemResult};
use byteorder::{LittleEndian, WriteBytesExt};
use std::io::{Cursor, Read, Write};

pub struct WavToWem {
    input: Cursor<Vec<u8>>,
    channels: u16,
    sample_rate: u32,
    bits_per_sample: u16,
    data_size: u32,
}

impl WavToWem {
    pub fn new<R: Read>(mut input: R) -> WemResult<Self> {
        // Read entire file into memory to avoid fs quirks with hound
        let mut buffer = Vec::new();
        input.read_to_end(&mut buffer).map_err(WemError::Io)?;
        let cursor = Cursor::new(buffer);

        // Use hound to read WAV header from memory
        let wav_reader = hound::WavReader::new(cursor.clone()).map_err(WemError::Wav)?;
        let spec = wav_reader.spec();
        let len = wav_reader.len();

        let channels = spec.channels;
        let sample_rate = spec.sample_rate;
        let bits_per_sample = spec.bits_per_sample;
        let data_size = len * (bits_per_sample as u32) / 8;

        Ok(Self {
            input: cursor,
            channels,
            sample_rate,
            bits_per_sample,
            data_size,
        })
    }

    pub fn process<W: Write>(&mut self, mut writer: W) -> WemResult<()> {
        // WEM PCM Structure
        // RIFF
        // fmt (0x0001)
        // data

        // Re-read from our in-memory input
        self.input.set_position(0);
        let mut wav_reader = hound::WavReader::new(self.input.clone()).map_err(WemError::Wav)?;

        // Start writing WEM
        // RIFF header
        let fmt_chunk_size = 16; // Standard PCM
        let riff_payload_size = 4 + // WAVE
                                8 + fmt_chunk_size + // fmt
                                8 + self.data_size; // data

        writer.write_all(b"RIFF")?;
        writer.write_u32::<LittleEndian>(riff_payload_size)?;
        writer.write_all(b"WAVE")?;

        // fmt chunk
        writer.write_all(b"fmt ")?;
        writer.write_u32::<LittleEndian>(fmt_chunk_size)?;
        writer.write_u16::<LittleEndian>(0x0001)?; // PCM
        writer.write_u16::<LittleEndian>(self.channels)?;
        writer.write_u32::<LittleEndian>(self.sample_rate)?;

        let block_align = self.channels * self.bits_per_sample / 8;
        let avg_bytes = self.sample_rate * block_align as u32;

        writer.write_u32::<LittleEndian>(avg_bytes)?;
        writer.write_u16::<LittleEndian>(block_align)?;
        writer.write_u16::<LittleEndian>(self.bits_per_sample)?;

        // data chunk
        writer.write_all(b"data")?;
        writer.write_u32::<LittleEndian>(self.data_size)?;

        // Copy samples
        match self.bits_per_sample {
            8 => {
                for s in wav_reader.samples::<i8>() {
                    let sample = s.map_err(WemError::Wav)?;
                    writer.write_i8(sample)?;
                }
            }
            16 => {
                for s in wav_reader.samples::<i16>() {
                    let sample = s.map_err(WemError::Wav)?;
                    writer.write_i16::<LittleEndian>(sample)?;
                }
            }
            24 => {
                for s in wav_reader.samples::<i32>() {
                    let sample = s.map_err(WemError::Wav)?;
                    writer.write_i24::<LittleEndian>(sample)?;
                }
            }
            32 => {
                for s in wav_reader.samples::<i32>() {
                    let sample = s.map_err(WemError::Wav)?;
                    writer.write_i32::<LittleEndian>(sample)?;
                }
            }
            _ => return Err(WemError::parse("Unsupported bit depth for wrapping")),
        }

        Ok(())
    }
}
