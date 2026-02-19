use anyhow::Result;
use clap::Subcommand;
use patch::process::{patch_apply, patch_create, patch_extract};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum PatchCommands {
    /// Create a patch from Source to Target
    Create {
        /// Source file (Dictionary/Original)
        source: PathBuf,
        /// Target file (New/Modified)
        target: PathBuf,
        /// Output Patch file
        output: PathBuf,
    },
    /// Apply a patch to Source to get Target
    Apply {
        /// Source file (Dictionary/Original)
        source: PathBuf,
        /// Patch file
        patch: PathBuf,
        /// Output Target file
        output: PathBuf,
    },
    /// Extract VCDiff patches from an RSBPatch file (PBSR)
    Extract {
        /// Input RSBPatch file
        #[arg(short, long)]
        input: PathBuf,
        /// Output directory for extracted .vcdiff files
        #[arg(short, long)]
        output: PathBuf,
    },
}

pub fn handle(cmd: PatchCommands) -> Result<()> {
    match cmd {
        PatchCommands::Create {
            source,
            target,
            output,
        } => Ok(patch_create(&source, &target, &output)?),
        PatchCommands::Apply {
            source,
            patch,
            output,
        } => Ok(patch_apply(&source, &patch, &output)?),
        PatchCommands::Extract { input, output } => Ok(patch_extract(&input, &output)?),
    }
}
