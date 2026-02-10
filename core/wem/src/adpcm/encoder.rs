use super::decoder::{IMA_INDEX_TABLE, IMA_STEP_TABLE};
use crate::error::{WemError, WemResult};
use byteorder::{LittleEndian, WriteBytesExt};
use std::io::{Cursor, Read, Write};

pub struct WavToAdpcm {
    input: Cursor<Vec<u8>>,
    channels: u16,
    sample_rate: u32,
    data_size: u32,
}

impl WavToAdpcm {
    pub fn new<R: Read>(mut input: R) -> WemResult<Self> {
        // Read entire file into memory to avoid fs quirks with hound
        let mut buffer = Vec::new();
        input.read_to_end(&mut buffer).map_err(WemError::Io)?;
        let cursor = Cursor::new(buffer);

        // Use hound to read WAV header from memory
        let wav_reader = hound::WavReader::new(cursor.clone()).map_err(WemError::Wav)?;
        let spec = wav_reader.spec();
        let len = wav_reader.len();

        // Validate format
        if spec.sample_format != hound::SampleFormat::Int {
            return Err(WemError::parse("Input WAV must be integer PCM"));
        }
        if spec.bits_per_sample != 16 {
            return Err(WemError::parse(
                "Input WAV must be 16-bit PCM for ADPCM conversion",
            ));
        }

        let channels = spec.channels;
        let sample_rate = spec.sample_rate;
        let _bits_per_sample = spec.bits_per_sample;
        let _num_samples = len;

        // Calculate output data size
        // Blocks of 64 samples = 36 bytes per channel.
        let samples_per_block = 64;
        let block_size_per_channel = 36;
        let num_blocks = (len + samples_per_block - 1) / samples_per_block;
        let data_size = num_blocks * block_size_per_channel as u32 * channels as u32;

        Ok(Self {
            input: cursor,
            channels,
            sample_rate,
            data_size,
        })
    }

    pub fn process<W: Write>(&mut self, mut writer: W) -> WemResult<()> {
        // Read all samples
        self.input.set_position(0);
        let mut wav_reader = hound::WavReader::new(self.input.clone()).map_err(WemError::Wav)?;
        let samples: Vec<i16> = wav_reader.samples::<i16>().map(|s| s.unwrap()).collect();

        // De-interleave samples
        let mut channel_samples: Vec<Vec<i16>> = vec![Vec::new(); self.channels as usize];
        for (i, sample) in samples.iter().enumerate() {
            let ch = i % self.channels as usize;
            channel_samples[ch].push(*sample);
        }

        // Encode blocks
        let samples_per_block = 64;
        // Pad samples to multiple of 64
        for ch_samples in &mut channel_samples {
            while ch_samples.len() % samples_per_block != 0 {
                ch_samples.push(0);
            }
        }
        let num_blocks = channel_samples[0].len() / samples_per_block;

        // Prepare Output Buffer for Data Chunk
        let mut data_chunk_buffer = Vec::with_capacity(self.data_size as usize);

        // Initial predictor/step state (start fresh or carry over?)
        // Wwise ADPCM blocks are independent. State resets per block,
        // BUT the header of the block contains the starting predictor/index.
        // Theoretically, to emulate continuous playback, we should use the end state of previous block
        // as the start state of current block.
        // However, Wwise format stores the predictor in the block header.

        let mut states: Vec<AdpcmState> = vec![AdpcmState::default(); self.channels as usize];

        // Interleave blocks: For each block index, write block for Ch0, Ch1, ...
        for block_idx in 0..num_blocks {
            for ch in 0..self.channels as usize {
                let start_sample = block_idx * samples_per_block;
                let end_sample = start_sample + samples_per_block;
                let block_samples = &channel_samples[ch][start_sample..end_sample];

                // Encode block
                let block_data = encode_block(block_samples, &mut states[ch]);
                data_chunk_buffer.extend_from_slice(&block_data);
            }
        }

        // Recalculate true data size
        self.data_size = data_chunk_buffer.len() as u32;

        // RIFF Header
        let _fmt_chunk_size = 0x20; // 32 bytes for ADPCM fmt? Check wav.rs logic for 0x8311?
        // wav.rs doesn't enforce size for 0x8311, but standard is usually 0x20 or less?
        // Let's use standard waveformat ex + 2 bytes extra = 18 + 2 = 20 (0x14)?
        // Or Wwise specific. Wwise 0x8311 usually has size 0x14 or 0x12.
        // Let's check `wav.rs`... `block_align` is read at offset 12.
        // `bits_per_sample` at 14.
        // `extra_size` at 16.
        // So standard 18 bytes.
        // Let's safe-bet on 18 bytes + 2 bytes padding/extra = 20 bytes?
        // Or just 18 bytes?
        // Wwise ADPCM usually has 0 extra bytes.
        // Let's try 0x12 (18 bytes).

        let fmt_chunk_len = 0x12;

        let riff_payload_size = 4 + // WAVE
                                8 + fmt_chunk_len + // fmt
                                8 + self.data_size; // data

        writer.write_all(b"RIFF")?;
        writer.write_u32::<LittleEndian>(riff_payload_size)?;
        writer.write_all(b"WAVE")?;

        // fmt chunk
        writer.write_all(b"fmt ")?;
        writer.write_u32::<LittleEndian>(fmt_chunk_len)?;
        writer.write_u16::<LittleEndian>(0x8311)?; // Wwise IMA ADPCM
        writer.write_u16::<LittleEndian>(self.channels)?;
        writer.write_u32::<LittleEndian>(self.sample_rate)?;

        // Block Align: 36 * channels
        let block_align = 36 * self.channels;
        // Avg Bytes: sample_rate / 64 * 36 * channels?
        // Or roughly: rate * channels * 36 / 64
        let avg_bytes = (self.sample_rate as u64 * block_align as u64 / 64) as u32;

        writer.write_u32::<LittleEndian>(avg_bytes)?;
        writer.write_u16::<LittleEndian>(block_align)?;

        // Bits per sample? 4?
        // Wwise might put 4 here.
        writer.write_u16::<LittleEndian>(4)?;

        // Extra size: 0
        writer.write_u16::<LittleEndian>(0)?;

        // data chunk
        writer.write_all(b"data")?;
        writer.write_u32::<LittleEndian>(self.data_size)?;
        writer.write_all(&data_chunk_buffer)?;

        Ok(())
    }
}

