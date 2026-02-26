use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::CodebookLibrary;
use crate::WwiseRiffVorbis;
use crate::aac;
use crate::aac::M4aToWem;
use crate::adpcm::WavToAdpcm;
use crate::error::{WemError, WemResult};
use crate::pcm::WavToWem;
use crate::vorbis::encoder::OggToWem;
use crate::wav;

pub fn wem_decode(
    input: &Path,
    output: &Option<PathBuf>,
    codebooks: &Option<String>,
    inline_codebooks: bool,
) -> WemResult<()> {
    let mut file =
        fs::File::open(input).map_err(|e| WemError::file_open(format!("{:?}: {}", input, e)))?;

    // Auto-detect format
    let format_tag = wav::get_wem_format(&mut file).unwrap_or(0);
    file.seek(SeekFrom::Start(0))?;

    let default_extension = match format_tag {
        0xFFFF => "ogg",          // Vorbis
        0xAAC0 => "m4a",          // AAC
        0x8311 => "wav",          // ADPCM
        0x0001 | 0xFFFE => "wav", // PCM
        _ => "wav",               // Default fallback
    };

    let out_path = match output {
        Some(p) => p.clone(),
        None => input.with_extension(default_extension),
    };

    println!(
        "Decoding {:?} -> {:?} (Format: {:#06X})",
        input, out_path, format_tag
    );

    // Codebook handling (always needed for Vorbis, even repack)
    let codebooks_lib = if let Some(path_str) = codebooks {
        CodebookLibrary::from_file(path_str)
            .map_err(|e| WemError::codebook(format!("Failed to load external codebooks: {}", e)))?
    } else {
        CodebookLibrary::embedded_aotuv()
    };

    let mut out_file = fs::File::create(&out_path)?;

    match format_tag {
        0xFFFF => {
            // Vorbis -> OGG (Repack)
            println!("  Format: Vorbis (repacking to OGG)");
            // Use builder pattern to respect inline_codebooks
            let mut converter =
                WwiseRiffVorbis::builder(std::io::BufReader::new(file), codebooks_lib)
                    .inline_codebooks(inline_codebooks)
                    .full_setup(inline_codebooks)
                    .build()
                    .map_err(|e| WemError::parse(format!("Vorbis init failed: {:?}", e)))?;

            converter
                .generate_ogg(&mut out_file)
                .map_err(|e| WemError::parse(format!("OGG generation failed: {:?}", e)))?;
        }
        0xAAC0 => {
            // AAC -> M4A (Extract)
            println!("  Format: AAC (extracting to M4A)");
            // Read entire file to buffer for extraction
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            let cursor = std::io::Cursor::new(buffer);
            aac::extract_wem_aac(cursor, &mut out_file)
                .map_err(|e| WemError::parse(format!("Failed to extract AAC: {}", e)))?;
        }
        0x8311 => {
            // ADPCM -> WAV (Decode)
            println!("  Format: Wwise IMA ADPCM (decoding to WAV)");
            let reader = std::io::BufReader::new(file);
            wav::wem_to_wav(reader, &mut out_file, &codebooks_lib)
                .map_err(|e| WemError::parse(format!("WAV decoding failed: {:?}", e)))?;
        }
        0x0001 | 0xFFFE => {
            // PCM -> WAV (Decode)
            println!("  Format: PCM (decoding to WAV)");
            let reader = std::io::BufReader::new(file);
            wav::wem_to_wav(reader, &mut out_file, &codebooks_lib)
                .map_err(|e| WemError::parse(format!("WAV decoding failed: {:?}", e)))?;
        }
        _ => {
            return Err(WemError::parse(format!(
                "Unsupported format tag: 0x{:04X}",
                format_tag
            )));
        }
    }

    println!("Decoding successful: {:?}", out_path);
    Ok(())
}

pub fn wem_encode(input: &Path, output: &Option<PathBuf>, adpcm: bool) -> WemResult<()> {
    let extension = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let output = match output {
        Some(p) => p.clone(),
        None => input.with_extension("wem"),
    };

    if extension == "m4a" || extension == "aac" {
        // M4A/AAC Path
        println!("Encoding M4A/AAC: {:?}", input);

        // 1. Probe Metadata
        match crate::aac::probe_m4a_metadata(input) {
            Ok((channels, sample_rate, avg_bytes)) => {
                println!("  Detected: {} Hz, {} Channels", sample_rate, channels);

                // 2. Pack
                let file = fs::File::open(input)?;
                let mut packer = M4aToWem::new(file)
                    .map_err(|e| WemError::parse(format!("M4A init failed: {:?}", e)))?;
                packer.set_metadata(channels, sample_rate, avg_bytes);

                let mut out_file = fs::File::create(&output)?;
                packer
                    .process(&mut out_file)
                    .map_err(|e| WemError::parse(format!("M4A processing failed: {:?}", e)))?;
            }
            Err(e) => return Err(WemError::parse(format!("Failed to probe M4A: {}", e))),
        }
    } else if extension == "wav" {
        // WAV Path (PCM or ADPCM)
        println!("Encoding WAV: {:?}", input);
        let file = fs::File::open(input)
            .map_err(|e| WemError::file_open(format!("{:?}: {}", input, e)))?;
        let mut out_file = fs::File::create(&output)?;

        if adpcm {
            println!("  Encoding to ADPCM...");
            let mut packer = WavToAdpcm::new(file)
                .map_err(|e| WemError::parse(format!("ADPCM init failed: {:?}", e)))?;
            packer
                .process(&mut out_file)
                .map_err(|e| WemError::parse(format!("ADPCM processing failed: {:?}", e)))?;
        } else {
            println!("  Encoding to PCM...");
            let mut packer = WavToWem::new(file)
                .map_err(|e| WemError::parse(format!("PCM init failed: {:?}", e)))?;
            packer
                .process(&mut out_file)
                .map_err(|e| WemError::parse(format!("PCM processing failed: {:?}", e)))?;
        }
    } else if extension == "ogg" || extension == "logg" {
        // OGG Path (Default)
        println!("Encoding OGG: {:?}", input);
        let file = fs::File::open(input)
            .map_err(|e| WemError::file_open(format!("{:?}: {}", input, e)))?;
        let mut packer = OggToWem::new(file);
        let mut out_file = fs::File::create(&output)?;
        packer
            .process(&mut out_file)
            .map_err(|e| WemError::parse(format!("Failed to pack WEM: {}", e)))?;
    } else {
        return Err(WemError::parse(format!(
            "Unsupported file extension: .{}",
            extension
        )));
    }

    println!("Encoded WEM to {:?}", output);
    Ok(())
}
