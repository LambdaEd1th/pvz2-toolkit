use anyhow::Result;
use clap::Subcommand;
use rton::process::{rton_decode, rton_decrypt_file, rton_encode, rton_encrypt_file};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum RtonCommands {
    /// Decode RTON to JSON
    Decode {
        /// Input RTON file
        input: PathBuf,
        /// Output JSON file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Encryption Seed (for encrypted RTONs)
        #[arg(long)]
        seed: Option<String>,
    },
    /// Encode JSON to RTON
    Encode {
        /// Input JSON file
        input: PathBuf,
        /// Output RTON file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Encryption Seed (for encrypted RTONs)
        #[arg(long)]
        seed: Option<String>,
    },
    /// Encrypt RTON/File
    Encrypt {
        /// Input file
        input: PathBuf,
        /// Output file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Encryption Seed (required)
        #[arg(long)]
        seed: String,
    },
    /// Decrypt RTON/File
    Decrypt {
        /// Input file
        input: PathBuf,
        /// Output file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Encryption Seed (required)
        #[arg(long)]
        seed: String,
    },
}

pub fn handle(cmd: RtonCommands) -> Result<()> {
    match cmd {
        RtonCommands::Decode {
            input,
            output,
            seed,
        } => Ok(rton_decode(&input, &output, seed.as_deref())?),
        RtonCommands::Encode {
            input,
            output,
            seed,
        } => Ok(rton_encode(&input, &output, seed.as_deref())?),
        RtonCommands::Encrypt {
            input,
            output,
            seed,
        } => Ok(rton_encrypt_file(&input, &output, &seed)?),
        RtonCommands::Decrypt {
            input,
            output,
            seed,
        } => Ok(rton_decrypt_file(&input, &output, &seed)?),
    }
}