#[derive(Clone, Default)]
struct AdpcmState {
    predictor: i32,
    step_index: i32,
}

fn encode_block(samples: &[i16], state: &mut AdpcmState) -> Vec<u8> {
    // 36 bytes: 4 byte header + 32 bytes data
    let mut block = vec![0u8; 36];

    // Header: Predictor (i16) -> 2 bytes
    //         Step Index (u8) -> 1 byte
    //         Reserved (u8) -> 1 byte

    // We use the first sample as the starting predictor for the block?
    // Wwise decoder logic:
    // Header sample IS the first output sample.
    // So `samples[0]` is written to header.
    // AND `samples[0]` becomes the predictor state for the *next* nibbles.

    let predictor = samples[0] as i32;
    // Step index? We need to find a good step index?
    // Or just carry over from previous block?
    // Wwise encoder likely carries over.
    // But for the very first block, defaults to 0.
    // Also, we should probably clamp it.

    // Write Header
    let pred_i16 = predictor as i16;
    block[0] = pred_i16 as u8;
    block[1] = (pred_i16 >> 8) as u8;
    block[2] = state.step_index as u8;
    block[3] = 0;

    // Update state to match what decoder will have after reading header
    state.predictor = predictor;
    // state.step_index remains same

    // Encode remaining 63 samples
    // Layout:
    // Byte 4 contains: Low nibble = Sample 1 (logic index), High nibble = Sample 2
    // Decoder:
    // for i in 1..64:
    //   byte_idx = 4 + (i-1)/2
    //   shift = ((i-1)&1) ? 4 : 0

    let mut bit_buffer = 0u8;

    for i in 1..64 {
        let sample = samples[i];
        let _diff = sample as i32 - state.predictor;
        let step = IMA_STEP_TABLE[state.step_index as usize];

        // Calculate best nibble
        // Decoder delta = ((nibble & 7) * 2 + 1) * step / 8
        // If nibble & 8, delta = -delta.

        // We want: predictor + delta ~= sample
        // delta ~= diff

        let mut best_nibble = 0;
        let mut min_error = i32::MAX;

        // Brute force 16 nibbles? It's fast enough.
        for nibble in 0..16 {
            let mut delta = (nibble & 0x7) as i32;
            delta = ((delta * 2 + 1) * step) >> 3;
            if (nibble & 8) != 0 {
                delta = -delta;
            }

            // Predict
            let pred = (state.predictor + delta).clamp(-32768, 32767);
            let error = (sample as i32 - pred).abs();

            if error < min_error {
                min_error = error;
                best_nibble = nibble;
            }
        }

        // Apply best nibble to update state
        let nibble = best_nibble;
        let mut delta = (nibble & 0x7) as i32;
        delta = ((delta * 2 + 1) * step) >> 3;
        if (nibble & 8) != 0 {
            delta = -delta;
        }
        state.predictor = (state.predictor + delta).clamp(-32768, 32767);
        state.step_index =
            (state.step_index + IMA_INDEX_TABLE[nibble as usize] as i32).clamp(0, 88);

        // Pack nibble
        // Decoder: (byte >> shift) & 0x0F
        // i=1 (first sample after header): (1-1)/2 = 0 -> byte 4. shift=0. -> Low nibble.
        // i=2: (2-1)/2 = 0 -> byte 4. shift=4 -> High nibble.

        if (i - 1) % 2 == 0 {
            bit_buffer = nibble as u8; // Low nibble
        } else {
            bit_buffer |= (nibble as u8) << 4; // High nibble
            // Write byte
            let byte_idx = 4 + (i - 1) / 2;
            block[byte_idx] = bit_buffer;
        }
    }

    // Note: The loop finishes at i=63.
    // i=63: (62)/2 = 31. byte_idx = 35. This is the last byte of 36-byte block (idx 0..35).
    // i=63 is odd -> High nibble written.
    // If we had even samples (impossible with 64), we'd need to flush.

    block
}

