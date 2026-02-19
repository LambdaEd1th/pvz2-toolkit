use anyhow::Result;
use clap::Subcommand;
use rsb::process_ptx::{ptx_decode, ptx_encode};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum PtxCommands {
    /// Decode PTX to PNG (Batch from manifest)
    Decode {
        /// Input rsb_manifest.json file
        #[arg(short, long)]
        input: PathBuf,
        /// Output directory (optional, defaults to input dir)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Treat RGBA8888 (Format 0) as PowerVR/iOS format (BGRA) instead of default (RGBA)
        #[arg(long)]
        powervr: bool,
    },
    /// Encode PNG to PTX (Batch from manifest)
    Encode {
        /// Input rsb_manifest.json file
        #[arg(short, long)]
        input: PathBuf,
        /// Output directory (optional, defaults to input dir)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Force PowerVR/iOS format (BGRA for Format 0)
        #[arg(long)]
        powervr: bool,
        /// Use Palette Alpha (4bpp with 16-color header) for ETC1A8 (Format 147)
        #[arg(long)]
        palette: bool,
    },
}

pub fn handle(cmd: PtxCommands) -> Result<()> {
    match cmd {
        PtxCommands::Decode {
            input,
            output,
            powervr,
        } => Ok(ptx_decode(&input, &output, powervr)?),
        PtxCommands::Encode {
            input,
            output,
            powervr,
            palette,
        } => Ok(ptx_encode(&input, &output, powervr, palette)?),
    }
}
