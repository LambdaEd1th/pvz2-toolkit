use anyhow::Result;
use clap::Subcommand;
use std::path::PathBuf;
use wem::process::{wem_decode, wem_encode};

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
        } => Ok(wem_decode(&input, &output, &codebooks, inline_codebooks)?),
        WemCommands::Encode {
            input,
            output,
            adpcm,
        } => Ok(wem_encode(&input, &output, adpcm)?),
    }
}