#[cfg(test)]
mod tests {
    use super::*;
    use hound;
    use std::io::Cursor;

    #[test]
    fn test_adpcm_roundtrip() {
        // 1. Generate Synthetic WAV (Mono, 16-bit, 44100Hz, Sine Wave)
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut wav_buffer = Vec::new();
        let mut wav_writer = hound::WavWriter::new(Cursor::new(&mut wav_buffer), spec).unwrap();

        // 1000 samples approx 0.02s
        for t in 0..1000 {
            let v = (t as f32 * 0.1).sin() * 10000.0;
            wav_writer.write_sample(v as i16).unwrap();
        }
        wav_writer.finalize().unwrap();

        // 2. Encode to ADPCM WEM
        let mut wem_buffer = Vec::new();
        let mut encoder = WavToAdpcm::new(Cursor::new(wav_buffer)).unwrap();
        encoder.process(&mut wem_buffer).unwrap();

        // 3. Decode back to WAV
        // We need CodebookLibrary for wem_to_wav even if ADPCM doesn't use it
        let codebooks = crate::CodebookLibrary::embedded_standard();

        // We need to use crate::wav::wem_to_wav.
        // Since we are in core/wem/src/adpcm/encoder.rs, crate::wav is reachable.

        let mut decoded_wav_buffer = Vec::new();
        crate::wav::wem_to_wav(
            Cursor::new(wem_buffer),
            Cursor::new(&mut decoded_wav_buffer),
            &codebooks,
        )
        .expect("Failed to decode ADPCM WEM");

        // 4. Verify Output
        let mut wav_reader = hound::WavReader::new(Cursor::new(decoded_wav_buffer)).unwrap();
        let decoded_spec = wav_reader.spec();

        assert_eq!(decoded_spec.channels, 1);
        assert_eq!(decoded_spec.sample_rate, 44100);
        assert_eq!(decoded_spec.bits_per_sample, 16);

        let samples: Vec<i16> = wav_reader.samples::<i16>().map(|s| s.unwrap()).collect();
        // Wwise ADPCM is block-based (64 samples).
        // 1000 samples -> ceil(1000/64) * 64 = 16 * 64 = 1024 samples expected?
        // Let's check logic.
        // Yes, decoder fills blocks.

        assert!(samples.len() >= 1000);
        assert!(samples.len() <= 1024); // Should be padded to block size
    }
}
