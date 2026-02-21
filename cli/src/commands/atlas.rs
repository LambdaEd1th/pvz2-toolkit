use anyhow::Result;
use atlas::{merge_atlas, split_atlas};
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
    /// Merge individual sprites into an Atlas
    Merge {
        /// Input Atlas JSON file (used as layout definition)
        json_path: PathBuf,
        /// Input directory containing sprites (optional, defaults to json name + .sprite/media)
        #[arg(short, long)]
        input: Option<PathBuf>,
        /// Output image file (optional, defaults to json name + .png)
        #[arg(long)]
        output_image: Option<PathBuf>,
        /// Output updated JSON file (optional, defaults to overwriting input json)
        #[arg(long)]
        output_json: Option<PathBuf>,
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
        AtlasCommand::Merge {
            json_path,
            input,
            output_image,
            output_json,
        } => Ok(merge_atlas(
            &json_path,
            input.as_deref(),
            output_image.as_deref(),
            output_json.as_deref(),
        )?),
    }
}
