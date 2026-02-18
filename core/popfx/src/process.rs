use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::{PopcapRenderEffectObject, decode_popfx, encode_popfx};

pub fn popfx_decode(input: &Path, output: &Option<PathBuf>) -> Result<()> {
    // Decode Popfx -> JSON
    let mut file = fs::File::open(input).context("Failed to open input file")?;
    let popfx = decode_popfx(&mut file).context("Failed to decode Popfx")?;

    let out_path = match output {
        Some(p) => p.clone(),
        None => input.with_extension("json"),
    };

    fs::write(
        &out_path,
        serde_json::to_string_pretty(&popfx).context("Failed to serialize to JSON")?,
    )
    .context("Failed to write output file")?;
    println!("Decoded Popfx to {:?}", out_path);
    Ok(())
}

pub fn popfx_encode(input: &Path, output: &Option<PathBuf>) -> Result<()> {
    // Encode JSON -> Popfx
    let content = fs::read_to_string(input).context("Failed to read input file")?;
    let popfx: PopcapRenderEffectObject =
        serde_json::from_str(&content).context("Failed to parse JSON")?;

    let out_path = match output {
        Some(p) => p.clone(),
        None => input.with_extension("pop"),
    };

    let mut file = fs::File::create(&out_path).context("Failed to create output file")?;
    encode_popfx(&popfx, &mut file).context("Failed to encode Popfx")?;
    println!("Encoded Popfx to {:?}", out_path);
    Ok(())
}
