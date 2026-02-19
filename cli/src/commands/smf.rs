use anyhow::Result;
use clap::Subcommand;
use smf::process::{smf_pack, smf_unpack};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum SmfCommands {
    /// Unpack (Decompress) a .smf file
    Unpack {
        /// Input file
        #[arg(short, long)]
        input: PathBuf,
        /// Output file (optional, defaults to input without .smf extension or .decoded)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Use 64-bit variant (16-byte header)
        #[arg(long)]
        use_64bit: bool,
    },
    /// Pack (Compress) a file into .smf format
    Pack {
        /// Input file
        #[arg(short, long)]
        input: PathBuf,
        /// Output file (optional, defaults to input + .smf)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Use 64-bit variant (16-byte header)
        #[arg(long)]
        use_64bit: bool,
    },
}

pub fn handle(cmd: SmfCommands) -> Result<()> {
    match cmd {
        SmfCommands::Unpack {
            input,
            output,
            use_64bit,
        } => Ok(smf_unpack(&input, &output, use_64bit)?),
        SmfCommands::Pack {
            input,
            output,
            use_64bit,
        } => Ok(smf_pack(&input, &output, use_64bit)?),
    }
}
