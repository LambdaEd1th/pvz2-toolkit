use anyhow::Result;
use clap::Subcommand;
use std::fs;
use std::path::PathBuf;

use reanim::{ReanimVersion, decode, encode};

#[derive(Subcommand)]
pub enum ReanimCommands {
    /// Decode a Reanim binary file to JSON
    Decode {
        #[arg(required = true, help = "Input Reanim file")]
        input: PathBuf,
        #[arg(required = true, help = "Output JSON file")]
        output: PathBuf,
    },
    /// Encode a JSON file to a Reanim binary file
    Encode {
        #[arg(required = true, help = "Input JSON file")]
        input: PathBuf,
        #[arg(required = true, help = "Output Reanim file")]
        output: PathBuf,
        #[arg(long, default_value = "pc", help = "Version: pc, phone32, phone64")]
        version: String,
    },
}

pub fn handle(cmd: ReanimCommands) -> Result<()> {
    match cmd {
        ReanimCommands::Decode { input, output } => {
            let data = fs::read(&input)?;
            let reanim = decode(&data)?;
            let json = serde_json::to_string_pretty(&reanim)?;
            fs::write(&output, json)?;
            println!("Decoded {} to {}", input.display(), output.display());
        }
        ReanimCommands::Encode {
            input,
            output,
            version,
        } => {
            let json = fs::read_to_string(&input)?;
            let reanim = serde_json::from_str(&json)?;
            let ver = match version.to_lowercase().as_str() {
                "pc" => ReanimVersion::PC,
                "phone32" => ReanimVersion::Phone32,
                "phone64" => ReanimVersion::Phone64,
                _ => anyhow::bail!("Invalid version, must be pc, phone32, or phone64"),
            };
            let out_data = encode(&reanim, ver)?;
            fs::write(&output, out_data)?;
            println!(
                "Encoded {} to {} ({:?})",
                input.display(),
                output.display(),
                ver
            );
        }
    }
    Ok(())
}
