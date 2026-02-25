#![allow(clippy::collapsible_if)]
use crate::error::{WemError, WemResult};
use byteorder::{LittleEndian, WriteBytesExt};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use symphonia::core::codecs::CODEC_TYPE_NULL;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;

pub fn probe_m4a_metadata(path: &Path) -> WemResult<(u16, u32, u32)> {
    let file = File::open(path).map_err(WemError::Io)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &Default::default(), &Default::default())
        .map_err(|e| WemError::parse(format!("Failed to probe M4A: {}", e)))?;

    let track = probed
        .format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| WemError::parse("No supported audio track found"))?;

    let codec_params = &track.codec_params;
    let channels = codec_params.channels.map(|c| c.count() as u16).unwrap_or(2);
    let sample_rate = codec_params.sample_rate.unwrap_or(44100);

    // Calculate average bytes using file size and duration
    let mut avg_bytes = 16000; // Fallback
    if let Some(n_frames) = codec_params.n_frames {
        let duration = n_frames as f64 / sample_rate as f64;
        if duration > 0.0 {
            if let Ok(meta) = std::fs::metadata(path) {
                avg_bytes = (meta.len() as f64 / duration) as u32;
            }
        }
    }

    Ok((channels, sample_rate, avg_bytes))
}

pub struct M4aToWem<R: Read + Seek> {
    input: R,
    channels: u16,
    sample_rate: u32,
    avg_bytes_per_second: u32,
    data_size: u32,
}

impl<R: Read + Seek + Send + Sync + 'static> M4aToWem<R> {
    pub fn new(mut input: R) -> WemResult<Self> {
        // Probe metadata using symphonia
        let stream_len = input.seek(SeekFrom::End(0))?;
        input.seek(SeekFrom::Start(0))?;

        // Clone input for probing (since symphonia consumes it or takes ownership of box)
        // Actually we can't clone R easily.
        // We will read the whole file into a buffer? No, that's memory heavy.
        // Symphonia takes `MediaSourceStream`.
        // If we pass `input`, we lose it.
        // But `M4aToWem` needs to keep `input` to write it to `data` chunk later.

        // We can just probe a "reasonable header size" or use `ReadOnlySource` which might check seekability.
        // If R is Seek, we can maybe re-open? But we don't have path.
        // Let's assume we can seek back after probe?
        // Symphonia `MediaSourceStream` takes `Box<dyn MediaSource>`.
        // If we give it our input, it consumes the Box.

        // Strategy: probe first, then we need to reset input.
        // Since we can't get back the input from MSS easily without destroying MSS,
        // and R is not Clone.
        // We might need to assume the caller provides a way to reopen, or we just fail if we can't.
        // BUT `pack-wem` opens a File.
        // Maybe we just take `File`? Or `input` is `R`.

        // For now, let's try to just read some header bytes for detection?
        // AAC/M4A metadata is complex (need to parse atoms).
        // Using symphonia is best.

        // Workaround: We require the input to be passed to `new`.
        // We will use a workaround to probe:
        // If we can't clone R, we are stuck unless we use a wrapper that shares the underlying handle/cursor.
        // But R is generic.
        // Let's assume for M4A packing, we usually deal with Files.
        // If `R` is `std::fs::File`, we can `try_clone`.
        // But `R` is generic.

        // Alternative: The `M4aToWem` takes the path? No, we want generic IO.
        // Let's assume small enough header? No atoms can be at end.

        // Let's read the whole file into memory?
        // If files are small (audio), it's okay. M4A can be big.

        // Better: We probe, get metadata, then we assume we can't use `input` anymore?
        // No, we need to write `input` to `data` chunk.

        // Let's change `new` to take a `Fn() -> R`? No.

        // Okay, `M4aToWem` is short lived.
        // Let's just implement `new` to take input, probe it (consuming it into MSS),
        // extracting metadata, AND THEN we need the data back.
        // Symphonia doesn't give data back easily.

        // Let's just interpret the input ourselves? Too hard.

        // Solution: Read into Vec<u8> if < 100MB?
        // Or just require `std::fs::File`?
        // The struct generic `R` is fine, but maybe `new` requires `File` or something cloneable?
        // No, `OggToWem` uses `PacketReader` which handles OGG.

        // Let's use `ReadOnlySource` with a shared reference?
        // `ReadOnlySource::new` takes `R`.

        // Okay, let's modify `new` to perform the probing on `&mut R` if possible?
        // MSS requires `Box<dyn MediaSource>`. `MediaSource` is implemented for `std::fs::File` and `Cursor`.
        // It is NOT implemented for `&mut R`.

        // We will read the whole file into a temporary buffer for probing?
        // No.

        // Let's just handle this in `pack_wem` CLI instead?
        // The `M4aToWem` struct should just take metadata arguments in `new`.
        // The caller (CLI) is responsible for probing.
        // CLI has the path, so it can open file twice.

        Ok(Self {
            input,
            channels: 0,
            sample_rate: 0,
            avg_bytes_per_second: 0,
            data_size: stream_len as u32,
        })
    }

    pub fn set_metadata(&mut self, channels: u16, sample_rate: u32, avg_bytes: u32) {
        self.channels = channels;
        self.sample_rate = sample_rate;
        self.avg_bytes_per_second = avg_bytes;
    }

    pub fn process<W: Write>(&mut self, mut writer: W) -> WemResult<()> {
        // M4A WEM structure
        // RIFF
        // fmt (0xAAC0)
        // data (raw M4A file)

        let fmt_chunk_size = 0x20; // 32 bytes (18 standard + 14 extra?) or just standard?
        // Wwise AAC often has 0x12 extra bytes? Or just 0?
        // Let's check `wav.rs` or `aac.rs`.
        // `wav.rs` doesn't enforce extra size for 0xAAC0.
        // It decodes whatever.

        // Standard WAVEFORMATEX is 18 bytes.
        // 0xAAC0 might need custom extra data?
        // Usually WEM AAC matches standard M4A content.
        // Let's use a basic header.

        // RIFF size
        let riff_payload_size = 4 + // WAVE
                                8 + fmt_chunk_size + // fmt
                                8 + self.data_size; // data

        writer.write_all(b"RIFF")?;
        writer.write_u32::<LittleEndian>(riff_payload_size)?;
        writer.write_all(b"WAVE")?;

        // fmt chunk
        writer.write_all(b"fmt ")?;
        writer.write_u32::<LittleEndian>(fmt_chunk_size)?;
        writer.write_u16::<LittleEndian>(0xAAC0)?; // AAC
        writer.write_u16::<LittleEndian>(self.channels)?;
        writer.write_u32::<LittleEndian>(self.sample_rate)?;
        writer.write_u32::<LittleEndian>(self.avg_bytes_per_second)?;
        writer.write_u16::<LittleEndian>(0)?; // Block Align
        writer.write_u16::<LittleEndian>(0)?; // Bits per sample
        writer.write_u16::<LittleEndian>(0)?; // Extra size

        // Pad to fmt_chunk_size if needed
        // 18 bytes written. 32 - 18 = 14 bytes padding?
        // Let's see if 0xAAC0 requires specific extra data.
        // Wwise doesn't seem to enforce much for AAC in my analysis.
        // Let's fill with zeros.
        let pad_len = fmt_chunk_size - 18;
        writer.write_all(&vec![0u8; pad_len as usize])?;

        // data chunk
        writer.write_all(b"data")?;
        writer.write_u32::<LittleEndian>(self.data_size)?;

        // Write raw M4A content
        self.input.seek(SeekFrom::Start(0))?;
        std::io::copy(&mut self.input, &mut writer)?;

        Ok(())
    }
}
