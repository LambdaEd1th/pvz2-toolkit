use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::{LawnStringsRoot, parse_lawn_strings, write_lawn_strings};

pub fn lawnstrings_decode(input: &Path, output: &Option<PathBuf>) -> Result<()> {
    // Decode LawnStrings -> JSON
    let content = fs::read_to_string(input).context("Failed to read input file")?;
    let strings = parse_lawn_strings(&content).context("Failed to parse LawnStrings")?;

    let out_path = match output {
        Some(p) => p.clone(),
        None => input.with_extension("json"),
    };

    fs::write(
        &out_path,
        serde_json::to_string_pretty(&strings).context("Failed to serialize to JSON")?,
    )
    .context("Failed to write output file")?;
    println!("Decoded LawnStrings to {:?}", out_path);
    Ok(())
}

pub fn lawnstrings_encode(input: &Path, output: &Option<PathBuf>) -> Result<()> {
    // Encode JSON -> LawnStrings
    let content = fs::read_to_string(input).context("Failed to read input file")?;
    let strings: LawnStringsRoot =
        serde_json::from_str(&content).context("Failed to parse JSON")?;

    let out_path = match output {
        Some(p) => p.clone(),
        None => input.with_extension("txt"), // LawnStrings are .txt usually, sometimes .st
    };

    let text_content = write_lawn_strings(&strings).context("Failed to write LawnStrings")?;
    fs::write(&out_path, text_content).context("Failed to write output file")?;
    println!("Encoded LawnStrings to {:?}", out_path);
    Ok(())
}
