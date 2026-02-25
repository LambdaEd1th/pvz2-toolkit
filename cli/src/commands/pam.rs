use anyhow::Result;
use clap::Subcommand;
use pam::process::{pam_decode, pam_encode, pam_render};
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
    /// Render PAM animation frames from JSON
    Render {
        /// Input PAM JSON file
        input: PathBuf,
        /// Directory containing extracted media elements (PNGs)
        #[arg(short, long)]
        media: PathBuf,
        /// Output directory for rendered frames
        #[arg(short, long)]
        output: PathBuf,
        /// Disable rendering of specific sprite indices (comma separated, e.g. '1,2,5')
        #[arg(short, long, value_delimiter = ',')]
        disable: Vec<i32>,
        /// Output format: 'png' (image sequence) or 'gif' (animated GIF)
        #[arg(short, long, default_value = "png")]
        format: String,
    },
}

pub fn handle(cmd: PamCommands) -> Result<()> {
    match cmd {
        PamCommands::Decode { input, output } => Ok(pam_decode(&input, &output)?),
        PamCommands::Encode { input, output } => Ok(pam_encode(&input, &output)?),
        PamCommands::Render {
            input,
            media,
            output,
            disable,
            format,
        } => Ok(pam_render(&input, &media, &output, disable, &format)?),
    }
}
