use anyhow::Result;
use clap::Subcommand;
use rsb::process::{pack_rsg_batch, unpack_rsg_batch};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum RsgCommands {
    /// Unpack RSG packets from rsb_manifest.json (or single RSG file)
    Unpack {
        /// Input rsb_manifest.json or .rsa/rsg file
        #[arg(short, long)]
        input: PathBuf,
        /// Output directory
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Pack RSG packet from folder/config
    Pack {
        /// Input directory or config
        #[arg(short, long)]
        input: PathBuf,
        /// Output RSG file
        #[arg(short, long)]
        output: PathBuf,
    },
}

pub fn handle(cmd: RsgCommands) -> Result<()> {
    match cmd {
        RsgCommands::Unpack { input, output } => Ok(unpack_rsg_batch(&input, &output)?),
        RsgCommands::Pack { input, output } => Ok(pack_rsg_batch(&input, &output)?),
    }
}
