use anyhow::{Context, Result};
use clap::Subcommand;
use newton::{MResourceGroup, decode_newton, encode_newton};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum NewtonCommands {
    /// Decode Newton to JSON
    Decode {
        /// Input Newton file
        input: PathBuf,
        /// Output JSON file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Encode JSON to Newton
    Encode {
        /// Input JSON file
        input: PathBuf,
        /// Output Newton file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

pub fn handle(cmd: NewtonCommands) -> Result<()> {
    match cmd {
        NewtonCommands::Decode { input, output } => newton_decode(&input, &output),
        NewtonCommands::Encode { input, output } => newton_encode(&input, &output),
    }
}

pub fn newton_decode(input: &Path, output: &Option<PathBuf>) -> Result<()> {
    // Decode NTON -> JSON
    let mut file = fs::File::open(input)
        .with_context(|| format!("Failed to open Newton file: {:?}", input))?;
    let root = decode_newton(&mut file).with_context(|| "Failed to parse Newton format")?;

    let out_path = match output {
        Some(p) => p.clone(),
        None => input.with_extension("json"),
    };

    let mut file = fs::File::create(&out_path)?;
    serde_json::to_writer_pretty(&mut file, &root)
        .with_context(|| "Failed to write JSON format")?;
    println!("Decoded NTON to {:?}", out_path);
    Ok(())
}

pub fn newton_encode(input: &Path, output: &Option<PathBuf>) -> Result<()> {
    // Encode JSON -> NTON
    let content = fs::read_to_string(input)
        .with_context(|| format!("Failed to read JSON file: {:?}", input))?;
    let root: MResourceGroup =
        serde_json::from_str(&content).with_context(|| "Failed to parse JSON")?;

    let out_path = match output {
        Some(p) => p.clone(),
        None => input.with_extension("nton"),
    };

    let mut file = fs::File::create(&out_path)?;
    encode_newton(&root, &mut file).with_context(|| "Failed to write NTON format")?;
    println!("Encoded NTON to {:?}", out_path);
    Ok(())
}
