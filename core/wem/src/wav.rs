use crate::adpcm::AdpcmParams;
use crate::pcm::PcmParams;
use crate::{CodebookLibrary, WwiseRiffVorbis, error::WemError};
use hound;
use std::io::{Cursor, Read, Seek, Write};
use symphonia::core::audio::AudioBufferRef;
use symphonia::core::codecs::{CODEC_TYPE_VORBIS, DecoderOptions};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub fn get_wem_format<R: Read + Seek>(mut input: R) -> Result<u16, WemError> {
    let mut header = [0u8; 12];
    input.read_exact(&mut header).map_err(WemError::Io)?;

    let is_little_endian = &header[0..4] == b"RIFF";
    let is_riff = is_little_endian || &header[0..4] == b"RIFX";

    if !is_riff {
        return Err(WemError::parse("Not a RIFF/RIFX file"));
    }

    let mut current_offset = 12u64;
    loop {
        input
            .seek(std::io::SeekFrom::Start(current_offset))
            .map_err(WemError::Io)?;
        let mut chunk_header = [0u8; 8];
        if input.read_exact(&mut chunk_header).is_err() {
            break;
        }

        let chunk_id = &chunk_header[0..4];
        let chunk_size = if is_little_endian {
            u32::from_le_bytes(chunk_header[4..8].try_into().unwrap())
        } else {
            u32::from_be_bytes(chunk_header[4..8].try_into().unwrap())
        };

        if chunk_id == b"fmt " {
            let mut fmt_data = vec![0u8; chunk_size as usize];
            input.read_exact(&mut fmt_data).map_err(WemError::Io)?;

            let format_tag = if is_little_endian {
                u16::from_le_bytes(fmt_data[0..2].try_into().unwrap())
            } else {
                u16::from_be_bytes(fmt_data[0..2].try_into().unwrap())
            };
            return Ok(format_tag);
        }

        current_offset += 8 + chunk_size as u64;
        if chunk_size % 2 != 0 {
            current_offset += 1;
        }
    }

    Err(WemError::parse("fmt chunk not found"))
}

