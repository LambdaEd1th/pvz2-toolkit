use crate::error::{WemError, WemResult};
use std::io::{Read, Seek, SeekFrom, Write};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::{MediaSourceStream, ReadOnlySource};
use symphonia::core::meta::MetadataOptions;

#[allow(dead_code)]
pub fn extract_aac<R: Read + Seek, W: Write>(
    mut input: R,
    mut output: W,
    data_offset: u64,
    data_size: u32,
) -> WemResult<()> {
    input.seek(SeekFrom::Start(data_offset))?;
    let mut handle = input.take(data_size as u64);
    std::io::copy(&mut handle, &mut output)?;
    Ok(())
}

pub fn extract_wem_aac<R: Read + Seek, W: Write>(mut input: R, output: W) -> WemResult<()> {
    // Scan for data chunk
    // Similar logic to wav::get_wem_format or wem_to_wav
    let mut current_offset = 12u64;
    let mut header = [0u8; 12];
    input.seek(SeekFrom::Start(0))?;
    input.read_exact(&mut header)?;

    let is_little_endian = &header[0..4] == b"RIFF";
    // Check RIFF/RIFX
    if &header[0..4] != b"RIFF" && &header[0..4] != b"RIFX" {
        return Err(WemError::parse("Not a RIFF/RIFX file"));
    }

    loop {
        input.seek(SeekFrom::Start(current_offset))?;
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

        if chunk_id == b"data" {
            let data_offset = current_offset + 8;
            return extract_aac(input, output, data_offset, chunk_size);
        }

        current_offset += 8 + chunk_size as u64;
        if chunk_size % 2 != 0 {
            current_offset += 1;
        }
    }

    Err(WemError::parse("data chunk not found"))
}

pub fn decode_aac_to_wav<R: Read + Seek + Send + Sync + 'static, W: Write + Seek>(
    mut input: R,
    output: W,
    data_offset: u64,
    data_size: u32,
    channels: u16,
    sample_rate: u32,
) -> WemResult<()> {
    input.seek(SeekFrom::Start(data_offset))?;

    // Create a media source stream from the input
    // We limit it to data_size so symphonia doesn't read past the chunk
    let constrained_input = Box::new(ReadOnlySource::new(input.take(data_size as u64)));
    let mss = MediaSourceStream::new(constrained_input, Default::default());

    // Hint: AAC/ADTS
    let mut hint = symphonia::core::probe::Hint::new();
    hint.with_extension("aac");
    hint.with_extension("m4a"); // just in case

    // Probe
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| WemError::parse(format!("Symphonia probe error: {}", e)))?;

    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| WemError::parse("No supported audio track found"))?;

    let track_id = track.id;
    let mut codec_params = track.codec_params.clone();

    // Force params from WEM header if missing or raw
    if codec_params.channels.is_none() {
        codec_params.channels = Some(match channels {
            1 => symphonia::core::audio::Channels::FRONT_CENTRE,
            2 => {
                symphonia::core::audio::Channels::FRONT_LEFT
                    | symphonia::core::audio::Channels::FRONT_RIGHT
            }
            _ => symphonia::core::audio::Channels::from_bits_truncate(channels as u32), // Fallback/Hope it matches mask
        });
    }
    if codec_params.sample_rate.is_none() {
        codec_params.sample_rate = Some(sample_rate);
    }
    // Wwise AAC is usually LC (?)
    if codec_params.codec == CODEC_TYPE_NULL {
        codec_params.codec = symphonia::core::codecs::CODEC_TYPE_AAC;
    }

    // Check extra data/magic cookie if needed?

    // Decoder

    // Decoder
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| WemError::parse(format!("Symphonia codec error: {}", e)))?;

    // WAV Writer
    let spec = hound::WavSpec {
        channels: codec_params
            .channels
            .ok_or(WemError::parse("Unknown channel count"))?
            .count() as u16,
        sample_rate: codec_params
            .sample_rate
            .ok_or(WemError::parse("Unknown sample rate"))?,
        bits_per_sample: 16, // Decode to 16-bit
        sample_format: hound::SampleFormat::Int,
    };
    let mut wav_writer = hound::WavWriter::new(output, spec).map_err(WemError::Wav)?;

    // Decode loop
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(_)) => break, // EOF
            Err(e) => return Err(WemError::parse(format!("Packet read error: {}", e))),
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                let mut sample_buf =
                    SampleBuffer::<i16>::new(decoded.capacity() as u64, *decoded.spec());
                sample_buf.copy_interleaved_ref(decoded);
                let samples = sample_buf.samples();
                for sample in samples {
                    wav_writer.write_sample(*sample).map_err(WemError::Wav)?;
                }
            }
            Err(e) => {
                // Ignore decode errors?
                eprintln!("Decode error: {}", e);
                // break;
            }
        }
    }

    wav_writer.finalize().map_err(WemError::Wav)?;

    Ok(())
}
