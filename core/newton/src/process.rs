use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::{MResourceGroup, decode_newton, encode_newton};

pub fn newton_decode(input: &Path, output: &Option<PathBuf>) -> Result<()> {
    // Decode NTON -> XML
    let mut file = fs::File::open(input).context("Failed to open input file")?;
    let root = decode_newton(&mut file).context("Failed to decode NTON")?;

    let out_path = match output {
        Some(p) => p.clone(),
        None => input.with_extension("xml"),
    };

    let mut file = fs::File::create(&out_path).context("Failed to create output file")?;
    serde_xml_rs::to_writer(&mut file, &root).context("Failed to serialize to XML")?;
    println!("Decoded NTON to {:?}", out_path);
    Ok(())
}

pub fn newton_encode(input: &Path, output: &Option<PathBuf>) -> Result<()> {
    // Encode XML -> NTON
    let content = fs::read_to_string(input).context("Failed to read input file")?;
    let root: MResourceGroup = serde_xml_rs::from_str(&content).context("Failed to parse XML")?;

    let out_path = match output {
        Some(p) => p.clone(),
        None => input.with_extension("nton"),
    };

    let mut file = fs::File::create(&out_path).context("Failed to create output file")?;
    encode_newton(&root, &mut file).context("Failed to encode NTON")?;
    println!("Encoded NTON to {:?}", out_path);
    Ok(())
}
