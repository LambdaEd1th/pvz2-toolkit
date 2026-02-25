use anyhow::Result;
use clap::Subcommand;
use rsb::{pack_rsb, unpack_rsb};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum RsbCommands {
    /// Unpack an RSB file
    Unpack {
        /// Input RSB file path
        input: PathBuf,
        /// Output directory (optional, defaults to file name stem)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Indicates if the PTX textures were encoded using PowerVR (affects ETC1 decoding)
        #[arg(long, default_value_t = false)]
        powervr: bool,
    },
    /// Pack a directory into an RSB file
    Pack {
        /// Input directory (containing rsb_manifest.json)
        input: PathBuf,
        /// Output RSB file
        output: PathBuf,
        #[arg(
            long,
            help = "Enable PowerVR decoding format for PTX resources (for older iOS files)"
        )]
        powervr: bool,
        #[arg(
            long,
            help = "Use Palette Alpha (4bpp with 16-color header) for ETC1A8 (Format 147)"
        )]
        palette: bool,
    },
}

pub fn handle(cmd: RsbCommands) -> Result<()> {
    match cmd {
        RsbCommands::Unpack {
            input,
            output,
            powervr,
        } => Ok(unpack_rsb(&input, &output, powervr)?),
        RsbCommands::Pack {
            input,
            output,
            powervr,
            palette,
        } => Ok(pack_rsb(&input, &output, powervr, palette)?),
    }
}
