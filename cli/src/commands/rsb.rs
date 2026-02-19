use anyhow::Result;
use clap::Subcommand;
use rsb::process::{pack_rsb, unpack_rsb};
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
    },
    /// Pack a directory into an RSB file
    Pack {
        /// Input directory (containing rsb_manifest.json)
        input: PathBuf,
        /// Output RSB file
        output: PathBuf,
    },
}

pub fn handle(cmd: RsbCommands) -> Result<()> {
    match cmd {
        RsbCommands::Unpack { input, output } => Ok(unpack_rsb(&input, &output)?),
        RsbCommands::Pack { input, output } => Ok(pack_rsb(&input, &output)?),
    }
}
