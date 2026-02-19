use anyhow::Result;
use clap::Subcommand;
use pam::process::{pam_decode, pam_encode};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum PamCommands {
    /// Decode PAM to JSON
    Decode {
        /// Input PAM file
        input: PathBuf,
        /// Output JSON file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Encode JSON/HTML to PAM
    Encode {
        /// Input JSON or HTML file
        input: PathBuf,
        /// Output PAM file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

pub fn handle(cmd: PamCommands) -> Result<()> {
    match cmd {
        PamCommands::Decode { input, output } => Ok(pam_decode(&input, &output)?),
        PamCommands::Encode { input, output } => Ok(pam_encode(&input, &output)?),
    }
}
