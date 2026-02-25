use anyhow::Result;
use clap::Subcommand;
use rsb::{pack_rsg_batch, unpack_rsg_batch};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum RsgCommands {
    /// Unpack an RSG file or a directory of RSG files (via manifest)
    Unpack {
        /// Input RSG file or rsb_manifest.json
        input: PathBuf,
        /// Output directory (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Pack a directory of files into an RSG file (or multiple RSG files via manifest)
    Pack {
        /// Input directory containing manifest.json or raw files
        input: PathBuf,
        /// Output RSG file path
        output: PathBuf,
    },
}

pub fn handle(cmd: RsgCommands) -> Result<()> {
    match cmd {
        RsgCommands::Unpack { input, output } => Ok(unpack_rsg_batch(&input, &output)?),
        RsgCommands::Pack { input, output } => Ok(pack_rsg_batch(&input, &output)?),
    }
}
