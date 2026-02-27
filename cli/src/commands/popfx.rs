use anyhow::Result;
use clap::Subcommand;
use popfx::{
    PopcapRenderEffectObject,
    codec::{decode_popfx, encode_popfx},
};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum PopfxCommands {
    /// Decode Popfx to JSON
    Decode {
        /// Input Popfx file
        input: PathBuf,
        /// Output JSON file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Encode JSON to Popfx
    Encode {
        /// Input JSON file
        input: PathBuf,
        /// Output Popfx file (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

pub fn handle(cmd: PopfxCommands) -> Result<()> {
    match cmd {
        PopfxCommands::Decode { input, output } => popfx_decode(&input, &output),
        PopfxCommands::Encode { input, output } => popfx_encode(&input, &output),
    }
}

pub fn popfx_decode(input: &Path, output: &Option<PathBuf>) -> Result<()> {
    // Decode Popfx -> JSON
    let mut file = fs::File::open(input)?;
    let popfx = decode_popfx(&mut file)?;

    let out_path = match output {
        Some(p) => p.clone(),
        None => input.with_extension("json"),
    };

    fs::write(&out_path, serde_json::to_string_pretty(&popfx)?)?;
    println!("Decoded Popfx to {:?}", out_path);
    Ok(())
}

pub fn popfx_encode(input: &Path, output: &Option<PathBuf>) -> Result<()> {
    // Encode JSON -> Popfx
    let content = fs::read_to_string(input)?;
    let popfx: PopcapRenderEffectObject = serde_json::from_str(&content)?;

    let out_path = match output {
        Some(p) => p.clone(),
        None => input.with_extension("pop"),
    };

    let mut file = fs::File::create(&out_path)?;
    encode_popfx(&popfx, &mut file)?;
    println!("Encoded Popfx to {:?}", out_path);
    Ok(())
}
