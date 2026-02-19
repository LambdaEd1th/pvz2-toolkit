use anyhow::Result;
use clap::Subcommand;
use lawnstrings::process::{lawnstrings_decode, lawnstrings_encode};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum LawnStringsCommands {
    /// Decode LawnStrings to JSON
    Decode {
        /// Input LawnStrings file
        input: PathBuf,
        /// Output JSON file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Encode JSON to LawnStrings
    Encode {
        /// Input JSON file
        input: PathBuf,
        /// Output LawnStrings file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

pub fn handle(cmd: LawnStringsCommands) -> Result<()> {
    match cmd {
        LawnStringsCommands::Decode { input, output } => Ok(lawnstrings_decode(&input, &output)?),
        LawnStringsCommands::Encode { input, output } => Ok(lawnstrings_encode(&input, &output)?),
    }
}
