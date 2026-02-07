use crate::error::{WemError, WemResult};
use hound;
use std::io::{Read, Seek, SeekFrom, Write};

pub struct PcmParams {
    pub channels: u16,
    pub sample_rate: u32,
    pub bits_per_sample: u16,
    pub is_little_endian: bool,
    pub data_offset: u64,
    pub data_size: u32,
}

pub fn process_pcm<R: Read + Seek, W: Write + Seek>(
    mut input: R,
    output: W,
    params: PcmParams,
) -> WemResult<()> {
    input.seek(SeekFrom::Start(params.data_offset))?;

    let spec = hound::WavSpec {
        channels: params.channels,
        sample_rate: params.sample_rate,
        bits_per_sample: params.bits_per_sample,
        sample_format: hound::SampleFormat::Int,
    };

    let mut wav_writer = hound::WavWriter::new(output, spec).map_err(WemError::Wav)?;
    let mut buffer = vec![0u8; params.data_size as usize];
    input.read_exact(&mut buffer)?;

    // Handle endianness swapping if input endianness differs from machine (which is LE for WAV)
    // WAV data is always Little Endian.
    // If input is Big Endian (RIFX), we need to swap.
    let should_swap = !params.is_little_endian;

    if should_swap {
        if params.bits_per_sample == 16 {
            let sample_count = params.data_size / 2;
            let mut cursor = std::io::Cursor::new(buffer);
            for _ in 0..sample_count {
                let mut sample_bytes = [0u8; 2];
                cursor.read_exact(&mut sample_bytes)?;
                let sample = i16::from_be_bytes(sample_bytes);
                wav_writer.write_sample(sample).map_err(WemError::Wav)?;
            }
        } else if params.bits_per_sample == 24 {
            eprintln!("Warning: 24-bit Big Endian PCM not fully implemented, writing raw bytes.");
            return Err(std::io::Error::other("Unsupported bit depth for big-endian PCM").into());
        } else {
            return Err(std::io::Error::other("Unsupported bit depth for big-endian PCM").into());
        }
    } else {
        // Little Endian input -> Little Endian WAV. Just copy samples.
        if params.bits_per_sample == 8 {
            for &byte in &buffer {
                wav_writer.write_sample(byte as i8).map_err(WemError::Wav)?;
            }
        } else if params.bits_per_sample == 16 {
            let sample_count = params.data_size / 2;
            let mut cursor = std::io::Cursor::new(buffer);
            for _ in 0..sample_count {
                let mut sample_bytes = [0u8; 2];
                cursor.read_exact(&mut sample_bytes)?;
                let sample = i16::from_le_bytes(sample_bytes);
                wav_writer.write_sample(sample).map_err(WemError::Wav)?;
            }
        }
    }

    wav_writer.finalize().map_err(WemError::Wav)?;
    Ok(())
}
