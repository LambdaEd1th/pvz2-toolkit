use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::{RtonValue, from_reader, to_writer};

pub fn rton_decode(input: &Path, output: &Option<PathBuf>, seed: Option<&str>) -> Result<()> {
    // Decode RTON -> JSON (Default for .rton or others)
    let mut file = fs::File::open(input).context("Failed to open input file")?;
    let rton_value: RtonValue = from_reader(&mut file, seed).context("Failed to decode RTON")?;

    let out_path = match output {
        Some(p) => p.clone(),
        None => input.with_extension("json"),
    };

    let json = serde_json::to_string_pretty(&rton_value).context("Failed to serialize to JSON")?;
    fs::write(&out_path, json).context("Failed to write output file")?;
    println!("Decoded RTON to {:?}", out_path);
    Ok(())
}

pub fn rton_encode(input: &Path, output: &Option<PathBuf>, seed: Option<&str>) -> Result<()> {
    // Encode JSON -> RTON
    let content = fs::read_to_string(input).context("Failed to read input file")?;
    let rton_value: RtonValue = serde_json::from_str(&content).context("Failed to parse JSON")?;

    let out_path = match output {
        Some(p) => p.clone(),
        None => input.with_extension("rton"),
    };

    let mut file = fs::File::create(&out_path).context("Failed to create output file")?;
    to_writer(&mut file, &rton_value, seed).context("Failed to encode RTON")?;
    println!("Encoded RTON to {:?}", out_path);
    Ok(())
}
