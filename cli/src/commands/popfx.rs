use anyhow::Result;
use clap::Subcommand;
use popfx::process::{popfx_decode, popfx_encode};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum PopfxCommands {
    /// Decode Popfx to JSON
    Decode {
        /// Input Popfx file
        input: PathBuf,
        /// Output JSON file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Encode JSON to Popfx
    Encode {
        /// Input JSON file
        input: PathBuf,
        /// Output Popfx file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

pub fn handle(cmd: PopfxCommands) -> Result<()> {
    match cmd {
        PopfxCommands::Decode { input, output } => Ok(popfx_decode(&input, &output)?),
        PopfxCommands::Encode { input, output } => Ok(popfx_encode(&input, &output)?),
    }
}
