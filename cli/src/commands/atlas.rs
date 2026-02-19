use anyhow::Result;
use atlas::split_atlas;
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum AtlasCommand {
    /// Split an Atlas into individual sprites
    Split {
        /// Input Atlas JSON file
        json_path: PathBuf,
        /// Input Image file (optional, defaults to json name + .png)
        #[arg(short, long)]
        image: Option<PathBuf>,
        /// Output directory (optional, defaults to json name + .sprite/media)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

pub fn handle(cmd: AtlasCommand) -> Result<()> {
    match cmd {
        AtlasCommand::Split {
            json_path,
            image,
            output,
        } => Ok(split_atlas(
            &json_path,
            image.as_deref(),
            output.as_deref(),
        )?),
    }
}
