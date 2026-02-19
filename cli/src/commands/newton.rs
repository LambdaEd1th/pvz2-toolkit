use anyhow::Result;
use clap::Subcommand;
use newton::process::{newton_decode, newton_encode};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum NewtonCommands {
    /// Decode Newton to XML
    Decode {
        /// Input Newton file
        input: PathBuf,
        /// Output XML file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Encode XML to Newton
    Encode {
        /// Input XML file
        input: PathBuf,
        /// Output Newton file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

pub fn handle(cmd: NewtonCommands) -> Result<()> {
    match cmd {
        NewtonCommands::Decode { input, output } => Ok(newton_decode(&input, &output)?),
        NewtonCommands::Encode { input, output } => Ok(newton_encode(&input, &output)?),
    }
}