pub fn wem_to_wav<R: Read + Seek + Send + Sync + 'static, W: Write + Seek>(
    input: R,
    output: W,
    codebooks: &CodebookLibrary,
) -> Result<(), WemError> {
    // Read RIFF header to determine format
    let mut reader = std::io::BufReader::new(input);
    let mut header = [0u8; 12];
    reader.read_exact(&mut header)?;

    // Check RIFF/RIFX
    let is_little_endian = &header[0..4] == b"RIFF";
    let is_riff = is_little_endian || &header[0..4] == b"RIFX";

    if !is_riff {
        return Err(WemError::parse("Not a RIFF/RIFX file"));
    }

    // Seek to fmt chunk
    let mut current_offset = 12u64;
    let mut fmt_found = false;
    let mut data_found = false;

    let mut format_tag = 0u16;
    let mut channels = 0u16;
    let mut sample_rate = 0u32;
    let mut block_align = 0u16;
    // let mut bits_per_sample = 0u16; // Unused for Vorbis/AAC, extracted for PCM

    let mut data_offset = 0u64;
    let mut data_size = 0u32;
    let mut bits_per_sample = 16; // Default or extracted

    loop {
        reader.seek(std::io::SeekFrom::Start(current_offset))?;
        let mut chunk_header = [0u8; 8];
        if reader.read_exact(&mut chunk_header).is_err() {
            break;
        }

        let chunk_id = &chunk_header[0..4];
        let chunk_size = if is_little_endian {
            u32::from_le_bytes(chunk_header[4..8].try_into().unwrap())
        } else {
            u32::from_be_bytes(chunk_header[4..8].try_into().unwrap())
        };

        if chunk_id == b"fmt " {
            let mut fmt_data = vec![0u8; chunk_size as usize];
            reader.read_exact(&mut fmt_data)?;

            if is_little_endian {
                format_tag = u16::from_le_bytes(fmt_data[0..2].try_into().unwrap());
                channels = u16::from_le_bytes(fmt_data[2..4].try_into().unwrap());
                sample_rate = u32::from_le_bytes(fmt_data[4..8].try_into().unwrap());
                // block_align at 12
                if chunk_size >= 14 {
                    block_align = u16::from_le_bytes(fmt_data[12..14].try_into().unwrap());
                }
                // valid for PCM/ADPCM
                if chunk_size >= 16 {
                    bits_per_sample = u16::from_le_bytes(fmt_data[14..16].try_into().unwrap());
                }
            } else {
                format_tag = u16::from_be_bytes(fmt_data[0..2].try_into().unwrap());
                channels = u16::from_be_bytes(fmt_data[2..4].try_into().unwrap());
                sample_rate = u32::from_be_bytes(fmt_data[4..8].try_into().unwrap());
                if chunk_size >= 14 {
                    block_align = u16::from_be_bytes(fmt_data[12..14].try_into().unwrap());
                }
                if chunk_size >= 16 {
                    bits_per_sample = u16::from_be_bytes(fmt_data[14..16].try_into().unwrap());
                }
            }
            fmt_found = true;
        } else if chunk_id == b"data" {
            data_offset = current_offset + 8;
            data_size = chunk_size;
            data_found = true;
        }

        if fmt_found && data_found {
            break;
        }

        current_offset += 8 + chunk_size as u64;
        // Pad byte if odd
        if chunk_size % 2 != 0 {
            current_offset += 1;
        }
    }

    if !fmt_found {
        return Err(WemError::parse("fmt chunk not found"));
    }
    if !data_found {
        return Err(WemError::parse("data chunk not found"));
    }

    // Reset reader if needed, but we pass data_offset so it's fine.
    // For Vorbis, WwiseRiffVorbis expects full file access.
    reader.seek(std::io::SeekFrom::Start(0))?;

    match format_tag {
        0xFFFF => {
            // Vorbis
            // 1. Convert WEM to Ogg (in-memory)
            let mut converter = WwiseRiffVorbis::new(reader, codebooks.clone())?;
            let mut ogg_buffer = Vec::new();
            converter.generate_ogg(&mut ogg_buffer)?;

            // 2. Decode Ogg to PCM using symphonia
            let cursor = Cursor::new(ogg_buffer);
            decode_ogg_to_wav(cursor, output)
                .map_err(|e| WemError::parse(format!("Vorbis decode error: {}", e)))?;
            Ok(())
        }
        0x8311 => {
            // Wwise IMA ADPCM
            crate::adpcm::process_adpcm(
                reader,
                output,
                AdpcmParams {
                    channels,
                    sample_rate,
                    block_align,
                    is_little_endian,
                    data_offset,
                    data_size,
                },
            )?;
            Ok(())
        }
        0xAAC0 => {
            // Wwise AAC (Decode to WAV)
            crate::aac::decode_aac_to_wav(
                reader,
                output,
                data_offset,
                data_size,
                channels,
                sample_rate,
            )
            .map_err(|e| WemError::parse(format!("AAC Decode error: {}", e)))?;
            Ok(())
        }
        0x0001 | 0xFFFE => {
            // PCM
            crate::pcm::process_pcm(
                reader,
                output,
                PcmParams {
                    channels,
                    sample_rate,
                    bits_per_sample,
                    is_little_endian,
                    data_offset,
                    data_size,
                },
            )?;
            Ok(())
        }
        _ => Err(WemError::parse(format!(
            "Unsupported format tag: 0x{:04X}",
            format_tag
        ))),
    }
}

// Helper to decode OGG (Vorbis) using symphonia
use symphonia::core::audio::Signal;

fn decode_ogg_to_wav<W: Write + Seek>(
    input: Cursor<Vec<u8>>,
    output: W,
) -> Result<(), Box<dyn std::error::Error>> {
    let mss = MediaSourceStream::new(Box::new(input), Default::default());

    let mut hint = Hint::new();
    hint.with_extension("ogg");

    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts: FormatOptions = Default::default();

    let probed = symphonia::default::get_probe().format(&hint, mss, &fmt_opts, &meta_opts)?;
    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec == CODEC_TYPE_VORBIS)
        .ok_or("no vorbis track found")?;

    let dec_opts: DecoderOptions = Default::default();
    let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &dec_opts)?;

    let track_id = track.id;

    let spec = hound::WavSpec {
        channels: track.codec_params.channels.unwrap().count() as u16,
        sample_rate: track.codec_params.sample_rate.unwrap(),
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut wav_writer = hound::WavWriter::new(output, spec)?;

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(e) => return Err(Box::new(e)),
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => match decoded {
                AudioBufferRef::F32(buf) => {
                    for frame in 0..buf.frames() {
                        for channel in 0..buf.spec().channels.count() {
                            let sample = buf.chan(channel)[frame];
                            let sample = (sample * i16::MAX as f32) as i16;
                            wav_writer.write_sample(sample)?;
                        }
                    }
                }
                AudioBufferRef::S16(buf) => {
                    for frame in 0..buf.frames() {
                        for channel in 0..buf.spec().channels.count() {
                            let sample = buf.chan(channel)[frame];
                            wav_writer.write_sample(sample)?;
                        }
                    }
                }
                _ => unimplemented!("unsupported audio buffer type"),
            },
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(e) => return Err(Box::new(e)),
        }
    }
    wav_writer.finalize()?;
    Ok(())
}
