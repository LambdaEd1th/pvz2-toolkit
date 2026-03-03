use anyhow::Result;
use clap::Subcommand;
use std::fs;
use std::path::PathBuf;

use reanim::{ReanimVersion, decode, decode_xfl, encode, encode_xfl};

#[derive(Subcommand)]
pub enum ReanimCommands {
    /// Decode a Reanim binary file to JSON or XFL directory
    Decode {
        #[arg(required = true, help = "Input Reanim file")]
        input: PathBuf,
        #[arg(
            required = true,
            help = "Output JSON file or XFL directory (must end in .xfl)"
        )]
        output: PathBuf,
    },
    /// Encode a JSON file or XFL directory to a Reanim binary file
    Encode {
        #[arg(
            required = true,
            help = "Input JSON file or XFL directory (must end in .xfl)"
        )]
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

            if output.extension().and_then(|e| e.to_str()) == Some("xfl") {
                encode_xfl(&reanim, &output)?;
                println!(
                    "Extracted {} to XFL directory {}",
                    input.display(),
                    output.display()
                );
            } else {
                let json = serde_json::to_string_pretty(&reanim)?;
                fs::write(&output, json)?;
                println!("Decoded {} to JSON {}", input.display(), output.display());
            }
        }
        ReanimCommands::Encode {
            input,
            output,
            version,
        } => {
            let reanim = if input.extension().and_then(|e| e.to_str()) == Some("xfl") {
                decode_xfl(&input)?
            } else {
                let json = fs::read_to_string(&input)?;
                serde_json::from_str(&json)?
            };
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
