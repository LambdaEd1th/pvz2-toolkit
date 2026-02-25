use anyhow::{Context, Result};
use clap::Subcommand;
use resource::{merge_rsb_desc, merge_rsg_res, split_rsb_desc, split_rsg_res};
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum ResourceCommands {
    /// Split a description.json file into a definition.json and subgroups folder
    SplitRsbDesc {
        #[clap(short, long, help = "Path to the input description.json file")]
        input: PathBuf,
        #[clap(short, long, help = "Path to the output directory")]
        output: PathBuf,
    },
    /// Merge a definition.json and subgroups folder back into a description.json file
    MergeRsbDesc {
        #[clap(
            short,
            long,
            help = "Path to the input directory (containing definition.json and subgroups/)"
        )]
        input: PathBuf,
        #[clap(short, long, help = "Path to the output description.json file")]
        output: PathBuf,
    },
    /// Split a resources.json file into a content.json and subgroup folder
    SplitRes {
        #[clap(short, long, help = "Path to the input resources.json file")]
        input: PathBuf,
        #[clap(short, long, help = "Path to the output directory")]
        output: PathBuf,
    },
    /// Merge a content.json and subgroup folder back into a resources.json file
    MergeRes {
        #[clap(
            short,
            long,
            help = "Path to the input directory (containing content.json and subgroup/)"
        )]
        input: PathBuf,
        #[clap(short, long, help = "Path to the output resources.json file")]
        output: PathBuf,
    },
}

pub fn handle(cmd: ResourceCommands) -> Result<()> {
    match cmd {
        ResourceCommands::SplitRsbDesc { input, output } => {
            split_rsb_desc(&input, &output)
                .with_context(|| format!("Failed to split RSB description: {:?}", input))?;
            println!("Successfully split RSB description to {:?}", output);
        }
        ResourceCommands::MergeRsbDesc { input, output } => {
            merge_rsb_desc(&input, &output)
                .with_context(|| format!("Failed to merge RSB description: {:?}", input))?;
            println!("Successfully merged RSB description to {:?}", output);
        }
        ResourceCommands::SplitRes { input, output } => {
            split_rsg_res(&input, &output)
                .with_context(|| format!("Failed to split PopCap resources: {:?}", input))?;
            println!("Successfully split PopCap resources to {:?}", output);
        }
        ResourceCommands::MergeRes { input, output } => {
            merge_rsg_res(&input, &output)
                .with_context(|| format!("Failed to merge PopCap resources: {:?}", input))?;
            println!("Successfully merged PopCap resources to {:?}", output);
        }
    }
    Ok(())
}
