use anyhow::{Context, Result, anyhow};
use clap::Subcommand;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use wem::aac;
use wem::aac::M4aToWem;
use wem::adpcm::WavToAdpcm;
use wem::pcm::WavToWem;
use wem::vorbis::encoder::OggToWem;
use wem::wav;
use wem::{CodebookLibrary, WwiseRiffVorbis};

#[derive(Subcommand)]
pub enum WemCommands {
    /// Decode WEM to WAV/OGG/M4A
    Decode {
        /// Input WEM file
        input: PathBuf,
        /// Output file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Path to codebooks.bin (for Vorbis)
        #[arg(short, long)]
        codebooks: Option<String>,
        /// Inline codebooks into OGG (for Vorbis)
        #[arg(long)]
        inline_codebooks: bool,
    },
    /// Encode WAV/OGG to WEM
    Encode {
        /// Input WAV/OGG file
        input: PathBuf,
        /// Output WEM file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Force ADPCM encoding (for WAV input)
        #[arg(short, long)]
        adpcm: bool,
    },
}

pub fn handle(cmd: WemCommands) -> Result<()> {
    match cmd {
        WemCommands::Decode {
            input,
            output,
            codebooks,
            inline_codebooks,
        } => wem_decode(&input, &output, &codebooks, inline_codebooks),
        WemCommands::Encode {
            input,
            output,
            adpcm,
        } => wem_encode(&input, &output, adpcm),
    }
}

pub fn wem_decode(
    input: &Path,
    output: &Option<PathBuf>,
    codebooks: &Option<String>,
    inline_codebooks: bool,
) -> Result<()> {
    let mut file = fs::File::open(input)
        .with_context(|| format!("Failed to open input WEM file: {:?}", input))?;

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
        CodebookLibrary::from_file(path_str).with_context(|| "Failed to load external codebooks")?
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
                    .with_context(|| "Vorbis init failed")?;

            converter
                .generate_ogg(&mut out_file)
                .with_context(|| "OGG generation failed")?;
        }
        0xAAC0 => {
            // AAC -> M4A (Extract)
            println!("  Format: AAC (extracting to M4A)");
            // Read entire file to buffer for extraction
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            let cursor = std::io::Cursor::new(buffer);
            aac::extract_wem_aac(cursor, &mut out_file).with_context(|| "Failed to extract AAC")?;
        }
        0x8311 => {
            // ADPCM -> WAV (Decode)
            println!("  Format: Wwise IMA ADPCM (decoding to WAV)");
            let reader = std::io::BufReader::new(file);
            wav::wem_to_wav(reader, &mut out_file, &codebooks_lib)
                .with_context(|| "WAV decoding failed")?;
        }
        0x0001 | 0xFFFE => {
            // PCM -> WAV (Decode)
            println!("  Format: PCM (decoding to WAV)");
            let reader = std::io::BufReader::new(file);
            wav::wem_to_wav(reader, &mut out_file, &codebooks_lib)
                .with_context(|| "WAV decoding failed")?;
        }
        _ => {
            return Err(anyhow!("Unsupported format tag: 0x{:04X}", format_tag));
        }
    }

    println!("Decoding successful: {:?}", out_path);
    Ok(())
}

pub fn wem_encode(input: &Path, output: &Option<PathBuf>, adpcm: bool) -> Result<()> {
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
        match wem::aac::probe_m4a_metadata(input) {
            Ok((channels, sample_rate, avg_bytes)) => {
                println!("  Detected: {} Hz, {} Channels", sample_rate, channels);

                // 2. Pack
                let file = fs::File::open(input)?;
                let mut packer = M4aToWem::new(file).with_context(|| "M4A init failed")?;
                packer.set_metadata(channels, sample_rate, avg_bytes);

                let mut out_file = fs::File::create(&output)?;
                packer
                    .process(&mut out_file)
                    .with_context(|| "M4A processing failed")?;
            }
            Err(e) => return Err(anyhow!("Failed to probe M4A: {}", e)),
        }
    } else if extension == "wav" {
        // WAV Path (PCM or ADPCM)
        println!("Encoding WAV: {:?}", input);
        let file = fs::File::open(input).with_context(|| "Failed to open WAV file")?;
        let mut out_file = fs::File::create(&output)?;

        if adpcm {
            println!("  Encoding to ADPCM...");
            let mut packer = WavToAdpcm::new(file).with_context(|| "ADPCM init failed")?;
            packer
                .process(&mut out_file)
                .with_context(|| "ADPCM processing failed")?;
        } else {
            println!("  Encoding to PCM...");
            let mut packer = WavToWem::new(file).with_context(|| "PCM init failed")?;
            packer
                .process(&mut out_file)
                .with_context(|| "PCM processing failed")?;
        }
    } else if extension == "ogg" || extension == "logg" {
        // OGG Path (Default)
        println!("Encoding OGG: {:?}", input);
        let file = fs::File::open(input).with_context(|| "Failed to open OGG file")?;
        let mut packer = OggToWem::new(file);
        let mut out_file = fs::File::create(&output)?;
        packer
            .process(&mut out_file)
            .with_context(|| "Failed to pack WEM")?;
    } else {
        return Err(anyhow!("Unsupported file extension: .{}", extension));
    }

    println!("Encoded WEM to {:?}", output);
    Ok(())
}
