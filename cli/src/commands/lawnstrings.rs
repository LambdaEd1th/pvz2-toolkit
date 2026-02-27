use anyhow::Result;
use clap::Subcommand;
use lawnstrings::{LawnStringsRoot, parse_lawn_strings, write_lawn_strings};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum LawnStringsCommands {
    /// Decode LawnStrings to JSON
    Decode {
        /// Input LawnStrings file
        input: PathBuf,
        /// Output JSON file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Encode JSON to LawnStrings
    Encode {
        /// Input JSON file
        input: PathBuf,
        /// Output LawnStrings file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

pub fn handle(cmd: LawnStringsCommands) -> Result<()> {
    match cmd {
        LawnStringsCommands::Decode { input, output } => lawnstrings_decode(&input, &output),
        LawnStringsCommands::Encode { input, output } => lawnstrings_encode(&input, &output),
    }
}

pub fn lawnstrings_decode(input: &Path, output: &Option<PathBuf>) -> Result<()> {
    // Decode LawnStrings -> JSON
    let content = fs::read_to_string(input)?;
    let strings = parse_lawn_strings(&content)?;

    let out_path = match output {
        Some(p) => p.clone(),
        None => input.with_extension("json"),
    };

    let json = serde_json::to_string_pretty(&strings)?;
    fs::write(&out_path, json)?;
    println!("Decoded LawnStrings to {:?}", out_path);
    Ok(())
}

pub fn lawnstrings_encode(input: &Path, output: &Option<PathBuf>) -> Result<()> {
    // Encode JSON -> LawnStrings
    let content = fs::read_to_string(input)?;
    let strings: LawnStringsRoot = serde_json::from_str(&content)?;

    let out_path = match output {
        Some(p) => p.clone(),
        None => input.with_extension("txt"), // LawnStrings are .txt usually, sometimes .st
    };

    let text_content = write_lawn_strings(&strings)?;
    fs::write(&out_path, text_content)?;
    println!("Encoded LawnStrings to {:?}", out_path);
    Ok(())
}
